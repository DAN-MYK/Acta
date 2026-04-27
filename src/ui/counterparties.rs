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

pub fn empty_counterparty_detail() -> CounterpartyDetailData {
    CounterpartyDetailData {
        detail: crate::CounterpartyDetails {
            id: "".into(),
            name: "".into(),
            kind: "".into(),
            edrpou: "".into(),
            ipn: "".into(),
            vat: "".into(),
            iban: "".into(),
            bank: "".into(),
            address: "".into(),
            director: "".into(),
            phone: "".into(),
            email: "".into(),
            client_since: "".into(),
            balance_str: "".into(),
            balance_is_negative: false,
            doc_count: 0,
            overdue_count: 0,
            overdue_amount_str: "".into(),
            last_contact_days: 0,
            last_contact_date: "".into(),
        },
        documents: Vec::new(),
        payments: Vec::new(),
    }
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
    let cp = db::counterparties::get_by_id(pool, company_id, cp_id)
        .await
        .ok()??;

    let (acts, invoices, payments) = tokio::join!(
        db::acts::list_filtered(pool, company_id, None, None, None, Some(cp_id), None, None),
        db::invoices::list_filtered(pool, company_id, None, None, None, Some(cp_id), None, None),
        db::payments::list_by_counterparty(pool, company_id, cp_id),
    );

    let mut docs: Vec<(chrono::NaiveDate, crate::DocumentItem)> = vec![];
    if let Ok(rows) = acts {
        for row in &rows {
            docs.push((row.date, act_row_to_document_item(row)));
        }
    }
    if let Ok(rows) = invoices {
        for row in &rows {
            docs.push((row.date, invoice_row_to_document_item(row)));
        }
    }
    docs.sort_by(|a, b| b.0.cmp(&a.0));
    let documents: Vec<crate::DocumentItem> = docs.into_iter().map(|(_, doc)| doc).collect();

    let payments: Vec<crate::PaymentItem> = payments
        .unwrap_or_default()
        .iter()
        .map(payment_row_to_item)
        .collect();

    Some(CounterpartyDetailData {
        detail: counterparty_to_details(&cp),
        documents,
        payments,
    })
}

pub fn apply_counterparties_to_ui(ui: &crate::AppWindow, data: CounterpartiesData) {
    ui.set_counterparties_screen(crate::CounterpartiesViewData {
        items: ModelRc::new(VecModel::from(data.items)),
    });
}

pub fn apply_counterparty_detail_to_ui(ui: &crate::AppWindow, data: CounterpartyDetailData) {
    ui.set_counterparty_detail(crate::CounterpartyDetailViewData {
        info: data.detail,
        documents: ModelRc::new(VecModel::from(data.documents)),
        payments: ModelRc::new(VecModel::from(data.payments)),
    });
}

pub fn wire_counterparty_callbacks(ui: &crate::AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_cp_selected({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id_str = id.to_string();
            tokio::spawn(async move {
                let company_id = ctx.company_id();
                let refresh_epoch = ctx.refresh_epoch();
                if let Ok(cp_id) = Uuid::parse_str(&id_str) {
                    if let Some(data) = prepare_counterparty_detail(ctx.pool(), company_id, cp_id).await {
                        if ctx.company_id() != company_id || ctx.refresh_epoch() != refresh_epoch {
                            tracing::debug!(
                                "counterparties: пропускаємо detail контрагента після switch компанії"
                            );
                            return;
                        }
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

    ui.on_cp_new(|| {
        tracing::warn!("TODO: створення контрагента ще не реалізоване");
    });

    ui.on_cp_create_doc({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id_str = id.to_string();
            tokio::spawn(async move {
                let Ok(counterparty_id) = Uuid::parse_str(&id_str) else {
                    tracing::warn!("cp_create_doc: некоректний id контрагента: {id_str}");
                    return;
                };

                match db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id).await {
                    Ok(Some(counterparty)) => {
                        ctx.set_active_screen(AppScreen::Documents);
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_current_screen(crate::NavScreen::Documents);
                            ui.set_cp_selected_id(counterparty_id.to_string().into());
                            ui.set_doc_draft_form(crate::DocumentDraftForm {
                                id: "".into(),
                                kind: "".into(),
                                counterparty_id: counterparty_id.to_string().into(),
                                counterparty_name: counterparty.name.into(),
                                title: "".into(),
                                number: "".into(),
                                date: "".into(),
                                notes: "".into(),
                            });
                            ui.set_show_doc_type_picker(true);
                            ui.set_show_doc_editor(false);
                        });
                    }
                    Ok(None) => {
                        tracing::warn!("cp_create_doc: контрагента не знайдено: {counterparty_id}");
                    }
                    Err(error) => {
                        tracing::error!("cp_create_doc: не вдалося завантажити контрагента: {error}");
                    }
                }
            });
        }
    });

    ui.on_cp_tab_changed(|_tab| {
        tracing::debug!(
            "cp_tab_changed: вибір вкладки зберігається у локальному стані Slint-компоненту"
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

    #[test]
    fn empty_counterparty_detail_resets_all_fields() {
        let details = super::empty_counterparty_detail();
        assert_eq!(details.detail.id.as_str(), "");
        assert_eq!(details.detail.name.as_str(), "");
        assert!(details.documents.is_empty());
        assert!(details.payments.is_empty());
        assert_eq!(details.detail.doc_count, 0);
        assert_eq!(details.detail.overdue_count, 0);
    }
}
