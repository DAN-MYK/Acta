use std::sync::Arc;

use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ui::helpers::{
    act_row_to_document_item, counterparty_to_details, counterparty_to_item,
    invoice_row_to_document_item, payment_row_to_item,
};
use acta::app_ctx::{AppCtx, AppScreen};
use acta::db;

pub struct CounterpartiesData {
    pub items: Vec<crate::CounterpartyItem>,
}

pub struct CounterpartyDetailData {
    pub detail: crate::CounterpartyDetails,
    pub documents: Vec<crate::DocumentItem>,
    pub payments: Vec<crate::PaymentItem>,
}

pub async fn prepare_counterparties_data(
    pool: &PgPool,
    company_id: Uuid,
    search: Option<&str>,
) -> CounterpartiesData {
    let rows = db::counterparties::list_filtered(pool, company_id, search, false)
        .await
        .unwrap_or_default();
    CounterpartiesData {
        items: rows.iter().map(counterparty_to_item).collect(),
    }
}

pub async fn prepare_counterparty_detail(
    pool: &PgPool,
    company_id: Uuid,
    cp_id: Uuid,
) -> Option<CounterpartyDetailData> {
    let cp = db::counterparties::get_by_id(pool, cp_id).await.ok()??;

    let (acts, invoices, payments) = tokio::join!(
        db::acts::list_filtered(pool, company_id, None, None, None, Some(cp_id), None, None),
        db::invoices::list_filtered(pool, company_id, None, None, None, Some(cp_id), None, None),
        db::payments::list_by_counterparty(pool, company_id, cp_id),
    );

    let mut docs: Vec<(chrono::NaiveDate, crate::DocumentItem)> = vec![];
    if let Ok(rows) = acts {
        for r in &rows {
            docs.push((r.date, act_row_to_document_item(r)));
        }
    }
    if let Ok(rows) = invoices {
        for r in &rows {
            docs.push((r.date, invoice_row_to_document_item(r)));
        }
    }
    docs.sort_by(|a, b| b.0.cmp(&a.0));
    let documents: Vec<crate::DocumentItem> = docs.into_iter().map(|(_, d)| d).collect();

    let pay_items: Vec<crate::PaymentItem> = payments
        .unwrap_or_default()
        .iter()
        .map(payment_row_to_item)
        .collect();

    Some(CounterpartyDetailData {
        detail: counterparty_to_details(&cp),
        documents,
        payments: pay_items,
    })
}

pub fn apply_counterparties_to_ui(ui: &crate::AppWindow, data: CounterpartiesData) {
    let previous = ui.get_counterparties_screen();
    ui.set_counterparties_screen(crate::CounterpartiesViewData {
        items: ModelRc::new(VecModel::from(data.items)),
        detail: previous.detail,
        documents: previous.documents,
        payments: previous.payments,
    });
}

pub fn apply_counterparty_detail_to_ui(ui: &crate::AppWindow, data: CounterpartyDetailData) {
    let previous = ui.get_counterparties_screen();
    ui.set_counterparties_screen(crate::CounterpartiesViewData {
        items: previous.items,
        detail: data.detail,
        documents: ModelRc::new(VecModel::from(data.documents)),
        payments: ModelRc::new(VecModel::from(data.payments)),
    });
}

/// Підписує всі counterparty callbacks на UI компонент.
pub fn wire_counterparty_callbacks(ui: &crate::AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_cp_selected({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id_str = id.to_string();
            tokio::spawn(async move {
                let cid = ctx.company_id();
                if let Ok(cp_id) = Uuid::parse_str(&id_str) {
                    if let Some(data) = prepare_counterparty_detail(ctx.pool(), cid, cp_id).await {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            apply_counterparty_detail_to_ui(&ui, data);
                            ui.set_cp_selected_id(id_str.into());
                        });
                    }
                }
            });
        }
    });

    ui.on_cp_search_changed({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |query| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let q = query.to_string();
            ctx.update_counterparty_state(|state| {
                state.query = q;
            });
            crate::bootstrap::spawn_refresh_screen(
                ui_weak.clone(),
                ctx.clone(),
                AppScreen::Counterparties,
            );
        }
    });

    // Epic 7: Реалізація базових no-op callbacks
    ui.on_cp_new(|| {
        tracing::warn!("TODO: створення контрагента — форма планується у наступному спринті");
    });
    ui.on_cp_create_doc(|id| {
        tracing::warn!(
            "TODO: створення документу для контрагента {} — форма планується у наступному спринті",
            id
        );
    });
    ui.on_cp_tab_changed(|_t| {
        tracing::debug!(
            "cp_tab_changed — вибір вкладки зберігається у локальному стані Slint-компоненту"
        );
    });
}

#[cfg(test)]
mod tests {
    use acta::models::counterparty::Counterparty;
    use chrono::Utc;
    use uuid::Uuid;

    fn sample_cp() -> Counterparty {
        Counterparty {
            id: Uuid::nil(),
            name: "ТОВ Тест".to_string(),
            edrpou: Some("12345678".to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            is_archived: false,
            bas_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn counterparty_maps_to_item() {
        let cp = sample_cp();
        let item = crate::ui::helpers::counterparty_to_item(&cp);
        assert_eq!(item.name.as_str(), "ТОВ Тест");
        assert_eq!(item.edrpou.as_str(), "12345678");
    }

    #[test]
    fn counterparty_maps_to_details() {
        let cp = sample_cp();
        let details = crate::ui::helpers::counterparty_to_details(&cp);
        assert_eq!(details.id.as_str(), Uuid::nil().to_string());
        assert_eq!(details.edrpou.as_str(), "12345678");
    }
}
