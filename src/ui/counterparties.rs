use std::sync::Arc;

use anyhow::{anyhow, Result};
use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ui::helpers::{
    act_row_to_document_item, counterparty_to_details, counterparty_to_item,
    invoice_row_to_document_item, payment_row_to_item,
};
use acta::app_ctx::{AppCtx, AppScreen};
use acta::db;
use acta::models::counterparty::{is_valid_edrpou, is_valid_iban, is_valid_ipn};
use acta::models::{NewCounterparty, UpdateCounterparty};

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

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn new_counterparty_form() -> crate::CounterpartyDraftForm {
    crate::CounterpartyDraftForm {
        id: "".into(),
        title: "Новий контрагент".into(),
        name: "".into(),
        edrpou: "".into(),
        ipn: "".into(),
        iban: "".into(),
        address: "".into(),
        phone: "".into(),
        email: "".into(),
        notes: "".into(),
    }
}

fn edit_counterparty_form(
    counterparty: &acta::models::counterparty::Counterparty,
) -> crate::CounterpartyDraftForm {
    crate::CounterpartyDraftForm {
        id: counterparty.id.to_string().into(),
        title: "Редагування контрагента".into(),
        name: counterparty.name.clone().into(),
        edrpou: counterparty.edrpou.clone().unwrap_or_default().into(),
        ipn: counterparty.ipn.clone().unwrap_or_default().into(),
        iban: counterparty.iban.clone().unwrap_or_default().into(),
        address: counterparty.address.clone().unwrap_or_default().into(),
        phone: counterparty.phone.clone().unwrap_or_default().into(),
        email: counterparty.email.clone().unwrap_or_default().into(),
        notes: counterparty.notes.clone().unwrap_or_default().into(),
    }
}

fn set_counterparty_form_state(
    ui: &crate::AppWindow,
    form: crate::CounterpartyDraftForm,
    show_editor: bool,
) {
    ui.set_cp_draft_form(form);
    ui.set_show_cp_editor(show_editor);
}

fn validate_counterparty_form(form: &crate::CounterpartyDraftForm) -> Result<NewCounterparty> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(anyhow!("Назва контрагента є обов'язковою"));
    }

    let edrpou = optional_string(form.edrpou.as_str());
    if let Some(value) = edrpou.as_deref() {
        if !is_valid_edrpou(value) {
            return Err(anyhow!("ЄДРПОУ має містити рівно 8 цифр"));
        }
    }

    let ipn = optional_string(form.ipn.as_str());
    if let Some(value) = ipn.as_deref() {
        if !is_valid_ipn(value) {
            return Err(anyhow!("ІПН має містити рівно 10 цифр"));
        }
    }

    let iban = optional_string(form.iban.as_str());
    if let Some(value) = iban.as_deref() {
        if !is_valid_iban(value) {
            return Err(anyhow!("IBAN має починатися з UA і містити 29 символів"));
        }
    }

    Ok(NewCounterparty {
        name: name.to_string(),
        edrpou,
        ipn,
        iban,
        address: optional_string(form.address.as_str()),
        phone: optional_string(form.phone.as_str()),
        email: optional_string(form.email.as_str()),
        notes: optional_string(form.notes.as_str()),
        bas_id: None,
    })
}

fn update_payload_from_form(form: &crate::CounterpartyDraftForm) -> Result<UpdateCounterparty> {
    let create_payload = validate_counterparty_form(form)?;
    Ok(UpdateCounterparty {
        name: create_payload.name,
        edrpou: create_payload.edrpou,
        ipn: create_payload.ipn,
        iban: create_payload.iban,
        address: create_payload.address,
        phone: create_payload.phone,
        email: create_payload.email,
        notes: create_payload.notes,
    })
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

    ui.on_cp_new({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                set_counterparty_form_state(&ui, new_counterparty_form(), true);
            });
        }
    });

    ui.on_cp_edit({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id_str = id.to_string();
            tokio::spawn(async move {
                let Ok(counterparty_id) = Uuid::parse_str(&id_str) else {
                    tracing::warn!("cp_edit: некоректний id контрагента: {id_str}");
                    return;
                };

                match db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id)
                    .await
                {
                    Ok(Some(counterparty)) => {
                        let form = edit_counterparty_form(&counterparty);
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            set_counterparty_form_state(&ui, form, true);
                        });
                    }
                    Ok(None) => {
                        tracing::warn!("cp_edit: контрагента не знайдено: {counterparty_id}");
                    }
                    Err(error) => {
                        tracing::error!("cp_edit: не вдалося завантажити контрагента: {error}");
                    }
                }
            });
        }
    });

    ui.on_cp_draft_saved({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |form| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                let maybe_id = optional_string(form.id.as_str())
                    .and_then(|value| Uuid::parse_str(&value).ok());

                let save_result = if let Some(counterparty_id) = maybe_id {
                    match update_payload_from_form(&form) {
                        Ok(payload) => {
                            db::counterparties::update(ctx.pool(), counterparty_id, &payload)
                                .await
                                .map(|row| row.map(|counterparty| counterparty.id))
                        }
                        Err(error) => Err(error),
                    }
                } else {
                    match validate_counterparty_form(&form) {
                        Ok(payload) => {
                            db::counterparties::create(ctx.pool(), ctx.company_id(), &payload)
                                .await
                                .map(|counterparty| Some(counterparty.id))
                        }
                        Err(error) => Err(error),
                    }
                };

                let counterparty_id = match save_result {
                    Ok(Some(counterparty_id)) => counterparty_id,
                    Ok(None) => {
                        tracing::warn!("cp_draft_saved: контрагента для оновлення не знайдено");
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_show_cp_editor(true);
                        });
                        return;
                    }
                    Err(error) => {
                        tracing::warn!("cp_draft_saved: {error}");
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_show_cp_editor(true);
                        });
                        return;
                    }
                };

                let list_data =
                    prepare_counterparties_data(ctx.pool(), ctx.company_id(), None).await;
                let detail_data =
                    prepare_counterparty_detail(ctx.pool(), ctx.company_id(), counterparty_id)
                        .await;
                let selected_id = counterparty_id.to_string();

                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    apply_counterparties_to_ui(&ui, list_data);
                    if let Some(detail) = detail_data {
                        apply_counterparty_detail_to_ui(&ui, detail);
                    }
                    ui.set_cp_selected_id(selected_id.into());
                    ui.set_show_cp_editor(false);
                });

                tracing::info!("counterparties: saved counterparty {}", counterparty_id);
            });
        }
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

                match db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id)
                    .await
                {
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
                        tracing::error!(
                            "cp_create_doc: не вдалося завантажити контрагента: {error}"
                        );
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
