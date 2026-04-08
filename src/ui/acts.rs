// ui/acts.rs — колбеки та дані для сторінки Акти виконаних робіт.

use std::sync::Arc;

use anyhow::Result;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};

use crate::{
    app_ctx::AppCtx,
    ui::helpers::*,
    ActCardItemRow, ActCardTaskRow, ActRow, ActStatus, FormItemRow, MainWindow, TaskRow,
};
use acta::{db, models::{NewAct, TaskStatus, UpdateAct}};

// ═══════════════════════════════════════════════════════════════════════════════
// ── Проміжні дані (Send-safe) ──────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct ActsUiData {
    pub act_rows: Vec<ActRow>,
    pub counts: Vec<i32>,
    pub kpi_acts_month: i32,
    pub kpi_revenue: SharedString,
    pub kpi_unpaid: SharedString,
    pub kpi_overdue: i32,
}

pub async fn prepare_acts_data(
    pool: &sqlx::PgPool,
    company_id: uuid::Uuid,
    status_filter: Option<ModelActStatus>,
    query: String,
) -> Result<ActsUiData> {
    let (acts_result, counts_result, kpi_result) = tokio::join!(
        db::acts::list_filtered(
            pool,
            company_id,
            status_filter,
            None,
            normalized_query(&query),
            None,
            None,
            None
        ),
        db::acts::count_by_status(pool, company_id),
        db::acts::get_kpi(pool, company_id)
    );
    let acts = acts_result?;
    let counts = counts_result?;
    let kpi = kpi_result?;

    Ok(ActsUiData {
        act_rows: to_act_rows(&acts),
        counts,
        kpi_acts_month: kpi.acts_this_month as i32,
        kpi_revenue: SharedString::from(format_kpi_amount(kpi.revenue_this_month).as_str()),
        kpi_unpaid: SharedString::from(format_kpi_amount(kpi.unpaid_total).as_str()),
        kpi_overdue: kpi.overdue_count as i32,
    })
}

pub fn apply_acts_to_ui(ui: &MainWindow, d: ActsUiData, close_form: bool) {
    ui.set_act_rows(ModelRc::new(VecModel::from(d.act_rows)));
    ui.set_act_status_counts(ModelRc::new(VecModel::from(d.counts)));
    ui.set_act_kpi_acts_month(d.kpi_acts_month);
    ui.set_act_kpi_revenue(d.kpi_revenue);
    ui.set_act_kpi_unpaid(d.kpi_unpaid);
    ui.set_act_kpi_overdue(d.kpi_overdue);
    if close_form {
        ui.set_show_act_form(false);
    }
}

pub async fn reload_acts(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    status_filter: Option<ModelActStatus>,
    query: String,
    close_form: bool,
) -> Result<()> {
    let d = prepare_acts_data(pool, company_id, status_filter, query).await?;
    ui_weak
        .upgrade_in_event_loop(move |ui| apply_acts_to_ui(&ui, d, close_form))
        .map_err(anyhow::Error::from)
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── open_act_card ──────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub async fn open_act_card(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    act_id: uuid::Uuid,
) -> Result<()> {
    let (act_result, tasks_result) = tokio::join!(
        db::acts::get_by_id(pool, act_id),
        db::tasks::list_by_act(pool, act_id),
    );

    let (act, items) = act_result?.ok_or_else(|| anyhow::anyhow!("Акт {act_id} не знайдено"))?;
    let tasks = tasks_result?;

    let counterparty = db::counterparties::get_by_id(pool, act.counterparty_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Контрагент акту не знайдено"))?;

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            let act_card_status = match act.status {
                ModelActStatus::Draft => ActStatus::Draft,
                ModelActStatus::Issued => ActStatus::Issued,
                ModelActStatus::Signed => ActStatus::Signed,
                ModelActStatus::Paid => ActStatus::Paid,
            };

            ui.set_act_card_id(SharedString::from(act.id.to_string()));
            ui.set_act_card_number(SharedString::from(act.number.as_str()));
            ui.set_act_card_date(SharedString::from(
                act.date.format("%d.%m.%Y").to_string(),
            ));
            ui.set_act_card_status(act_card_status);
            ui.set_act_card_status_label(SharedString::from(act.status.label()));
            ui.set_act_card_counterparty(SharedString::from(counterparty.name.as_str()));
            ui.set_act_card_total(SharedString::from(format_amount_ua(act.total_amount)));
            ui.set_act_card_expected_payment(SharedString::from(
                act.expected_payment_date
                    .map(|d| d.format("%d.%m.%Y").to_string())
                    .unwrap_or_default(),
            ));
            ui.set_act_card_notes(SharedString::from(act.notes.as_deref().unwrap_or("")));

            ui.set_act_card_items(ModelRc::new(VecModel::from(
                items
                    .iter()
                    .map(|i| ActCardItemRow {
                        description: SharedString::from(i.description.as_str()),
                        quantity: SharedString::from(i.quantity.to_string()),
                        unit: SharedString::from(i.unit.as_str()),
                        unit_price: SharedString::from(format_amount_ua(i.unit_price)),
                        amount: SharedString::from(format_amount_ua(i.amount)),
                    })
                    .collect::<Vec<_>>(),
            )));

            ui.set_act_card_tasks(ModelRc::new(VecModel::from(
                tasks
                    .iter()
                    .map(|t| ActCardTaskRow {
                        title: SharedString::from(t.title.as_str()),
                        status: SharedString::from(t.status.label()),
                        priority: SharedString::from(t.priority.as_str()),
                        due_date: SharedString::from(
                            t.due_date
                                .map(|d| d.format("%d.%m.%Y").to_string())
                                .unwrap_or_else(|| "—".to_string()),
                        ),
                    })
                    .collect::<Vec<_>>(),
            )));

            ui.set_show_act_card(true);
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── spawn_save_act ─────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn spawn_save_act(
    pool: sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    act_state: Arc<std::sync::Mutex<ActListState>>,
    doc_state: Arc<std::sync::Mutex<DocListState>>,
    company_id: uuid::Uuid,
    number: String,
    date_str: String,
    cp_id_str: String,
    notes: Option<String>,
    cat_id_str: String,
    con_id_str: String,
    exp_date_str: String,
    items: Vec<acta::models::NewActItem>,
    is_draft: bool,
) {
    tokio::spawn(async move {
        if number.trim().is_empty() {
            show_toast(ui_weak.clone(), "Номер акту не може бути порожнім".to_string(), true);
            return;
        }
        if date_str.trim().is_empty() {
            show_toast(ui_weak.clone(), "Дата акту не може бути порожньою".to_string(), true);
            return;
        }
        if cp_id_str.trim().is_empty() {
            show_toast(ui_weak.clone(), "Контрагент не вибраний".to_string(), true);
            return;
        }

        let date = match NaiveDate::parse_from_str(&date_str, "%d.%m.%Y") {
            Ok(d) => d,
            Err(_) => {
                show_toast(ui_weak.clone(), format!("Невірний формат дати: '{date_str}'"), true);
                return;
            }
        };

        let cp_uuid = match uuid::Uuid::parse_str(&cp_id_str) {
            Ok(id) => id,
            Err(_) => {
                show_toast(ui_weak.clone(), "Контрагент не вибраний".to_string(), true);
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

        let new_act = NewAct {
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
            status: if is_draft { ModelActStatus::Draft } else { ModelActStatus::Issued },
            notes,
            bas_id: None,
            items,
        };

        match db::acts::create(&pool, company_id, &new_act).await {
            Ok(act) => {
                tracing::info!("Акт '{}' збережено (id={}).", act.number, act.id);
                show_toast(ui_weak.clone(), format!("Акт '{}' збережено", act.number), false);

                let (query, status_filter) = {
                    let state = act_state.lock().unwrap();
                    (state.query.clone(), state.status_filter.clone())
                };
                if let Err(e) =
                    reload_acts(&pool, ui_weak.clone(), company_id, status_filter, query, true).await
                {
                    tracing::error!(
                        "Помилка оновлення списку актів після збереження: {e}"
                    );
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
                    ui_weak.clone(),
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
                        "Помилка оновлення документів після збереження акту: {e}"
                    );
                }
            }
            Err(e) => {
                tracing::error!("Помилка збереження акту: {e}");
                show_toast(ui_weak.clone(), format!("Помилка: {e}"), true);
            }
        }
    });
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── populate_act_form — підготовка і відображення форми редагування акту ───────
// ═══════════════════════════════════════════════════════════════════════════════

/// Завантажує акт, контрагентів, задачі та категорії з БД і заповнює форму редагування.
/// `close_card`: true — перед відкриттям форми приховати картку акту (on_act_card_edit_clicked).
async fn populate_act_form(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    uuid: uuid::Uuid,
    cid: uuid::Uuid,
    close_card: bool,
) -> Result<()> {
    let (act_result, cp_result, tasks_result, cat_result) = tokio::join!(
        db::acts::get_for_edit(pool, uuid),
        db::acts::counterparties_for_select(pool, cid),
        db::tasks::list_by_act(pool, uuid),
        db::categories::list_all_for_select(pool, cid),
    );

    let counterparties: Vec<(uuid::Uuid, String)> = cp_result
        .map_err(|e| { tracing::error!("Помилка завантаження контрагентів: {e}"); e })?;
    let (act, items) = act_result
        .map_err(|e| { tracing::error!("Помилка завантаження акту: {e}"); e })?
        .ok_or_else(|| { tracing::warn!("Акт {uuid} не знайдено."); anyhow::anyhow!("not found") })?;
    let tasks = tasks_result.unwrap_or_default();
    let categories = cat_result.unwrap_or_default();

    let cp_names: Vec<SharedString> = counterparties
        .iter()
        .map(|(_, n)| SharedString::from(n.as_str()))
        .collect();
    let cp_ids: Vec<SharedString> = counterparties
        .iter()
        .map(|(id, _)| SharedString::from(id.to_string().as_str()))
        .collect();
    let cp_index = counterparties
        .iter()
        .position(|(id, _)| *id == act.counterparty_id)
        .unwrap_or(0) as i32;

    let mut cat_names: Vec<SharedString> = vec![SharedString::from("— без категорії —")];
    let mut cat_ids: Vec<SharedString> = vec![SharedString::from("")];
    for c in &categories {
        let prefix = if c.depth > 0 { "  └─ " } else { "" };
        cat_names.push(SharedString::from(format!("{}{}", prefix, c.name)));
        cat_ids.push(SharedString::from(c.id.to_string()));
    }
    let cat_id_str = act.category_id.map(|id| id.to_string()).unwrap_or_default();
    let cat_index = cat_ids.iter().position(|id| id.as_str() == cat_id_str).unwrap_or(0) as i32;

    let form_items: Vec<FormItemRow> = items
        .iter()
        .map(|it| FormItemRow {
            description: SharedString::from(it.description.as_str()),
            quantity: SharedString::from(format!("{}", it.quantity).as_str()),
            unit: SharedString::from(it.unit.as_str()),
            price: SharedString::from(format!("{}", it.unit_price).as_str()),
            amount: SharedString::from(format!("{:.2}", it.amount).as_str()),
        })
        .collect();
    let task_rows = to_task_rows(&tasks);

    let act_number = act.number.clone();
    let act_date = act.date.format("%d.%m.%Y").to_string();
    let act_notes = act.notes.clone().unwrap_or_default();
    let act_id_str = act.id.to_string();
    let total_str = format!("{:.2}", act.total_amount);
    let exp_date_str = act
        .expected_payment_date
        .map(|d| d.format("%d.%m.%Y").to_string())
        .unwrap_or_default();

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            if close_card {
                ui.set_show_act_card(false);
            }
            ui.set_act_form_number(SharedString::from(act_number.as_str()));
            ui.set_act_form_date(SharedString::from(act_date.as_str()));
            ui.set_act_form_notes(SharedString::from(act_notes.as_str()));
            ui.set_act_form_cp_index(cp_index);
            ui.set_act_form_edit_id(SharedString::from(act_id_str.as_str()));
            ui.set_act_form_total(SharedString::from(total_str.as_str()));
            ui.set_act_form_is_edit(true);
            ui.set_act_form_cp_names(ModelRc::new(VecModel::from(cp_names)));
            ui.set_act_form_cp_ids(ModelRc::new(VecModel::from(cp_ids)));
            ui.set_act_form_category_names(ModelRc::new(VecModel::from(cat_names)));
            ui.set_act_form_category_ids(ModelRc::new(VecModel::from(cat_ids)));
            ui.set_act_form_category_index(cat_index);
            ui.set_act_form_expected_payment_date(SharedString::from(exp_date_str.as_str()));
            ui.set_act_form_items(ModelRc::new(VecModel::from(form_items)));
            ui.set_act_task_rows(ModelRc::new(VecModel::from(task_rows)));
            ui.set_act_tasks_loading(false);
            ui.set_show_act_form(true);
        })
        .ok();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── setup — реєстрація колбеків ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn setup(ui: &MainWindow, ctx: Arc<AppCtx>) {
    // ── Фільтр статусу ────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.act_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_status_filter_changed(move |filter_idx| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let status_filter = match filter_idx {
            1 => Some(ModelActStatus::Draft),
            2 => Some(ModelActStatus::Issued),
            3 => Some(ModelActStatus::Signed),
            4 => Some(ModelActStatus::Paid),
            _ => None,
        };
        let query = {
            let mut s = state.lock().unwrap();
            s.status_filter = status_filter.clone();
            s.query.clone()
        };
        tokio::spawn(async move {
            if let Err(e) = reload_acts(&pool, ui_handle, cid, status_filter, query, false).await {
                tracing::error!("Помилка фільтру актів: {e}");
            }
        });
    });

    // ── Пошук ─────────────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.act_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_search_changed(move |query| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let (query, status_filter) = {
            let mut s = state.lock().unwrap();
            s.query = query.to_string();
            (s.query.clone(), s.status_filter.clone())
        };
        tokio::spawn(async move {
            if let Err(e) = reload_acts(&pool, ui_handle, cid, status_filter, query, false).await {
                tracing::error!("Помилка пошуку актів: {e}");
            }
        });
    });

    // ── Вибір акту — відкрити картку ──────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    ui.on_act_selected(move |id| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let id_str = id.to_string();
        tokio::spawn(async move {
            let Ok(act_id) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Картка акту: некоректний UUID: {id_str}");
                return;
            };
            if let Err(e) = open_act_card(&pool, ui_weak, act_id).await {
                tracing::error!("Помилка відкриття картки акту: {e}");
            }
        });
    });

    // ── Закрити картку акту ───────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_act_card_close_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_show_act_card(false);
        }
    });

    // ── Редагувати з картки акту ──────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_card_edit_clicked(move |id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let id_str = id.to_string();
        let cid = *company_id_arc.lock().unwrap();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Редагувати акт з картки: некоректний UUID: {id_str}");
                return;
            };
            if let Err(e) = populate_act_form(&pool, ui_handle, uuid, cid, true).await {
                tracing::error!("Помилка підготовки форми акту: {e}");
            }
        });
    });

    // ── PDF з картки акту ─────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_card_pdf_clicked(move |id| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let id_str = id.to_string();
        let cid = *company_id_arc.lock().unwrap();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else { return; };
            let (act_result, company_result) = tokio::join!(
                db::acts::get_by_id(&pool, uuid),
                db::companies::get_by_id(&pool, cid)
            );
            let Some((act, items)) = (match act_result {
                Ok(v) => v,
                Err(e) => { tracing::error!("PDF (картка): {e}"); return; }
            }) else { return; };
            let company = match company_result {
                Ok(Some(v)) => v,
                _ => { tracing::warn!("PDF (картка): компанія не знайдена."); return; }
            };
            let cp = match db::counterparties::get_by_id(&pool, act.counterparty_id).await {
                Ok(Some(v)) => v,
                _ => { tracing::warn!("PDF (картка): контрагент не знайдено."); return; }
            };
            let pdf_items: Vec<acta::pdf::generator::PdfActItem> = items
                .iter()
                .enumerate()
                .map(|(i, item)| acta::pdf::generator::PdfActItem {
                    num: (i + 1) as u32,
                    name: item.description.clone(),
                    qty: item.quantity.to_string(),
                    unit: item.unit.clone(),
                    price: item.unit_price.to_string(),
                    amount: item.amount.to_string(),
                })
                .collect();
            let data = acta::pdf::generator::PdfActData {
                number: act.number.clone(),
                date: act.date.format("%d.%m.%Y").to_string(),
                company: acta::pdf::generator::PdfCompany {
                    name: company.name.clone(),
                    edrpou: company.edrpou.unwrap_or_default(),
                    iban: company.iban.unwrap_or_default(),
                    address: company.legal_address.unwrap_or_default(),
                },
                client: acta::pdf::generator::PdfCompany {
                    name: cp.name.clone(),
                    edrpou: cp.edrpou.unwrap_or_default(),
                    iban: cp.iban.unwrap_or_default(),
                    address: cp.address.unwrap_or_default(),
                },
                items: pdf_items,
                total: format!("{:.2}", act.total_amount),
                total_words: acta::pdf::generator::amount_to_words(&act.total_amount),
                notes: act.notes.clone().unwrap_or_default(),
            };
            let output_path = match acta::pdf::generator::ensure_output_dir(&act.number) {
                Ok(p) => p,
                Err(e) => { tracing::error!("PDF (картка): директорія: {e}"); return; }
            };
            if let Err(e) = acta::pdf::generator::generate_act_pdf(&data, &output_path) {
                tracing::error!("PDF (картка): генерація: {e}"); return;
            }
            tracing::info!("PDF '{}' → {}", act.number, output_path.display());
            if let Err(e) = open::that(&output_path) {
                tracing::error!("PDF (картка): відкриття: {e}");
            }
            if let Err(e) = open_act_card(&pool, ui_weak, uuid).await {
                tracing::error!("Оновлення картки після PDF: {e}");
            }
        });
    });

    // ── Наступний статус з картки акту ────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let act_state = ctx.act_state.clone();
    ui.on_act_card_advance_status_clicked(move |id, new_status| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let id_str = id.to_string();
        let cid = *company_id_arc.lock().unwrap();
        let act_state_clone = act_state.clone();
        let new_status = act_status_from_ui(new_status);
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else { return; };
            if let Err(e) = db::acts::change_status(&pool, uuid, new_status).await {
                tracing::error!("Advance status (картка): {e}");
                return;
            }
            let state = act_state_clone.lock().unwrap().clone();
            let _ = tokio::join!(
                open_act_card(&pool, ui_weak.clone(), uuid),
                reload_acts(
                    &pool,
                    ui_weak.clone(),
                    cid,
                    state.status_filter,
                    state.query,
                    false
                ),
            );
        });
    });

    // ── Новий акт — відкрити форму ────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_create_clicked(move || {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        tokio::spawn(async move {
            let (cp_result, num_result, cat_result) = tokio::join!(
                db::acts::counterparties_for_select(&pool, cid),
                db::acts::generate_next_number(&pool, cid),
                db::categories::list_all_for_select(&pool, cid),
            );
            let counterparties = match cp_result {
                Ok(v) => v,
                Err(e) => { tracing::error!("Помилка завантаження контрагентів: {e}"); return; }
            };
            let next_number = match num_result {
                Ok(n) => n,
                Err(e) => { tracing::error!("Помилка генерації номеру: {e}"); return; }
            };
            let categories = cat_result.unwrap_or_default();

            let cp_names: Vec<SharedString> = counterparties
                .iter()
                .map(|(_, name)| SharedString::from(name.as_str()))
                .collect();
            let cp_ids: Vec<SharedString> = counterparties
                .iter()
                .map(|(id, _)| SharedString::from(id.to_string().as_str()))
                .collect();

            let mut cat_names: Vec<SharedString> =
                vec![SharedString::from("— без категорії —")];
            let mut cat_ids: Vec<SharedString> = vec![SharedString::from("")];
            for c in &categories {
                let prefix = if c.depth > 0 { "  └─ " } else { "" };
                cat_names.push(SharedString::from(format!("{}{}", prefix, c.name)));
                cat_ids.push(SharedString::from(c.id.to_string()));
            }

            let today = chrono::Local::now().date_naive().format("%d.%m.%Y").to_string();

            ui_handle
                .upgrade_in_event_loop(move |ui| {
                    ui.set_act_form_number(SharedString::from(next_number.as_str()));
                    ui.set_act_form_date(SharedString::from(today.as_str()));
                    ui.set_act_form_notes(SharedString::from(""));
                    ui.set_act_form_total(SharedString::from("0.00"));
                    ui.set_act_form_cp_index(0);
                    ui.set_act_form_is_edit(false);
                    ui.set_act_form_cp_names(ModelRc::new(VecModel::from(cp_names)));
                    ui.set_act_form_cp_ids(ModelRc::new(VecModel::from(cp_ids)));
                    ui.set_act_form_category_names(ModelRc::new(VecModel::from(cat_names)));
                    ui.set_act_form_category_ids(ModelRc::new(VecModel::from(cat_ids)));
                    ui.set_act_form_category_index(0);
                    ui.set_act_form_expected_payment_date(SharedString::from(""));
                    ui.set_act_form_items(ModelRc::new(VecModel::from(
                        Vec::<FormItemRow>::new(),
                    )));
                    ui.set_act_task_rows(ModelRc::new(VecModel::from(Vec::<TaskRow>::new())));
                    ui.set_act_tasks_loading(false);
                    ui.set_show_act_form(true);
                })
                .ok();
        });
    });

    // ── Наступний статус акту зі списку ──────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let act_state = ctx.act_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_advance_status_clicked(move |id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let id_str = id.to_string();
        let act_state = act_state.clone();
        let cid = *company_id_arc.lock().unwrap();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID акту: {id_str}");
                return;
            };
            match db::acts::advance_status(&pool, uuid).await {
                Ok(Some(act)) => {
                    tracing::info!(
                        "Акт {} переведено до статусу '{}'.",
                        act.number,
                        act.status.label()
                    );
                    show_toast(
                        ui_handle.clone(),
                        format!("Акт '{}' → {}", act.number, act.status.label()),
                        false,
                    );
                    let (query, status_filter) = {
                        let state = act_state.lock().unwrap();
                        (state.query.clone(), state.status_filter.clone())
                    };
                    if let Err(e) =
                        reload_acts(&pool, ui_handle.clone(), cid, status_filter, query, false)
                            .await
                    {
                        tracing::error!(
                            "Помилка оновлення списку актів після зміни статусу: {e}"
                        );
                    }
                }
                Ok(None) => tracing::warn!("Акт {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка зміни статусу: {e}"),
            }
        });
    });

    // ── PDF акту зі списку ────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_pdf_clicked(move |act_id| {
        let pool = pool.clone();
        let id_str = act_id.to_string();
        let cid = *company_id_arc.lock().unwrap();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("PDF: некоректний UUID акту: {id_str}");
                return;
            };
            let (act_result, company_result) = tokio::join!(
                db::acts::get_by_id(&pool, uuid),
                db::companies::get_by_id(&pool, cid)
            );
            let Some((act, items)) = (match act_result {
                Ok(v) => v,
                Err(e) => { tracing::error!("PDF: помилка завантаження акту: {e}"); return; }
            }) else {
                tracing::warn!("PDF: акт {uuid} не знайдено.");
                return;
            };
            let company = match company_result {
                Ok(Some(v)) => v,
                Ok(None) => { tracing::warn!("PDF: компанія {cid} не знайдена."); return; }
                Err(e) => { tracing::error!("PDF: помилка компанії: {e}"); return; }
            };
            let cp = match db::counterparties::get_by_id(&pool, act.counterparty_id).await {
                Ok(Some(v)) => v,
                Ok(None) => { tracing::warn!("PDF: контрагент не знайдено."); return; }
                Err(e) => { tracing::error!("PDF: помилка контрагента: {e}"); return; }
            };
            let pdf_items: Vec<acta::pdf::generator::PdfActItem> = items
                .iter()
                .enumerate()
                .map(|(i, item)| acta::pdf::generator::PdfActItem {
                    num: (i + 1) as u32,
                    name: item.description.clone(),
                    qty: item.quantity.to_string(),
                    unit: item.unit.clone(),
                    price: item.unit_price.to_string(),
                    amount: item.amount.to_string(),
                })
                .collect();
            let data = acta::pdf::generator::PdfActData {
                number: act.number.clone(),
                date: act.date.format("%d.%m.%Y").to_string(),
                company: acta::pdf::generator::PdfCompany {
                    name: company.name.clone(),
                    edrpou: company.edrpou.unwrap_or_default(),
                    iban: company.iban.unwrap_or_default(),
                    address: company.legal_address.unwrap_or_default(),
                },
                client: acta::pdf::generator::PdfCompany {
                    name: cp.name.clone(),
                    edrpou: cp.edrpou.unwrap_or_default(),
                    iban: cp.iban.unwrap_or_default(),
                    address: cp.address.unwrap_or_default(),
                },
                items: pdf_items,
                total: format!("{:.2}", act.total_amount),
                total_words: acta::pdf::generator::amount_to_words(&act.total_amount),
                notes: act.notes.unwrap_or_default(),
            };
            let output_path = match acta::pdf::generator::ensure_output_dir(&act.number) {
                Ok(p) => p,
                Err(e) => { tracing::error!("PDF: помилка директорії: {e}"); return; }
            };
            if let Err(e) = acta::pdf::generator::generate_act_pdf(&data, &output_path) {
                tracing::error!("PDF: помилка генерації: {e}");
                return;
            }
            tracing::info!("PDF '{}' → {}", act.number, output_path.display());
            if let Err(e) = open::that(&output_path) {
                tracing::error!("PDF: не вдалось відкрити файл: {e}");
            }
        });
    });

    // ── Відкрити акт для редагування ─────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_edit_clicked(move |act_id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let id_str = act_id.to_string();
        let cid = *company_id_arc.lock().unwrap();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID акту: {id_str}");
                return;
            };
            if let Err(e) = populate_act_form(&pool, ui_handle, uuid, cid, false).await {
                tracing::error!("Помилка підготовки форми акту: {e}");
            }
        });
    });

    // ── Оновити акт ──────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let act_state = ctx.act_state.clone();
    let doc_state = ctx.doc_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_form_update(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str, exp_date_str| {
        let Some(ui_ref) = ui_weak.upgrade() else { return; };
        let edit_id = ui_ref.get_act_form_edit_id().to_string();
        let items = collect_form_items(&ui_ref);
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let number = number.to_string();
        let date_str = date_str.to_string();
        let cp_id_str = cp_id_str.to_string();
        let notes_opt = if notes.trim().is_empty() { None } else { Some(notes.to_string()) };
        let cat_id_str = cat_id_str.to_string();
        let con_id_str = con_id_str.to_string();
        let exp_date_str = exp_date_str.to_string();
        let act_state = act_state.clone();
        let doc_state_spawn = doc_state.clone();
        tokio::spawn(async move {
            let Ok(uuid) = edit_id.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний edit_id: {edit_id}");
                return;
            };
            if number.trim().is_empty() {
                tracing::error!("Номер акту не може бути порожнім");
                return;
            }
            if date_str.trim().is_empty() {
                tracing::error!("Дата акту не може бути порожньою");
                return;
            }
            if cp_id_str.trim().is_empty() {
                tracing::error!("Контрагент не вибраний");
                return;
            }
            let date = match NaiveDate::parse_from_str(&date_str, "%d.%m.%Y") {
                Ok(d) => d,
                Err(_) => { tracing::error!("Невірний формат дати: '{date_str}'"); return; }
            };
            let cp_uuid = match uuid::Uuid::parse_str(&cp_id_str) {
                Ok(id) => id,
                Err(_) => { tracing::error!("Некоректний UUID контрагента: '{cp_id_str}'"); return; }
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
            let update_data = UpdateAct {
                number: number.clone(),
                counterparty_id: cp_uuid,
                contract_id: con_id_opt,
                category_id: cat_id_opt,
                date,
                expected_payment_date: exp_date_opt,
                notes: notes_opt,
            };
            match db::acts::update_with_items(&pool, uuid, update_data, items).await {
                Ok(act) => {
                    tracing::info!("Акт '{}' оновлено (id={}).", act.number, act.id);
                    show_toast(
                        ui_weak.clone(),
                        format!("Акт '{}' оновлено", act.number),
                        false,
                    );
                    let (query, status_filter) = {
                        let state = act_state.lock().unwrap();
                        (state.query.clone(), state.status_filter.clone())
                    };
                    if let Err(e) =
                        reload_acts(&pool, ui_weak.clone(), cid, status_filter, query, true).await
                    {
                        tracing::error!(
                            "Помилка оновлення списку актів після редагування: {e}"
                        );
                    }
                    let (doc_tab, doc_direction, doc_query, doc_cp, doc_df, doc_dt) = {
                        let s = doc_state_spawn.lock().unwrap();
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
                            "Помилка оновлення документів після редагування акту: {e}"
                        );
                    }
                }
                Err(e) => tracing::error!("Помилка оновлення акту: {e}"),
            }
        });
    });

    // ── Скасувати форму акту ──────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_act_form_cancel(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_show_act_form(false);
        }
    });

    // ── Додати позицію ─────────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_act_form_add_item(move || {
        let Some(ui) = ui_weak.upgrade() else { return; };
        let mut items: Vec<FormItemRow> = (0..ui.get_act_form_items().row_count())
            .filter_map(|j| ui.get_act_form_items().row_data(j))
            .collect();
        items.push(FormItemRow {
            description: SharedString::from("Нова послуга"),
            quantity: SharedString::from("1"),
            unit: SharedString::from("шт"),
            price: SharedString::from("0.00"),
            amount: SharedString::from("0.00"),
        });
        ui.set_act_form_items(ModelRc::new(VecModel::from(items)));
    });

    // ── Видалити позицію ──────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_act_form_remove_item(move |idx| {
        let Some(ui) = ui_weak.upgrade() else { return; };
        let i = idx as usize;
        let mut items: Vec<FormItemRow> = (0..ui.get_act_form_items().row_count())
            .filter_map(|j| ui.get_act_form_items().row_data(j))
            .collect();
        if i < items.len() {
            items.remove(i);
        }
        ui.set_act_form_items(ModelRc::new(VecModel::from(items)));
    });

    // ── Редагування поля позиції ──────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_act_form_item_changed(move |idx, field, value| {
        let Some(ui) = ui_weak.upgrade() else { return; };
        let i = idx as usize;
        let val = value.to_string();
        let mut items: Vec<FormItemRow> = (0..ui.get_act_form_items().row_count())
            .filter_map(|j| ui.get_act_form_items().row_data(j))
            .collect();
        if let Some(item) = items.get_mut(i) {
            match field.as_str() {
                "desc" => item.description = SharedString::from(val.as_str()),
                "qty" => item.quantity = SharedString::from(val.as_str()),
                "unit" => item.unit = SharedString::from(val.as_str()),
                "price" => item.price = SharedString::from(val.as_str()),
                _ => return,
            }
        } else {
            return;
        }
        if field == "qty" || field == "price" {
            let mut total = Decimal::ZERO;
            for it in &mut items {
                let qty = it.quantity.parse::<Decimal>().unwrap_or_default();
                let price = it.price.parse::<Decimal>().unwrap_or_default();
                let amt = qty * price;
                it.amount = SharedString::from(format!("{:.2}", amt).as_str());
                total += amt;
            }
            ui.set_act_form_total(SharedString::from(format!("{:.2}", total).as_str()));
        }
        ui.set_act_form_items(ModelRc::new(VecModel::from(items)));
    });

    // ── Зберегти акт ─────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let act_state = ctx.act_state.clone();
    let doc_state = ctx.doc_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_form_save(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str, exp_date_str| {
        let Some(ui_ref) = ui_weak.upgrade() else { return; };
        let items = collect_form_items(&ui_ref);
        let cid = *company_id_arc.lock().unwrap();
        spawn_save_act(
            pool.clone(),
            ui_weak.clone(),
            act_state.clone(),
            doc_state.clone(),
            cid,
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.trim().is_empty() { None } else { Some(notes.to_string()) },
            cat_id_str.to_string(),
            con_id_str.to_string(),
            exp_date_str.to_string(),
            items,
            false,
        );
    });

    // ── Зберегти як чернетку ─────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let act_state = ctx.act_state.clone();
    let doc_state = ctx.doc_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_act_form_save_draft(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str, exp_date_str| {
        let Some(ui_ref) = ui_weak.upgrade() else { return; };
        let items = collect_form_items(&ui_ref);
        let cid = *company_id_arc.lock().unwrap();
        spawn_save_act(
            pool.clone(),
            ui_weak.clone(),
            act_state.clone(),
            doc_state.clone(),
            cid,
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.trim().is_empty() { None } else { Some(notes.to_string()) },
            cat_id_str.to_string(),
            con_id_str.to_string(),
            exp_date_str.to_string(),
            items,
            true,
        );
    });

    // ── Колбеки задач в акті ──────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_act_task_create_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let act_id = ui.get_act_form_edit_id().to_string();
            ui.set_task_form_is_edit(false);
            ui.set_task_form_edit_id(SharedString::from(""));
            ui.set_task_form_title(SharedString::from(""));
            ui.set_task_form_description(SharedString::from(""));
            ui.set_task_form_priority_index(1);
            ui.set_task_form_due_date(SharedString::from(""));
            ui.set_task_form_reminder_at(SharedString::from(""));
            ui.set_task_form_act_id(SharedString::from(act_id.as_str()));
            ui.set_task_form_return_page(1);
            ui.set_current_page(5);
            ui.set_show_task_form(true);
        }
    });

    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    ui.on_act_task_edit_clicked(move |task_id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let id_str = task_id.to_string();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID задачі: {id_str}");
                return;
            };
            match db::tasks::get_by_id(&pool, uuid).await {
                Ok(Some(task)) => {
                    let due = format_task_datetime(task.due_date);
                    let reminder = format_task_datetime(task.reminder_at);
                    ui_handle
                        .upgrade_in_event_loop(move |ui| {
                            ui.set_task_form_is_edit(true);
                            ui.set_task_form_edit_id(SharedString::from(
                                task.id.to_string().as_str(),
                            ));
                            ui.set_task_form_title(SharedString::from(task.title.as_str()));
                            ui.set_task_form_description(SharedString::from(
                                task.description.as_deref().unwrap_or(""),
                            ));
                            ui.set_task_form_priority_index(task_priority_index(&task.priority));
                            ui.set_task_form_due_date(due);
                            ui.set_task_form_reminder_at(reminder);
                            ui.set_task_form_act_id(SharedString::from(
                                task.act_id
                                    .map(|id| id.to_string())
                                    .unwrap_or_default()
                                    .as_str(),
                            ));
                            ui.set_task_form_return_page(1);
                            ui.set_current_page(5);
                            ui.set_show_task_form(true);
                        })
                        .ok();
                }
                Ok(None) => tracing::warn!("Задачу {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка завантаження задачі: {e}"),
            }
        });
    });

    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    ui.on_act_task_toggle_status_clicked(move |task_id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let id_str = task_id.to_string();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID задачі: {id_str}");
                return;
            };
            if let Ok(Some(task)) = db::tasks::set_status(&pool, uuid, TaskStatus::Done).await {
                show_toast(
                    ui_handle.clone(),
                    format!("Задачу '{}' завершено", task.title),
                    false,
                );
                let act_id = ui_handle.upgrade().and_then(|ui| {
                    let act_id = ui.get_act_form_edit_id().to_string();
                    act_id.parse::<uuid::Uuid>().ok()
                });
                if let Some(act_id) = act_id {
                    if let Err(e) =
                        crate::ui::tasks::reload_act_tasks(&pool, ui_handle.clone(), act_id).await
                    {
                        tracing::error!("Помилка оновлення задач акту: {e}");
                    }
                }
            }
        });
    });

    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    ui.on_act_task_delete_clicked(move |task_id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let id_str = task_id.to_string();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID задачі: {id_str}");
                return;
            };
            match db::tasks::delete(&pool, uuid).await {
                Ok(true) => {
                    show_toast(ui_handle.clone(), "Задачу видалено".to_string(), false);
                    let act_id = ui_handle.upgrade().and_then(|ui| {
                        let act_id = ui.get_act_form_edit_id().to_string();
                        act_id.parse::<uuid::Uuid>().ok()
                    });
                    if let Some(act_id) = act_id {
                        if let Err(e) = crate::ui::tasks::reload_act_tasks(
                            &pool,
                            ui_handle.clone(),
                            act_id,
                        )
                        .await
                        {
                            tracing::error!(
                                "Помилка оновлення задач акту після видалення: {e}"
                            );
                        }
                    }
                }
                Ok(false) => tracing::warn!("Задачу {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка видалення задачі: {e}"),
            }
        });
    });
}
