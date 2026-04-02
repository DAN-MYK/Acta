// Acta — програма управлінського обліку
//
// Підключаємо Rust типи, згенеровані з .slint файлів.
// Після цього доступний MainWindow (та інші export компоненти).
// ВАЖЛИВО: має бути на рівні модуля — не всередині функції.
slint::include_modules!();

use acta::{config::AppConfig, db, models, notifications};

use anyhow::Result;
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use slint::{Model, ModelRc, SharedString, StandardListViewItem, VecModel, Weak};
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};

use models::{
    ActStatus, Company, CompanySummary, InvoiceStatus, NewAct, NewActItem, NewCompany,
    NewCounterparty, NewInvoice, NewInvoiceItem, NewTask, Task, TaskPriority, TaskStatus,
    UpdateAct, UpdateCompany, UpdateCounterparty, UpdateInvoice,
};

// UUID дефолтної компанії (з міграції 012) — використовується якщо ще не обрано іншу.
const DEFAULT_COMPANY_ID: uuid::Uuid =
    uuid::Uuid::from_bytes([0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,1]);
const COUNTERPARTY_PAGE_SIZE: usize = 10;

#[derive(Clone, Default)]
struct CounterpartyListState {
    query: String,
    include_archived: bool,
    page: usize,
}

#[derive(Clone, Default)]
struct ActListState {
    query: String,
    status_filter: Option<ActStatus>,
}

#[derive(Clone, Default)]
struct InvoiceListState {
    query: String,
    status_filter: Option<models::InvoiceStatus>,
}

#[derive(Clone, Default)]
struct TaskListState {
    query: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let _ = dotenvy::dotenv();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL не задано. Перевір .env файл.");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Міграції застосовано.");

    tokio::spawn(notifications::reminder_loop(Arc::new(pool.clone())));

    // ── Створення вікна ──────────────────────────────────────────────────────
    // MainWindow — тип згенерований з ui/main.slint
    let ui = MainWindow::new()?;
    ui.set_counterparty_show_archived(false);

    let counterparty_state = Arc::new(Mutex::new(CounterpartyListState::default()));
    let act_state = Arc::new(Mutex::new(ActListState::default()));
    let invoice_state = Arc::new(Mutex::new(InvoiceListState::default()));
    let task_state = Arc::new(Mutex::new(TaskListState::default()));

    // ── Активна компанія — спільна між усіма callbacks ───────────────────────
    // Починаємо з DEFAULT_COMPANY_ID (дефолтна компанія з міграції).
    // При виборі компанії в UI → оновлюємо цей Arc.
    let active_company_id: Arc<Mutex<uuid::Uuid>> =
        Arc::new(Mutex::new(DEFAULT_COMPANY_ID));

    // ── Початкове завантаження компаній ─────────────────────────────────────
    let config = AppConfig::load();
    {
        let companies = db::companies::list(&pool).await.unwrap_or_default();
        let company_rows = db::companies::list_with_summary(&pool).await.unwrap_or_default();
        // Відображаємо список у UI (для сторінки Компанії)
        apply_company_rows(&ui, &company_rows, *active_company_id.lock().unwrap());
        ui.set_active_company_subtitle(SharedString::from(
            "Оберіть компанію для роботи",
        ));

        match companies.len() {
            0 => {
                // Немає жодної компанії → одразу на сторінку Компанії (создати)
                ui.set_current_page(6);
                ui.set_active_company_name(SharedString::from("Оберіть компанію"));
                ui.set_active_company_id(SharedString::from(""));
                ui.set_active_company_subtitle(SharedString::from(
                    "Створіть першу компанію",
                ));
                reset_company_form(&ui);
                ui.set_show_company_form(true);
            }
            1 => {
                // Єдина компанія — обираємо автоматично
                let c = &companies[0];
                *active_company_id.lock().unwrap() = c.id;
                ui.set_active_company_name(SharedString::from(
                    company_display_name(c).as_str(),
                ));
                ui.set_active_company_id(SharedString::from(c.id.to_string().as_str()));
                ui.set_active_company_subtitle(SharedString::from(
                    company_subtitle(c).as_str(),
                ));
                tracing::info!("Активна компанія: '{}'", c.name);
            }
            _ => {
                // Кілька компаній — відновити останню або показати вибір
                let restored = config.last_company_id.and_then(|lid| {
                    companies.iter().find(|c| c.id == lid).cloned()
                });
                if let Some(c) = restored {
                    *active_company_id.lock().unwrap() = c.id;
                    ui.set_active_company_name(SharedString::from(
                        company_display_name(&c).as_str(),
                    ));
                    ui.set_active_company_id(SharedString::from(c.id.to_string().as_str()));
                    ui.set_active_company_subtitle(SharedString::from(
                        company_subtitle(&c).as_str(),
                    ));
                    tracing::info!("Відновлено останню компанію: '{}'", c.name);
                } else {
                    ui.set_show_company_picker(true);
                    ui.set_active_company_name(SharedString::from("Оберіть компанію"));
                    ui.set_active_company_id(SharedString::from(""));
                    ui.set_active_company_subtitle(SharedString::from(
                        "Доступно кілька компаній",
                    ));
                }
            }
        }
    }

    // ── Початкове завантаження ───────────────────────────────────────────────
    // Тут ми ще в main thread (до ui.run()), тому ModelRc будувати безпечно.
    let cid = *active_company_id.lock().unwrap();
    reload_counterparties(&pool, ui.as_weak(), cid, String::new(), false, 0, false).await?;

    // ── Початкове завантаження актів ─────────────────────────────────────────
    reload_acts(&pool, ui.as_weak(), cid, None, String::new(), false).await?;

    // ── Початкове завантаження накладних ─────────────────────────────────────
    reload_invoices(&pool, ui.as_weak(), cid, None, String::new(), false).await?;

    // ── Початкове завантаження задач ────────────────────────────────────────
    ui.set_tasks_loading(true);
    reload_tasks(&pool, ui.as_weak(), String::new(), false).await?;

    // ── Колбек: пошук ────────────────────────────────────────────────────────
    //
    // Ключова проблема async + Slint:
    //   Slint event loop → main thread (не Send)
    //   tokio::spawn    → інший потік (потребує Send)
    //
    // ModelRc базується на Rc (не atomic), тому НЕ є Send.
    // Рішення: передаємо між потоками Vec<_> (Send),
    //   а ModelRc будуємо всередині upgrade_in_event_loop (main thread).
    let pool_search = pool.clone();
    let ui_weak = ui.as_weak();
    let counterparty_state_search = counterparty_state.clone();
    let active_company_id_search = active_company_id.clone();

    ui.on_counterparty_search_changed(move |query| {
        let pool = pool_search.clone();
        let ui_handle = ui_weak.clone();
        let cid = *active_company_id_search.lock().unwrap();
        let (query_str, include_archived) = {
            let mut state = counterparty_state_search.lock().unwrap();
            state.query = query.to_string();
            state.page = 0;
            (state.query.clone(), state.include_archived)
        };

        tokio::spawn(async move {
            if let Err(e) =
                reload_counterparties(&pool, ui_handle, cid, query_str, include_archived, 0, false).await
            {
                tracing::error!("Помилка пошуку: {e}");
            }
        });
    });

    // ── Колбек: вибір контрагента — лише виділити рядок у UI ───────────────
    ui.on_counterparty_selected(|id| {
        tracing::debug!("Вибрано контрагента: {id}");
    });

    // ── Колбек: відкрити контрагента для редагування ────────────────────────
    let pool_cp_select = pool.clone();
    let ui_weak_cp_select = ui.as_weak();

    ui.on_counterparty_edit_clicked(move |id| {
        let pool = pool_cp_select.clone();
        let ui_handle = ui_weak_cp_select.clone();
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
                            ui.set_cp_form_edit_id(SharedString::from(cp.id.to_string().as_str()));
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

    // ── Колбек: новий контрагент — відкрити порожню форму ───────────────────
    let ui_weak_cp_create = ui.as_weak();

    ui.on_counterparty_create_clicked(move || {
        if let Some(ui) = ui_weak_cp_create.upgrade() {
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

    // ── Колбек: фільтр контрагентів ──────────────────────────────────────────
    let pool_cp_filter = pool.clone();
    let ui_weak_cp_filter = ui.as_weak();
    let counterparty_state_filter = counterparty_state.clone();
    let active_company_id_cp_filter = active_company_id.clone();

    ui.on_counterparty_filter_clicked(move || {
        let pool = pool_cp_filter.clone();
        let ui_handle = ui_weak_cp_filter.clone();
        let cid = *active_company_id_cp_filter.lock().unwrap();
        let (query, include_archived) = {
            let mut state = counterparty_state_filter.lock().unwrap();
            state.include_archived = !state.include_archived;
            state.page = 0;
            (state.query.clone(), state.include_archived)
        };

        tokio::spawn(async move {
            if let Err(e) =
                reload_counterparties(&pool, ui_handle, cid, query, include_archived, 0, false).await
            {
                tracing::error!("Помилка фільтра контрагентів: {e}");
            }
        });
    });

    let pool_cp_prev_page = pool.clone();
    let ui_weak_cp_prev_page = ui.as_weak();
    let counterparty_state_prev_page = counterparty_state.clone();
    let active_company_id_prev_page = active_company_id.clone();
    ui.on_counterparty_prev_page_clicked(move || {
        let pool = pool_cp_prev_page.clone();
        let ui_handle = ui_weak_cp_prev_page.clone();
        let cid = *active_company_id_prev_page.lock().unwrap();
        let (query, include_archived, page, should_reload) = {
            let mut state = counterparty_state_prev_page.lock().unwrap();
            if state.page == 0 {
                (state.query.clone(), state.include_archived, state.page, false)
            } else {
                state.page -= 1;
                (state.query.clone(), state.include_archived, state.page, true)
            }
        };

        if should_reload {
            tokio::spawn(async move {
                if let Err(e) =
                    reload_counterparties(&pool, ui_handle, cid, query, include_archived, page, false).await
                {
                    tracing::error!("Помилка пагінації контрагентів: {e}");
                }
            });
        }
    });

    let pool_cp_next_page = pool.clone();
    let ui_weak_cp_next_page = ui.as_weak();
    let counterparty_state_next_page = counterparty_state.clone();
    let active_company_id_next_page = active_company_id.clone();
    ui.on_counterparty_next_page_clicked(move || {
        let pool = pool_cp_next_page.clone();
        let ui_handle = ui_weak_cp_next_page.clone();
        let cid = *active_company_id_next_page.lock().unwrap();
        let (query, include_archived, page) = {
            let mut state = counterparty_state_next_page.lock().unwrap();
            state.page += 1;
            (state.query.clone(), state.include_archived, state.page)
        };

        tokio::spawn(async move {
            if let Err(e) =
                reload_counterparties(&pool, ui_handle, cid, query, include_archived, page, false).await
            {
                tracing::error!("Помилка пагінації контрагентів: {e}");
            }
        });
    });

    // ── Колбек: фільтр статусу актів ─────────────────────────────────────────
    //
    // Індекс ComboBox: 0=Усі, 1=Чернетка, 2=Виставлено, 3=Підписано, 4=Оплачено
    let pool_acts_filter = pool.clone();
    let ui_weak_acts_filter = ui.as_weak();
    let act_state_filter = act_state.clone();
    let active_company_id_acts_filter = active_company_id.clone();

    ui.on_act_status_filter_changed(move |filter_idx| {
        let pool = pool_acts_filter.clone();
        let ui_handle = ui_weak_acts_filter.clone();
        let cid = *active_company_id_acts_filter.lock().unwrap();

        // Перетворюємо індекс ComboBox в Option<ActStatus>
        let status_filter = match filter_idx {
            1 => Some(ActStatus::Draft),
            2 => Some(ActStatus::Issued),
            3 => Some(ActStatus::Signed),
            4 => Some(ActStatus::Paid),
            _ => None, // 0 = "Усі"
        };
        let query = {
            let mut state = act_state_filter.lock().unwrap();
            state.status_filter = status_filter.clone();
            state.query.clone()
        };

        tokio::spawn(async move {
            if let Err(e) = reload_acts(&pool, ui_handle, cid, status_filter, query, false).await {
                tracing::error!("Помилка фільтру актів: {e}");
            }
        });
    });

    let pool_acts_search = pool.clone();
    let ui_weak_acts_search = ui.as_weak();
    let act_state_search = act_state.clone();
    let active_company_id_acts_search = active_company_id.clone();

    ui.on_act_search_changed(move |query| {
        let pool = pool_acts_search.clone();
        let ui_handle = ui_weak_acts_search.clone();
        let cid = *active_company_id_acts_search.lock().unwrap();
        let (query, status_filter) = {
            let mut state = act_state_search.lock().unwrap();
            state.query = query.to_string();
            (state.query.clone(), state.status_filter.clone())
        };

        tokio::spawn(async move {
            if let Err(e) = reload_acts(&pool, ui_handle, cid, status_filter, query, false).await {
                tracing::error!("Помилка пошуку актів: {e}");
            }
        });
    });

    // ── Колбек: вибір акту ───────────────────────────────────────────────────
    ui.on_act_selected(|id| {
        tracing::debug!("Вибрано акт: {id}");
        // TODO: відкрити картку акту
    });

    // ── Колбек: новий акт — відкрити форму ──────────────────────────────────
    //
    // Перед показом форми потрібно:
    //   1. Завантажити список контрагентів для ComboBox
    //   2. Згенерувати наступний номер акту
    // Обидві операції виконуються паралельно через tokio::join!
    let pool_create_act = pool.clone();
    let ui_weak_create_act = ui.as_weak();
    let active_company_id_create = active_company_id.clone();

    ui.on_act_create_clicked(move || {
        let pool = pool_create_act.clone();
        let ui_handle = ui_weak_create_act.clone();
        let cid = *active_company_id_create.lock().unwrap();

        tokio::spawn(async move {
            // tokio::join! — запускає обидва futures паралельно (не послідовно)
            let (cp_result, num_result) = tokio::join!(
                db::acts::counterparties_for_select(&pool, cid),
                db::acts::generate_next_number(&pool, cid),
            );

            let counterparties = match cp_result {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Помилка завантаження контрагентів: {e}");
                    return;
                }
            };
            let next_number = match num_result {
                Ok(n) => n,
                Err(e) => {
                    tracing::error!("Помилка генерації номеру: {e}");
                    return;
                }
            };

            // Розбиваємо Vec<(Uuid, String)> на два паралельних Vec<SharedString>
            let cp_names: Vec<SharedString> = counterparties
                .iter()
                .map(|(_, name)| SharedString::from(name.as_str()))
                .collect();
            let cp_ids: Vec<SharedString> = counterparties
                .iter()
                .map(|(id, _)| SharedString::from(id.to_string().as_str()))
                .collect();

            // Сьогоднішня дата у форматі ДД.ММ.РРРР — обчислюємо до closure (sync)
            let today = chrono::Local::now()
                .date_naive()
                .format("%d.%m.%Y")
                .to_string();

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
                    // Очищаємо позиції з попереднього відкриття форми
                    let empty: Vec<SharedString> = vec![];
                    ui.set_act_form_item_descriptions(ModelRc::new(VecModel::from(empty.clone())));
                    ui.set_act_form_item_quantities(ModelRc::new(VecModel::from(empty.clone())));
                    ui.set_act_form_item_units(ModelRc::new(VecModel::from(empty.clone())));
                    ui.set_act_form_item_prices(ModelRc::new(VecModel::from(empty.clone())));
                    ui.set_act_form_item_amounts(ModelRc::new(VecModel::from(empty)));
                    ui.set_act_task_rows(ModelRc::new(VecModel::from(Vec::<
                        ModelRc<StandardListViewItem>,
                    >::new(
                    ))));
                    ui.set_act_task_row_ids(ModelRc::new(VecModel::from(
                        Vec::<SharedString>::new(),
                    )));
                    ui.set_act_task_row_statuses(ModelRc::new(VecModel::from(
                        Vec::<SharedString>::new(),
                    )));
                    ui.set_act_task_row_priorities(ModelRc::new(VecModel::from(
                        Vec::<SharedString>::new(),
                    )));
                    ui.set_act_tasks_loading(false);
                    // Перемикаємо на форму
                    ui.set_show_act_form(true);
                })
                .ok();
        });
    });

    // ── Колбек: наступний статус акту ────────────────────────────────────────
    let pool_acts_status = pool.clone();
    let ui_weak_acts_status = ui.as_weak();
    let act_state_advance = act_state.clone();
    let active_company_id_advance = active_company_id.clone();

    ui.on_act_advance_status_clicked(move |id| {
        let pool = pool_acts_status.clone();
        let ui_handle = ui_weak_acts_status.clone();
        let id_str = id.to_string();
        let act_state = act_state_advance.clone();
        let cid = *active_company_id_advance.lock().unwrap();

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
                        reload_acts(&pool, ui_handle.clone(), cid, status_filter, query, false).await
                    {
                        tracing::error!("Помилка оновлення списку актів після зміни статусу: {e}");
                    }
                }
                Ok(None) => tracing::warn!("Акт {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка зміни статусу: {e}"),
            }
        });
    });

    // ── Колбек: відкрити акт для редагування ────────────────────────────────
    //
    // Паралельно завантажуємо акт з позиціями та список контрагентів,
    // потім заповнюємо всі поля форми та перемикаємось у edit-mode.
    let pool_edit = pool.clone();
    let ui_weak_edit = ui.as_weak();
    let active_company_id_edit = active_company_id.clone();

    ui.on_act_edit_clicked(move |act_id| {
        let pool = pool_edit.clone();
        let ui_handle = ui_weak_edit.clone();
        let id_str = act_id.to_string();
        let cid = *active_company_id_edit.lock().unwrap();

        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID акту: {id_str}");
                return;
            };

            // tokio::join! — два незалежних запити паралельно (урок 2026-04-01)
            let (act_result, cp_result, tasks_result) = tokio::join!(
                db::acts::get_for_edit(&pool, uuid),
                db::acts::counterparties_for_select(&pool, cid),
                db::tasks::list_by_act(&pool, uuid),
            );

            let act_opt = match act_result {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Помилка завантаження акту: {e}");
                    return;
                }
            };
            let counterparties = match cp_result {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Помилка завантаження контрагентів: {e}");
                    return;
                }
            };
            let tasks = match tasks_result {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Помилка завантаження задач акту: {e}");
                    return;
                }
            };

            let Some((act, items)) = act_opt else {
                tracing::warn!("Акт {uuid} не знайдено.");
                return;
            };

            let cp_names: Vec<SharedString> = counterparties
                .iter()
                .map(|(_, n)| SharedString::from(n.as_str()))
                .collect();
            let cp_ids: Vec<SharedString> = counterparties
                .iter()
                .map(|(id, _)| SharedString::from(id.to_string().as_str()))
                .collect();

            // Шукаємо індекс контрагента акту у відсортованому списку
            let cp_index = counterparties
                .iter()
                .position(|(id, _)| *id == act.counterparty_id)
                .unwrap_or(0) as i32;

            let item_descs: Vec<SharedString> = items
                .iter()
                .map(|it| SharedString::from(it.description.as_str()))
                .collect();
            let item_qtys: Vec<SharedString> = items
                .iter()
                .map(|it| SharedString::from(format!("{}", it.quantity).as_str()))
                .collect();
            let item_units: Vec<SharedString> = items
                .iter()
                .map(|it| SharedString::from(it.unit.as_str()))
                .collect();
            let item_prices: Vec<SharedString> = items
                .iter()
                .map(|it| SharedString::from(format!("{}", it.unit_price).as_str()))
                .collect();
            let item_amounts: Vec<SharedString> = items
                .iter()
                .map(|it| SharedString::from(format!("{:.2}", it.amount).as_str()))
                .collect();
            let task_data = to_tasks_table_data(&tasks);

            let act_number = act.number.clone();
            // Дата у форматі ДД.ММ.РРРР (урок 2026-04-01)
            let act_date = act.date.format("%d.%m.%Y").to_string();
            let act_notes = act.notes.clone().unwrap_or_default();
            let act_id_str = act.id.to_string();
            let total_str = format!("{:.2}", act.total_amount);

            ui_handle
                .upgrade_in_event_loop(move |ui| {
                    let (task_rows, task_ids, task_statuses, task_priorities) =
                        build_task_models(task_data);
                    ui.set_act_form_number(SharedString::from(act_number.as_str()));
                    ui.set_act_form_date(SharedString::from(act_date.as_str()));
                    ui.set_act_form_notes(SharedString::from(act_notes.as_str()));
                    ui.set_act_form_cp_index(cp_index);
                    ui.set_act_form_edit_id(SharedString::from(act_id_str.as_str()));
                    ui.set_act_form_total(SharedString::from(total_str.as_str()));
                    ui.set_act_form_is_edit(true);
                    ui.set_act_form_cp_names(ModelRc::new(VecModel::from(cp_names)));
                    ui.set_act_form_cp_ids(ModelRc::new(VecModel::from(cp_ids)));
                    ui.set_act_form_item_descriptions(ModelRc::new(VecModel::from(item_descs)));
                    ui.set_act_form_item_quantities(ModelRc::new(VecModel::from(item_qtys)));
                    ui.set_act_form_item_units(ModelRc::new(VecModel::from(item_units)));
                    ui.set_act_form_item_prices(ModelRc::new(VecModel::from(item_prices)));
                    ui.set_act_form_item_amounts(ModelRc::new(VecModel::from(item_amounts)));
                    ui.set_act_task_rows(task_rows);
                    ui.set_act_task_row_ids(task_ids);
                    ui.set_act_task_row_statuses(task_statuses);
                    ui.set_act_task_row_priorities(task_priorities);
                    ui.set_act_tasks_loading(false);
                    ui.set_show_act_form(true);
                })
                .ok();
        });
    });

    // ── Колбек: оновити акт з позиціями (режим редагування) ─────────────────
    //
    // Читаємо edit_id синхронно з UI (main thread),
    // потім spawn для async DB операції.
    let pool_update = pool.clone();
    let ui_weak_update = ui.as_weak();
    let act_state_update = act_state.clone();
    let active_company_id_update = active_company_id.clone();

    ui.on_act_form_update(move |number, date_str, cp_id_str, notes| {
        let Some(ui_ref) = ui_weak_update.upgrade() else {
            return;
        };
        // Читаємо edit_id та позиції поки ще в main thread
        let edit_id = ui_ref.get_act_form_edit_id().to_string();
        let items = collect_form_items(&ui_ref);

        let pool = pool_update.clone();
        let ui_weak = ui_weak_update.clone();
        let cid = *active_company_id_update.lock().unwrap();
        let number = number.to_string();
        let date_str = date_str.to_string();
        let cp_id_str = cp_id_str.to_string();
        let notes_opt = if notes.trim().is_empty() {
            None
        } else {
            Some(notes.to_string())
        };
        let act_state = act_state_update.clone();

        tokio::spawn(async move {
            let Ok(uuid) = edit_id.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний edit_id: {edit_id}");
                return;
            };

            // Валідація обов'язкових полів форми
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

            // Парсимо дату (урок 2026-04-01)
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
                    tracing::error!("Некоректний UUID контрагента: '{cp_id_str}'");
                    return;
                }
            };

            let update_data = UpdateAct {
                number: number.clone(),
                counterparty_id: cp_uuid,
                contract_id: None,
                date,
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
                        tracing::error!("Помилка оновлення списку актів після редагування: {e}");
                    }
                }
                Err(e) => tracing::error!("Помилка оновлення акту: {e}"),
            }
        });
    });

    // ── Колбек: скасувати форму контрагента ─────────────────────────────────
    let ui_weak_cp_cancel = ui.as_weak();
    ui.on_cp_form_cancel(move || {
        if let Some(ui) = ui_weak_cp_cancel.upgrade() {
            ui.set_show_cp_form(false);
        }
    });

    // ── Колбек: зберегти нового контрагента ──────────────────────────────────
    let pool_cp_save = pool.clone();
    let ui_weak_cp_save = ui.as_weak();
    let counterparty_state_save = counterparty_state.clone();
    let active_company_id_cp_save = active_company_id.clone();

    ui.on_cp_form_save(move |name, edrpou, ipn, iban, phone, email, address, notes| {
        let pool = pool_cp_save.clone();
        let ui_weak = ui_weak_cp_save.clone();
        let name_s = name.to_string();
        let edrpou_s = edrpou.to_string();
        let ipn_s = ipn.to_string();
        let iban_s = iban.to_string();
        let phone_s = phone.to_string();
        let email_s = email.to_string();
        let address_s = address.to_string();
        let notes_s = notes.to_string();
        let counterparty_state = counterparty_state_save.clone();
        let cid = *active_company_id_cp_save.lock().unwrap();

        tokio::spawn(async move {
            if name_s.trim().is_empty() {
                tracing::error!("Назва контрагента не може бути порожньою");
                show_toast(ui_weak, "Введіть назву контрагента".to_string(), true);
                return;
            }

            let data = NewCounterparty {
                name: name_s.clone(),
                edrpou: if edrpou_s.trim().is_empty() {
                    None
                } else {
                    Some(edrpou_s)
                },
                ipn: if ipn_s.trim().is_empty() {
                    None
                } else {
                    Some(ipn_s)
                },
                iban: if iban_s.trim().is_empty() {
                    None
                } else {
                    Some(iban_s)
                },
                phone: if phone_s.trim().is_empty() {
                    None
                } else {
                    Some(phone_s)
                },
                email: if email_s.trim().is_empty() {
                    None
                } else {
                    Some(email_s)
                },
                address: if address_s.trim().is_empty() {
                    None
                } else {
                    Some(address_s)
                },
                notes: if notes_s.trim().is_empty() {
                    None
                } else {
                    Some(notes_s)
                },
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
                        let state = counterparty_state.lock().unwrap();
                        (state.query.clone(), state.include_archived, state.page)
                    };
                    if let Err(e) =
                        reload_counterparties(&pool, ui_weak, cid, query, include_archived, page, true).await
                    {
                        tracing::error!(
                            "Помилка оновлення списку контрагентів після створення: {e}"
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

    // ── Колбек: оновити контрагента (режим редагування) ──────────────────────
    let pool_cp_update = pool.clone();
    let ui_weak_cp_update = ui.as_weak();
    let counterparty_state_update = counterparty_state.clone();
    let active_company_id_cp_update = active_company_id.clone();

    ui.on_cp_form_update(move |name, edrpou, ipn, iban, phone, email, address, notes| {
        let Some(ui_ref) = ui_weak_cp_update.upgrade() else {
            return;
        };
        let edit_id = ui_ref.get_cp_form_edit_id().to_string();

        let pool = pool_cp_update.clone();
        let ui_weak = ui_weak_cp_update.clone();
        let cid = *active_company_id_cp_update.lock().unwrap();
        let name_s = name.to_string();
        let edrpou_s = edrpou.to_string();
        let ipn_s = ipn.to_string();
        let iban_s = iban.to_string();
        let phone_s = phone.to_string();
        let email_s = email.to_string();
        let address_s = address.to_string();
        let notes_s = notes.to_string();
        let counterparty_state = counterparty_state_update.clone();

        tokio::spawn(async move {
            let Ok(uuid) = edit_id.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний edit_id: {edit_id}");
                return;
            };

            if name_s.trim().is_empty() {
                show_toast(ui_weak, "Введіть назву контрагента".to_string(), true);
                return;
            }

            let data = UpdateCounterparty {
                name: name_s,
                edrpou: if edrpou_s.trim().is_empty() {
                    None
                } else {
                    Some(edrpou_s)
                },
                ipn: if ipn_s.trim().is_empty() {
                    None
                } else {
                    Some(ipn_s)
                },
                iban: if iban_s.trim().is_empty() {
                    None
                } else {
                    Some(iban_s)
                },
                phone: if phone_s.trim().is_empty() {
                    None
                } else {
                    Some(phone_s)
                },
                email: if email_s.trim().is_empty() {
                    None
                } else {
                    Some(email_s)
                },
                address: if address_s.trim().is_empty() {
                    None
                } else {
                    Some(address_s)
                },
                notes: if notes_s.trim().is_empty() {
                    None
                } else {
                    Some(notes_s)
                },
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
                        let state = counterparty_state.lock().unwrap();
                        (state.query.clone(), state.include_archived, state.page)
                    };
                    if let Err(e) =
                        reload_counterparties(&pool, ui_weak, cid, query, include_archived, page, true).await
                    {
                        tracing::error!(
                            "Помилка оновлення списку контрагентів після редагування: {e}"
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

    // ── Колбек: архівувати ───────────────────────────────────────────────────
    let pool_archive = pool.clone();
    let ui_weak_archive = ui.as_weak();
    let counterparty_state_archive = counterparty_state.clone();
    let active_company_id_archive = active_company_id.clone();

    ui.on_counterparty_archive_clicked(move |id| {
        let pool = pool_archive.clone();
        let ui_handle = ui_weak_archive.clone();
        let id_str = id.to_string();
        let counterparty_state = counterparty_state_archive.clone();
        let cid = *active_company_id_archive.lock().unwrap();

        tokio::spawn(async move {
            // Перетворюємо рядок у UUID — let-else для чистого раннього виходу
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID: {id_str}");
                return;
            };

            match db::counterparties::archive(&pool, uuid).await {
                Ok(true) => {
                    tracing::info!("Контрагента {uuid} архівовано.");
                    show_toast(
                        ui_handle.clone(),
                        "Контрагента архівовано".to_string(),
                        false,
                    );
                    let (query, include_archived, page) = {
                        let state = counterparty_state.lock().unwrap();
                        (state.query.clone(), state.include_archived, state.page)
                    };
                    if let Err(e) =
                        reload_counterparties(&pool, ui_handle, cid, query, include_archived, page, false)
                            .await
                    {
                        tracing::error!(
                            "Помилка оновлення списку контрагентів після архівування: {e}"
                        );
                    }
                }
                Ok(false) => tracing::warn!("Контрагента {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка архівування: {e}"),
            }
        });
    });

    // ── Колбек: скасувати форму — повернутись до списку ─────────────────────
    //
    // Синхронний колбек (немає DB операцій) — просто скидаємо прапор.
    // Викликається без tokio::spawn: ми вже в main thread.
    let ui_weak_cancel = ui.as_weak();
    ui.on_act_form_cancel(move || {
        if let Some(ui) = ui_weak_cancel.upgrade() {
            ui.set_show_act_form(false);
        }
    });

    // ── Колбек: додати позицію до форми ─────────────────────────────────────
    //
    // Додає порожній рядок у кожен паралельний масив позицій.
    // Синхронно: оновлюємо ModelRc у main thread.
    // Редагування значень позиції — майбутня функція (TODO: edit-item колбек).
    let ui_weak_add = ui.as_weak();
    ui.on_act_form_add_item(move || {
        let Some(ui) = ui_weak_add.upgrade() else {
            return;
        };

        // Локальна функція (не closure) — не захоплює змінних, може бути вбудована
        fn append(model: ModelRc<SharedString>, val: &str) -> ModelRc<SharedString> {
            let mut v: Vec<SharedString> = (0..model.row_count())
                .filter_map(|i| model.row_data(i))
                .collect();
            v.push(SharedString::from(val));
            ModelRc::new(VecModel::from(v))
        }

        ui.set_act_form_item_descriptions(append(
            ui.get_act_form_item_descriptions(),
            "Нова послуга",
        ));
        ui.set_act_form_item_quantities(append(ui.get_act_form_item_quantities(), "1"));
        ui.set_act_form_item_units(append(ui.get_act_form_item_units(), "шт"));
        ui.set_act_form_item_prices(append(ui.get_act_form_item_prices(), "0.00"));
        ui.set_act_form_item_amounts(append(ui.get_act_form_item_amounts(), "0.00"));

        // Перераховуємо total (після append amount = 0.00, тому total не змінюється)
        // Повноцінний перерахунок — після реалізації edit-item
    });

    // ── Колбек: видалити позицію з форми ────────────────────────────────────
    let ui_weak_remove = ui.as_weak();
    ui.on_act_form_remove_item(move |idx| {
        let Some(ui) = ui_weak_remove.upgrade() else {
            return;
        };
        let i = idx as usize;

        fn remove_at(model: ModelRc<SharedString>, i: usize) -> ModelRc<SharedString> {
            let mut v: Vec<SharedString> = (0..model.row_count())
                .filter_map(|j| model.row_data(j))
                .collect();
            if i < v.len() {
                v.remove(i);
            }
            ModelRc::new(VecModel::from(v))
        }

        ui.set_act_form_item_descriptions(remove_at(ui.get_act_form_item_descriptions(), i));
        ui.set_act_form_item_quantities(remove_at(ui.get_act_form_item_quantities(), i));
        ui.set_act_form_item_units(remove_at(ui.get_act_form_item_units(), i));
        ui.set_act_form_item_prices(remove_at(ui.get_act_form_item_prices(), i));
        ui.set_act_form_item_amounts(remove_at(ui.get_act_form_item_amounts(), i));
    });

    // ── Колбек: редагування поля позиції акту ───────────────────────────────
    //
    // Синхронний колбек (немає DB) — лише перебудовуємо ModelRc.
    // При зміні qty або price — перераховуємо amounts та total.
    //
    // Чому не оновлюємо qty/price через set_row_data: ModelRc не дає
    // доступу до внутрішнього VecModel після побудови. Замість цього
    // створюємо новий ModelRc — Slint порівнює значення і не скидає
    // фокус LineEdit якщо значення не змінилось.
    let ui_weak_item = ui.as_weak();

    ui.on_act_form_item_changed(move |idx, field, value| {
        let Some(ui) = ui_weak_item.upgrade() else {
            return;
        };
        let i = idx as usize;
        let val = value.to_string();

        // Перебудувати ModelRc з одним зміненим елементом
        fn set_at(model: ModelRc<SharedString>, i: usize, val: &str) -> ModelRc<SharedString> {
            let mut v: Vec<SharedString> = (0..model.row_count())
                .filter_map(|j| model.row_data(j))
                .collect();
            if i < v.len() {
                v[i] = SharedString::from(val);
            }
            ModelRc::new(VecModel::from(v))
        }

        match field.as_str() {
            "desc" => ui.set_act_form_item_descriptions(set_at(
                ui.get_act_form_item_descriptions(),
                i,
                &val,
            )),
            "qty" => {
                ui.set_act_form_item_quantities(set_at(ui.get_act_form_item_quantities(), i, &val))
            }
            "unit" => ui.set_act_form_item_units(set_at(ui.get_act_form_item_units(), i, &val)),
            "price" => ui.set_act_form_item_prices(set_at(ui.get_act_form_item_prices(), i, &val)),
            _ => return,
        }

        // Перераховуємо суми рядків та total лише при зміні qty або price
        if field == "qty" || field == "price" {
            let qtys = ui.get_act_form_item_quantities();
            let prices = ui.get_act_form_item_prices();
            let n = qtys.row_count();

            let mut new_amounts: Vec<SharedString> = Vec::with_capacity(n);
            let mut total = Decimal::ZERO;

            for j in 0..n {
                let qty = qtys
                    .row_data(j)
                    .unwrap_or_default()
                    .parse::<Decimal>()
                    .unwrap_or_default();
                let price = prices
                    .row_data(j)
                    .unwrap_or_default()
                    .parse::<Decimal>()
                    .unwrap_or_default();
                let amt = qty * price;
                total += amt;
                new_amounts.push(SharedString::from(format!("{:.2}", amt).as_str()));
            }

            ui.set_act_form_item_amounts(ModelRc::new(VecModel::from(new_amounts)));
            ui.set_act_form_total(SharedString::from(format!("{:.2}", total).as_str()));
        }
    });

    // ── Колбек: зберегти акт ("Зберегти") ───────────────────────────────────
    //
    // Читаємо поля форми + позиції синхронно (ми в main thread),
    // потім передаємо в tokio::spawn для async DB операції.
    let pool_save = pool.clone();
    let ui_weak_save = ui.as_weak();
    let act_state_save = act_state.clone();
    let active_company_id_save = active_company_id.clone();

    ui.on_act_form_save(move |number, date_str, cp_id_str, notes| {
        let Some(ui_ref) = ui_weak_save.upgrade() else {
            return;
        };
        let items = collect_form_items(&ui_ref);
        let cid = *active_company_id_save.lock().unwrap();

        spawn_save_act(
            pool_save.clone(),
            ui_weak_save.clone(),
            act_state_save.clone(),
            cid,
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.trim().is_empty() {
                None
            } else {
                Some(notes.to_string())
            },
            items,
        );
    });

    // ── Колбек: зберегти як чернетку ("Чернетка") ───────────────────────────
    //
    // Наразі ідентичний до on_act_form_save — обидва створюють акт зі статусом Draft
    // (статус 'draft' є DEFAULT у БД, змінити можна лише через advance_status).
    // TODO: у майбутньому on_act_form_save може одразу переводити до Issued.
    let pool_draft = pool.clone();
    let ui_weak_draft = ui.as_weak();
    let act_state_draft = act_state.clone();
    let active_company_id_draft = active_company_id.clone();

    ui.on_act_form_save_draft(move |number, date_str, cp_id_str, notes| {
        let Some(ui_ref) = ui_weak_draft.upgrade() else {
            return;
        };
        let items = collect_form_items(&ui_ref);
        let cid = *active_company_id_draft.lock().unwrap();

        spawn_save_act(
            pool_draft.clone(),
            ui_weak_draft.clone(),
            act_state_draft.clone(),
            cid,
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.trim().is_empty() {
                None
            } else {
                Some(notes.to_string())
            },
            items,
        );
    });

    // ── Колбеки задач ──────────────────────────────────────────────────────
    // Act card task callbacks
    let ui_weak_act_tasks_create = ui.as_weak();
    ui.on_act_task_create_clicked(move || {
        if let Some(ui) = ui_weak_act_tasks_create.upgrade() {
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

    let pool_act_tasks_edit = pool.clone();
    let ui_weak_act_tasks_edit = ui.as_weak();

    ui.on_act_task_edit_clicked(move |task_id| {
        let pool = pool_act_tasks_edit.clone();
        let ui_handle = ui_weak_act_tasks_edit.clone();
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

    let pool_act_tasks_toggle = pool.clone();
    let ui_weak_act_tasks_toggle = ui.as_weak();

    ui.on_act_task_toggle_status_clicked(move |task_id| {
        let pool = pool_act_tasks_toggle.clone();
        let ui_handle = ui_weak_act_tasks_toggle.clone();
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
                    if let Err(e) = reload_act_tasks(&pool, ui_handle.clone(), act_id).await {
                        tracing::error!("Помилка оновлення задач акту: {e}");
                    }
                }
            }
        });
    });

    let pool_act_tasks_delete = pool.clone();
    let ui_weak_act_tasks_delete = ui.as_weak();

    ui.on_act_task_delete_clicked(move |task_id| {
        let pool = pool_act_tasks_delete.clone();
        let ui_handle = ui_weak_act_tasks_delete.clone();
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
                        if let Err(e) = reload_act_tasks(&pool, ui_handle.clone(), act_id).await {
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
    let pool_tasks_search = pool.clone();
    let ui_weak_tasks_search = ui.as_weak();
    let task_state_search = task_state.clone();

    ui.on_task_search_changed(move |query| {
        let pool = pool_tasks_search.clone();
        let ui_handle = ui_weak_tasks_search.clone();
        let query_str = {
            let mut state = task_state_search.lock().unwrap();
            state.query = query.to_string();
            state.query.clone()
        };

        if let Some(ui) = ui_handle.upgrade() {
            ui.set_tasks_loading(true);
        }

        tokio::spawn(async move {
            if let Err(e) = reload_tasks(&pool, ui_handle, query_str, false).await {
                tracing::error!("Помилка пошуку задач: {e}");
            }
        });
    });

    ui.on_task_selected(|id| {
        tracing::debug!("Вибрано задачу: {id}");
    });

    let ui_weak_tasks_create = ui.as_weak();
    ui.on_task_create_clicked(move || {
        if let Some(ui) = ui_weak_tasks_create.upgrade() {
            ui.set_task_form_is_edit(false);
            ui.set_task_form_edit_id(SharedString::from(""));
            ui.set_task_form_title(SharedString::from(""));
            ui.set_task_form_description(SharedString::from(""));
            ui.set_task_form_priority_index(1);
            ui.set_task_form_due_date(SharedString::from(""));
            ui.set_task_form_reminder_at(SharedString::from(""));
            ui.set_task_form_act_id(SharedString::from(""));
            ui.set_task_form_return_page(5);
            ui.set_show_task_form(true);
        }
    });

    let pool_tasks_edit = pool.clone();
    let ui_weak_tasks_edit = ui.as_weak();

    ui.on_task_edit_clicked(move |task_id| {
        let pool = pool_tasks_edit.clone();
        let ui_handle = ui_weak_tasks_edit.clone();
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
                            ui.set_task_form_return_page(5);
                            ui.set_show_task_form(true);
                        })
                        .ok();
                }
                Ok(None) => tracing::warn!("Задачу {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка завантаження задачі: {e}"),
            }
        });
    });

    let pool_tasks_toggle = pool.clone();
    let ui_weak_tasks_toggle = ui.as_weak();
    let task_state_toggle = task_state.clone();

    ui.on_task_toggle_status_clicked(move |task_id| {
        let pool = pool_tasks_toggle.clone();
        let ui_handle = ui_weak_tasks_toggle.clone();
        let id_str = task_id.to_string();
        let task_state = task_state_toggle.clone();

        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID задачі: {id_str}");
                return;
            };

            match db::tasks::set_status(&pool, uuid, TaskStatus::Done).await {
                Ok(Some(task)) => {
                    tracing::info!("Задачу '{}' завершено.", task.title);
                    show_toast(
                        ui_handle.clone(),
                        format!("Задачу '{}' завершено", task.title),
                        false,
                    );
                    let query = {
                        let state = task_state.lock().unwrap();
                        state.query.clone()
                    };
                    if let Err(e) = reload_tasks(&pool, ui_handle.clone(), query, true).await {
                        tracing::error!("Помилка оновлення списку задач: {e}");
                    }
                }
                Ok(None) => tracing::warn!("Задачу {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка зміни статусу задачі: {e}"),
            }
        });
    });

    let pool_tasks_delete = pool.clone();
    let ui_weak_tasks_delete = ui.as_weak();
    let task_state_delete = task_state.clone();

    ui.on_task_delete_clicked(move |task_id| {
        let pool = pool_tasks_delete.clone();
        let ui_handle = ui_weak_tasks_delete.clone();
        let id_str = task_id.to_string();
        let task_state = task_state_delete.clone();

        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID задачі: {id_str}");
                return;
            };

            match db::tasks::delete(&pool, uuid).await {
                Ok(true) => {
                    show_toast(ui_handle.clone(), "Задачу видалено".to_string(), false);
                    let query = {
                        let state = task_state.lock().unwrap();
                        state.query.clone()
                    };
                    if let Err(e) = reload_tasks(&pool, ui_handle.clone(), query, true).await {
                        tracing::error!("Помилка оновлення списку задач після видалення: {e}");
                    }
                }
                Ok(false) => tracing::warn!("Задачу {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка видалення задачі: {e}"),
            }
        });
    });

    let pool_tasks_save = pool.clone();
    let ui_weak_tasks_save = ui.as_weak();
    let task_state_save = task_state.clone();

    ui.on_task_form_save(
        move |title, description, priority_idx, due_str, reminder_str| {
            let act_id = ui_weak_tasks_save
                .upgrade()
                .map(|ui| ui.get_task_form_act_id().to_string())
                .unwrap_or_default();
            spawn_save_task(
                pool_tasks_save.clone(),
                ui_weak_tasks_save.clone(),
                task_state_save.clone(),
                None,
                title.to_string(),
                description.to_string(),
                priority_idx,
                due_str.to_string(),
                reminder_str.to_string(),
                act_id,
            );
        },
    );

    let pool_tasks_update = pool.clone();
    let ui_weak_tasks_update = ui.as_weak();
    let task_state_update = task_state.clone();

    ui.on_task_form_update(
        move |title, description, priority_idx, due_str, reminder_str| {
            let edit_id = ui_weak_tasks_update
                .upgrade()
                .map(|ui| ui.get_task_form_edit_id().to_string())
                .unwrap_or_default();
            let act_id = ui_weak_tasks_update
                .upgrade()
                .map(|ui| ui.get_task_form_act_id().to_string())
                .unwrap_or_default();

            spawn_save_task(
                pool_tasks_update.clone(),
                ui_weak_tasks_update.clone(),
                task_state_update.clone(),
                Some(edit_id),
                title.to_string(),
                description.to_string(),
                priority_idx,
                due_str.to_string(),
                reminder_str.to_string(),
                act_id,
            );
        },
    );

    let ui_weak_tasks_cancel = ui.as_weak();
    ui.on_task_form_cancel(move || {
        if let Some(ui) = ui_weak_tasks_cancel.upgrade() {
            ui.set_show_task_form(false);
            ui.set_current_page(ui.get_task_form_return_page());
        }
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // ── Видаткові накладні ──────────────────────────────────────────────────────
    // ═══════════════════════════════════════════════════════════════════════════

    let pool_inv_filter = pool.clone();
    let ui_weak_inv_filter = ui.as_weak();
    let invoice_state_filter = invoice_state.clone();
    let active_company_id_inv_filter = active_company_id.clone();
    ui.on_invoice_status_filter_changed(move |filter_idx| {
        let pool = pool_inv_filter.clone();
        let ui_handle = ui_weak_inv_filter.clone();
        let inv_state = invoice_state_filter.clone();
        let cid = *active_company_id_inv_filter.lock().unwrap();
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
            if let Err(e) = reload_invoices(&pool, ui_handle, cid, status_filter, query, false).await {
                tracing::error!("Помилка фільтру накладних: {e}");
            }
        });
    });

    let pool_inv_search = pool.clone();
    let ui_weak_inv_search = ui.as_weak();
    let invoice_state_search = invoice_state.clone();
    let active_company_id_inv_search = active_company_id.clone();
    ui.on_invoice_search_changed(move |query| {
        let pool = pool_inv_search.clone();
        let ui_handle = ui_weak_inv_search.clone();
        let inv_state = invoice_state_search.clone();
        let cid = *active_company_id_inv_search.lock().unwrap();
        let query = query.to_string();
        tokio::spawn(async move {
            let (status_filter, query) = {
                let mut state = inv_state.lock().unwrap();
                state.query = query.clone();
                (state.status_filter.clone(), query)
            };
            if let Err(e) = reload_invoices(&pool, ui_handle, cid, status_filter, query, false).await {
                tracing::error!("Помилка пошуку накладних: {e}");
            }
        });
    });

    ui.on_invoice_selected(|_id| {});

    let pool_inv_create = pool.clone();
    let ui_weak_inv_create = ui.as_weak();
    let active_company_id_inv_create = active_company_id.clone();
    ui.on_invoice_create_clicked(move || {
        let pool = pool_inv_create.clone();
        let ui_weak = ui_weak_inv_create.clone();
        let cid = *active_company_id_inv_create.lock().unwrap();
        tokio::spawn(async move {
            let (cps, next_number) = tokio::join!(
                db::invoices::counterparties_for_select(&pool, cid),
                db::invoices::generate_next_number(&pool, cid),
            );
            let cps = cps.unwrap_or_default();
            let next_number = next_number.unwrap_or_else(|_| "НАК-001".into());
            let today = chrono::Local::now().format("%d.%m.%Y").to_string();
            ui_weak.upgrade_in_event_loop(move |ui| {
                let (names, ids): (Vec<SharedString>, Vec<SharedString>) = cps.iter()
                    .map(|(id, name)| (SharedString::from(name.as_str()), SharedString::from(id.to_string().as_str())))
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
                let empty: Vec<SharedString> = vec![];
                ui.set_invoice_form_item_descriptions(ModelRc::new(VecModel::from(empty.clone())));
                ui.set_invoice_form_item_quantities(ModelRc::new(VecModel::from(empty.clone())));
                ui.set_invoice_form_item_units(ModelRc::new(VecModel::from(empty.clone())));
                ui.set_invoice_form_item_prices(ModelRc::new(VecModel::from(empty.clone())));
                ui.set_invoice_form_item_amounts(ModelRc::new(VecModel::from(empty)));
                ui.set_show_invoice_form(true);
            }).ok();
        });
    });

    let pool_inv_advance = pool.clone();
    let ui_weak_inv_advance = ui.as_weak();
    let invoice_state_advance = invoice_state.clone();
    let active_company_id_inv_advance = active_company_id.clone();
    ui.on_invoice_advance_status_clicked(move |id| {
        let pool = pool_inv_advance.clone();
        let ui_weak = ui_weak_inv_advance.clone();
        let inv_state = invoice_state_advance.clone();
        let cid = *active_company_id_inv_advance.lock().unwrap();
        let id_str = id.to_string();
        tokio::spawn(async move {
            let invoice_id = match uuid::Uuid::parse_str(&id_str) {
                Ok(id) => id,
                Err(_) => { tracing::error!("Невалідний UUID накладної: {id_str}"); return; }
            };
            match db::invoices::advance_status(&pool, invoice_id).await {
                Ok(Some(inv)) => {
                    let (status_filter, query) = {
                        let state = inv_state.lock().unwrap();
                        (state.status_filter.clone(), state.query.clone())
                    };
                    if let Err(e) = reload_invoices(&pool, ui_weak, cid, status_filter, query, false).await {
                        tracing::error!("Помилка оновлення накладних: {e}");
                    }
                    let _ = inv; // suppress unused
                }
                Ok(None) => tracing::error!("Накладну {id_str} не знайдено"),
                Err(e) => tracing::error!("Помилка зміни статусу накладної: {e}"),
            }
        });
    });

    let pool_inv_edit = pool.clone();
    let ui_weak_inv_edit = ui.as_weak();
    let active_company_id_inv_edit = active_company_id.clone();
    ui.on_invoice_edit_clicked(move |inv_id| {
        let pool = pool_inv_edit.clone();
        let ui_weak = ui_weak_inv_edit.clone();
        let cid = *active_company_id_inv_edit.lock().unwrap();
        let id_str = inv_id.to_string();
        tokio::spawn(async move {
            let invoice_uuid = match uuid::Uuid::parse_str(&id_str) {
                Ok(id) => id,
                Err(_) => { tracing::error!("Невалідний UUID накладної: {id_str}"); return; }
            };
            let (result, cps) = tokio::join!(
                db::invoices::get_for_edit(&pool, invoice_uuid),
                db::invoices::counterparties_for_select(&pool, cid),
            );
            let (invoice, items) = match result {
                Ok(Some(data)) => data,
                Ok(None) => { tracing::error!("Накладна {id_str} не знайдена"); return; }
                Err(e) => { tracing::error!("Помилка завантаження накладної: {e}"); return; }
            };
            let cps = cps.unwrap_or_default();
            let cp_id_str = invoice.counterparty_id.to_string();
            let cp_index = cps.iter().position(|(id, _)| id.to_string() == cp_id_str).unwrap_or(0);
            ui_weak.upgrade_in_event_loop(move |ui| {
                let (names, ids): (Vec<SharedString>, Vec<SharedString>) = cps.iter()
                    .map(|(id, name)| (SharedString::from(name.as_str()), SharedString::from(id.to_string().as_str())))
                    .unzip();
                ui.set_invoice_form_cp_names(ModelRc::new(VecModel::from(names)));
                ui.set_invoice_form_cp_ids(ModelRc::new(VecModel::from(ids)));
                ui.set_invoice_form_number(SharedString::from(invoice.number.as_str()));
                ui.set_invoice_form_date(SharedString::from(invoice.date.format("%d.%m.%Y").to_string().as_str()));
                ui.set_invoice_form_notes(SharedString::from(invoice.notes.as_deref().unwrap_or("")));
                ui.set_invoice_form_cp_index(cp_index as i32);
                ui.set_invoice_form_is_edit(true);
                ui.set_invoice_form_edit_id(SharedString::from(invoice.id.to_string().as_str()));
                ui.set_invoice_form_total(SharedString::from(invoice.total_amount.to_string().as_str()));
                let descs: Vec<SharedString> = items.iter().map(|it| SharedString::from(it.description.as_str())).collect();
                let qtys: Vec<SharedString> = items.iter().map(|it| SharedString::from(it.quantity.to_string().as_str())).collect();
                let units_v: Vec<SharedString> = items.iter().map(|it| SharedString::from(it.unit.as_deref().unwrap_or(""))).collect();
                let prices: Vec<SharedString> = items.iter().map(|it| SharedString::from(it.price.to_string().as_str())).collect();
                let amounts: Vec<SharedString> = items.iter().map(|it| SharedString::from(it.amount.to_string().as_str())).collect();
                ui.set_invoice_form_item_descriptions(ModelRc::new(VecModel::from(descs)));
                ui.set_invoice_form_item_quantities(ModelRc::new(VecModel::from(qtys)));
                ui.set_invoice_form_item_units(ModelRc::new(VecModel::from(units_v)));
                ui.set_invoice_form_item_prices(ModelRc::new(VecModel::from(prices)));
                ui.set_invoice_form_item_amounts(ModelRc::new(VecModel::from(amounts)));
                ui.set_show_invoice_form(true);
            }).ok();
        });
    });

    let ui_weak_inv_cancel = ui.as_weak();
    ui.on_invoice_form_cancel(move || {
        if let Some(ui) = ui_weak_inv_cancel.upgrade() {
            ui.set_show_invoice_form(false);
        }
    });

    let ui_weak_inv_add = ui.as_weak();
    ui.on_invoice_form_add_item(move || {
        if let Some(ui) = ui_weak_inv_add.upgrade() {
            fn push_inv(model: ModelRc<SharedString>, val: &str) -> ModelRc<SharedString> {
                use slint::Model;
                let mut v: Vec<SharedString> = (0..model.row_count()).filter_map(|i| model.row_data(i)).collect();
                v.push(SharedString::from(val));
                ModelRc::new(VecModel::from(v))
            }
            ui.set_invoice_form_item_descriptions(push_inv(ui.get_invoice_form_item_descriptions(), ""));
            ui.set_invoice_form_item_quantities(push_inv(ui.get_invoice_form_item_quantities(), "1"));
            ui.set_invoice_form_item_units(push_inv(ui.get_invoice_form_item_units(), "шт"));
            ui.set_invoice_form_item_prices(push_inv(ui.get_invoice_form_item_prices(), "0.00"));
            ui.set_invoice_form_item_amounts(push_inv(ui.get_invoice_form_item_amounts(), "0.00"));
        }
    });

    let ui_weak_inv_remove = ui.as_weak();
    ui.on_invoice_form_remove_item(move |idx| {
        if let Some(ui) = ui_weak_inv_remove.upgrade() {
            use slint::Model;
            fn remove_inv(model: ModelRc<SharedString>, idx: usize) -> ModelRc<SharedString> {
                let mut v: Vec<SharedString> = (0..model.row_count()).filter_map(|i| model.row_data(i)).collect();
                if idx < v.len() { v.remove(idx); }
                ModelRc::new(VecModel::from(v))
            }
            let idx = idx as usize;
            ui.set_invoice_form_item_descriptions(remove_inv(ui.get_invoice_form_item_descriptions(), idx));
            ui.set_invoice_form_item_quantities(remove_inv(ui.get_invoice_form_item_quantities(), idx));
            ui.set_invoice_form_item_units(remove_inv(ui.get_invoice_form_item_units(), idx));
            ui.set_invoice_form_item_prices(remove_inv(ui.get_invoice_form_item_prices(), idx));
            ui.set_invoice_form_item_amounts(remove_inv(ui.get_invoice_form_item_amounts(), idx));
            recalculate_invoice_total(&ui);
        }
    });

    let ui_weak_inv_item = ui.as_weak();
    ui.on_invoice_form_item_changed(move |idx, field, value| {
        if let Some(ui) = ui_weak_inv_item.upgrade() {
            use slint::Model;
            fn update_inv(model: ModelRc<SharedString>, idx: usize, val: SharedString) -> ModelRc<SharedString> {
                let mut v: Vec<SharedString> = (0..model.row_count()).filter_map(|i| model.row_data(i)).collect();
                if idx < v.len() { v[idx] = val; }
                ModelRc::new(VecModel::from(v))
            }
            let idx = idx as usize;
            match field.as_str() {
                "desc"  => { ui.set_invoice_form_item_descriptions(update_inv(ui.get_invoice_form_item_descriptions(), idx, value)); }
                "qty"   => { ui.set_invoice_form_item_quantities(update_inv(ui.get_invoice_form_item_quantities(), idx, value)); recalculate_invoice_total(&ui); }
                "unit"  => { ui.set_invoice_form_item_units(update_inv(ui.get_invoice_form_item_units(), idx, value)); }
                "price" => { ui.set_invoice_form_item_prices(update_inv(ui.get_invoice_form_item_prices(), idx, value)); recalculate_invoice_total(&ui); }
                _ => {}
            }
        }
    });

    let pool_inv_save = pool.clone();
    let ui_weak_inv_save = ui.as_weak();
    let invoice_state_save = invoice_state.clone();
    let active_company_id_inv_save = active_company_id.clone();
    ui.on_invoice_form_save(move |number, date_str, cp_id_str, notes| {
        let cid = *active_company_id_inv_save.lock().unwrap();
        let items = collect_invoice_items_from_ui(&ui_weak_inv_save);
        spawn_save_invoice(
            pool_inv_save.clone(),
            ui_weak_inv_save.clone(),
            invoice_state_save.clone(),
            cid,
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.is_empty() { None } else { Some(notes.to_string()) },
            items,
        );
    });

    let pool_inv_upd = pool.clone();
    let ui_weak_inv_upd = ui.as_weak();
    let invoice_state_upd = invoice_state.clone();
    let active_company_id_inv_upd = active_company_id.clone();
    ui.on_invoice_form_update(move |number, date_str, cp_id_str, notes| {
        let cid = *active_company_id_inv_upd.lock().unwrap();
        let edit_id = ui_weak_inv_upd
            .upgrade()
            .map(|ui| ui.get_invoice_form_edit_id().to_string())
            .unwrap_or_default();
        let items = collect_invoice_items_from_ui(&ui_weak_inv_upd);
        let pool = pool_inv_upd.clone();
        let ui_weak = ui_weak_inv_upd.clone();
        let inv_state = invoice_state_upd.clone();
        let number = number.to_string();
        let date_str = date_str.to_string();
        let cp_id_str = cp_id_str.to_string();
        let notes = notes.to_string();
        tokio::spawn(async move {
            let invoice_uuid = match uuid::Uuid::parse_str(&edit_id) {
                Ok(id) => id,
                Err(_) => { tracing::error!("Невалідний UUID накладної: {edit_id}"); return; }
            };
            let date = match NaiveDate::parse_from_str(&date_str, "%d.%m.%Y") {
                Ok(d) => d,
                Err(_) => { tracing::error!("Невірний формат дати: '{date_str}'"); return; }
            };
            let cp_uuid = match uuid::Uuid::parse_str(&cp_id_str) {
                Ok(id) => id,
                Err(_) => { tracing::error!("Невалідний UUID контрагента: '{cp_id_str}'"); return; }
            };
            let update_data = UpdateInvoice {
                number: number.clone(),
                counterparty_id: cp_uuid,
                contract_id: None,
                date,
                notes: if notes.is_empty() { None } else { Some(notes) },
            };
            match db::invoices::update_with_items(&pool, invoice_uuid, update_data, items).await {
                Ok(inv) => {
                    tracing::info!("Накладну '{}' оновлено.", inv.number);
                    show_toast(ui_weak.clone(), format!("Накладну '{}' оновлено", inv.number), false);
                    let (status_filter, query) = {
                        let state = inv_state.lock().unwrap();
                        (state.status_filter.clone(), state.query.clone())
                    };
                    if let Err(e) = reload_invoices(&pool, ui_weak, cid, status_filter, query, true).await {
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

    // ── Колбек: переключити активну компанію ─────────────────────────────────
    let ui_weak_switch = ui.as_weak();
    ui.on_switch_company(move || {
        if let Some(ui) = ui_weak_switch.upgrade() {
            ui.set_show_company_picker(true);
        }
    });

    // ── Колбек: обрати активну компанію ──────────────────────────────────────
    let pool_company_select = pool.clone();
    let ui_weak_company_select = ui.as_weak();
    let active_company_id_select = active_company_id.clone();
    let counterparty_state_company_select = counterparty_state.clone();
    let act_state_company_select = act_state.clone();

    ui.on_company_select_clicked(move |id_str| {
        let pool = pool_company_select.clone();
        let ui_handle = ui_weak_company_select.clone();
        let active_company_id = active_company_id_select.clone();
        let counterparty_state = counterparty_state_company_select.clone();
        let act_state = act_state_company_select.clone();
        let id_s = id_str.to_string();

        tokio::spawn(async move {
            let Ok(uuid) = id_s.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID компанії: {id_s}");
                return;
            };

            match db::companies::get_by_id(&pool, uuid).await {
                Ok(Some(company)) => {
                    // Оновлюємо активну компанію
                    *active_company_id.lock().unwrap() = company.id;

                    // Зберігаємо вибір у конфігу
                    let mut cfg = AppConfig::load();
                    cfg.last_company_id = Some(company.id);
                    cfg.save();

                    let name = company_display_name(&company);
                    let subtitle = company_subtitle(&company);
                    let id_str = company.id.to_string();
                    let company_id = company.id;
                    let (cp_query, include_archived, cp_page) = {
                        let state = counterparty_state.lock().unwrap();
                        (state.query.clone(), state.include_archived, state.page)
                    };
                    let (act_query, status_filter) = {
                        let state = act_state.lock().unwrap();
                        (state.query.clone(), state.status_filter.clone())
                    };

                    ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.set_active_company_name(SharedString::from(name.as_str()));
                        ui.set_active_company_id(SharedString::from(id_str.as_str()));
                        ui.set_active_company_subtitle(SharedString::from(subtitle.as_str()));
                        ui.set_show_company_picker(false);
                        ui.set_show_cp_form(false);
                        ui.set_show_act_form(false);
                        ui.set_show_task_form(false);
                    }).ok();

                    if let Err(e) = reload_counterparties(
                        &pool,
                        ui_handle.clone(),
                        company_id,
                        cp_query,
                        include_archived,
                        cp_page,
                        false,
                    )
                    .await
                    {
                        tracing::error!("Помилка оновлення контрагентів після вибору компанії: {e}");
                    }

                    if let Err(e) = reload_acts(
                        &pool,
                        ui_handle.clone(),
                        company_id,
                        status_filter,
                        act_query,
                        false,
                    )
                    .await
                    {
                        tracing::error!("Помилка оновлення актів після вибору компанії: {e}");
                    }
                }
                Ok(None) => tracing::warn!("Компанію {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка вибору компанії: {e}"),
            }
        });
    });

    // ── Колбек: додати нову компанію ─────────────────────────────────────────
    let ui_weak_company_add = ui.as_weak();
    ui.on_company_add_clicked(move || {
        if let Some(ui) = ui_weak_company_add.upgrade() {
            reset_company_form(&ui);
            ui.set_show_company_picker(false);
            ui.set_current_page(6);
            ui.set_show_company_form(true);
        }
    });

    // ── Колбек: редагувати компанію ───────────────────────────────────────────
    let pool_company_edit = pool.clone();
    let ui_weak_company_edit = ui.as_weak();

    ui.on_company_edit_clicked(move |id_str| {
        let pool = pool_company_edit.clone();
        let ui_handle = ui_weak_company_edit.clone();
        let id_s = id_str.to_string();

        tokio::spawn(async move {
            let Ok(uuid) = id_s.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID компанії: {id_s}");
                return;
            };

            match db::companies::get_by_id(&pool, uuid).await {
                Ok(Some(c)) => {
                    ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.set_company_form_is_edit(true);
                        ui.set_company_form_edit_id(SharedString::from(c.id.to_string().as_str()));
                        ui.set_company_form_name(SharedString::from(c.name.as_str()));
                        ui.set_company_form_edrpou(SharedString::from(c.edrpou.as_deref().unwrap_or("")));
                        ui.set_company_form_iban(SharedString::from(c.iban.as_deref().unwrap_or("")));
                        ui.set_company_form_legal_address(SharedString::from(c.legal_address.as_deref().unwrap_or("")));
                        ui.set_company_form_director(SharedString::from(c.director_name.as_deref().unwrap_or("")));
                        ui.set_company_form_accountant(SharedString::from(c.accountant_name.as_deref().unwrap_or("")));
                        ui.set_company_form_is_vat(c.is_vat_payer);
                        ui.set_show_company_form(true);
                    }).ok();
                }
                Ok(None) => tracing::warn!("Компанію {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка завантаження компанії: {e}"),
            }
        });
    });

    // ── Колбек: архівувати компанію ───────────────────────────────────────────
    let pool_company_archive = pool.clone();
    let ui_weak_company_archive = ui.as_weak();
    let active_company_id_archive = active_company_id.clone();

    ui.on_company_archive_clicked(move |id_str| {
        let pool = pool_company_archive.clone();
        let ui_handle = ui_weak_company_archive.clone();
        let active_company_id = active_company_id_archive.clone();
        let id_s = id_str.to_string();

        tokio::spawn(async move {
            let Ok(uuid) = id_s.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID компанії: {id_s}");
                return;
            };

            match db::companies::archive(&pool, uuid).await {
                Ok(true) => {
                    show_toast(ui_handle.clone(), "Компанію архівовано".to_string(), false);
                    let active_id = *active_company_id.lock().unwrap();
                    if let Err(e) = reload_companies(&pool, ui_handle.clone(), active_id).await {
                        tracing::error!("Помилка оновлення списку компаній: {e}");
                    }

                    if *active_company_id.lock().unwrap() == uuid {
                        match db::companies::list(&pool).await {
                            Ok(companies) if !companies.is_empty() => {
                                let replacement = companies[0].clone();
                                *active_company_id.lock().unwrap() = replacement.id;

                                let mut cfg = AppConfig::load();
                                cfg.last_company_id = Some(replacement.id);
                                cfg.save();

                                let name = company_display_name(&replacement);
                                let subtitle = company_subtitle(&replacement);
                                let replacement_id = replacement.id.to_string();

                                ui_handle
                                    .upgrade_in_event_loop(move |ui| {
                                        ui.set_active_company_name(SharedString::from(name.as_str()));
                                        ui.set_active_company_id(SharedString::from(
                                            replacement_id.as_str(),
                                        ));
                                        ui.set_active_company_subtitle(SharedString::from(
                                            subtitle.as_str(),
                                        ));
                                    })
                                    .ok();
                            }
                            Ok(_) => {
                                ui_handle
                                    .upgrade_in_event_loop(|ui| {
                                        ui.set_active_company_name(SharedString::from(
                                            "Оберіть компанію",
                                        ));
                                        ui.set_active_company_id(SharedString::from(""));
                                        ui.set_active_company_subtitle(SharedString::from(
                                            "Створіть першу компанію",
                                        ));
                                        ui.set_show_company_picker(false);
                                        ui.set_current_page(6);
                                        reset_company_form(&ui);
                                        ui.set_show_company_form(true);
                                    })
                                    .ok();
                            }
                            Err(e) => tracing::error!(
                                "Помилка пошуку заміни активної компанії після архівації: {e}"
                            ),
                        }
                    }
                }
                Ok(false) => tracing::warn!("Компанію {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка архівування компанії: {e}"),
            }
        });
    });

    // ── Колбек: зберегти нову компанію ──────────────────────────────────────
    let pool_company_save = pool.clone();
    let ui_weak_company_save = ui.as_weak();
    let active_company_id_company_save = active_company_id.clone();
    let counterparty_state_company_save = counterparty_state.clone();
    let act_state_company_save = act_state.clone();

    ui.on_company_form_save(move |name, edrpou, iban, address, director, _accountant, is_vat| {
        let pool = pool_company_save.clone();
        let ui_weak = ui_weak_company_save.clone();
        let active_company_id = active_company_id_company_save.clone();
        let counterparty_state = counterparty_state_company_save.clone();
        let act_state = act_state_company_save.clone();
        let data = NewCompany {
            name: name.to_string(),
            short_name: None,
            edrpou: if edrpou.trim().is_empty() { None } else { Some(edrpou.to_string()) },
            ipn: None,
            iban: if iban.trim().is_empty() { None } else { Some(iban.to_string()) },
            legal_address: if address.trim().is_empty() { None } else { Some(address.to_string()) },
            director_name: if director.trim().is_empty() { None } else { Some(director.to_string()) },
            tax_system: None,
            is_vat_payer: is_vat,
        };

        tokio::spawn(async move {
            if data.name.trim().is_empty() {
                show_toast(ui_weak, "Введіть назву компанії".to_string(), true);
                return;
            }
            match db::companies::create(&pool, &data).await {
                Ok(c) => {
                    tracing::info!("Компанію '{}' створено (id={}).", c.name, c.id);
                    show_toast(ui_weak.clone(), format!("Компанію '{}' створено", c.name), false);
                    *active_company_id.lock().unwrap() = c.id;

                    let mut cfg = AppConfig::load();
                    cfg.last_company_id = Some(c.id);
                    cfg.save();

                    let active_id = c.id;
                    if let Err(e) = reload_companies(&pool, ui_weak.clone(), active_id).await {
                        tracing::error!("Помилка оновлення списку компаній: {e}");
                    }

                    let (cp_query, include_archived, cp_page) = {
                        let state = counterparty_state.lock().unwrap();
                        (state.query.clone(), state.include_archived, state.page)
                    };
                    let (act_query, status_filter) = {
                        let state = act_state.lock().unwrap();
                        (state.query.clone(), state.status_filter.clone())
                    };

                    if let Err(e) = reload_counterparties(
                        &pool,
                        ui_weak.clone(),
                        c.id,
                        cp_query,
                        include_archived,
                        cp_page,
                        false,
                    )
                    .await
                    {
                        tracing::error!("Помилка оновлення контрагентів після створення компанії: {e}");
                    }

                    if let Err(e) = reload_acts(
                        &pool,
                        ui_weak.clone(),
                        c.id,
                        status_filter,
                        act_query,
                        false,
                    )
                    .await
                    {
                        tracing::error!("Помилка оновлення актів після створення компанії: {e}");
                    }

                    let name = company_display_name(&c);
                    let subtitle = company_subtitle(&c);
                    let id = c.id.to_string();
                    ui_weak
                        .upgrade_in_event_loop(move |ui| {
                            ui.set_active_company_name(SharedString::from(name.as_str()));
                            ui.set_active_company_id(SharedString::from(id.as_str()));
                            ui.set_active_company_subtitle(SharedString::from(subtitle.as_str()));
                            ui.set_show_company_picker(false);
                            ui.set_show_company_form(false);
                            ui.set_current_page(0);
                        })
                        .ok();
                }
                Err(e) => {
                    tracing::error!("Помилка створення компанії: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });

    // ── Колбек: оновити компанію ─────────────────────────────────────────────
    let pool_company_update = pool.clone();
    let ui_weak_company_update = ui.as_weak();
    let active_company_id_company_update = active_company_id.clone();

    ui.on_company_form_update(move |name, edrpou, iban, address, director, accountant, is_vat| {
        let Some(ui_ref) = ui_weak_company_update.upgrade() else { return; };
        let edit_id = ui_ref.get_company_form_edit_id().to_string();
        let pool = pool_company_update.clone();
        let ui_weak = ui_weak_company_update.clone();
        let active_company_id = active_company_id_company_update.clone();

        tokio::spawn(async move {
            let Ok(uuid) = edit_id.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний edit_id компанії: {edit_id}");
                return;
            };
            let data = UpdateCompany {
                name: name.to_string(),
                short_name: None,
                edrpou: if edrpou.trim().is_empty() { None } else { Some(edrpou.to_string()) },
                iban: if iban.trim().is_empty() { None } else { Some(iban.to_string()) },
                legal_address: if address.trim().is_empty() { None } else { Some(address.to_string()) },
                director_name: if director.trim().is_empty() { None } else { Some(director.to_string()) },
                accountant_name: if accountant.trim().is_empty() { None } else { Some(accountant.to_string()) },
                tax_system: None,
                is_vat_payer: is_vat,
                logo_path: None,
            };
            match db::companies::update(&pool, uuid, &data).await {
                Ok(Some(c)) => {
                    tracing::info!("Компанію '{}' оновлено.", c.name);
                    show_toast(ui_weak.clone(), format!("Компанію '{}' оновлено", c.name), false);
                    let active_id = *active_company_id.lock().unwrap();
                    if let Err(e) = reload_companies(&pool, ui_weak.clone(), active_id).await {
                        tracing::error!("Помилка оновлення списку компаній: {e}");
                    }

                    if *active_company_id.lock().unwrap() == c.id {
                        let name = company_display_name(&c);
                        let subtitle = company_subtitle(&c);
                        let id = c.id.to_string();
                        ui_weak
                            .upgrade_in_event_loop(move |ui| {
                                ui.set_active_company_name(SharedString::from(name.as_str()));
                                ui.set_active_company_id(SharedString::from(id.as_str()));
                                ui.set_active_company_subtitle(SharedString::from(
                                    subtitle.as_str(),
                                ));
                                ui.set_show_company_form(false);
                            })
                            .ok();
                    } else {
                        ui_weak
                            .upgrade_in_event_loop(|ui| {
                                ui.set_show_company_form(false);
                            })
                            .ok();
                    }
                }
                Ok(None) => tracing::warn!("Компанію {uuid} не знайдено."),
                Err(e) => {
                    tracing::error!("Помилка оновлення компанії: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });

    // ── Колбек: скасувати форму компанії ─────────────────────────────────────
    let ui_weak_company_cancel = ui.as_weak();
    ui.on_company_form_cancel(move || {
        if let Some(ui) = ui_weak_company_cancel.upgrade() {
            ui.set_show_company_form(false);
        }
    });

    // run() блокує до закриття вікна
    ui.run()?;
    Ok(())
}

// ── Проміжний формат даних (Send) ────────────────────────────────────────────
//
// Чому не повертати ModelRc напряму?
// ModelRc = Rc<dyn Model> — не є Send (не можна передати між потоками).
// Ці прості Vec є Send і можна безпечно передати в upgrade_in_event_loop.
struct TableData {
    // Рядки таблиці: зовнішній Vec = рядки, внутрішній = комірки
    rows: Vec<Vec<SharedString>>,
    // Паралельний масив UUID — rows[i] відповідає ids[i]
    ids: Vec<SharedString>,
    // Паралельний масив архівованості — true якщо контрагент в архіві
    archived: Vec<bool>,
}

// Конвертуємо контрагентів у проміжний формат.
// Колонки: Назва, ЄДРПОУ, IBAN, Телефон (email не відображається в таблиці).
fn to_table_data(cps: &[models::Counterparty]) -> TableData {
    let rows = cps
        .iter()
        .map(|cp| {
            vec![
                SharedString::from(cp.name.as_str()),
                SharedString::from(cp.edrpou.as_deref().unwrap_or("—")),
                SharedString::from(cp.iban.as_deref().unwrap_or("—")),
                SharedString::from(cp.phone.as_deref().unwrap_or("—")),
            ]
        })
        .collect();

    let ids = cps
        .iter()
        .map(|cp| SharedString::from(cp.id.to_string().as_str()))
        .collect();

    let archived = cps.iter().map(|cp| cp.is_archived).collect();

    TableData {
        rows,
        ids,
        archived,
    }
}

// ── Проміжний формат для актів (Send) ───────────────────────────────────────
struct ActsTableData {
    rows: Vec<Vec<SharedString>>,
    ids: Vec<SharedString>,
    // Сирі рядки статусів ("draft", "issued", ...) для логіки кнопки в Slint
    statuses: Vec<SharedString>,
}

fn to_acts_table_data(acts: &[models::ActListRow]) -> ActsTableData {
    let rows = acts
        .iter()
        .map(|a| {
            vec![
                SharedString::from(a.number.as_str()),
                // NaiveDate → "дд.мм.рррр" для відображення в таблиці
                SharedString::from(a.date.format("%d.%m.%Y").to_string().as_str()),
                SharedString::from(a.counterparty_name.as_str()),
                SharedString::from(format!("{:.2}", a.total_amount).as_str()),
                SharedString::from(a.status.label()),
            ]
        })
        .collect();

    let ids = acts
        .iter()
        .map(|a| SharedString::from(a.id.to_string().as_str()))
        .collect();

    let statuses = acts
        .iter()
        .map(|a| SharedString::from(a.status.as_str()))
        .collect();

    ActsTableData {
        rows,
        ids,
        statuses,
    }
}

fn build_acts_models(
    data: ActsTableData,
) -> (
    ModelRc<ModelRc<StandardListViewItem>>,
    ModelRc<SharedString>,
    ModelRc<SharedString>,
) {
    let rows: Vec<ModelRc<StandardListViewItem>> = data
        .rows
        .into_iter()
        .map(|cells| {
            let items: Vec<StandardListViewItem> = cells
                .iter()
                .map(|s| StandardListViewItem::from(s.as_str()))
                .collect();
            ModelRc::new(VecModel::from(items))
        })
        .collect();

    (
        ModelRc::new(VecModel::from(rows)),
        ModelRc::new(VecModel::from(data.ids)),
        ModelRc::new(VecModel::from(data.statuses)),
    )
}

// Будуємо Slint моделі з TableData.
// Викликати ТІЛЬКИ з main thread (або з upgrade_in_event_loop).
//
// StandardListViewItem::from(&str) — офіційний спосіб побудови
// (struct non-exhaustive, тому { text: ... } не компілюється).
fn build_models(
    data: TableData,
) -> (
    ModelRc<ModelRc<StandardListViewItem>>,
    ModelRc<SharedString>,
    ModelRc<bool>,
) {
    // Кожен рядок → ModelRc<StandardListViewItem>
    let rows: Vec<ModelRc<StandardListViewItem>> = data
        .rows
        .into_iter()
        .map(|cells| {
            let items: Vec<StandardListViewItem> = cells
                .iter()
                .map(|s| StandardListViewItem::from(s.as_str()))
                .collect();
            ModelRc::new(VecModel::from(items))
        })
        .collect();

    (
        ModelRc::new(VecModel::from(rows)),
        ModelRc::new(VecModel::from(data.ids)),
        ModelRc::new(VecModel::from(data.archived)),
    )
}

// ── Допоміжні функції для форми актів ────────────────────────────────────────

/// Зчитує поточний стан позицій з паралельних масивів форми.
///
/// Викликати ТІЛЬКИ з main thread (в синхронному колбеку Slint),
/// бо MainWindow не є Send і не може бути передана між потоками.
///
/// Позиції з невалідними числами (quantity або price) — мовчки пропускаються.
fn collect_form_items(ui: &MainWindow) -> Vec<NewActItem> {
    let descs = ui.get_act_form_item_descriptions();
    let qtys = ui.get_act_form_item_quantities();
    let units = ui.get_act_form_item_units();
    let prices = ui.get_act_form_item_prices();

    (0..descs.row_count())
        .filter_map(|i| {
            let description = descs.row_data(i)?.to_string();
            let qty_str = qtys.row_data(i)?;
            let unit = units.row_data(i)?.to_string();
            let price_str = prices.row_data(i)?;

            // parse::<Decimal>() — стандартний FromStr для rust_decimal
            // Якщо поле порожнє або не є числом — filter_map видаляє рядок
            let quantity = qty_str.parse::<Decimal>().ok()?;
            let unit_price = price_str.parse::<Decimal>().ok()?;

            Some(NewActItem {
                description,
                quantity,
                unit,
                unit_price,
            })
        })
        .collect()
}

async fn reload_counterparties(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    query: String,
    include_archived: bool,
    page: usize,
    close_form: bool,
) -> Result<()> {
    let filter_query = normalized_query(&query);
    let all_counterparties =
        db::counterparties::list_filtered(pool, company_id, filter_query, true).await?;

    let total_all = all_counterparties.len();
    let active_all = all_counterparties.iter().filter(|cp| !cp.is_archived).count();
    let archived_all = all_counterparties.iter().filter(|cp| cp.is_archived).count();

    let filtered_counterparties: Vec<_> = if include_archived {
        all_counterparties
    } else {
        all_counterparties
            .into_iter()
            .filter(|cp| !cp.is_archived)
            .collect()
    };

    let current_page = page.min(total_filtered_pages(filtered_counterparties.len()).saturating_sub(1));

    let start = current_page * COUNTERPARTY_PAGE_SIZE;
    let end = (start + COUNTERPARTY_PAGE_SIZE).min(filtered_counterparties.len());
    let page_slice = if start < filtered_counterparties.len() {
        &filtered_counterparties[start..end]
    } else {
        &filtered_counterparties[0..0]
    };

    let data = to_table_data(page_slice);
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
    let pagination = SharedString::from(page_label.as_str());
    let current_page_ui = (current_page + 1) as i32;

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            let (rows, ids, archived) = build_models(data);
            ui.set_counterparty_rows(rows);
            ui.set_counterparty_ids(ids);
            ui.set_counterparty_archived(archived);
            ui.set_counterparty_total_count(total_all as i32);
            ui.set_counterparty_active_count(active_all as i32);
            ui.set_counterparty_archived_count(archived_all as i32);
            ui.set_counterparty_pagination_text(pagination);
            ui.set_counterparty_show_archived(include_archived);
            ui.set_counterparty_current_page(current_page_ui);
            ui.set_counterparty_total_pages(total_pages.max(1));
            if close_form {
                ui.set_show_cp_form(false);
            }
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

fn total_filtered_pages(total_items: usize) -> usize {
    let pages = total_items.div_ceil(COUNTERPARTY_PAGE_SIZE);
    pages.max(1)
}

async fn reload_acts(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    status_filter: Option<ActStatus>,
    query: String,
    close_form: bool,
) -> Result<()> {
    let acts = db::acts::list_filtered(pool, company_id, status_filter, normalized_query(&query)).await?;
    let data = to_acts_table_data(&acts);

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            let (rows, ids, statuses) = build_acts_models(data);
            ui.set_act_rows(rows);
            ui.set_act_row_ids(ids);
            ui.set_act_row_statuses(statuses);
            if close_form {
                ui.set_show_act_form(false);
            }
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

/// Завантажити список накладних компанії і оновити UI.
async fn reload_invoices(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    status_filter: Option<InvoiceStatus>,
    query: String,
    close_form: bool,
) -> Result<()> {
    let invoices = db::invoices::list_filtered(pool, company_id, status_filter, normalized_query(&query)).await?;
    let rows: Vec<Vec<SharedString>> = invoices
        .iter()
        .map(|inv| {
            let status_label = match inv.status {
                InvoiceStatus::Draft => "Чернетка",
                InvoiceStatus::Issued => "Виставлено",
                InvoiceStatus::Signed => "Підписано",
                InvoiceStatus::Paid => "Оплачено",
            };
            vec![
                SharedString::from(inv.number.as_str()),
                SharedString::from(inv.date.format("%d.%m.%Y").to_string().as_str()),
                SharedString::from(inv.counterparty_name.as_str()),
                SharedString::from(format!("{:.2}", inv.total_amount).as_str()),
                SharedString::from(status_label),
            ]
        })
        .collect();
    let ids: Vec<SharedString> = invoices.iter().map(|inv| SharedString::from(inv.id.to_string().as_str())).collect();
    let statuses: Vec<SharedString> = invoices.iter().map(|inv| SharedString::from(inv.status.as_str())).collect();

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            let slint_rows: Vec<_> = rows
                .into_iter()
                .map(|row| {
                    let items: Vec<StandardListViewItem> = row
                        .into_iter()
                        .map(StandardListViewItem::from)
                        .collect();
                    ModelRc::new(VecModel::from(items))
                })
                .collect();
            ui.set_invoice_rows(ModelRc::new(VecModel::from(slint_rows)));
            ui.set_invoice_row_ids(ModelRc::new(VecModel::from(ids)));
            ui.set_invoice_row_statuses(ModelRc::new(VecModel::from(statuses)));
            if close_form {
                ui.set_show_invoice_form(false);
            }
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

/// Зібрати позиції накладної з UI-полів у Vec<NewInvoiceItem>.
fn collect_invoice_items_from_ui(ui_weak: &Weak<MainWindow>) -> Vec<NewInvoiceItem> {
    use slint::Model;
    let Some(ui) = ui_weak.upgrade() else { return vec![]; };
    let descs = ui.get_invoice_form_item_descriptions();
    let qtys = ui.get_invoice_form_item_quantities();
    let units = ui.get_invoice_form_item_units();
    let prices = ui.get_invoice_form_item_prices();
    let count = descs.row_count();
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let desc = descs.row_data(i).unwrap_or_default().to_string();
        let qty_str = qtys.row_data(i).unwrap_or_default().to_string();
        let unit = units.row_data(i).unwrap_or_default().to_string();
        let price_str = prices.row_data(i).unwrap_or_default().to_string();
        let quantity = qty_str.parse::<Decimal>().unwrap_or(Decimal::ONE);
        let price = price_str.parse::<Decimal>().unwrap_or(Decimal::ZERO);
        items.push(NewInvoiceItem {
            position: (i + 1) as i16,
            description: desc,
            unit: if unit.is_empty() { None } else { Some(unit) },
            quantity,
            price,
        });
    }
    items
}

/// Перерахувати total-amount у формі накладної на основі позицій.
fn recalculate_invoice_total(ui: &MainWindow) {
    use slint::Model;
    let qtys = ui.get_invoice_form_item_quantities();
    let prices = ui.get_invoice_form_item_prices();
    let mut total = Decimal::ZERO;
    for i in 0..qtys.row_count() {
        let qty = qtys.row_data(i).unwrap_or_default().to_string().parse::<Decimal>().unwrap_or(Decimal::ZERO);
        let price = prices.row_data(i).unwrap_or_default().to_string().parse::<Decimal>().unwrap_or(Decimal::ZERO);
        // Оновлюємо amount для рядка (відображення)
        let amounts = ui.get_invoice_form_item_amounts();
        let mut amounts_v: Vec<SharedString> = (0..amounts.row_count()).filter_map(|j| amounts.row_data(j)).collect();
        let amount = (qty * price).round_dp(2);
        if i < amounts_v.len() {
            amounts_v[i] = SharedString::from(amount.to_string().as_str());
        }
        ui.set_invoice_form_item_amounts(ModelRc::new(VecModel::from(amounts_v)));
        total += amount;
    }
    ui.set_invoice_form_total(SharedString::from(format!("{:.2}", total).as_str()));
}

/// Завантажити список компаній і оновити UI.
async fn reload_companies(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    active_company_id: uuid::Uuid,
) -> Result<()> {
    let companies = db::companies::list_with_summary(pool).await?;
    ui_weak.upgrade_in_event_loop(move |ui| {
        apply_company_rows(&ui, &companies, active_company_id);
    }).map_err(anyhow::Error::from)?;
    Ok(())
}

/// Перетворити Vec<CompanySummary> у ModelRc і встановити у UI.
fn apply_company_rows(ui: &MainWindow, companies: &[CompanySummary], active_company_id: uuid::Uuid) {
    let items: Vec<CompanyItem> = companies.iter().map(|c| CompanyItem {
        id:           SharedString::from(c.id.to_string().as_str()),
        name:         SharedString::from(c.name.as_str()),
        short_name:   SharedString::from(c.short_name.as_deref().unwrap_or("")),
        edrpou:       SharedString::from(c.edrpou.as_deref().unwrap_or("")),
        is_vat:       c.is_vat_payer,
        act_count:    c.act_count as i32,
        total_amount: SharedString::from(format_company_total(&c.total_amount).as_str()),
        is_current:   c.id == active_company_id,
        initials:     SharedString::from(company_initials(c).as_str()),
    }).collect();
    ui.set_company_rows(ModelRc::new(VecModel::from(items)));
}

fn company_display_name(company: &Company) -> String {
    company
        .short_name
        .clone()
        .unwrap_or_else(|| company.name.clone())
}

fn company_subtitle(company: &Company) -> String {
    company
        .edrpou
        .as_ref()
        .map(|edrpou| format!("ЄДРПОУ: {edrpou}"))
        .unwrap_or_else(|| "ЄДРПОУ не вказано".to_string())
}

fn company_initials(company: &CompanySummary) -> String {
    let source = company
        .short_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(company.name.as_str());

    let mut letters = source
        .split(|c: char| c.is_whitespace() || c == '-' || c == '—')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();

    if letters.is_empty() {
        letters = "К".to_string();
    }

    letters
}

fn format_company_total(amount: &Decimal) -> String {
    format!("{} грн", amount.round_dp(2))
}

fn reset_company_form(ui: &MainWindow) {
    ui.set_company_form_is_edit(false);
    ui.set_company_form_edit_id(SharedString::from(""));
    ui.set_company_form_name(SharedString::from(""));
    ui.set_company_form_edrpou(SharedString::from(""));
    ui.set_company_form_iban(SharedString::from(""));
    ui.set_company_form_legal_address(SharedString::from(""));
    ui.set_company_form_director(SharedString::from(""));
    ui.set_company_form_accountant(SharedString::from(""));
    ui.set_company_form_is_vat(false);
}

fn normalized_query(query: &str) -> Option<&str> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Запускає tokio::spawn для збереження акту в БД.
///
/// Повертається відразу (non-blocking).
/// Після успішного збереження:
///   - Перезавантажує список актів
///   - Через upgrade_in_event_loop оновлює UI та ховає форму
///
/// `pool` — клонований PgPool (дешево: пул використовує Arc всередині).
fn spawn_save_act(
    pool: sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    act_state: Arc<Mutex<ActListState>>,
    company_id: uuid::Uuid,
    number: String,
    date_str: String,
    cp_id_str: String,
    notes: Option<String>,
    items: Vec<NewActItem>,
) {
    tokio::spawn(async move {
        // Валідація обов'язкових полів форми
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

        // Парсимо дату зі строки ДД.ММ.РРРР → chrono::NaiveDate
        let date = match NaiveDate::parse_from_str(&date_str, "%d.%m.%Y") {
            Ok(d) => d,
            Err(_) => {
                tracing::error!("Невірний формат дати: '{date_str}'. Очікується ДД.ММ.РРРР");
                return;
            }
        };

        // UUID контрагента — якщо порожній або невалідний → помилка
        let cp_uuid = match uuid::Uuid::parse_str(&cp_id_str) {
            Ok(id) => id,
            Err(_) => {
                tracing::error!("Контрагент не вибраний або UUID некоректний: '{cp_id_str}'");
                return;
            }
        };

        let new_act = NewAct {
            number: number.clone(),
            counterparty_id: cp_uuid,
            contract_id: None, // договір — майбутня функція
            date,
            notes,
            bas_id: None,
            items,
        };

        match db::acts::create(&pool, company_id, &new_act).await {
            Ok(act) => {
                tracing::info!("Акт '{}' збережено (id={}).", act.number, act.id);
                show_toast(
                    ui_weak.clone(),
                    format!("Акт '{}' збережено", act.number),
                    false,
                );

                // Оновлюємо список та повертаємось до нього
                let (query, status_filter) = {
                    let state = act_state.lock().unwrap();
                    (state.query.clone(), state.status_filter.clone())
                };
                if let Err(e) =
                    reload_acts(&pool, ui_weak.clone(), company_id, status_filter, query, true).await
                {
                    tracing::error!("Помилка оновлення списку актів після збереження: {e}");
                }
            }
            Err(e) => {
                tracing::error!("Помилка збереження акту: {e}");
                show_toast(ui_weak.clone(), format!("Помилка: {e}"), true);
            }
        }
    });
}

/// Запустити збереження нової накладної у фоновому tokio завданні.
fn spawn_save_invoice(
    pool: sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    inv_state: Arc<Mutex<InvoiceListState>>,
    company_id: uuid::Uuid,
    number: String,
    date_str: String,
    cp_id_str: String,
    notes: Option<String>,
    items: Vec<NewInvoiceItem>,
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
            Err(_) => { tracing::error!("Невірний формат дати: '{date_str}'"); return; }
        };
        let cp_uuid = match uuid::Uuid::parse_str(&cp_id_str) {
            Ok(id) => id,
            Err(_) => { tracing::error!("Невалідний UUID контрагента: '{cp_id_str}'"); return; }
        };
        let new_invoice = NewInvoice {
            number: number.clone(),
            counterparty_id: cp_uuid,
            contract_id: None,
            date,
            notes,
            bas_id: None,
            items,
        };
        match db::invoices::create(&pool, company_id, &new_invoice).await {
            Ok(inv) => {
                tracing::info!("Накладну '{}' збережено (id={}).", inv.number, inv.id);
                show_toast(ui_weak.clone(), format!("Накладну '{}' збережено", inv.number), false);
                let (status_filter, query) = {
                    let state = inv_state.lock().unwrap();
                    (state.status_filter.clone(), state.query.clone())
                };
                if let Err(e) = reload_invoices(&pool, ui_weak, company_id, status_filter, query, true).await {
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

/// Показує toast-сповіщення на 3 секунди, потім автоматично прибирає.
fn show_toast(ui_weak: Weak<MainWindow>, message: String, is_error: bool) {
    let msg = SharedString::from(message.as_str());
    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_toast_message(msg);
            ui.set_toast_is_error(is_error);
        })
        .ok();

    let clear_handle = ui_weak.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        clear_handle
            .upgrade_in_event_loop(|ui| {
                ui.set_toast_message(SharedString::from(""));
            })
            .ok();
    });
}

struct TasksTableData {
    rows: Vec<Vec<SharedString>>,
    ids: Vec<SharedString>,
    statuses: Vec<SharedString>,
    priorities: Vec<SharedString>,
}

fn task_priority_from_index(index: i32) -> TaskPriority {
    match index {
        0 => TaskPriority::Low,
        1 => TaskPriority::Normal,
        2 => TaskPriority::High,
        _ => TaskPriority::Critical,
    }
}

fn task_priority_index(priority: &TaskPriority) -> i32 {
    match priority {
        TaskPriority::Low => 0,
        TaskPriority::Normal => 1,
        TaskPriority::High => 2,
        TaskPriority::Critical => 3,
    }
}

fn format_task_datetime(value: Option<DateTime<Utc>>) -> SharedString {
    value
        .map(|dt| SharedString::from(dt.format("%d.%m.%Y %H:%M").to_string().as_str()))
        .unwrap_or_else(|| SharedString::from("—"))
}

fn parse_task_datetime(input: &str) -> Result<Option<DateTime<Utc>>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let naive = NaiveDateTime::parse_from_str(trimmed, "%d.%m.%Y %H:%M").map_err(|_| {
        anyhow::anyhow!("Невірний формат дати/часу: '{trimmed}'. Очікується ДД.ММ.РРРР ГГ:ХХ")
    })?;

    Ok(Some(Utc.from_utc_datetime(&naive)))
}

fn task_matches_query(task: &Task, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };

    let needle = query.to_lowercase();
    task.title.to_lowercase().contains(&needle)
        || task
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(&needle)
}

fn to_tasks_table_data(tasks: &[Task]) -> TasksTableData {
    let rows = tasks
        .iter()
        .map(|task| {
            vec![
                SharedString::from(task.title.as_str()),
                SharedString::from(task.priority.label()),
                format_task_datetime(task.due_date),
                format_task_datetime(task.reminder_at),
                SharedString::from(task.status.label()),
            ]
        })
        .collect();

    let ids = tasks
        .iter()
        .map(|task| SharedString::from(task.id.to_string().as_str()))
        .collect();

    let statuses = tasks
        .iter()
        .map(|task| SharedString::from(task.status.as_str()))
        .collect();

    let priorities = tasks
        .iter()
        .map(|task| SharedString::from(task.priority.as_str()))
        .collect();

    TasksTableData {
        rows,
        ids,
        statuses,
        priorities,
    }
}

fn build_task_models(
    data: TasksTableData,
) -> (
    ModelRc<ModelRc<StandardListViewItem>>,
    ModelRc<SharedString>,
    ModelRc<SharedString>,
    ModelRc<SharedString>,
) {
    let rows: Vec<ModelRc<StandardListViewItem>> = data
        .rows
        .into_iter()
        .map(|cells| {
            let items: Vec<StandardListViewItem> = cells
                .iter()
                .map(|s| StandardListViewItem::from(s.as_str()))
                .collect();
            ModelRc::new(VecModel::from(items))
        })
        .collect();

    (
        ModelRc::new(VecModel::from(rows)),
        ModelRc::new(VecModel::from(data.ids)),
        ModelRc::new(VecModel::from(data.statuses)),
        ModelRc::new(VecModel::from(data.priorities)),
    )
}

async fn reload_tasks(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    query: String,
    close_form: bool,
) -> Result<()> {
    let tasks = db::tasks::list_open(pool).await?;
    let filtered: Vec<Task> = tasks
        .into_iter()
        .filter(|task| task_matches_query(task, normalized_query(&query)))
        .collect();
    let data = to_tasks_table_data(&filtered);

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            let (rows, ids, statuses, priorities) = build_task_models(data);
            ui.set_task_rows(rows);
            ui.set_task_row_ids(ids);
            ui.set_task_row_statuses(statuses);
            ui.set_task_row_priorities(priorities);
            ui.set_tasks_loading(false);
            if close_form {
                ui.set_show_task_form(false);
                ui.set_current_page(ui.get_task_form_return_page());
            }
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

async fn reload_act_tasks(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    act_id: uuid::Uuid,
) -> Result<()> {
    let tasks = db::tasks::list_by_act(pool, act_id).await?;
    let data = to_tasks_table_data(&tasks);

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            let (rows, ids, statuses, priorities) = build_task_models(data);
            ui.set_act_task_rows(rows);
            ui.set_act_task_row_ids(ids);
            ui.set_act_task_row_statuses(statuses);
            ui.set_act_task_row_priorities(priorities);
            ui.set_act_tasks_loading(false);
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

fn spawn_save_task(
    pool: sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    task_state: Arc<Mutex<TaskListState>>,
    task_id: Option<String>,
    title: String,
    description: String,
    priority_idx: i32,
    due_str: String,
    reminder_str: String,
    act_id: String,
) {
    tokio::spawn(async move {
        let return_page = ui_weak
            .upgrade()
            .map(|ui| ui.get_task_form_return_page())
            .unwrap_or(5);

        if title.trim().is_empty() {
            tracing::error!("Назва задачі не може бути порожньою");
            show_toast(
                ui_weak.clone(),
                "Назва задачі не може бути порожньою".to_string(),
                true,
            );
            return;
        }

        let due_date = match parse_task_datetime(&due_str) {
            Ok(value) => value,
            Err(e) => {
                tracing::error!("Помилка дедлайну: {e}");
                show_toast(ui_weak.clone(), e.to_string(), true);
                return;
            }
        };

        let reminder_at = match parse_task_datetime(&reminder_str) {
            Ok(value) => value,
            Err(e) => {
                tracing::error!("Помилка нагадування: {e}");
                show_toast(ui_weak.clone(), e.to_string(), true);
                return;
            }
        };

        let task = NewTask {
            title: title.clone(),
            description: if description.trim().is_empty() {
                None
            } else {
                Some(description.clone())
            },
            priority: task_priority_from_index(priority_idx),
            due_date,
            reminder_at,
            counterparty_id: None,
            act_id: if act_id.trim().is_empty() {
                None
            } else {
                act_id.parse::<uuid::Uuid>().ok()
            },
        };

        let is_update = task_id
            .as_deref()
            .map(|id| !id.trim().is_empty())
            .unwrap_or(false);
        let act_uuid = if return_page == 1 && !act_id.trim().is_empty() {
            act_id.parse::<uuid::Uuid>().ok()
        } else {
            None
        };

        let result = if is_update {
            let Some(id_str) = task_id.as_deref() else {
                unreachable!("checked above");
            };
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                show_toast(ui_weak.clone(), "Некоректний UUID задачі".to_string(), true);
                return;
            };
            db::tasks::update(&pool, uuid, &task).await
        } else {
            db::tasks::create(&pool, &task).await.map(Some)
        };

        match result {
            Ok(Some(saved)) => {
                let message = if is_update {
                    format!("Задачу '{}' оновлено", saved.title)
                } else {
                    format!("Задачу '{}' створено", saved.title)
                };
                show_toast(ui_weak.clone(), message, false);
                if let Some(act_uuid) = act_uuid {
                    if let Err(e) = reload_act_tasks(&pool, ui_weak.clone(), act_uuid).await {
                        tracing::error!("Помилка перезавантаження задач акту: {e}");
                    }
                    ui_weak
                        .upgrade_in_event_loop(|ui| {
                            ui.set_show_task_form(false);
                            ui.set_current_page(ui.get_task_form_return_page());
                        })
                        .ok();
                } else {
                    let query = {
                        let state = task_state.lock().unwrap();
                        state.query.clone()
                    };
                    if let Err(e) = reload_tasks(&pool, ui_weak.clone(), query, true).await {
                        tracing::error!("Помилка перезавантаження задач: {e}");
                    }
                }
            }
            Ok(None) => {
                tracing::warn!("Задачу не знайдено для оновлення");
                show_toast(ui_weak.clone(), "Задачу не знайдено".to_string(), true);
            }
            Err(e) => {
                tracing::error!("Помилка збереження задачі: {e}");
                show_toast(ui_weak.clone(), format!("Помилка: {e}"), true);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rust_decimal::Decimal;
    use uuid::Uuid;

    use crate::models::{ActListRow, ActStatus, Counterparty};

    use super::{normalized_query, to_acts_table_data, to_table_data};

    #[test]
    fn normalized_query_returns_none_for_empty_input() {
        assert_eq!(normalized_query(""), None);
        assert_eq!(normalized_query("   "), None);
    }

    #[test]
    fn normalized_query_trims_non_empty_text() {
        assert_eq!(normalized_query("  тест  "), Some("тест"));
    }

    #[test]
    fn to_table_data_uses_placeholders_for_missing_optional_fields() {
        let cp = Counterparty {
            id: Uuid::new_v4(),
            name: "ТОВ Приклад".to_string(),
            edrpou: None,
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
        };

        let table = to_table_data(&[cp]);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0][0].as_str(), "ТОВ Приклад");
        assert_eq!(table.rows[0][1].as_str(), "—");
        assert_eq!(table.rows[0][2].as_str(), "—");
        assert_eq!(table.rows[0][3].as_str(), "—");
        assert_eq!(table.archived, vec![false]);
    }

    #[test]
    fn to_acts_table_data_formats_date_amount_and_status() {
        let act = ActListRow {
            id: Uuid::new_v4(),
            number: "АКТ-2026-007".to_string(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 4, 1).expect("valid date"),
            counterparty_name: "ФОП Іваненко".to_string(),
            total_amount: Decimal::new(12345, 2),
            status: ActStatus::Issued,
        };

        let table = to_acts_table_data(&[act]);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0][0].as_str(), "АКТ-2026-007");
        assert_eq!(table.rows[0][1].as_str(), "01.04.2026");
        assert_eq!(table.rows[0][2].as_str(), "ФОП Іваненко");
        assert_eq!(table.rows[0][3].as_str(), "123.45");
        assert_eq!(table.rows[0][4].as_str(), "Виставлено");
        assert_eq!(table.statuses[0].as_str(), "issued");
    }
}
