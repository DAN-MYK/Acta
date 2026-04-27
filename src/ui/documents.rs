use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDate, Utc};
use notify_rust::{Notification, Timeout};
use rust_decimal::Decimal;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ui::helpers::{
    act_row_to_document_item, date_to_str, invoice_row_to_document_item, waybill_row_to_document_item,
    OperationResult,
};
use acta::app_ctx::{AppCtx, AppScreen};
use acta::db;
use acta::models::act::{ActItem, ActStatus, NewAct, NewActItem, UpdateAct};
use acta::models::invoice::{InvoiceItem, NewInvoice, NewInvoiceItem, UpdateInvoice};
use acta::models::waybill::{NewWaybill, NewWaybillItem, UpdateWaybill, WaybillItem};
use acta::models::DocumentDirection;

pub struct DocumentsData {
    pub items: Vec<crate::DocumentItem>,
    pub total: i32,
}

#[derive(Clone, Copy)]
enum DocumentRef {
    Act(Uuid),
    Invoice(Uuid),
    Waybill(Uuid),
}

pub async fn prepare_documents_data(
    pool: &PgPool,
    company_id: Uuid,
    search: Option<&str>,
    _tab: Option<&str>,
) -> DocumentsData {
    // Завжди завантажуємо всі типи: фільтрація по вкладці робиться у Slint.
    let (acts, invoices, waybills) = tokio::join!(
        db::acts::list_filtered(pool, company_id, None, None, search, None, None, None),
        db::invoices::list_filtered(pool, company_id, None, None, search, None, None, None),
        db::waybills::list_filtered(pool, company_id, None, None, search, None, None, None),
    );

    let mut combined: Vec<(chrono::NaiveDate, crate::DocumentItem)> = vec![];

    if let Ok(rows) = acts {
        for row in &rows {
            combined.push((row.date, act_row_to_document_item(row)));
        }
    }
    if let Ok(rows) = invoices {
        for row in &rows {
            combined.push((row.date, invoice_row_to_document_item(row)));
        }
    }
    if let Ok(rows) = waybills {
        for row in &rows {
            combined.push((row.date, waybill_row_to_document_item(row)));
        }
    }

    combined.sort_by(|a, b| b.0.cmp(&a.0));
    let items = combined.into_iter().map(|(_, item)| item).collect::<Vec<_>>();
    let total = items.len() as i32;

    DocumentsData { items, total }
}

pub fn apply_documents_to_ui(ui: &crate::AppWindow, data: DocumentsData) {
    let mut invoice_items = vec![];
    let mut act_items = vec![];
    let mut waybill_items = vec![];

    for item in &data.items {
        if item.id.starts_with("inv:") {
            invoice_items.push(item.clone());
        } else if item.id.starts_with("act:") {
            act_items.push(item.clone());
        } else if item.id.starts_with("wbl:") {
            waybill_items.push(item.clone());
        }
    }

    ui.set_documents(crate::DocumentsViewData {
        items: ModelRc::new(VecModel::from(data.items)),
        invoice_items: ModelRc::new(VecModel::from(invoice_items)),
        act_items: ModelRc::new(VecModel::from(act_items)),
        waybill_items: ModelRc::new(VecModel::from(waybill_items)),
        selected_ids: ModelRc::new(VecModel::<slint::SharedString>::default()),
        total_count: data.total,
        page_count: 1,
        chain_steps: ModelRc::new(VecModel::<crate::ChainStep>::default()),
        cp_doc_chains: ModelRc::new(VecModel::<crate::DocChainGroup>::default()),
    });
}

fn notify_user(summary: &str, body: &str) {
    let _ = Notification::new()
        .appname("Acta")
        .summary(summary)
        .body(body)
        .timeout(Timeout::Milliseconds(6_000))
        .show();
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_document_ref(id: &str) -> Option<DocumentRef> {
    if let Some(uuid) = id.strip_prefix("act:").and_then(|v| Uuid::parse_str(v).ok()) {
        return Some(DocumentRef::Act(uuid));
    }
    if let Some(uuid) = id.strip_prefix("inv:").and_then(|v| Uuid::parse_str(v).ok()) {
        return Some(DocumentRef::Invoice(uuid));
    }
    id.strip_prefix("wbl:")
        .and_then(|v| Uuid::parse_str(v).ok())
        .map(DocumentRef::Waybill)
}

fn kind_title(kind: &str, is_new: bool) -> &'static str {
    match (kind, is_new) {
        ("act", true) => "Новий акт",
        ("invoice", true) => "Новий рахунок",
        ("waybill", true) => "Нова накладна",
        ("act", false) => "Редагування акта",
        ("invoice", false) => "Редагування рахунку",
        ("waybill", false) => "Редагування накладної",
        _ => "Документ",
    }
}

fn parse_ui_date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%d.%m.%Y")
        .with_context(|| format!("Некоректна дата: {value}. Очікується формат дд.мм.рррр"))
}

fn set_document_state(
    ui: &crate::AppWindow,
    form: crate::DocumentDraftForm,
    items: Vec<crate::DocumentDraftItem>,
    show_type_picker: bool,
    show_editor: bool,
) {
    ui.set_doc_draft_form(form);
    ui.set_doc_draft_items(ModelRc::new(VecModel::from(items)));
    ui.set_show_doc_type_picker(show_type_picker);
    ui.set_show_doc_editor(show_editor);
}

fn parse_decimal_input(value: &str, field: &str) -> Result<Decimal> {
    let normalized = value.trim().replace(' ', "").replace(',', ".");
    Decimal::from_str_exact(&normalized)
        .with_context(|| format!("Некоректне число в полі \"{field}\": {value}"))
}

fn act_items_to_draft(items: Vec<ActItem>) -> Vec<crate::DocumentDraftItem> {
    items
        .into_iter()
        .map(|item| crate::DocumentDraftItem {
            description: item.description.into(),
            unit: item.unit.into(),
            quantity: item.quantity.to_string().into(),
            price: item.unit_price.to_string().into(),
        })
        .collect()
}

fn invoice_items_to_draft(items: Vec<InvoiceItem>) -> Vec<crate::DocumentDraftItem> {
    items
        .into_iter()
        .map(|item| crate::DocumentDraftItem {
            description: item.description.into(),
            unit: item.unit.unwrap_or_default().into(),
            quantity: item.quantity.to_string().into(),
            price: item.price.to_string().into(),
        })
        .collect()
}

fn waybill_items_to_draft(items: Vec<WaybillItem>) -> Vec<crate::DocumentDraftItem> {
    items
        .into_iter()
        .map(|item| crate::DocumentDraftItem {
            description: item.description.into(),
            unit: item.unit.unwrap_or_default().into(),
            quantity: item.quantity.to_string().into(),
            price: item.price.to_string().into(),
        })
        .collect()
}

fn draft_items_to_new_act(items: Vec<crate::DocumentDraftItem>) -> Result<Vec<NewActItem>> {
    items
        .into_iter()
        .map(|item| {
            Ok(NewActItem {
                description: item.description.to_string(),
                quantity: parse_decimal_input(item.quantity.as_str(), "Кількість")?,
                unit: item.unit.to_string(),
                unit_price: parse_decimal_input(item.price.as_str(), "Ціна")?,
            })
        })
        .collect()
}

fn draft_items_to_new_invoice(items: Vec<crate::DocumentDraftItem>) -> Result<Vec<NewInvoiceItem>> {
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            Ok(NewInvoiceItem {
                position: (index + 1) as i16,
                description: item.description.to_string(),
                unit: optional_string(item.unit.as_str()),
                quantity: parse_decimal_input(item.quantity.as_str(), "Кількість")?,
                price: parse_decimal_input(item.price.as_str(), "Ціна")?,
            })
        })
        .collect()
}

fn draft_items_to_new_waybill(items: Vec<crate::DocumentDraftItem>) -> Result<Vec<NewWaybillItem>> {
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            Ok(NewWaybillItem {
                position: (index + 1) as i16,
                description: item.description.to_string(),
                unit: optional_string(item.unit.as_str()),
                quantity: parse_decimal_input(item.quantity.as_str(), "Кількість")?,
                price: parse_decimal_input(item.price.as_str(), "Ціна")?,
            })
        })
        .collect()
}

async fn load_counterparty_name(pool: &PgPool, company_id: Uuid, counterparty_id: Uuid) -> Result<String> {
    let counterparty = db::counterparties::get_by_id(pool, company_id, counterparty_id)
        .await?
        .ok_or_else(|| anyhow!("Контрагента не знайдено"))?;
    Ok(counterparty.name)
}

async fn build_existing_document_form(
    pool: &PgPool,
    company_id: Uuid,
    doc_ref: DocumentRef,
) -> Result<(crate::DocumentDraftForm, Vec<crate::DocumentDraftItem>)> {
    match doc_ref {
        DocumentRef::Act(id) => {
            let (act, items) = db::acts::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Акт не знайдено"))?;
            let counterparty_name = load_counterparty_name(pool, company_id, act.counterparty_id).await?;
            Ok((
                crate::DocumentDraftForm {
                    id: format!("act:{id}").into(),
                    kind: "act".into(),
                    counterparty_id: act.counterparty_id.to_string().into(),
                    counterparty_name: counterparty_name.into(),
                    title: kind_title("act", false).into(),
                    number: act.number.into(),
                    date: date_to_str(act.date),
                    notes: act.notes.unwrap_or_default().into(),
                },
                act_items_to_draft(items),
            ))
        }
        DocumentRef::Invoice(id) => {
            let (invoice, items) = db::invoices::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Рахунок не знайдено"))?;
            let counterparty_name =
                load_counterparty_name(pool, company_id, invoice.counterparty_id).await?;
            Ok((
                crate::DocumentDraftForm {
                    id: format!("inv:{id}").into(),
                    kind: "invoice".into(),
                    counterparty_id: invoice.counterparty_id.to_string().into(),
                    counterparty_name: counterparty_name.into(),
                    title: kind_title("invoice", false).into(),
                    number: invoice.number.into(),
                    date: date_to_str(invoice.date),
                    notes: invoice.notes.unwrap_or_default().into(),
                },
                invoice_items_to_draft(items),
            ))
        }
        DocumentRef::Waybill(id) => {
            let (waybill, items) = db::waybills::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Накладну не знайдено"))?;
            let counterparty_name =
                load_counterparty_name(pool, company_id, waybill.counterparty_id).await?;
            Ok((
                crate::DocumentDraftForm {
                    id: format!("wbl:{id}").into(),
                    kind: "waybill".into(),
                    counterparty_id: waybill.counterparty_id.to_string().into(),
                    counterparty_name: counterparty_name.into(),
                    title: kind_title("waybill", false).into(),
                    number: waybill.number.into(),
                    date: date_to_str(waybill.date),
                    notes: waybill.notes.unwrap_or_default().into(),
                },
                waybill_items_to_draft(items),
            ))
        }
    }
}

async fn create_draft_form(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    counterparty_name: String,
    kind: &str,
) -> Result<crate::DocumentDraftForm> {
    let today = Utc::now().date_naive();

    match kind {
        "act" => {
            let number = db::acts::generate_next_number(pool, company_id).await?;
            let act = db::acts::create(
                pool,
                company_id,
                &NewAct {
                    number: number.clone(),
                    counterparty_id,
                    contract_id: None,
                    category_id: None,
                    direction: DocumentDirection::Outgoing,
                    date: today,
                    expected_payment_date: None,
                    status: ActStatus::Draft,
                    notes: None,
                    bas_id: None,
                    items: vec![],
                },
            )
            .await?;

            Ok(crate::DocumentDraftForm {
                id: format!("act:{}", act.id).into(),
                kind: "act".into(),
                counterparty_id: counterparty_id.to_string().into(),
                counterparty_name: counterparty_name.into(),
                title: kind_title("act", true).into(),
                number: number.into(),
                date: date_to_str(today),
                notes: "".into(),
            })
        }
        "invoice" => {
            let number = db::invoices::generate_next_number(pool, company_id).await?;
            let invoice = db::invoices::create(
                pool,
                company_id,
                &NewInvoice {
                    number: number.clone(),
                    counterparty_id,
                    contract_id: None,
                    category_id: None,
                    direction: DocumentDirection::Outgoing,
                    date: today,
                    expected_payment_date: None,
                    notes: None,
                    bas_id: None,
                    items: vec![],
                },
            )
            .await?;

            Ok(crate::DocumentDraftForm {
                id: format!("inv:{}", invoice.id).into(),
                kind: "invoice".into(),
                counterparty_id: counterparty_id.to_string().into(),
                counterparty_name: counterparty_name.into(),
                title: kind_title("invoice", true).into(),
                number: number.into(),
                date: date_to_str(today),
                notes: "".into(),
            })
        }
        "waybill" => {
            let number = db::waybills::generate_next_number(pool, company_id).await?;
            let waybill = db::waybills::create(
                pool,
                company_id,
                &NewWaybill {
                    number: number.clone(),
                    counterparty_id,
                    contract_id: None,
                    category_id: None,
                    direction: DocumentDirection::Outgoing,
                    date: today,
                    notes: None,
                    bas_id: None,
                    items: vec![],
                },
            )
            .await?;

            Ok(crate::DocumentDraftForm {
                id: format!("wbl:{}", waybill.id).into(),
                kind: "waybill".into(),
                counterparty_id: counterparty_id.to_string().into(),
                counterparty_name: counterparty_name.into(),
                title: kind_title("waybill", true).into(),
                number: number.into(),
                date: date_to_str(today),
                notes: "".into(),
            })
        }
        _ => Err(anyhow!("Невідомий тип документа: {kind}")),
    }
}

async fn save_document_form(
    pool: &PgPool,
    form: crate::DocumentDraftForm,
    items: Vec<crate::DocumentDraftItem>,
) -> Result<()> {
    let doc_ref = parse_document_ref(form.id.as_str())
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документа"))?;
    let date = parse_ui_date(form.date.as_str())?;
    let notes = optional_string(form.notes.as_str());

    if items.is_empty() {
        return Err(anyhow!("Документ має містити хоча б одну позицію"));
    }

    match doc_ref {
        DocumentRef::Act(id) => {
            let (act, _) = db::acts::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Акт не знайдено"))?;
            db::acts::update_with_items(
                pool,
                id,
                UpdateAct {
                    number: form.number.to_string(),
                    counterparty_id: act.counterparty_id,
                    contract_id: act.contract_id,
                    category_id: act.category_id,
                    date,
                    expected_payment_date: act.expected_payment_date,
                    notes,
                },
                draft_items_to_new_act(items)?,
            )
            .await?;
        }
        DocumentRef::Invoice(id) => {
            let (invoice, _) = db::invoices::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Рахунок не знайдено"))?;
            db::invoices::update_with_items(
                pool,
                id,
                UpdateInvoice {
                    number: form.number.to_string(),
                    counterparty_id: invoice.counterparty_id,
                    contract_id: invoice.contract_id,
                    category_id: invoice.category_id,
                    date,
                    expected_payment_date: invoice.expected_payment_date,
                    notes,
                },
                draft_items_to_new_invoice(items)?,
            )
            .await?;
        }
        DocumentRef::Waybill(id) => {
            let (waybill, _) = db::waybills::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Накладну не знайдено"))?;
            db::waybills::update_with_items(
                pool,
                id,
                UpdateWaybill {
                    number: form.number.to_string(),
                    counterparty_id: waybill.counterparty_id,
                    contract_id: waybill.contract_id,
                    category_id: waybill.category_id,
                    date,
                    notes,
                },
                draft_items_to_new_waybill(items)?,
            )
            .await?;
        }
    }

    Ok(())
}

pub fn wire_document_callbacks(ui: &crate::AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_doc_search_changed({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |query| {
            let q = query.to_string();
            ctx.update_documents_state(|state| {
                state.query = q;
            });
            crate::bootstrap::spawn_refresh_screen(
                ui_weak.clone(),
                ctx.clone(),
                AppScreen::Documents,
            );
        }
    });

    ui.on_doc_tab_changed({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |tab| {
            let t = tab.to_string();
            ctx.update_documents_state(|state| {
                state.tab = t;
            });
            crate::bootstrap::spawn_refresh_screen(
                ui_weak.clone(),
                ctx.clone(),
                AppScreen::Documents,
            );
        }
    });

    ui.on_doc_send({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                let Some(doc_ref) = parse_document_ref(&id) else {
                    notify_user("Помилка надсилання", "Некоректний ідентифікатор документа.");
                    return;
                };

                let result = match doc_ref {
                    DocumentRef::Act(uuid) => {
                        db::acts::advance_status(ctx.pool(), uuid).await.map(|_| ())
                    }
                    DocumentRef::Invoice(uuid) => {
                        db::invoices::advance_status(ctx.pool(), uuid).await.map(|_| ())
                    }
                    DocumentRef::Waybill(uuid) => {
                        db::waybills::advance_status(ctx.pool(), uuid).await.map(|_| ())
                    }
                };

                match result {
                    Ok(_) => {
                        tracing::info!("documents: sent successfully: {id}");
                        notify_user("Успішно", "Документ надіслано.");
                        crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
                    }
                    Err(error) => {
                        tracing::error!("documents: send failed for {id}: {error}");
                        notify_user("Помилка надсилання", &format!("{}", error));
                    }
                }
            });
        }
    });

    ui.on_doc_delete({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                let Some(doc_ref) = parse_document_ref(&id) else {
                    notify_user("Помилка видалення", "Некоректний ідентифікатор документа.");
                    return;
                };

                let result = match doc_ref {
                    DocumentRef::Act(uuid) => {
                        db::acts::delete(ctx.pool(), uuid).await
                    }
                    DocumentRef::Invoice(uuid) => {
                        db::invoices::delete(ctx.pool(), uuid).await
                    }
                    DocumentRef::Waybill(uuid) => {
                        db::waybills::delete(ctx.pool(), uuid).await
                    }
                };

                match result {
                    Ok(_) => {
                        tracing::info!("documents: deleted successfully: {id}");
                        notify_user("Успішно", "Документ видалено.");
                        crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
                    }
                    Err(error) => {
                        tracing::error!("documents: delete failed for {id}: {error}");
                        notify_user("Помилка видалення", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_doc_new({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move || {
            let selected_counterparty = ui_weak
                .upgrade()
                .map(|ui| ui.get_cp_selected_id().to_string())
                .unwrap_or_default();

            if selected_counterparty.is_empty() {
                ctx.set_active_screen(AppScreen::Counterparties);
                let _ = ui_weak.upgrade_in_event_loop(|ui| {
                    ui.set_current_screen(crate::NavScreen::Counterparties);
                });
                notify_user(
                    "Оберіть контрагента",
                    "Щоб створити документ, спочатку відкрийте картку контрагента.",
                );
                return;
            }

            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                let Ok(counterparty_id) = Uuid::parse_str(&selected_counterparty) else {
                    notify_user("Помилка створення", "Некоректний ідентифікатор контрагента.");
                    return;
                };

                match load_counterparty_name(ctx.pool(), ctx.company_id(), counterparty_id).await {
                    Ok(counterparty_name) => {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            set_document_state(
                                &ui,
                                crate::DocumentDraftForm {
                                    id: "".into(),
                                    kind: "".into(),
                                    counterparty_id: counterparty_id.to_string().into(),
                                    counterparty_name: counterparty_name.into(),
                                    title: "".into(),
                                    number: "".into(),
                                    date: "".into(),
                                    notes: "".into(),
                                },
                                vec![],
                                true,
                                false,
                            );
                        });
                    }
                    Err(error) => {
                        tracing::error!("documents: new context failed: {error}");
                        notify_user("Помилка створення", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_doc_open({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                let Some(doc_ref) = parse_document_ref(&id) else {
                    notify_user("Помилка відкриття", "Некоректний ідентифікатор документа.");
                    return;
                };

                match build_existing_document_form(ctx.pool(), ctx.company_id(), doc_ref).await {
                    Ok((form, items)) => {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            set_document_state(&ui, form, items, false, true);
                        });
                    }
                    Err(error) => {
                        tracing::error!("documents: open failed: {error}");
                        notify_user("Помилка відкриття документа", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_doc_edit({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                let Some(doc_ref) = parse_document_ref(&id) else {
                    notify_user("Помилка редагування", "Некоректний ідентифікатор документа.");
                    return;
                };

                match build_existing_document_form(ctx.pool(), ctx.company_id(), doc_ref).await {
                    Ok((form, items)) => {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            set_document_state(&ui, form, items, false, true);
                        });
                    }
                    Err(error) => {
                        tracing::error!("documents: edit failed: {error}");
                        notify_user("Помилка редагування документа", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_doc_toggled(|_id, _sel| {
        tracing::debug!(
            "doc_toggled — вибір рядка зберігається у локальному стані Slint-компоненту"
        );
    });
    ui.on_doc_selection_cleared(|| {
        tracing::debug!("doc_selection_cleared — очищення вибору в UI");
    });
    ui.on_doc_more_actions(|id| {
        let id = id.to_string();
        tracing::debug!("doc_more_actions({id}) — menu visibility handled in Slint");
    });
    ui.on_doc_bulk_send({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move || {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let selected_ids = ui_weak
                .upgrade()
                .map(|ui| {
                    ui.get_documents()
                        .selected_ids
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();

            tokio::spawn(async move {
                let total = selected_ids.len();
                let mut result = OperationResult::new(total);

                for id in selected_ids {
                    let Some(doc_ref) = parse_document_ref(&id) else {
                        result.add_error(format!("Некоректний ID: {}", id));
                        continue;
                    };

                    let op_result = match doc_ref {
                        DocumentRef::Act(uuid) => {
                            db::acts::advance_status(ctx.pool(), uuid).await.map(|_| ())
                        }
                        DocumentRef::Invoice(uuid) => {
                            db::invoices::advance_status(ctx.pool(), uuid).await.map(|_| ())
                        }
                        DocumentRef::Waybill(uuid) => {
                            db::waybills::advance_status(ctx.pool(), uuid).await.map(|_| ())
                        }
                    };

                    match op_result {
                        Ok(_) => {
                            result.add_success();
                        }
                        Err(error) => {
                            result.add_error(format!("{}: {}", id, error));
                        }
                    }
                }

                tracing::info!(
                    "documents: bulk send completed: {}/{} succeeded{}",
                    result.succeeded,
                    result.total,
                    if !result.errors.is_empty() {
                        format!(" — {}", result.error_log())
                    } else {
                        String::new()
                    }
                );

                notify_user(
                    if result.all_succeeded() {
                        "Успішно"
                    } else {
                        "Частково"
                    },
                    &result.user_message(),
                );

                if result.has_successes() {
                    crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
                }
            });
        }
    });
    ui.on_doc_bulk_archive({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move || {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let selected_ids: Vec<String> = {
                if let Some(ui) = ui_weak.upgrade() {
                    use slint::Model;
                    let documents = ui.get_documents();
                    let model = documents.selected_ids;
                    (0..model.row_count())
                        .filter_map(|i| model.row_data(i))
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    return;
                }
            };

            tokio::spawn(async move {
                let total = selected_ids.len();
                let mut result = OperationResult::new(total);

                for id in selected_ids {
                    let Some(doc_ref) = parse_document_ref(&id) else {
                        result.add_error(format!("Некоректний ID: {}", id));
                        continue;
                    };

                    let op_result = match doc_ref {
                        DocumentRef::Act(uuid) => {
                            db::acts::advance_status(ctx.pool(), uuid).await.map(|_| ())
                        }
                        DocumentRef::Invoice(uuid) => {
                            db::invoices::advance_status(ctx.pool(), uuid).await.map(|_| ())
                        }
                        DocumentRef::Waybill(uuid) => {
                            db::waybills::advance_status(ctx.pool(), uuid).await.map(|_| ())
                        }
                    };

                    match op_result {
                        Ok(_) => {
                            result.add_success();
                        }
                        Err(error) => {
                            result.add_error(format!("{}: {}", id, error));
                        }
                    }
                }

                tracing::info!(
                    "documents: bulk archive completed: {}/{} succeeded{}",
                    result.succeeded,
                    result.total,
                    if !result.errors.is_empty() {
                        format!(" — {}", result.error_log())
                    } else {
                        String::new()
                    }
                );

                notify_user(
                    if result.all_succeeded() {
                        "Успішно"
                    } else {
                        "Частково"
                    },
                    &result.user_message(),
                );

                if result.has_successes() {
                    let _ = crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
                }
            });
        }
    });
    ui.on_doc_bulk_delete({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move || {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let selected_ids = ui_weak
                .upgrade()
                .map(|ui| {
                    ui.get_documents().selected_ids
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();

            if selected_ids.is_empty() {
                notify_user("Вибір порожній", "Виберіть документи для видалення.");
                return;
            }

            tokio::spawn(async move {
                for id_str in selected_ids {
                    if let Some(doc_ref) = parse_document_ref(&id_str) {
                        match doc_ref {
                            DocumentRef::Act(id) => {
                                let _ = db::acts::delete(ctx.pool(), id).await;
                            }
                            DocumentRef::Invoice(id) => {
                                let _ = db::invoices::delete(ctx.pool(), id).await;
                            }
                            DocumentRef::Waybill(id) => {
                                let _ = db::waybills::delete(ctx.pool(), id).await;
                            }
                        }
                    }
                }
                crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
                notify_user("Операція завершена", "Документи видалено.");
            });
        }
    });
    ui.on_doc_page_changed(|_p| {
        tracing::debug!(
            "doc_page_changed — пагінація зберігається у локальному стані Slint-компоненту"
        );
    });

    ui.on_doc_create_kind_selected({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |kind| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let kind = kind.to_string();

            let Some((counterparty_id, counterparty_name)) = ui_weak.upgrade().map(|ui| {
                let form = ui.get_doc_draft_form();
                (
                    form.counterparty_id.to_string(),
                    form.counterparty_name.to_string(),
                )
            }) else {
                return;
            };

            tokio::spawn(async move {
                let Ok(counterparty_id) = Uuid::parse_str(&counterparty_id) else {
                    notify_user("Помилка створення", "Спочатку оберіть контрагента.");
                    return;
                };

                match create_draft_form(
                    ctx.pool(),
                    ctx.company_id(),
                    counterparty_id,
                    counterparty_name,
                    &kind,
                )
                .await
                {
                    Ok(form) => {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            set_document_state(&ui, form, vec![], false, true);
                        });
                        crate::bootstrap::spawn_refresh_screen(
                            ui_weak,
                            ctx,
                            AppScreen::Documents,
                        );
                    }
                    Err(error) => {
                        tracing::error!("documents: create draft failed: {error}");
                        notify_user("Помилка створення документа", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_doc_draft_saved({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |form| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let draft_items = ui_weak
                .upgrade()
                .map(|ui| {
                    ui.get_doc_draft_items()
                        .iter()
                        .collect::<Vec<crate::DocumentDraftItem>>()
                })
                .unwrap_or_default();
            tokio::spawn(async move {
                match save_document_form(ctx.pool(), form, draft_items).await {
                    Ok(()) => {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_show_doc_editor(false);
                        });
                        crate::bootstrap::refresh_screen(
                            ui_weak,
                            ctx,
                            AppScreen::Documents,
                        )
                        .await;
                        notify_user("Документ збережено", "Шапку документа оновлено.");
                    }
                    Err(error) => {
                        tracing::error!("documents: save failed: {error}");
                        notify_user("Помилка збереження документа", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_doc_draft_item_upserted({
        let ui_weak = ui.as_weak();
        move |index, item| {
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                let mut items = ui
                    .get_doc_draft_items()
                    .iter()
                    .collect::<Vec<crate::DocumentDraftItem>>();
                if index >= 0 && (index as usize) < items.len() {
                    items[index as usize] = item;
                } else {
                    items.push(item);
                }
                ui.set_doc_draft_items(ModelRc::new(VecModel::from(items)));
            });
        }
    });

    ui.on_doc_draft_item_removed({
        let ui_weak = ui.as_weak();
        move |index| {
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                let mut items = ui
                    .get_doc_draft_items()
                    .iter()
                    .collect::<Vec<crate::DocumentDraftItem>>();
                if index >= 0 && (index as usize) < items.len() {
                    items.remove(index as usize);
                }
                ui.set_doc_draft_items(ModelRc::new(VecModel::from(items)));
            });
        }
    });

    ui.on_context_send({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id: slint::SharedString| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                if let Some(uuid_str) = id.strip_prefix("act:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::acts::advance_status(ctx.pool(), uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("inv:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::invoices::advance_status(ctx.pool(), uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("wbl:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::waybills::advance_status(ctx.pool(), uuid).await;
                    }
                }
                crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
            });
        }
    });

    ui.on_context_archive({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id: slint::SharedString| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                if let Some(uuid_str) = id.strip_prefix("act:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::acts::advance_status(ctx.pool(), uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("inv:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::invoices::advance_status(ctx.pool(), uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("wbl:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::waybills::advance_status(ctx.pool(), uuid).await;
                    }
                }
                crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
                notify_user("Документ архівований", "Статус оновлено.");
            });
        }
    });

    ui.on_context_delete({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id: slint::SharedString| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                if let Some(uuid_str) = id.strip_prefix("act:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::acts::delete(ctx.pool(), uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("inv:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::invoices::delete(ctx.pool(), uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("wbl:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::waybills::delete(ctx.pool(), uuid).await;
                    }
                }
                crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
                notify_user("Документ видалено", "");
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    #[test]
    fn sort_combined_documents_by_date_descending() {
        let pairs = vec![
            (
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                "A".to_string(),
            ),
            (
                NaiveDate::from_ymd_opt(2026, 4, 21).unwrap(),
                "B".to_string(),
            ),
            (
                NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
                "C".to_string(),
            ),
        ];
        let mut sorted = pairs;
        sorted.sort_by(|a, b| b.0.cmp(&a.0));
        assert_eq!(sorted[0].1, "B");
        assert_eq!(sorted[2].1, "C");
    }

    #[test]
    fn parse_document_ref_detects_supported_prefixes() {
        use uuid::Uuid;

        let uuid = Uuid::nil();
        assert!(matches!(
            super::parse_document_ref(&format!("act:{uuid}")),
            Some(super::DocumentRef::Act(_))
        ));
        assert!(matches!(
            super::parse_document_ref(&format!("inv:{uuid}")),
            Some(super::DocumentRef::Invoice(_))
        ));
        assert!(matches!(
            super::parse_document_ref(&format!("wbl:{uuid}")),
            Some(super::DocumentRef::Waybill(_))
        ));
        assert!(super::parse_document_ref("bad-value").is_none());
    }

    #[test]
    fn parse_ui_date_accepts_dd_mm_yyyy() {
        let parsed = super::parse_ui_date("27.04.2026").unwrap();
        assert_eq!(parsed, NaiveDate::from_ymd_opt(2026, 4, 27).unwrap());
    }

    #[test]
    fn optional_string_trims_empty_values() {
        assert_eq!(super::optional_string(""), None);
        assert_eq!(super::optional_string("   "), None);
        assert_eq!(super::optional_string(" Нотатка "), Some("Нотатка".to_string()));
    }
}
