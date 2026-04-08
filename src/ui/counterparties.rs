// ui/counterparties.rs — колбеки та дані для сторінки Контрагенти.

use std::sync::Arc;

use anyhow::Result;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

use crate::{
    app_ctx::AppCtx,
    ui::helpers::*,
    MainWindow,
};
use acta::db;

// ═══════════════════════════════════════════════════════════════════════════════
// ── Проміжні дані (Send-safe) ──────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct CounterpartiesUiData {
    pub table_data: TableData,
    pub total_all: i32,
    pub active_all: i32,
    pub archived_all: i32,
    pub pagination: SharedString,
    pub include_archived: bool,
    pub current_page: i32,
    pub total_pages: i32,
}

pub async fn prepare_counterparties_data(
    pool: &sqlx::PgPool,
    company_id: uuid::Uuid,
    query: String,
    include_archived: bool,
    page: usize,
) -> Result<CounterpartiesUiData> {
    let filter_query = normalized_query(&query);
    let all_counterparties =
        db::counterparties::list_filtered(pool, company_id, filter_query, true).await?;

    let total_all = all_counterparties.len();
    let active_all = all_counterparties.iter().filter(|cp| !cp.is_archived).count();
    let archived_all = all_counterparties.iter().filter(|cp| cp.is_archived).count();

    let filtered_counterparties: Vec<_> = if include_archived {
        all_counterparties
    } else {
        all_counterparties.into_iter().filter(|cp| !cp.is_archived).collect()
    };

    let current_page =
        page.min(total_filtered_pages(filtered_counterparties.len()).saturating_sub(1));
    let start = current_page * crate::COUNTERPARTY_PAGE_SIZE;
    let end = (start + crate::COUNTERPARTY_PAGE_SIZE).min(filtered_counterparties.len());
    let page_slice = if start < filtered_counterparties.len() {
        &filtered_counterparties[start..end]
    } else {
        &filtered_counterparties[0..0]
    };

    let table_data = to_table_data(page_slice);
    let total_pages = total_filtered_pages(filtered_counterparties.len()) as i32;
    let page_label = if filtered_counterparties.is_empty() {
        "Показано 0 з 0 контрагентів".to_string()
    } else {
        format!(
            "Показано {}-{} з {} контрагентів",
            start + 1,
            end,
            filtered_counterparties.len()
        )
    };

    Ok(CounterpartiesUiData {
        table_data,
        total_all: total_all as i32,
        active_all: active_all as i32,
        archived_all: archived_all as i32,
        pagination: SharedString::from(page_label.as_str()),
        include_archived,
        current_page: (current_page + 1) as i32,
        total_pages: total_pages.max(1),
    })
}

/// Застосувати дані контрагентів до UI. Викликати тільки з main thread.
pub fn apply_counterparties_to_ui(
    ui: &MainWindow,
    d: CounterpartiesUiData,
    close_form: bool,
) {
    let (rows, ids, archived) = build_models(d.table_data);
    ui.set_counterparty_rows(rows);
    ui.set_counterparty_ids(ids);
    ui.set_counterparty_archived(archived);
    ui.set_counterparty_total_count(d.total_all);
    ui.set_counterparty_active_count(d.active_all);
    ui.set_counterparty_archived_count(d.archived_all);
    ui.set_counterparty_pagination_text(d.pagination);
    ui.set_counterparty_show_archived(d.include_archived);
    ui.set_counterparty_current_page(d.current_page);
    ui.set_counterparty_total_pages(d.total_pages);
    if close_form {
        ui.set_show_cp_form(false);
    }
}

pub async fn reload_counterparties(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    query: String,
    include_archived: bool,
    page: usize,
    close_form: bool,
) -> Result<()> {
    let d = prepare_counterparties_data(pool, company_id, query, include_archived, page).await?;
    ui_weak
        .upgrade_in_event_loop(move |ui| apply_counterparties_to_ui(&ui, d, close_form))
        .map_err(anyhow::Error::from)
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── open_counterparty_card ─────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub async fn open_counterparty_card(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    counterparty_id: uuid::Uuid,
) -> Result<()> {
    use acta::models;
    use crate::{
        CounterpartyContractSummary, CounterpartyDocSummary, CounterpartyPaymentSummary,
        CounterpartyTaskSummary,
    };

    let counterparty = db::counterparties::get_by_id(pool, counterparty_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Контрагента не знайдено"))?;
    let acts =
        db::acts::list_filtered(pool, company_id, None, None, None, Some(counterparty_id), None, None)
            .await?;
    let invoices =
        db::invoices::list_filtered(pool, company_id, None, None, None, Some(counterparty_id), None, None)
            .await?;
    let payments = db::payments::list_by_counterparty(pool, company_id, counterparty_id).await?;
    let contracts =
        db::contracts::list_by_counterparty(pool, company_id, counterparty_id).await?;
    let tasks = db::tasks::list_by_counterparty(pool, counterparty_id).await?;

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_counterparty_card_id(SharedString::from(
                counterparty.id.to_string().as_str(),
            ));
            ui.set_counterparty_card_name(SharedString::from(counterparty.name.as_str()));
            ui.set_counterparty_card_edrpou(SharedString::from(
                counterparty.edrpou.as_deref().unwrap_or(""),
            ));
            ui.set_counterparty_card_ipn(SharedString::from(
                counterparty.ipn.as_deref().unwrap_or(""),
            ));
            ui.set_counterparty_card_iban(SharedString::from(
                counterparty.iban.as_deref().unwrap_or(""),
            ));
            ui.set_counterparty_card_phone(SharedString::from(
                counterparty.phone.as_deref().unwrap_or(""),
            ));
            ui.set_counterparty_card_email(SharedString::from(
                counterparty.email.as_deref().unwrap_or(""),
            ));
            ui.set_counterparty_card_address(SharedString::from(
                counterparty.address.as_deref().unwrap_or(""),
            ));
            ui.set_counterparty_card_stat_acts(SharedString::from(
                acts.len().to_string().as_str(),
            ));
            ui.set_counterparty_card_stat_invoices(SharedString::from(
                invoices.len().to_string().as_str(),
            ));
            ui.set_counterparty_card_stat_payments(SharedString::from(
                payments.len().to_string().as_str(),
            ));
            ui.set_counterparty_card_stat_contracts(SharedString::from(
                contracts.len().to_string().as_str(),
            ));

            ui.set_counterparty_card_acts(ModelRc::new(VecModel::from(
                acts.iter()
                    .map(|row| CounterpartyDocSummary {
                        number: SharedString::from(row.number.as_str()),
                        date: SharedString::from(row.date.format("%d.%m.%Y").to_string()),
                        amount: SharedString::from(format_amount_ua(row.total_amount)),
                        status: SharedString::from(row.status.label()),
                    })
                    .collect::<Vec<_>>(),
            )));

            ui.set_counterparty_card_invoices(ModelRc::new(VecModel::from(
                invoices
                    .iter()
                    .map(|row| CounterpartyDocSummary {
                        number: SharedString::from(row.number.as_str()),
                        date: SharedString::from(row.date.format("%d.%m.%Y").to_string()),
                        amount: SharedString::from(format_amount_ua(row.total_amount)),
                        status: SharedString::from(row.status.label()),
                    })
                    .collect::<Vec<_>>(),
            )));

            ui.set_counterparty_card_payments(ModelRc::new(VecModel::from(
                payments
                    .iter()
                    .map(|row| CounterpartyPaymentSummary {
                        date: SharedString::from(row.date.as_str()),
                        amount: SharedString::from(format_amount_ua(row.amount)),
                        direction: SharedString::from(row.direction.label()),
                        description: SharedString::from(
                            row.description.as_deref().unwrap_or(""),
                        ),
                    })
                    .collect::<Vec<_>>(),
            )));

            ui.set_counterparty_card_contracts(ModelRc::new(VecModel::from(
                contracts
                    .iter()
                    .map(|row| CounterpartyContractSummary {
                        number: SharedString::from(row.number.as_str()),
                        subject: SharedString::from(row.subject.as_deref().unwrap_or("")),
                        date: SharedString::from(row.date.as_str()),
                        amount: SharedString::from(
                            row.amount
                                .map(format_amount_ua)
                                .unwrap_or_else(|| "—".to_string()),
                        ),
                        status: SharedString::from(match row.status {
                            models::contract::ContractStatus::Active => "Активний",
                            models::contract::ContractStatus::Expired => "Завершений",
                            models::contract::ContractStatus::Terminated => "Розірваний",
                        }),
                    })
                    .collect::<Vec<_>>(),
            )));

            ui.set_counterparty_card_tasks(ModelRc::new(VecModel::from(
                tasks
                    .iter()
                    .map(|row| CounterpartyTaskSummary {
                        title: SharedString::from(row.title.as_str()),
                        due_date: SharedString::from(
                            row.due_date
                                .map(|date| date.format("%d.%m.%Y").to_string())
                                .unwrap_or_else(|| "—".to_string()),
                        ),
                        status: SharedString::from(row.status.label()),
                    })
                    .collect::<Vec<_>>(),
            )));

            ui.set_show_counterparty_card(true);
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── setup — реєстрація колбеків ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn setup(ui: &MainWindow, ctx: Arc<AppCtx>) {
    // ── Пошук ─────────────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.counterparty_state.clone();
    let cid_arc = ctx.active_company_id.clone();
    ui.on_counterparty_search_changed(move |query| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let cid = *cid_arc.lock().unwrap();
        let (query_str, include_archived) = {
            let mut s = state.lock().unwrap();
            s.query = query.to_string();
            s.page = 0;
            (s.query.clone(), s.include_archived)
        };
        tokio::spawn(async move {
            if let Err(e) =
                reload_counterparties(&pool, ui_handle, cid, query_str, include_archived, 0, false)
                    .await
            {
                tracing::error!("Помилка пошуку: {e}");
            }
        });
    });

    // ── Вибір контрагента — відкрити картку ────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let cid_arc = ctx.active_company_id.clone();
    ui.on_counterparty_selected(move |id| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *cid_arc.lock().unwrap();
        let id_str = id.to_string();
        tokio::spawn(async move {
            let Ok(counterparty_id) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID контрагента: {id_str}");
                return;
            };
            if let Err(e) = open_counterparty_card(&pool, ui_weak, cid, counterparty_id).await {
                tracing::error!("Помилка відкриття картки контрагента: {e}");
            }
        });
    });

    // ── Редагувати контрагента ─────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    ui.on_counterparty_edit_clicked(move |id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let id_str = id.to_string();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID контрагента: {id_str}");
                return;
            };
            match db::counterparties::get_by_id(&pool, uuid).await {
                Ok(Some(cp)) => {
                    ui_handle
                        .upgrade_in_event_loop(move |ui| {
                            ui.set_cp_form_name(SharedString::from(cp.name.as_str()));
                            ui.set_cp_form_edrpou(SharedString::from(
                                cp.edrpou.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_ipn(SharedString::from(
                                cp.ipn.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_iban(SharedString::from(
                                cp.iban.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_phone(SharedString::from(
                                cp.phone.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_email(SharedString::from(
                                cp.email.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_address(SharedString::from(
                                cp.address.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_notes(SharedString::from(
                                cp.notes.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_edit_id(SharedString::from(
                                cp.id.to_string().as_str(),
                            ));
                            ui.set_cp_form_is_edit(true);
                            ui.set_show_cp_form(true);
                        })
                        .ok();
                }
                Ok(None) => tracing::warn!("Контрагента {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка завантаження контрагента: {e}"),
            }
        });
    });

    // ── Закрити картку контрагента ─────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_counterparty_card_close_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_show_counterparty_card(false);
        }
    });

    // ── Редагувати з картки ────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    ui.on_counterparty_card_edit_clicked(move |id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let id_str = id.to_string();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID контрагента для редагування: {id_str}");
                return;
            };
            match db::counterparties::get_by_id(&pool, uuid).await {
                Ok(Some(cp)) => {
                    ui_handle
                        .upgrade_in_event_loop(move |ui| {
                            ui.set_cp_form_name(SharedString::from(cp.name.as_str()));
                            ui.set_cp_form_edrpou(SharedString::from(
                                cp.edrpou.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_ipn(SharedString::from(
                                cp.ipn.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_iban(SharedString::from(
                                cp.iban.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_phone(SharedString::from(
                                cp.phone.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_email(SharedString::from(
                                cp.email.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_address(SharedString::from(
                                cp.address.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_notes(SharedString::from(
                                cp.notes.as_deref().unwrap_or(""),
                            ));
                            ui.set_cp_form_edit_id(SharedString::from(
                                cp.id.to_string().as_str(),
                            ));
                            ui.set_cp_form_is_edit(true);
                            ui.set_show_counterparty_card(false);
                            ui.set_show_cp_form(true);
                        })
                        .ok();
                }
                Ok(None) => tracing::warn!("Контрагента {uuid} не знайдено."),
                Err(e) => {
                    tracing::error!("Помилка відкриття контрагента на редагування: {e}")
                }
            }
        });
    });

    // ── Створити нового контрагента ────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_counterparty_create_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_cp_form_name(SharedString::from(""));
            ui.set_cp_form_edrpou(SharedString::from(""));
            ui.set_cp_form_ipn(SharedString::from(""));
            ui.set_cp_form_iban(SharedString::from(""));
            ui.set_cp_form_phone(SharedString::from(""));
            ui.set_cp_form_email(SharedString::from(""));
            ui.set_cp_form_address(SharedString::from(""));
            ui.set_cp_form_notes(SharedString::from(""));
            ui.set_cp_form_edit_id(SharedString::from(""));
            ui.set_cp_form_is_edit(false);
            ui.set_show_cp_form(true);
        }
    });

    // ── Фільтр (архівовані) ────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.counterparty_state.clone();
    let cid_arc = ctx.active_company_id.clone();
    ui.on_counterparty_filter_clicked(move || {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let cid = *cid_arc.lock().unwrap();
        let (query, include_archived) = {
            let mut s = state.lock().unwrap();
            s.include_archived = !s.include_archived;
            s.page = 0;
            (s.query.clone(), s.include_archived)
        };
        tokio::spawn(async move {
            if let Err(e) =
                reload_counterparties(&pool, ui_handle, cid, query, include_archived, 0, false)
                    .await
            {
                tracing::error!("Помилка фільтра контрагентів: {e}");
            }
        });
    });

    // ── Попередня сторінка ─────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.counterparty_state.clone();
    let cid_arc = ctx.active_company_id.clone();
    ui.on_counterparty_prev_page_clicked(move || {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let cid = *cid_arc.lock().unwrap();
        let (query, include_archived, page, should_reload) = {
            let mut s = state.lock().unwrap();
            if s.page == 0 {
                (s.query.clone(), s.include_archived, s.page, false)
            } else {
                s.page -= 1;
                (s.query.clone(), s.include_archived, s.page, true)
            }
        };
        if should_reload {
            tokio::spawn(async move {
                if let Err(e) =
                    reload_counterparties(&pool, ui_handle, cid, query, include_archived, page, false)
                        .await
                {
                    tracing::error!("Помилка пагінації контрагентів: {e}");
                }
            });
        }
    });

    // ── Наступна сторінка ──────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.counterparty_state.clone();
    let cid_arc = ctx.active_company_id.clone();
    ui.on_counterparty_next_page_clicked(move || {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let cid = *cid_arc.lock().unwrap();
        let (query, include_archived, page) = {
            let mut s = state.lock().unwrap();
            s.page += 1;
            (s.query.clone(), s.include_archived, s.page)
        };
        tokio::spawn(async move {
            if let Err(e) =
                reload_counterparties(&pool, ui_handle, cid, query, include_archived, page, false)
                    .await
            {
                tracing::error!("Помилка пагінації контрагентів: {e}");
            }
        });
    });

    // ── Скасувати форму ────────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_cp_form_cancel(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_show_cp_form(false);
        }
    });

    // ── Зберегти нового контрагента ────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.counterparty_state.clone();
    let cid_arc = ctx.active_company_id.clone();
    ui.on_cp_form_save(move |name, edrpou, ipn, iban, phone, email, address, notes| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let state = state.clone();
        let cid = *cid_arc.lock().unwrap();
        let name_s = name.to_string();
        let edrpou_s = edrpou.to_string();
        let ipn_s = ipn.to_string();
        let iban_s = iban.to_string();
        let phone_s = phone.to_string();
        let email_s = email.to_string();
        let address_s = address.to_string();
        let notes_s = notes.to_string();
        tokio::spawn(async move {
            if name_s.trim().is_empty() {
                show_toast(ui_weak, "Введіть назву контрагента".to_string(), true);
                return;
            }
            let data = acta::models::NewCounterparty {
                name: name_s.clone(),
                edrpou: if edrpou_s.trim().is_empty() { None } else { Some(edrpou_s) },
                ipn: if ipn_s.trim().is_empty() { None } else { Some(ipn_s) },
                iban: if iban_s.trim().is_empty() { None } else { Some(iban_s) },
                phone: if phone_s.trim().is_empty() { None } else { Some(phone_s) },
                email: if email_s.trim().is_empty() { None } else { Some(email_s) },
                address: if address_s.trim().is_empty() { None } else { Some(address_s) },
                notes: if notes_s.trim().is_empty() { None } else { Some(notes_s) },
                bas_id: None,
            };
            match db::counterparties::create(&pool, cid, &data).await {
                Ok(cp) => {
                    tracing::info!("Контрагента '{}' створено (id={}).", cp.name, cp.id);
                    show_toast(
                        ui_weak.clone(),
                        format!("Контрагента '{}' створено", cp.name),
                        false,
                    );
                    let (query, include_archived, page) = {
                        let s = state.lock().unwrap();
                        (s.query.clone(), s.include_archived, s.page)
                    };
                    if let Err(e) =
                        reload_counterparties(&pool, ui_weak.clone(), cid, query, include_archived, page, true)
                            .await
                    {
                        tracing::error!(
                            "Помилка оновлення списку контрагентів після створення: {e}"
                        );
                    }
                    if let Err(e) =
                        crate::ui::payments::reload_payment_counterparty_options(
                            &pool,
                            ui_weak.clone(),
                            cid,
                        )
                        .await
                    {
                        tracing::error!(
                            "Помилка оновлення контрагентів для форми платежу після створення: {e}"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("Помилка створення контрагента: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });

    // ── Оновити контрагента ────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.counterparty_state.clone();
    let cid_arc = ctx.active_company_id.clone();
    ui.on_cp_form_update(move |name, edrpou, ipn, iban, phone, email, address, notes| {
        let Some(ui_ref) = ui_weak.upgrade() else { return; };
        let edit_id = ui_ref.get_cp_form_edit_id().to_string();
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *cid_arc.lock().unwrap();
        let state = state.clone();
        let name_s = name.to_string();
        let edrpou_s = edrpou.to_string();
        let ipn_s = ipn.to_string();
        let iban_s = iban.to_string();
        let phone_s = phone.to_string();
        let email_s = email.to_string();
        let address_s = address.to_string();
        let notes_s = notes.to_string();
        tokio::spawn(async move {
            let Ok(uuid) = edit_id.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний edit_id: {edit_id}");
                return;
            };
            if name_s.trim().is_empty() {
                show_toast(ui_weak, "Введіть назву контрагента".to_string(), true);
                return;
            }
            let data = acta::models::UpdateCounterparty {
                name: name_s,
                edrpou: if edrpou_s.trim().is_empty() { None } else { Some(edrpou_s) },
                ipn: if ipn_s.trim().is_empty() { None } else { Some(ipn_s) },
                iban: if iban_s.trim().is_empty() { None } else { Some(iban_s) },
                phone: if phone_s.trim().is_empty() { None } else { Some(phone_s) },
                email: if email_s.trim().is_empty() { None } else { Some(email_s) },
                address: if address_s.trim().is_empty() { None } else { Some(address_s) },
                notes: if notes_s.trim().is_empty() { None } else { Some(notes_s) },
            };
            match db::counterparties::update(&pool, uuid, &data).await {
                Ok(Some(cp)) => {
                    tracing::info!("Контрагента '{}' оновлено (id={}).", cp.name, cp.id);
                    show_toast(
                        ui_weak.clone(),
                        format!("Контрагента '{}' оновлено", cp.name),
                        false,
                    );
                    let (query, include_archived, page) = {
                        let s = state.lock().unwrap();
                        (s.query.clone(), s.include_archived, s.page)
                    };
                    if let Err(e) =
                        reload_counterparties(&pool, ui_weak.clone(), cid, query, include_archived, page, true)
                            .await
                    {
                        tracing::error!(
                            "Помилка оновлення списку контрагентів після редагування: {e}"
                        );
                    }
                    if let Err(e) =
                        crate::ui::payments::reload_payment_counterparty_options(
                            &pool,
                            ui_weak.clone(),
                            cid,
                        )
                        .await
                    {
                        tracing::error!(
                            "Помилка оновлення контрагентів для форми платежу після редагування: {e}"
                        );
                    }
                }
                Ok(None) => tracing::warn!("Контрагента {uuid} не знайдено."),
                Err(e) => {
                    tracing::error!("Помилка оновлення контрагента: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });

    // ── Архівувати контрагента ─────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = ctx.counterparty_state.clone();
    let cid_arc = ctx.active_company_id.clone();
    ui.on_counterparty_archive_clicked(move |id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let state = state.clone();
        let cid = *cid_arc.lock().unwrap();
        let id_str = id.to_string();
        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID: {id_str}");
                return;
            };
            match db::counterparties::archive(&pool, uuid).await {
                Ok(true) => {
                    tracing::info!("Контрагента {uuid} архівовано.");
                    show_toast(ui_handle.clone(), "Контрагента архівовано".to_string(), false);
                    let (query, include_archived, page) = {
                        let s = state.lock().unwrap();
                        (s.query.clone(), s.include_archived, s.page)
                    };
                    if let Err(e) = reload_counterparties(
                        &pool,
                        ui_handle.clone(),
                        cid,
                        query,
                        include_archived,
                        page,
                        false,
                    )
                    .await
                    {
                        tracing::error!(
                            "Помилка оновлення списку контрагентів після архівування: {e}"
                        );
                    }
                    if let Err(e) =
                        crate::ui::payments::reload_payment_counterparty_options(
                            &pool,
                            ui_handle.clone(),
                            cid,
                        )
                        .await
                    {
                        tracing::error!(
                            "Помилка оновлення контрагентів для форми платежу після архівування: {e}"
                        );
                    }
                }
                Ok(false) => tracing::warn!("Контрагента {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка архівування: {e}"),
            }
        });
    });
}
