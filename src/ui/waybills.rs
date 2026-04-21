// ui/waybills.rs — колбеки та дані для сторінки Накладні.

use std::sync::Arc;

use anyhow::Result;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

use acta::app_ctx::AppCtx;
use crate::{
    ui::helpers::*,
    FormItemRow, WaybillRow, WaybillStatus, MainWindow,
};
use acta::{
    db,
    models::{WaybillStatus as ModelWaybillStatus, NewWaybill, UpdateWaybill},
};

// ═══════════════════════════════════════════════════════════════════════════════
// ── Проміжні дані ──────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct WaybillsUiData {
    pub waybill_rows: Vec<WaybillRow>,
    pub counts: Vec<i32>,
    pub kpi_waybills_month: i32,
    pub kpi_delivered: SharedString,
    pub kpi_unsigned: SharedString,
    pub kpi_overdue: i32,
}

pub async fn prepare_waybills_data(
    pool: &sqlx::PgPool,
    company_id: uuid::Uuid,
    status_filter: Option<ModelWaybillStatus>,
    query: String,
) -> Result<WaybillsUiData> {
    let (waybills_result, counts_result, kpi_result) = tokio::join!(
        db::waybills::list_filtered(
            pool,
            company_id,
            status_filter,
            None,
            normalized_query(&query),
            None,
            None,
            None,
        ),
        db::waybills::count_by_status(pool, company_id),
        db::waybills::get_kpi(pool, company_id),
    );
    let waybills = waybills_result?;
    let counts = counts_result?;
    let kpi = kpi_result?;

    let waybill_rows = waybills
        .iter()
        .map(|wb| WaybillRow {
            id: SharedString::from(wb.id.to_string().as_str()),
            num: SharedString::from(wb.number.as_str()),
            date: SharedString::from(wb.date.format("%d.%m.%Y").to_string().as_str()),
            counterparty: SharedString::from(wb.counterparty_name.as_str()),
            amount: SharedString::from(format_amount_ua(wb.total_amount).as_str()),
            status_label: SharedString::from(wb.status.label()),
            status: match wb.status {
                ModelWaybillStatus::Draft     => WaybillStatus::Draft,
                ModelWaybillStatus::Issued    => WaybillStatus::Issued,
                ModelWaybillStatus::Signed    => WaybillStatus::Signed,
                ModelWaybillStatus::Delivered => WaybillStatus::Delivered,
            },
        })
        .collect();

    Ok(WaybillsUiData {
        waybill_rows,
        counts,
        kpi_waybills_month: kpi.waybills_this_month as i32,
        kpi_delivered: SharedString::from(format_kpi_amount(kpi.delivered_this_month).as_str()),
        kpi_unsigned: SharedString::from(format_kpi_amount(kpi.unsigned_total).as_str()),
        kpi_overdue: kpi.overdue_count as i32,
    })
}

pub fn apply_waybills_to_ui(ui: &MainWindow, d: WaybillsUiData, close_form: bool) {
    ui.set_waybill_rows(ModelRc::new(VecModel::from(d.waybill_rows)));
    ui.set_waybill_status_counts(ModelRc::new(VecModel::from(d.counts)));
    ui.set_waybill_kpi_waybills_month(d.kpi_waybills_month);
    ui.set_waybill_kpi_delivered(d.kpi_delivered);
    ui.set_waybill_kpi_unsigned(d.kpi_unsigned);
    ui.set_waybill_kpi_overdue(d.kpi_overdue);
    if close_form {
        ui.set_show_waybill_form(false);
    }
}

pub async fn reload_waybills(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    status_filter: Option<ModelWaybillStatus>,
    query: String,
    close_form: bool,
) -> Result<()> {
    let d = prepare_waybills_data(pool, company_id, status_filter, query).await?;
    ui_weak
        .upgrade_in_event_loop(move |ui| apply_waybills_to_ui(&ui, d, close_form))
        .map_err(anyhow::Error::from)
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── spawn_save_waybill ─────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn spawn_save_waybill(
    pool: sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    wb_state: Arc<std::sync::Mutex<WaybillListState>>,
    company_id: uuid::Uuid,
    number: String,
    date_str: String,
    cp_id_str: String,
    notes: Option<String>,
    cat_id_str: String,
    con_id_str: String,
    items: Vec<acta::models::waybill::NewWaybillItem>,
) {
    tokio::spawn(async move {
        if number.trim().is_empty() {
            show_toast(ui_weak.clone(), "Номер накладної не може бути порожнім".to_string(), true);
            return;
        }
        if date_str.trim().is_empty() {
            show_toast(ui_weak.clone(), "Дата накладної не може бути порожньою".to_string(), true);
            return;
        }
        if cp_id_str.trim().is_empty() {
            show_toast(ui_weak.clone(), "Контрагент не вибраний".to_string(), true);
            return;
        }
        let Some(date) = parse_date_ui(&date_str) else {
            show_toast(ui_weak.clone(), format!("Невірний формат дати: '{date_str}'"), true);
            return;
        };
        let Some(cp_uuid) = parse_uuid_or_log(&cp_id_str, "контрагента") else {
            show_toast(ui_weak.clone(), "Контрагент не вибраний".to_string(), true);
            return;
        };
        let cat_id_opt = parse_opt_uuid(&cat_id_str);
        let con_id_opt = parse_opt_uuid(&con_id_str);
        let new_waybill = NewWaybill {
            number: number.clone(),
            counterparty_id: cp_uuid,
            contract_id: con_id_opt,
            category_id: cat_id_opt,
            direction: "outgoing".to_string(),
            date,
            notes,
            bas_id: None,
            items,
        };
        match db::waybills::create(&pool, company_id, &new_waybill).await {
            Ok(wb) => {
                tracing::info!("Накладну '{}' збережено (id={}).", wb.number, wb.id);
                show_toast(
                    ui_weak.clone(),
                    format!("Накладну '{}' збережено", wb.number),
                    false,
                );
                let (status_filter, query) = {
                    let state = wb_state.lock().unwrap();
                    (state.status_filter.clone(), state.query.clone())
                };
                if let Err(e) =
                    reload_waybills(&pool, ui_weak.clone(), company_id, status_filter, query, true)
                        .await
                {
                    tracing::error!("Помилка оновлення списку накладних: {e}");
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
    let state = ctx.waybill_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_waybill_status_filter_changed(move |filter_idx| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let wb_state = state.clone();
        let cid = *company_id_arc.lock().unwrap();
        tokio::spawn(async move {
            let status_filter = match filter_idx {
                1 => Some(ModelWaybillStatus::Draft),
                2 => Some(ModelWaybillStatus::Issued),
                3 => Some(ModelWaybillStatus::Signed),
                4 => Some(ModelWaybillStatus::Delivered),
                _ => None,
            };
            let query = {
                let mut state = wb_state.lock().unwrap();
                state.status_filter = status_filter.clone();
                state.query.clone()
            };
            if let Err(e) =
                reload_waybills(&pool, ui_handle, cid, status_filter, query, false).await
            {
                tracing::error!("Помилка фільтру накладних: {e}");
            }
        });
    });

    // ── Пошук ─────────────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.waybill_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    let search_task: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>> =
        Arc::new(std::sync::Mutex::new(None));
    ui.on_waybill_search_changed(move |query| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let wb_state = state.clone();
        let cid = *company_id_arc.lock().unwrap();
        let query = query.to_string();
        let handle = tokio::spawn(async move {
            let (status_filter, query) = {
                let mut state = wb_state.lock().unwrap();
                state.query = query.clone();
                (state.status_filter.clone(), query)
            };
            if let Err(e) =
                reload_waybills(&pool, ui_handle, cid, status_filter, query, false).await
            {
                tracing::error!("Помилка пошуку накладних: {e}");
            }
        });
        if let Some(old) = search_task.lock().unwrap().replace(handle) {
            old.abort();
        }
    });

    ui.on_waybill_selected(|_id| {});

    // ── Нова накладна ─────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_waybill_create_clicked(move || {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        tokio::spawn(async move {
            let (cps, next_number, categories) = tokio::join!(
                db::waybills::counterparties_for_select(&pool, cid),
                db::waybills::generate_next_number(&pool, cid),
                db::categories::list_all_for_select(&pool, cid),
            );
            let cps = cps.unwrap_or_default();
            let next_number = next_number.unwrap_or_else(|_| "НАК-001".into());
            let categories = categories.unwrap_or_default();
            let today = chrono::Local::now().format("%d.%m.%Y").to_string();

            let (cat_names, cat_ids) = build_category_select(&categories);

            let (cp_names, cp_ids) = build_cp_select(&cps);

            ui_weak
                .upgrade_in_event_loop(move |ui| {
                    ui.set_waybill_form_cp_names(ModelRc::new(VecModel::from(cp_names)));
                    ui.set_waybill_form_cp_ids(ModelRc::new(VecModel::from(cp_ids)));
                    ui.set_waybill_form_number(SharedString::from(next_number.as_str()));
                    ui.set_waybill_form_date(SharedString::from(today.as_str()));
                    ui.set_waybill_form_notes(SharedString::from(""));
                    ui.set_waybill_form_cp_index(0);
                    ui.set_waybill_form_is_edit(false);
                    ui.set_waybill_form_edit_id(SharedString::from(""));
                    ui.set_waybill_form_total(SharedString::from("0.00"));
                    ui.set_waybill_form_category_names(ModelRc::new(VecModel::from(cat_names)));
                    ui.set_waybill_form_category_ids(ModelRc::new(VecModel::from(cat_ids)));
                    ui.set_waybill_form_category_index(0);
                    ui.set_waybill_form_items(ModelRc::new(VecModel::from(
                        Vec::<FormItemRow>::new(),
                    )));
                    ui.set_show_waybill_form(true);
                })
                .warn_if_terminated();
        });
    });

    // ── Наступний статус ──────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.waybill_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_waybill_advance_status_clicked(move |id| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let wb_state = state.clone();
        let cid = *company_id_arc.lock().unwrap();
        let id_str = id.to_string();
        tokio::spawn(async move {
            let waybill_id = match uuid::Uuid::parse_str(&id_str) {
                Ok(id) => id,
                Err(_) => {
                    tracing::error!("Невалідний UUID накладної: {id_str}");
                    return;
                }
            };
            match db::waybills::advance_status(&pool, waybill_id).await {
                Ok(Some(wb)) => {
                    let (status_filter, query) = {
                        let state = wb_state.lock().unwrap();
                        (state.status_filter.clone(), state.query.clone())
                    };
                    if let Err(e) =
                        reload_waybills(&pool, ui_weak, cid, status_filter, query, false).await
                    {
                        tracing::error!("Помилка оновлення накладних: {e}");
                    }
                    let _ = wb;
                }
                Ok(None) => tracing::error!("Накладну {id_str} не знайдено"),
                Err(e)   => tracing::error!("Помилка зміни статусу накладної: {e}"),
            }
        });
    });

    // ── Відкрити накладну для редагування ────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_waybill_edit_clicked(move |wb_id| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let id_str = wb_id.to_string();
        tokio::spawn(async move {
            let waybill_uuid = match uuid::Uuid::parse_str(&id_str) {
                Ok(id) => id,
                Err(_) => {
                    tracing::error!("Невалідний UUID накладної: {id_str}");
                    return;
                }
            };
            let (result, cps, categories) = tokio::join!(
                db::waybills::get_for_edit(&pool, waybill_uuid),
                db::waybills::counterparties_for_select(&pool, cid),
                db::categories::list_all_for_select(&pool, cid),
            );
            let (waybill, items) = match result {
                Ok(Some(data)) => data,
                Ok(None) => { tracing::error!("Накладна {id_str} не знайдена"); return; }
                Err(e)   => { tracing::error!("Помилка завантаження накладної: {e}"); return; }
            };
            let cps = cps.unwrap_or_default();
            let categories = categories.unwrap_or_default();
            let cp_id_str = waybill.counterparty_id.to_string();
            let cp_index =
                cps.iter().position(|(id, _)| id.to_string() == cp_id_str).unwrap_or(0);

            let (cat_names, cat_ids) = build_category_select(&categories);
            let cat_id_str =
                waybill.category_id.map(|id| id.to_string()).unwrap_or_default();
            let cat_index =
                cat_ids.iter().position(|id| id.as_str() == cat_id_str).unwrap_or(0);

            let (cp_names, cp_ids) = build_cp_select(&cps);

            ui_weak
                .upgrade_in_event_loop(move |ui| {
                    ui.set_waybill_form_cp_names(ModelRc::new(VecModel::from(cp_names)));
                    ui.set_waybill_form_cp_ids(ModelRc::new(VecModel::from(cp_ids)));
                    ui.set_waybill_form_number(SharedString::from(waybill.number.as_str()));
                    ui.set_waybill_form_date(SharedString::from(
                        waybill.date.format("%d.%m.%Y").to_string().as_str(),
                    ));
                    ui.set_waybill_form_notes(SharedString::from(
                        waybill.notes.as_deref().unwrap_or(""),
                    ));
                    ui.set_waybill_form_cp_index(cp_index as i32);
                    ui.set_waybill_form_is_edit(true);
                    ui.set_waybill_form_edit_id(SharedString::from(
                        waybill.id.to_string().as_str(),
                    ));
                    ui.set_waybill_form_total(SharedString::from(
                        waybill.total_amount.to_string().as_str(),
                    ));
                    ui.set_waybill_form_category_names(ModelRc::new(VecModel::from(cat_names)));
                    ui.set_waybill_form_category_ids(ModelRc::new(VecModel::from(cat_ids)));
                    ui.set_waybill_form_category_index(cat_index as i32);
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
                    ui.set_waybill_form_items(ModelRc::new(VecModel::from(form_items)));
                    ui.set_show_waybill_form(true);
                })
                .warn_if_terminated();
        });
    });

    // ── Скасувати форму ───────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_waybill_form_cancel(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_show_waybill_form(false);
        }
    });

    // ── Додати позицію ────────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_waybill_form_add_item(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let mut items = collect_model(&ui.get_waybill_form_items());
            items.push(default_form_item());
            ui.set_waybill_form_items(ModelRc::new(VecModel::from(items)));
        }
    });

    // ── Видалити позицію ──────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_waybill_form_remove_item(move |idx| {
        if let Some(ui) = ui_weak.upgrade() {
            let mut items = collect_model(&ui.get_waybill_form_items());
            let idx = idx as usize;
            if idx < items.len() {
                items.remove(idx);
            }
            ui.set_waybill_form_items(ModelRc::new(VecModel::from(items)));
            recalculate_waybill_total(&ui);
        }
    });

    // ── Редагування поля позиції ──────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_waybill_form_item_changed(move |idx, field, value| {
        if let Some(ui) = ui_weak.upgrade() {
            let mut items = collect_model(&ui.get_waybill_form_items());
            let needs_recalc = apply_form_item_change(&mut items, idx as usize, field.as_str(), value);
            ui.set_waybill_form_items(ModelRc::new(VecModel::from(items)));
            if needs_recalc {
                recalculate_waybill_total(&ui);
            }
        }
    });

    // ── Зберегти нову накладну ────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.waybill_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_waybill_form_save(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str| {
        let cid = *company_id_arc.lock().unwrap();
        let items = collect_waybill_items_from_ui(&ui_weak);
        spawn_save_waybill(
            pool.clone(),
            ui_weak.clone(),
            state.clone(),
            cid,
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.is_empty() { None } else { Some(notes.to_string()) },
            cat_id_str.to_string(),
            con_id_str.to_string(),
            items,
        );
    });

    // ── Оновити накладну ──────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.waybill_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_waybill_form_update(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str| {
        let cid = *company_id_arc.lock().unwrap();
        let edit_id = ui_weak
            .upgrade()
            .map(|ui| ui.get_waybill_form_edit_id().to_string())
            .unwrap_or_default();
        let items = collect_waybill_items_from_ui(&ui_weak);
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let wb_state = state.clone();
        let number = number.to_string();
        let date_str = date_str.to_string();
        let cp_id_str = cp_id_str.to_string();
        let notes = notes.to_string();
        let cat_id_str = cat_id_str.to_string();
        let con_id_str = con_id_str.to_string();
        tokio::spawn(async move {
            let waybill_uuid = match uuid::Uuid::parse_str(&edit_id) {
                Ok(id) => id,
                Err(_) => {
                    tracing::error!("Невалідний UUID накладної: {edit_id}");
                    return;
                }
            };
            let Some(date) = parse_date_ui(&date_str) else {
                return;
            };
            let Some(cp_uuid) = parse_uuid_or_log(&cp_id_str, "контрагента") else {
                return;
            };
            let cat_id_opt = parse_opt_uuid(&cat_id_str);
            let con_id_opt = parse_opt_uuid(&con_id_str);
            let update_data = UpdateWaybill {
                number: number.clone(),
                counterparty_id: cp_uuid,
                contract_id: con_id_opt,
                category_id: cat_id_opt,
                date,
                notes: if notes.is_empty() { None } else { Some(notes) },
            };
            match db::waybills::update_with_items(&pool, waybill_uuid, update_data, items).await {
                Ok(wb) => {
                    tracing::info!("Накладну '{}' оновлено.", wb.number);
                    show_toast(
                        ui_weak.clone(),
                        format!("Накладну '{}' оновлено", wb.number),
                        false,
                    );
                    let (status_filter, query) = {
                        let state = wb_state.lock().unwrap();
                        (state.status_filter.clone(), state.query.clone())
                    };
                    if let Err(e) = reload_waybills(
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
                }
                Err(e) => {
                    tracing::error!("Помилка оновлення накладної: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });

    // ── PDF накладної (заглушка — логіка аналогічна invoice PDF) ─────────────
    ui.on_waybill_pdf_clicked(move |_id| {
        tracing::info!("PDF накладної: ще не реалізовано");
    });
}
