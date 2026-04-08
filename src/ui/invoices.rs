// ui/invoices.rs — колбеки та дані для сторінки Видаткові накладні.

use std::sync::Arc;

use anyhow::Result;
use chrono::NaiveDate;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};

use crate::{
    app_ctx::AppCtx,
    ui::helpers::*,
    FormItemRow, InvoiceRow, MainWindow,
};
use acta::{
    db,
    models::{InvoiceStatus, NewInvoice, UpdateInvoice},
};

// ═══════════════════════════════════════════════════════════════════════════════
// ── Проміжні дані ──────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct InvoicesUiData {
    pub invoice_rows: Vec<InvoiceRow>,
}

pub async fn prepare_invoices_data(
    pool: &sqlx::PgPool,
    company_id: uuid::Uuid,
    status_filter: Option<InvoiceStatus>,
    query: String,
) -> Result<InvoicesUiData> {
    let invoices = db::invoices::list_filtered(
        pool,
        company_id,
        status_filter,
        None,
        normalized_query(&query),
        None,
        None,
        None,
    )
    .await?;
    let invoice_rows = invoices
        .iter()
        .map(|inv| InvoiceRow {
            id: SharedString::from(inv.id.to_string().as_str()),
            num: SharedString::from(inv.number.as_str()),
            date: SharedString::from(inv.date.format("%d.%m.%Y").to_string().as_str()),
            counterparty: SharedString::from(inv.counterparty_name.as_str()),
            amount: SharedString::from(format_amount_ua(inv.total_amount).as_str()),
            status_label: SharedString::from(match inv.status {
                InvoiceStatus::Draft => "Чернетка",
                InvoiceStatus::Issued => "Виставлено",
                InvoiceStatus::Signed => "Підписано",
                InvoiceStatus::Paid => "Оплачено",
            }),
            status: SharedString::from(inv.status.as_str()),
        })
        .collect();
    Ok(InvoicesUiData { invoice_rows })
}

pub fn apply_invoices_to_ui(ui: &MainWindow, d: InvoicesUiData, close_form: bool) {
    ui.set_invoice_rows(ModelRc::new(VecModel::from(d.invoice_rows)));
    if close_form {
        ui.set_show_invoice_form(false);
    }
}

pub async fn reload_invoices(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    status_filter: Option<InvoiceStatus>,
    query: String,
    close_form: bool,
) -> Result<()> {
    let d = prepare_invoices_data(pool, company_id, status_filter, query).await?;
    ui_weak
        .upgrade_in_event_loop(move |ui| apply_invoices_to_ui(&ui, d, close_form))
        .map_err(anyhow::Error::from)
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── spawn_save_invoice ─────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn spawn_save_invoice(
    pool: sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    inv_state: Arc<std::sync::Mutex<InvoiceListState>>,
    doc_state: Arc<std::sync::Mutex<DocListState>>,
    company_id: uuid::Uuid,
    number: String,
    date_str: String,
    cp_id_str: String,
    notes: Option<String>,
    cat_id_str: String,
    con_id_str: String,
    exp_date_str: String,
    items: Vec<acta::models::NewInvoiceItem>,
) {
    tokio::spawn(async move {
        if number.trim().is_empty() {
            tracing::error!("Номер накладної не може бути порожнім");
            return;
        }
        if date_str.trim().is_empty() {
            tracing::error!("Дата накладної не може бути порожньою");
            return;
        }
        if cp_id_str.trim().is_empty() {
            tracing::error!("Контрагент не вибраний");
            return;
        }
        let date = match NaiveDate::parse_from_str(&date_str, "%d.%m.%Y") {
            Ok(d) => d,
            Err(_) => {
                tracing::error!("Невірний формат дати: '{date_str}'");
                return;
            }
        };
        let cp_uuid = match uuid::Uuid::parse_str(&cp_id_str) {
            Ok(id) => id,
            Err(_) => {
                tracing::error!("Невалідний UUID контрагента: '{cp_id_str}'");
                return;
            }
        };
        let cat_id_opt: Option<uuid::Uuid> = if cat_id_str.trim().is_empty() {
            None
        } else {
            uuid::Uuid::parse_str(cat_id_str.as_str()).ok()
        };
        let con_id_opt: Option<uuid::Uuid> = if con_id_str.trim().is_empty() {
            None
        } else {
            uuid::Uuid::parse_str(con_id_str.as_str()).ok()
        };
        let exp_date_opt: Option<chrono::NaiveDate> = if exp_date_str.trim().is_empty() {
            None
        } else {
            NaiveDate::parse_from_str(exp_date_str.as_str(), "%d.%m.%Y").ok()
        };
        let new_invoice = NewInvoice {
            number: number.clone(),
            counterparty_id: cp_uuid,
            contract_id: con_id_opt,
            category_id: cat_id_opt,
            direction: {
                let s = doc_state.lock().unwrap();
                s.direction.clone()
            },
            date,
            expected_payment_date: exp_date_opt,
            notes,
            bas_id: None,
            items,
        };
        match db::invoices::create(&pool, company_id, &new_invoice).await {
            Ok(inv) => {
                tracing::info!("Накладну '{}' збережено (id={}).", inv.number, inv.id);
                show_toast(
                    ui_weak.clone(),
                    format!("Накладну '{}' збережено", inv.number),
                    false,
                );
                let (status_filter, query) = {
                    let state = inv_state.lock().unwrap();
                    (state.status_filter.clone(), state.query.clone())
                };
                if let Err(e) =
                    reload_invoices(&pool, ui_weak.clone(), company_id, status_filter, query, true)
                        .await
                {
                    tracing::error!("Помилка оновлення списку накладних: {e}");
                }
                let (doc_tab, doc_direction, doc_query, doc_cp, doc_df, doc_dt) = {
                    let s = doc_state.lock().unwrap();
                    (
                        s.tab,
                        s.direction.clone(),
                        s.query.clone(),
                        s.counterparty_id,
                        s.date_from,
                        s.date_to,
                    )
                };
                if let Err(e) = crate::ui::documents::reload_documents(
                    &pool,
                    ui_weak,
                    company_id,
                    doc_tab,
                    &doc_direction,
                    &doc_query,
                    doc_cp,
                    doc_df,
                    doc_dt,
                )
                .await
                {
                    tracing::error!(
                        "Помилка оновлення документів після збереження накладної: {e}"
                    );
                }
            }
            Err(e) => {
                tracing::error!("Помилка збереження накладної: {e}");
                show_toast(ui_weak.clone(), format!("Помилка: {e}"), true);
            }
        }
    });
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── setup — реєстрація колбеків ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn setup(ui: &MainWindow, ctx: Arc<AppCtx>) {
    // ── Фільтр статусу ────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.invoice_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_invoice_status_filter_changed(move |filter_idx| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let inv_state = state.clone();
        let cid = *company_id_arc.lock().unwrap();
        tokio::spawn(async move {
            let status_filter = match filter_idx {
                1 => Some(InvoiceStatus::Draft),
                2 => Some(InvoiceStatus::Issued),
                3 => Some(InvoiceStatus::Signed),
                4 => Some(InvoiceStatus::Paid),
                _ => None,
            };
            let query = {
                let mut state = inv_state.lock().unwrap();
                state.status_filter = status_filter.clone();
                state.query.clone()
            };
            if let Err(e) =
                reload_invoices(&pool, ui_handle, cid, status_filter, query, false).await
            {
                tracing::error!("Помилка фільтру накладних: {e}");
            }
        });
    });

    // ── Пошук ─────────────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.invoice_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    let search_task: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>> = Arc::new(std::sync::Mutex::new(None));
    ui.on_invoice_search_changed(move |query| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let inv_state = state.clone();
        let cid = *company_id_arc.lock().unwrap();
        let query = query.to_string();
        let handle = tokio::spawn(async move {
            let (status_filter, query) = {
                let mut state = inv_state.lock().unwrap();
                state.query = query.clone();
                (state.status_filter.clone(), query)
            };
            if let Err(e) =
                reload_invoices(&pool, ui_handle, cid, status_filter, query, false).await
            {
                tracing::error!("Помилка пошуку накладних: {e}");
            }
        });
        if let Some(old) = search_task.lock().unwrap().replace(handle) {
            old.abort();
        }
    });

    ui.on_invoice_selected(|_id| {});

    // ── Новий рахунок ─────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_invoice_create_clicked(move || {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        tokio::spawn(async move {
            let (cps, next_number, categories) = tokio::join!(
                db::invoices::counterparties_for_select(&pool, cid),
                db::invoices::generate_next_number(&pool, cid),
                db::categories::list_all_for_select(&pool, cid),
            );
            let cps = cps.unwrap_or_default();
            let next_number = next_number.unwrap_or_else(|_| "НАК-001".into());
            let categories = categories.unwrap_or_default();
            let today = chrono::Local::now().format("%d.%m.%Y").to_string();

            let mut cat_names: Vec<SharedString> =
                vec![SharedString::from("— без категорії —")];
            let mut cat_ids: Vec<SharedString> = vec![SharedString::from("")];
            for c in &categories {
                let prefix = if c.depth > 0 { "  └─ " } else { "" };
                cat_names.push(SharedString::from(format!("{}{}", prefix, c.name)));
                cat_ids.push(SharedString::from(c.id.to_string()));
            }

            ui_weak
                .upgrade_in_event_loop(move |ui| {
                    let (names, ids): (Vec<SharedString>, Vec<SharedString>) = cps
                        .iter()
                        .map(|(id, name)| {
                            (
                                SharedString::from(name.as_str()),
                                SharedString::from(id.to_string().as_str()),
                            )
                        })
                        .unzip();
                    ui.set_invoice_form_cp_names(ModelRc::new(VecModel::from(names)));
                    ui.set_invoice_form_cp_ids(ModelRc::new(VecModel::from(ids)));
                    ui.set_invoice_form_number(SharedString::from(next_number.as_str()));
                    ui.set_invoice_form_date(SharedString::from(today.as_str()));
                    ui.set_invoice_form_notes(SharedString::from(""));
                    ui.set_invoice_form_cp_index(0);
                    ui.set_invoice_form_is_edit(false);
                    ui.set_invoice_form_edit_id(SharedString::from(""));
                    ui.set_invoice_form_total(SharedString::from("0.00"));
                    ui.set_invoice_form_category_names(ModelRc::new(VecModel::from(cat_names)));
                    ui.set_invoice_form_category_ids(ModelRc::new(VecModel::from(cat_ids)));
                    ui.set_invoice_form_category_index(0);
                    ui.set_invoice_form_expected_payment_date(SharedString::from(""));
                    ui.set_invoice_form_items(ModelRc::new(VecModel::from(
                        Vec::<FormItemRow>::new(),
                    )));
                    ui.set_show_invoice_form(true);
                })
                .warn_if_terminated();
        });
    });

    // ── Наступний статус ──────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.invoice_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_invoice_advance_status_clicked(move |id| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let inv_state = state.clone();
        let cid = *company_id_arc.lock().unwrap();
        let id_str = id.to_string();
        tokio::spawn(async move {
            let invoice_id = match uuid::Uuid::parse_str(&id_str) {
                Ok(id) => id,
                Err(_) => {
                    tracing::error!("Невалідний UUID накладної: {id_str}");
                    return;
                }
            };
            match db::invoices::advance_status(&pool, invoice_id).await {
                Ok(Some(inv)) => {
                    let (status_filter, query) = {
                        let state = inv_state.lock().unwrap();
                        (state.status_filter.clone(), state.query.clone())
                    };
                    if let Err(e) =
                        reload_invoices(&pool, ui_weak, cid, status_filter, query, false).await
                    {
                        tracing::error!("Помилка оновлення накладних: {e}");
                    }
                    let _ = inv;
                }
                Ok(None) => tracing::error!("Накладну {id_str} не знайдено"),
                Err(e) => tracing::error!("Помилка зміни статусу накладної: {e}"),
            }
        });
    });

    // ── Відкрити накладну для редагування ────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_invoice_edit_clicked(move |inv_id| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let id_str = inv_id.to_string();
        tokio::spawn(async move {
            let invoice_uuid = match uuid::Uuid::parse_str(&id_str) {
                Ok(id) => id,
                Err(_) => {
                    tracing::error!("Невалідний UUID накладної: {id_str}");
                    return;
                }
            };
            let (result, cps, categories) = tokio::join!(
                db::invoices::get_for_edit(&pool, invoice_uuid),
                db::invoices::counterparties_for_select(&pool, cid),
                db::categories::list_all_for_select(&pool, cid),
            );
            let (invoice, items) = match result {
                Ok(Some(data)) => data,
                Ok(None) => { tracing::error!("Накладна {id_str} не знайдена"); return; }
                Err(e) => { tracing::error!("Помилка завантаження накладної: {e}"); return; }
            };
            let cps = cps.unwrap_or_default();
            let categories = categories.unwrap_or_default();
            let cp_id_str = invoice.counterparty_id.to_string();
            let cp_index =
                cps.iter().position(|(id, _)| id.to_string() == cp_id_str).unwrap_or(0);

            let mut cat_names: Vec<SharedString> =
                vec![SharedString::from("— без категорії —")];
            let mut cat_ids: Vec<SharedString> = vec![SharedString::from("")];
            for c in &categories {
                let prefix = if c.depth > 0 { "  └─ " } else { "" };
                cat_names.push(SharedString::from(format!("{}{}", prefix, c.name)));
                cat_ids.push(SharedString::from(c.id.to_string()));
            }
            let cat_id_str =
                invoice.category_id.map(|id| id.to_string()).unwrap_or_default();
            let cat_index =
                cat_ids.iter().position(|id| id.as_str() == cat_id_str).unwrap_or(0);
            let exp_date_str = invoice
                .expected_payment_date
                .map(|d| d.format("%d.%m.%Y").to_string())
                .unwrap_or_default();

            ui_weak
                .upgrade_in_event_loop(move |ui| {
                    let (names, ids): (Vec<SharedString>, Vec<SharedString>) = cps
                        .iter()
                        .map(|(id, name)| {
                            (
                                SharedString::from(name.as_str()),
                                SharedString::from(id.to_string().as_str()),
                            )
                        })
                        .unzip();
                    ui.set_invoice_form_cp_names(ModelRc::new(VecModel::from(names)));
                    ui.set_invoice_form_cp_ids(ModelRc::new(VecModel::from(ids)));
                    ui.set_invoice_form_number(SharedString::from(invoice.number.as_str()));
                    ui.set_invoice_form_date(SharedString::from(
                        invoice.date.format("%d.%m.%Y").to_string().as_str(),
                    ));
                    ui.set_invoice_form_notes(SharedString::from(
                        invoice.notes.as_deref().unwrap_or(""),
                    ));
                    ui.set_invoice_form_cp_index(cp_index as i32);
                    ui.set_invoice_form_is_edit(true);
                    ui.set_invoice_form_edit_id(SharedString::from(
                        invoice.id.to_string().as_str(),
                    ));
                    ui.set_invoice_form_total(SharedString::from(
                        invoice.total_amount.to_string().as_str(),
                    ));
                    ui.set_invoice_form_category_names(ModelRc::new(VecModel::from(cat_names)));
                    ui.set_invoice_form_category_ids(ModelRc::new(VecModel::from(cat_ids)));
                    ui.set_invoice_form_category_index(cat_index as i32);
                    ui.set_invoice_form_expected_payment_date(SharedString::from(
                        exp_date_str.as_str(),
                    ));
                    let form_items: Vec<FormItemRow> = items
                        .iter()
                        .map(|it| FormItemRow {
                            description: SharedString::from(it.description.as_str()),
                            quantity: SharedString::from(it.quantity.to_string().as_str()),
                            unit: SharedString::from(it.unit.as_deref().unwrap_or("")),
                            price: SharedString::from(it.price.to_string().as_str()),
                            amount: SharedString::from(it.amount.to_string().as_str()),
                        })
                        .collect();
                    ui.set_invoice_form_items(ModelRc::new(VecModel::from(form_items)));
                    ui.set_show_invoice_form(true);
                })
                .warn_if_terminated();
        });
    });

    // ── Скасувати форму ───────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_invoice_form_cancel(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_show_invoice_form(false);
        }
    });

    // ── Додати позицію ────────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_invoice_form_add_item(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let mut items: Vec<FormItemRow> = (0..ui.get_invoice_form_items().row_count())
                .filter_map(|i| ui.get_invoice_form_items().row_data(i))
                .collect();
            items.push(FormItemRow {
                description: SharedString::from(""),
                quantity: SharedString::from("1"),
                unit: SharedString::from("шт"),
                price: SharedString::from("0.00"),
                amount: SharedString::from("0.00"),
            });
            ui.set_invoice_form_items(ModelRc::new(VecModel::from(items)));
        }
    });

    // ── Видалити позицію ──────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_invoice_form_remove_item(move |idx| {
        if let Some(ui) = ui_weak.upgrade() {
            let mut items: Vec<FormItemRow> = (0..ui.get_invoice_form_items().row_count())
                .filter_map(|i| ui.get_invoice_form_items().row_data(i))
                .collect();
            let idx = idx as usize;
            if idx < items.len() {
                items.remove(idx);
            }
            ui.set_invoice_form_items(ModelRc::new(VecModel::from(items)));
            recalculate_invoice_total(&ui);
        }
    });

    // ── Редагування поля позиції ──────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_invoice_form_item_changed(move |idx, field, value| {
        if let Some(ui) = ui_weak.upgrade() {
            let mut items: Vec<FormItemRow> = (0..ui.get_invoice_form_items().row_count())
                .filter_map(|i| ui.get_invoice_form_items().row_data(i))
                .collect();
            let idx = idx as usize;
            if idx < items.len() {
                match field.as_str() {
                    "desc" => {
                        items[idx].description = value;
                    }
                    "qty" => {
                        items[idx].quantity = value;
                    }
                    "unit" => {
                        items[idx].unit = value;
                    }
                    "price" => {
                        items[idx].price = value;
                    }
                    _ => {}
                }
                ui.set_invoice_form_items(ModelRc::new(VecModel::from(items)));
                if matches!(field.as_str(), "qty" | "price") {
                    recalculate_invoice_total(&ui);
                }
            }
        }
    });

    // ── Зберегти нову накладну ────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.invoice_state.clone();
    let doc_state = ctx.doc_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_invoice_form_save(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str, exp_date_str| {
        let cid = *company_id_arc.lock().unwrap();
        let items = collect_invoice_items_from_ui(&ui_weak);
        spawn_save_invoice(
            pool.clone(),
            ui_weak.clone(),
            state.clone(),
            doc_state.clone(),
            cid,
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.is_empty() { None } else { Some(notes.to_string()) },
            cat_id_str.to_string(),
            con_id_str.to_string(),
            exp_date_str.to_string(),
            items,
        );
    });

    // ── Оновити накладну ──────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.invoice_state.clone();
    let doc_state = ctx.doc_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_invoice_form_update(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str, exp_date_str| {
        let cid = *company_id_arc.lock().unwrap();
        let edit_id = ui_weak
            .upgrade()
            .map(|ui| ui.get_invoice_form_edit_id().to_string())
            .unwrap_or_default();
        let items = collect_invoice_items_from_ui(&ui_weak);
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let inv_state = state.clone();
        let doc_state_u = doc_state.clone();
        let number = number.to_string();
        let date_str = date_str.to_string();
        let cp_id_str = cp_id_str.to_string();
        let notes = notes.to_string();
        let cat_id_str = cat_id_str.to_string();
        let con_id_str = con_id_str.to_string();
        let exp_date_str = exp_date_str.to_string();
        tokio::spawn(async move {
            let invoice_uuid = match uuid::Uuid::parse_str(&edit_id) {
                Ok(id) => id,
                Err(_) => {
                    tracing::error!("Невалідний UUID накладної: {edit_id}");
                    return;
                }
            };
            let date = match NaiveDate::parse_from_str(&date_str, "%d.%m.%Y") {
                Ok(d) => d,
                Err(_) => {
                    tracing::error!("Невірний формат дати: '{date_str}'");
                    return;
                }
            };
            let cp_uuid = match uuid::Uuid::parse_str(&cp_id_str) {
                Ok(id) => id,
                Err(_) => {
                    tracing::error!("Невалідний UUID контрагента: '{cp_id_str}'");
                    return;
                }
            };
            let cat_id_opt: Option<uuid::Uuid> = if cat_id_str.trim().is_empty() {
                None
            } else {
                uuid::Uuid::parse_str(cat_id_str.as_str()).ok()
            };
            let con_id_opt: Option<uuid::Uuid> = if con_id_str.trim().is_empty() {
                None
            } else {
                uuid::Uuid::parse_str(con_id_str.as_str()).ok()
            };
            let exp_date_opt: Option<chrono::NaiveDate> = if exp_date_str.trim().is_empty() {
                None
            } else {
                NaiveDate::parse_from_str(exp_date_str.as_str(), "%d.%m.%Y").ok()
            };
            let update_data = UpdateInvoice {
                number: number.clone(),
                counterparty_id: cp_uuid,
                contract_id: con_id_opt,
                category_id: cat_id_opt,
                date,
                expected_payment_date: exp_date_opt,
                notes: if notes.is_empty() { None } else { Some(notes) },
            };
            match db::invoices::update_with_items(&pool, invoice_uuid, update_data, items).await {
                Ok(inv) => {
                    tracing::info!("Накладну '{}' оновлено.", inv.number);
                    show_toast(
                        ui_weak.clone(),
                        format!("Накладну '{}' оновлено", inv.number),
                        false,
                    );
                    let (status_filter, query) = {
                        let state = inv_state.lock().unwrap();
                        (state.status_filter.clone(), state.query.clone())
                    };
                    if let Err(e) = reload_invoices(
                        &pool,
                        ui_weak.clone(),
                        cid,
                        status_filter,
                        query,
                        true,
                    )
                    .await
                    {
                        tracing::error!("Помилка оновлення списку накладних: {e}");
                    }
                    let (doc_tab, doc_direction, doc_query, doc_cp, doc_df, doc_dt) = {
                        let s = doc_state_u.lock().unwrap();
                        (
                            s.tab,
                            s.direction.clone(),
                            s.query.clone(),
                            s.counterparty_id,
                            s.date_from,
                            s.date_to,
                        )
                    };
                    if let Err(e) = crate::ui::documents::reload_documents(
                        &pool,
                        ui_weak.clone(),
                        cid,
                        doc_tab,
                        &doc_direction,
                        &doc_query,
                        doc_cp,
                        doc_df,
                        doc_dt,
                    )
                    .await
                    {
                        tracing::error!(
                            "Помилка оновлення документів після редагування накладної: {e}"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("Помилка оновлення накладної: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });
}
