// Acta — програма управлінського обліку
//
// Підключаємо Rust типи, згенеровані з .slint файлів.
// Після цього доступний MainWindow (та інші export компоненти).
// ВАЖЛИВО: має бути на рівні модуля — не всередині функції.
slint::include_modules!();

use acta::{config::AppConfig, db, models, notifications, pdf};

use anyhow::Result;
use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use slint::{Model, ModelRc, SharedString, StandardListViewItem, VecModel, Weak};
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};

use models::{
    ActStatus as ModelActStatus, Company, CompanySummary, InvoiceStatus, NewAct, NewActItem,
    NewCompany, NewCounterparty, NewInvoice, NewInvoiceItem, NewTask, Task, TaskPriority,
    TaskStatus, UpdateAct, UpdateCompany, UpdateCounterparty, UpdateInvoice,
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
    status_filter: Option<ModelActStatus>,
}

#[derive(Clone, Default)]
struct InvoiceListState {
    query: String,
    status_filter: Option<models::InvoiceStatus>,
}

#[derive(Clone)]
struct DocListState {
    tab:               i32,                // 0=Всі, 1=Акти, 2=Накладні
    direction:         String,             // "outgoing" | "incoming"
    counterparty_index: i32,               // 0 = всі контрагенти
    query:             String,
    counterparty_id:   Option<uuid::Uuid>, // None = всі контрагенти
    date_from:         Option<chrono::NaiveDate>,
    date_to:           Option<chrono::NaiveDate>,
}

impl Default for DocListState {
    fn default() -> Self {
        Self {
            tab: 0,
            direction: "outgoing".to_string(),
            counterparty_index: 0,
            query: String::new(),
            counterparty_id: None,
            date_from: None,
            date_to: None,
        }
    }
}

#[derive(Clone, Default)]
struct TaskListState {
    query: String,
}

#[derive(Clone, Default)]
struct PaymentListState {
    query: String,
    direction_filter: Option<models::payment::PaymentDirection>,
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
    let doc_state = Arc::new(Mutex::new(DocListState::default()));
    let payment_state = Arc::new(Mutex::new(PaymentListState::default()));
    // UUID-и контрагентів для фільтру в списку документів.
    // Індекс 0 = "Всі контрагенти" (None), індекс n = cp_ids[n-1].
    let doc_cp_ids: Arc<Mutex<Vec<uuid::Uuid>>> = Arc::new(Mutex::new(vec![]));

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

    // ── Початкове завантаження платежів ──────────────────────────────────────
    reload_payments(&pool, ui.as_weak(), cid, None, "").await?;

    // ── Початкове завантаження єдиного списку документів + фільтру контрагентів
    reload_doc_cp_filter(&pool, ui.as_weak(), cid, &doc_cp_ids).await?;
    reload_documents(&pool, ui.as_weak(), cid, 0, "outgoing", "", None, None, None).await?;
    reload_settings(&pool, ui.as_weak(), cid).await?;
    reload_payment_counterparty_options(&pool, ui.as_weak(), cid).await?;

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

    // ── Колбек: вибір контрагента — відкрити картку ─────────────────────────
    let pool_cp_card = pool.clone();
    let ui_weak_cp_card = ui.as_weak();
    let active_company_id_cp_card = active_company_id.clone();
    ui.on_counterparty_selected(move |id| {
        let pool = pool_cp_card.clone();
        let ui_weak = ui_weak_cp_card.clone();
        let cid = *active_company_id_cp_card.lock().unwrap();
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

    let ui_weak_cp_card_close = ui.as_weak();
    ui.on_counterparty_card_close_clicked(move || {
        if let Some(ui) = ui_weak_cp_card_close.upgrade() {
            ui.set_show_counterparty_card(false);
        }
    });

    let pool_cp_card_edit = pool.clone();
    let ui_weak_cp_card_edit = ui.as_weak();
    ui.on_counterparty_card_edit_clicked(move |id| {
        let pool = pool_cp_card_edit.clone();
        let ui_handle = ui_weak_cp_card_edit.clone();
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
                            ui.set_cp_form_edit_id(SharedString::from(cp.id.to_string().as_str()));
                            ui.set_cp_form_is_edit(true);
                            ui.set_show_counterparty_card(false);
                            ui.set_show_cp_form(true);
                        })
                        .ok();
                }
                Ok(None) => tracing::warn!("Контрагента {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка відкриття контрагента на редагування: {e}"),
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

// Перетворюємо індекс ComboBox в Option<ModelActStatus>
        let status_filter = match filter_idx {
        1 => Some(ModelActStatus::Draft),
        2 => Some(ModelActStatus::Issued),
        3 => Some(ModelActStatus::Signed),
        4 => Some(ModelActStatus::Paid),
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

    // ── Колбек: вибір акту — відкрити картку ────────────────────────────────
    let pool_act_card = pool.clone();
    let ui_weak_act_card = ui.as_weak();
    ui.on_act_selected(move |id| {
        let pool = pool_act_card.clone();
        let ui_weak = ui_weak_act_card.clone();
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

    // ── Колбек: закрити картку акту ──────────────────────────────────────────
    let ui_weak_act_card_close = ui.as_weak();
    ui.on_act_card_close_clicked(move || {
        if let Some(ui) = ui_weak_act_card_close.upgrade() {
            ui.set_show_act_card(false);
        }
    });

    // ── Колбек: редагувати з картки акту ─────────────────────────────────────
    let pool_act_card_edit = pool.clone();
    let ui_weak_act_card_edit = ui.as_weak();
    let active_company_id_act_card_edit = active_company_id.clone();
    ui.on_act_card_edit_clicked(move |id| {
        let pool = pool_act_card_edit.clone();
        let ui_handle = ui_weak_act_card_edit.clone();
        let id_str = id.to_string();
        let cid = *active_company_id_act_card_edit.lock().unwrap();

        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Редагувати акт з картки: некоректний UUID: {id_str}");
                return;
            };

            // Закриваємо картку і відкриваємо форму редагування через ту ж логіку
            let (act_result, cp_result, tasks_result, cat_result) = tokio::join!(
                db::acts::get_for_edit(&pool, uuid),
                db::acts::counterparties_for_select(&pool, cid),
                db::tasks::list_by_act(&pool, uuid),
                db::categories::list_all_for_select(&pool, cid),
            );

            let act_opt = match act_result {
                Ok(v) => v,
                Err(e) => { tracing::error!("Помилка завантаження акту: {e}"); return; }
            };
            let counterparties: Vec<(uuid::Uuid, String)> = match cp_result {
                Ok(v) => v,
                Err(e) => { tracing::error!("Помилка завантаження контрагентів: {e}"); return; }
            };
            let tasks = tasks_result.unwrap_or_default();
            let categories = cat_result.unwrap_or_default();

            let Some((act, items)) = act_opt else {
                tracing::warn!("Акт {uuid} не знайдено.");
                return;
            };

            let cp_names: Vec<SharedString> = counterparties.iter()
                .map(|(_, n)| SharedString::from(n.as_str())).collect();
            let cp_ids: Vec<SharedString> = counterparties.iter()
                .map(|(id, _)| SharedString::from(id.to_string().as_str())).collect();
            let cp_index = counterparties.iter()
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

            let form_items: Vec<FormItemRow> = items.iter().map(|it| FormItemRow {
                description: SharedString::from(it.description.as_str()),
                quantity: SharedString::from(format!("{}", it.quantity).as_str()),
                unit: SharedString::from(it.unit.as_str()),
                price: SharedString::from(format!("{}", it.unit_price).as_str()),
                amount: SharedString::from(format!("{:.2}", it.amount).as_str()),
            }).collect();
            let task_rows = to_task_rows(&tasks);

            let act_number = act.number.clone();
            let act_date = act.date.format("%d.%m.%Y").to_string();
            let act_notes = act.notes.clone().unwrap_or_default();
            let act_id_str = act.id.to_string();
            let total_str = format!("{:.2}", act.total_amount);
            let exp_date_str = act.expected_payment_date
                .map(|d| d.format("%d.%m.%Y").to_string())
                .unwrap_or_default();

            ui_handle.upgrade_in_event_loop(move |ui| {
                ui.set_show_act_card(false);
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
            }).ok();
        });
    });

    // ── Колбек: PDF з картки акту ─────────────────────────────────────────────
    let pool_act_card_pdf = pool.clone();
    let ui_weak_act_card_pdf = ui.as_weak();
    let active_company_id_act_card_pdf = active_company_id.clone();
    ui.on_act_card_pdf_clicked(move |id| {
        let pool = pool_act_card_pdf.clone();
        let ui_weak = ui_weak_act_card_pdf.clone();
        let id_str = id.to_string();
        let cid = *active_company_id_act_card_pdf.lock().unwrap();

        // Делегуємо в існуючий handler через on_act_pdf_clicked-логіку
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

            let pdf_items: Vec<pdf::generator::PdfActItem> = items.iter().enumerate()
                .map(|(i, item)| pdf::generator::PdfActItem {
                    num: (i + 1) as u32,
                    name: item.description.clone(),
                    qty: item.quantity.to_string(),
                    unit: item.unit.clone(),
                    price: item.unit_price.to_string(),
                    amount: item.amount.to_string(),
                }).collect();

            let data = pdf::generator::PdfActData {
                number: act.number.clone(),
                date: act.date.format("%d.%m.%Y").to_string(),
                company: pdf::generator::PdfCompany {
                    name: company.name.clone(),
                    edrpou: company.edrpou.unwrap_or_default(),
                    iban: company.iban.unwrap_or_default(),
                    address: company.legal_address.unwrap_or_default(),
                },
                client: pdf::generator::PdfCompany {
                    name: cp.name.clone(),
                    edrpou: cp.edrpou.unwrap_or_default(),
                    iban: cp.iban.unwrap_or_default(),
                    address: cp.address.unwrap_or_default(),
                },
                items: pdf_items,
                total: format!("{:.2}", act.total_amount),
                total_words: pdf::generator::amount_to_words(&act.total_amount),
                notes: act.notes.unwrap_or_default(),
            };

            let output_path = match pdf::generator::ensure_output_dir(&act.number) {
                Ok(p) => p,
                Err(e) => { tracing::error!("PDF (картка): директорія: {e}"); return; }
            };

            if let Err(e) = pdf::generator::generate_act_pdf(&data, &output_path) {
                tracing::error!("PDF (картка): генерація: {e}"); return;
            }
            tracing::info!("PDF '{}' → {}", act.number, output_path.display());
            if let Err(e) = std::process::Command::new("cmd")
                .args(["/C", "start", "", &output_path.to_string_lossy()])
                .spawn()
            {
                tracing::error!("PDF (картка): відкриття: {e}");
            }

            // Перечитуємо картку щоб оновити статус (якщо він змінився)
            if let Err(e) = open_act_card(&pool, ui_weak, uuid).await {
                tracing::error!("Оновлення картки після PDF: {e}");
            }
        });
    });

    // ── Колбек: наступний статус з картки акту ───────────────────────────────
    let pool_act_card_adv = pool.clone();
    let ui_weak_act_card_adv = ui.as_weak();
    let active_company_id_act_card_adv = active_company_id.clone();
    let act_state_for_card = act_state.clone();
    ui.on_act_card_advance_status_clicked(move |id, new_status| {
        let pool = pool_act_card_adv.clone();
        let ui_weak = ui_weak_act_card_adv.clone();
        let id_str = id.to_string();
        let cid = *active_company_id_act_card_adv.lock().unwrap();
        let act_state_clone = act_state_for_card.clone();
        let new_status = act_status_from_ui(new_status);

        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else { return; };
            if let Err(e) = db::acts::change_status(&pool, uuid, new_status).await {
                tracing::error!("Advance status (картка): {e}");
                return;
            }
            // Оновити картку і список паралельно
            let state = act_state_clone.lock().unwrap().clone();
            let _ = tokio::join!(
                open_act_card(&pool, ui_weak.clone(), uuid),
                reload_acts(&pool, ui_weak.clone(), cid, state.status_filter, state.query, false),
            );
        });
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
            // tokio::join! — запускає три futures паралельно
            let (cp_result, num_result, cat_result) = tokio::join!(
                db::acts::counterparties_for_select(&pool, cid),
                db::acts::generate_next_number(&pool, cid),
                db::categories::list_all_for_select(&pool, cid),
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
            let categories = cat_result.unwrap_or_default();

            // Розбиваємо Vec<(Uuid, String)> на два паралельних Vec<SharedString>
            let cp_names: Vec<SharedString> = counterparties
                .iter()
                .map(|(_, name)| SharedString::from(name.as_str()))
                .collect();
            let cp_ids: Vec<SharedString> = counterparties
                .iter()
                .map(|(id, _)| SharedString::from(id.to_string().as_str()))
                .collect();

            // Категорії: перший елемент = "— без категорії —" (порожній id)
            let mut cat_names: Vec<SharedString> =
                vec![SharedString::from("— без категорії —")];
            let mut cat_ids: Vec<SharedString> = vec![SharedString::from("")];
            for c in &categories {
                let prefix = if c.depth > 0 { "  └─ " } else { "" };
                cat_names.push(SharedString::from(format!("{}{}", prefix, c.name)));
                cat_ids.push(SharedString::from(c.id.to_string()));
            }

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
                    ui.set_act_form_category_names(ModelRc::new(VecModel::from(cat_names)));
                    ui.set_act_form_category_ids(ModelRc::new(VecModel::from(cat_ids)));
                    ui.set_act_form_category_index(0);
                    ui.set_act_form_expected_payment_date(SharedString::from(""));
                    // Очищаємо позиції та задачі з попереднього відкриття форми
                    ui.set_act_form_items(ModelRc::new(VecModel::from(Vec::<FormItemRow>::new())));
                    ui.set_act_task_rows(ModelRc::new(VecModel::from(Vec::<TaskRow>::new())));
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

    // ── Колбек: генерувати PDF акту ──────────────────────────────────────────
    //
    // 1. Завантажуємо акт + компанію паралельно (tokio::join!).
    // 2. Потім — контрагента (потрібен act.counterparty_id).
    // 3. Генеруємо PDF через Typst і відкриваємо у системному переглядачі.
    let pool_pdf = pool.clone();
    let active_company_id_pdf = active_company_id.clone();

    ui.on_act_pdf_clicked(move |act_id| {
        let pool = pool_pdf.clone();
        let id_str = act_id.to_string();
        let cid = *active_company_id_pdf.lock().unwrap();

        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("PDF: некоректний UUID акту: {id_str}");
                return;
            };

            // Акт і компанія — незалежні, беремо паралельно
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

            // Контрагент — потребує act.counterparty_id, тому після join!
            let cp = match db::counterparties::get_by_id(&pool, act.counterparty_id).await {
                Ok(Some(v)) => v,
                Ok(None) => { tracing::warn!("PDF: контрагент не знайдено."); return; }
                Err(e) => { tracing::error!("PDF: помилка контрагента: {e}"); return; }
            };

            let pdf_items: Vec<pdf::generator::PdfActItem> = items
                .iter()
                .enumerate()
                .map(|(i, item)| pdf::generator::PdfActItem {
                    num: (i + 1) as u32,
                    name: item.description.clone(),
                    qty: item.quantity.to_string(),
                    unit: item.unit.clone(),
                    price: item.unit_price.to_string(),
                    amount: item.amount.to_string(),
                })
                .collect();

            let data = pdf::generator::PdfActData {
                number: act.number.clone(),
                date: act.date.format("%d.%m.%Y").to_string(),
                company: pdf::generator::PdfCompany {
                    name: company.name.clone(),
                    edrpou: company.edrpou.unwrap_or_default(),
                    iban: company.iban.unwrap_or_default(),
                    address: company.legal_address.unwrap_or_default(),
                },
                client: pdf::generator::PdfCompany {
                    name: cp.name.clone(),
                    edrpou: cp.edrpou.unwrap_or_default(),
                    iban: cp.iban.unwrap_or_default(),
                    address: cp.address.unwrap_or_default(),
                },
                items: pdf_items,
                total: format!("{:.2}", act.total_amount),
                total_words: pdf::generator::amount_to_words(&act.total_amount),
                notes: act.notes.unwrap_or_default(),
            };

            let output_path = match pdf::generator::ensure_output_dir(&act.number) {
                Ok(p) => p,
                Err(e) => { tracing::error!("PDF: помилка директорії: {e}"); return; }
            };

            if let Err(e) = pdf::generator::generate_act_pdf(&data, &output_path) {
                tracing::error!("PDF: помилка генерації: {e}");
                return;
            }

            tracing::info!("PDF '{}' → {}", act.number, output_path.display());

            // Відкриваємо у системному переглядачі PDF (Windows)
            if let Err(e) = std::process::Command::new("cmd")
                .args(["/C", "start", "", &output_path.to_string_lossy()])
                .spawn()
            {
                tracing::error!("PDF: не вдалось відкрити файл: {e}");
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

            // tokio::join! — незалежні запити паралельно (урок 2026-04-01)
            let (act_result, cp_result, tasks_result, cat_result) = tokio::join!(
                db::acts::get_for_edit(&pool, uuid),
                db::acts::counterparties_for_select(&pool, cid),
                db::tasks::list_by_act(&pool, uuid),
                db::categories::list_all_for_select(&pool, cid),
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
            let categories = cat_result.unwrap_or_default();

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

            let form_items: Vec<FormItemRow> = items.iter().map(|it| FormItemRow {
                description: SharedString::from(it.description.as_str()),
                quantity: SharedString::from(format!("{}", it.quantity).as_str()),
                unit: SharedString::from(it.unit.as_str()),
                price: SharedString::from(format!("{}", it.unit_price).as_str()),
                amount: SharedString::from(format!("{:.2}", it.amount).as_str()),
            }).collect();
            let task_rows = to_task_rows(&tasks);

            let act_number = act.number.clone();
            // Дата у форматі ДД.ММ.РРРР (урок 2026-04-01)
            let act_date = act.date.format("%d.%m.%Y").to_string();
            let act_notes = act.notes.clone().unwrap_or_default();
            let act_id_str = act.id.to_string();
            let total_str = format!("{:.2}", act.total_amount);

            let mut cat_names: Vec<SharedString> = vec![SharedString::from("— без категорії —")];
            let mut cat_ids: Vec<SharedString> = vec![SharedString::from("")];
            for c in &categories {
                let prefix = if c.depth > 0 { "  └─ " } else { "" };
                cat_names.push(SharedString::from(format!("{}{}", prefix, c.name)));
                cat_ids.push(SharedString::from(c.id.to_string()));
            }
            let cat_id_str = act.category_id.map(|id| id.to_string()).unwrap_or_default();
            let cat_index = cat_ids.iter().position(|id| id.as_str() == cat_id_str).unwrap_or(0);
            let exp_date_str = act.expected_payment_date
                .map(|d| d.format("%d.%m.%Y").to_string())
                .unwrap_or_default();

            ui_handle
                .upgrade_in_event_loop(move |ui| {
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
                    ui.set_act_form_category_index(cat_index as i32);
                    ui.set_act_form_expected_payment_date(SharedString::from(exp_date_str.as_str()));
                    ui.set_act_form_items(ModelRc::new(VecModel::from(form_items)));
                    ui.set_act_task_rows(ModelRc::new(VecModel::from(task_rows)));
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
    let doc_state_update = doc_state.clone();
    let active_company_id_update = active_company_id.clone();

    ui.on_act_form_update(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str, exp_date_str| {
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
        let cat_id_str = cat_id_str.to_string();
        let con_id_str = con_id_str.to_string();
        let exp_date_str = exp_date_str.to_string();
        let act_state = act_state_update.clone();
        let doc_state_spawn = doc_state_update.clone();

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
                        tracing::error!("Помилка оновлення списку актів після редагування: {e}");
                    }
                    let (doc_tab, doc_direction, doc_query, doc_cp, doc_df, doc_dt) = {
                        let s = doc_state_spawn.lock().unwrap();
                        (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_from, s.date_to)
                    };
                    if let Err(e) = reload_documents(&pool, ui_weak.clone(), cid, doc_tab, &doc_direction, &doc_query, doc_cp, doc_df, doc_dt).await {
                        tracing::error!("Помилка оновлення документів після редагування акту: {e}");
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
                        reload_counterparties(&pool, ui_weak.clone(), cid, query, include_archived, page, true).await
                    {
                        tracing::error!(
                            "Помилка оновлення списку контрагентів після створення: {e}"
                        );
                    }
                    if let Err(e) = reload_payment_counterparty_options(&pool, ui_weak.clone(), cid).await {
                        tracing::error!("Помилка оновлення контрагентів для форми платежу після створення: {e}");
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
                        reload_counterparties(&pool, ui_weak.clone(), cid, query, include_archived, page, true).await
                    {
                        tracing::error!(
                            "Помилка оновлення списку контрагентів після редагування: {e}"
                        );
                    }
                    if let Err(e) = reload_payment_counterparty_options(&pool, ui_weak.clone(), cid).await {
                        tracing::error!("Помилка оновлення контрагентів для форми платежу після редагування: {e}");
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
                        reload_counterparties(&pool, ui_handle.clone(), cid, query, include_archived, page, false)
                            .await
                    {
                        tracing::error!(
                            "Помилка оновлення списку контрагентів після архівування: {e}"
                        );
                    }
                    if let Err(e) = reload_payment_counterparty_options(&pool, ui_handle.clone(), cid).await {
                        tracing::error!("Помилка оновлення контрагентів для форми платежу після архівування: {e}");
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

    // ── Колбек: видалити позицію з форми ────────────────────────────────────
    let ui_weak_remove = ui.as_weak();
    ui.on_act_form_remove_item(move |idx| {
        let Some(ui) = ui_weak_remove.upgrade() else {
            return;
        };
        let i = idx as usize;
        let mut items: Vec<FormItemRow> = (0..ui.get_act_form_items().row_count())
            .filter_map(|j| ui.get_act_form_items().row_data(j))
            .collect();
        if i < items.len() {
            items.remove(i);
        }
        ui.set_act_form_items(ModelRc::new(VecModel::from(items)));
    });

    // ── Колбек: редагування поля позиції акту ───────────────────────────────
    //
    // При зміні qty або price — перераховуємо amounts та total.
    let ui_weak_item = ui.as_weak();

    ui.on_act_form_item_changed(move |idx, field, value| {
        let Some(ui) = ui_weak_item.upgrade() else {
            return;
        };
        let i = idx as usize;
        let val = value.to_string();

        let mut items: Vec<FormItemRow> = (0..ui.get_act_form_items().row_count())
            .filter_map(|j| ui.get_act_form_items().row_data(j))
            .collect();

        if let Some(item) = items.get_mut(i) {
            match field.as_str() {
                "desc"  => item.description = SharedString::from(val.as_str()),
                "qty"   => item.quantity    = SharedString::from(val.as_str()),
                "unit"  => item.unit        = SharedString::from(val.as_str()),
                "price" => item.price       = SharedString::from(val.as_str()),
                _ => return,
            }
        } else {
            return;
        }

        // Перераховуємо суми рядків та total лише при зміні qty або price
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

    // ── Колбек: зберегти акт ("Зберегти") ───────────────────────────────────
    //
    // Читаємо поля форми + позиції синхронно (ми в main thread),
    // потім передаємо в tokio::spawn для async DB операції.
    let pool_save = pool.clone();
    let ui_weak_save = ui.as_weak();
    let act_state_save = act_state.clone();
    let doc_state_save = doc_state.clone();
    let active_company_id_save = active_company_id.clone();

    ui.on_act_form_save(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str, exp_date_str| {
        let Some(ui_ref) = ui_weak_save.upgrade() else {
            return;
        };
        let items = collect_form_items(&ui_ref);
        let cid = *active_company_id_save.lock().unwrap();
        let cat_id_str = cat_id_str.to_string();
        let con_id_str = con_id_str.to_string();
        let exp_date_str = exp_date_str.to_string();

        spawn_save_act(
            pool_save.clone(),
            ui_weak_save.clone(),
            act_state_save.clone(),
            doc_state_save.clone(),
            cid,
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.trim().is_empty() {
                None
            } else {
                Some(notes.to_string())
            },
            cat_id_str,
            con_id_str,
            exp_date_str,
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
    let doc_state_draft = doc_state.clone();
    let active_company_id_draft = active_company_id.clone();

    ui.on_act_form_save_draft(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str, exp_date_str| {
        let Some(ui_ref) = ui_weak_draft.upgrade() else {
            return;
        };
        let items = collect_form_items(&ui_ref);
        let cid = *active_company_id_draft.lock().unwrap();
        let cat_id_str = cat_id_str.to_string();
        let con_id_str = con_id_str.to_string();
        let exp_date_str = exp_date_str.to_string();

        spawn_save_act(
            pool_draft.clone(),
            ui_weak_draft.clone(),
            act_state_draft.clone(),
            doc_state_draft.clone(),
            cid,
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.trim().is_empty() {
                None
            } else {
                Some(notes.to_string())
            },
            cat_id_str,
            con_id_str,
            exp_date_str,
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
    let active_company_id_tasks_save = active_company_id.clone();

    ui.on_task_form_save(
        move |title, description, priority_idx, due_str, reminder_str| {
            let company_id = *active_company_id_tasks_save.lock().unwrap();
            let act_id = ui_weak_tasks_save
                .upgrade()
                .map(|ui| ui.get_task_form_act_id().to_string())
                .unwrap_or_default();
            spawn_save_task(
                pool_tasks_save.clone(),
                ui_weak_tasks_save.clone(),
                task_state_save.clone(),
                company_id,
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
    let active_company_id_tasks_update = active_company_id.clone();

    ui.on_task_form_update(
        move |title, description, priority_idx, due_str, reminder_str| {
            let company_id = *active_company_id_tasks_update.lock().unwrap();
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
                company_id,
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
            let (cps, next_number, categories) = tokio::join!(
                db::invoices::counterparties_for_select(&pool, cid),
                db::invoices::generate_next_number(&pool, cid),
                db::categories::list_all_for_select(&pool, cid),
            );
            let cps = cps.unwrap_or_default();
            let next_number = next_number.unwrap_or_else(|_| "НАК-001".into());
            let categories = categories.unwrap_or_default();
            let today = chrono::Local::now().format("%d.%m.%Y").to_string();

            let mut cat_names: Vec<SharedString> = vec![SharedString::from("— без категорії —")];
            let mut cat_ids: Vec<SharedString> = vec![SharedString::from("")];
            for c in &categories {
                let prefix = if c.depth > 0 { "  └─ " } else { "" };
                cat_names.push(SharedString::from(format!("{}{}", prefix, c.name)));
                cat_ids.push(SharedString::from(c.id.to_string()));
            }

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
                ui.set_invoice_form_category_names(ModelRc::new(VecModel::from(cat_names)));
                ui.set_invoice_form_category_ids(ModelRc::new(VecModel::from(cat_ids)));
                ui.set_invoice_form_category_index(0);
                ui.set_invoice_form_expected_payment_date(SharedString::from(""));
                ui.set_invoice_form_items(ModelRc::new(VecModel::from(Vec::<FormItemRow>::new())));
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
            let cp_index = cps.iter().position(|(id, _)| id.to_string() == cp_id_str).unwrap_or(0);

            let mut cat_names: Vec<SharedString> = vec![SharedString::from("— без категорії —")];
            let mut cat_ids: Vec<SharedString> = vec![SharedString::from("")];
            for c in &categories {
                let prefix = if c.depth > 0 { "  └─ " } else { "" };
                cat_names.push(SharedString::from(format!("{}{}", prefix, c.name)));
                cat_ids.push(SharedString::from(c.id.to_string()));
            }
            let cat_id_str = invoice.category_id.map(|id| id.to_string()).unwrap_or_default();
            let cat_index = cat_ids.iter().position(|id| id.as_str() == cat_id_str).unwrap_or(0);
            let exp_date_str = invoice.expected_payment_date
                .map(|d| d.format("%d.%m.%Y").to_string())
                .unwrap_or_default();

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
                ui.set_invoice_form_category_names(ModelRc::new(VecModel::from(cat_names)));
                ui.set_invoice_form_category_ids(ModelRc::new(VecModel::from(cat_ids)));
                ui.set_invoice_form_category_index(cat_index as i32);
                ui.set_invoice_form_expected_payment_date(SharedString::from(exp_date_str.as_str()));
                let form_items: Vec<FormItemRow> = items.iter().map(|it| FormItemRow {
                    description: SharedString::from(it.description.as_str()),
                    quantity: SharedString::from(it.quantity.to_string().as_str()),
                    unit: SharedString::from(it.unit.as_deref().unwrap_or("")),
                    price: SharedString::from(it.price.to_string().as_str()),
                    amount: SharedString::from(it.amount.to_string().as_str()),
                }).collect();
                ui.set_invoice_form_items(ModelRc::new(VecModel::from(form_items)));
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
            use slint::Model;
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

    let ui_weak_inv_remove = ui.as_weak();
    ui.on_invoice_form_remove_item(move |idx| {
        if let Some(ui) = ui_weak_inv_remove.upgrade() {
            use slint::Model;
            let mut items: Vec<FormItemRow> = (0..ui.get_invoice_form_items().row_count())
                .filter_map(|i| ui.get_invoice_form_items().row_data(i))
                .collect();
            let idx = idx as usize;
            if idx < items.len() { items.remove(idx); }
            ui.set_invoice_form_items(ModelRc::new(VecModel::from(items)));
            recalculate_invoice_total(&ui);
        }
    });

    let ui_weak_inv_item = ui.as_weak();
    ui.on_invoice_form_item_changed(move |idx, field, value| {
        if let Some(ui) = ui_weak_inv_item.upgrade() {
            use slint::Model;
            let mut items: Vec<FormItemRow> = (0..ui.get_invoice_form_items().row_count())
                .filter_map(|i| ui.get_invoice_form_items().row_data(i))
                .collect();
            let idx = idx as usize;
            if idx < items.len() {
                match field.as_str() {
                    "desc"  => { items[idx].description = value; }
                    "qty"   => { items[idx].quantity = value; }
                    "unit"  => { items[idx].unit = value; }
                    "price" => { items[idx].price = value; }
                    _ => {}
                }
                ui.set_invoice_form_items(ModelRc::new(VecModel::from(items)));
                if matches!(field.as_str(), "qty" | "price") {
                    recalculate_invoice_total(&ui);
                }
            }
        }
    });

    let pool_inv_save = pool.clone();
    let ui_weak_inv_save = ui.as_weak();
    let invoice_state_save = invoice_state.clone();
    let doc_state_inv_save = doc_state.clone();
    let active_company_id_inv_save = active_company_id.clone();
    ui.on_invoice_form_save(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str, exp_date_str| {
        let cid = *active_company_id_inv_save.lock().unwrap();
        let items = collect_invoice_items_from_ui(&ui_weak_inv_save);
        let cat_id_str = cat_id_str.to_string();
        let con_id_str = con_id_str.to_string();
        let exp_date_str = exp_date_str.to_string();
        spawn_save_invoice(
            pool_inv_save.clone(),
            ui_weak_inv_save.clone(),
            invoice_state_save.clone(),
            doc_state_inv_save.clone(),
            cid,
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.is_empty() { None } else { Some(notes.to_string()) },
            cat_id_str,
            con_id_str,
            exp_date_str,
            items,
        );
    });

    let pool_inv_upd = pool.clone();
    let ui_weak_inv_upd = ui.as_weak();
    let invoice_state_upd = invoice_state.clone();
    let doc_state_inv_upd = doc_state.clone();
    let active_company_id_inv_upd = active_company_id.clone();
    ui.on_invoice_form_update(move |number, date_str, cp_id_str, notes, cat_id_str, con_id_str, exp_date_str| {
        let cid = *active_company_id_inv_upd.lock().unwrap();
        let edit_id = ui_weak_inv_upd
            .upgrade()
            .map(|ui| ui.get_invoice_form_edit_id().to_string())
            .unwrap_or_default();
        let items = collect_invoice_items_from_ui(&ui_weak_inv_upd);
        let pool = pool_inv_upd.clone();
        let ui_weak = ui_weak_inv_upd.clone();
        let inv_state = invoice_state_upd.clone();
        let doc_state_u = doc_state_inv_upd.clone();
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
                    show_toast(ui_weak.clone(), format!("Накладну '{}' оновлено", inv.number), false);
                    let (status_filter, query) = {
                        let state = inv_state.lock().unwrap();
                        (state.status_filter.clone(), state.query.clone())
                    };
                    if let Err(e) = reload_invoices(&pool, ui_weak.clone(), cid, status_filter, query, true).await {
                        tracing::error!("Помилка оновлення списку накладних: {e}");
                    }
                    let (doc_tab, doc_direction, doc_query, doc_cp, doc_df, doc_dt) = {
                        let s = doc_state_u.lock().unwrap();
                        (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_from, s.date_to)
                    };
                    if let Err(e) = reload_documents(&pool, ui_weak.clone(), cid, doc_tab, &doc_direction, &doc_query, doc_cp, doc_df, doc_dt).await {
                        tracing::error!("Помилка оновлення документів після редагування накладної: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("Помилка оновлення накладної: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });

    // ── Платежі ──────────────────────────────────────────────────────────────

    let pool_pay_filter = pool.clone();
    let ui_weak_pay_filter = ui.as_weak();
    let active_company_id_pay_filter = active_company_id.clone();
    let payment_state_filter = payment_state.clone();
    ui.on_payment_direction_filter_changed(move |index| {
        use crate::models::payment::PaymentDirection;
        let pool = pool_pay_filter.clone();
        let ui_weak = ui_weak_pay_filter.clone();
        let cid = *active_company_id_pay_filter.lock().unwrap();
        let direction: Option<PaymentDirection> = match index {
            1 => Some(PaymentDirection::Income),
            2 => Some(PaymentDirection::Expense),
            _ => None,
        };
        let query = {
            let mut state = payment_state_filter.lock().unwrap();
            state.direction_filter = direction.clone();
            state.query.clone()
        };
        tokio::spawn(async move {
            if let Err(e) = reload_payments(&pool, ui_weak, cid, direction, &query).await {
                tracing::error!("Помилка фільтрації платежів: {e}");
            }
        });
    });

    let pool_pay_search = pool.clone();
    let ui_weak_pay_search = ui.as_weak();
    let active_company_id_pay_search = active_company_id.clone();
    let payment_state_search = payment_state.clone();
    ui.on_payment_search_changed(move |query| {
        let pool = pool_pay_search.clone();
        let ui_weak = ui_weak_pay_search.clone();
        let cid = *active_company_id_pay_search.lock().unwrap();
        let (query, direction) = {
            let mut state = payment_state_search.lock().unwrap();
            state.query = query.to_string();
            (state.query.clone(), state.direction_filter.clone())
        };
        tokio::spawn(async move {
            if let Err(e) = reload_payments(&pool, ui_weak, cid, direction, &query).await {
                tracing::error!("Помилка пошуку платежів: {e}");
            }
        });
    });

    let pool_pay_reconcile = pool.clone();
    let ui_weak_pay_reconcile = ui.as_weak();
    let active_company_id_pay_reconcile = active_company_id.clone();
    let payment_state_reconcile = payment_state.clone();
    ui.on_payment_reconcile_clicked(move |id_str| {
        let pool = pool_pay_reconcile.clone();
        let ui_weak = ui_weak_pay_reconcile.clone();
        let cid = *active_company_id_pay_reconcile.lock().unwrap();
        let (query, direction) = {
            let state = payment_state_reconcile.lock().unwrap();
            (state.query.clone(), state.direction_filter.clone())
        };
        let id_s = id_str.to_string();
        tokio::spawn(async move {
            let Ok(uuid) = id_s.parse::<uuid::Uuid>() else {
                tracing::error!("Невалідний UUID платежу: {id_s}");
                return;
            };
            if let Err(e) = db::payments::mark_reconciled(&pool, uuid).await {
                tracing::error!("Помилка зведення платежу: {e}");
                return;
            }
            if let Err(e) = reload_payments(&pool, ui_weak, cid, direction, &query).await {
                tracing::error!("Помилка оновлення платежів: {e}");
            }
        });
    });

    let ui_weak_payment_create = ui.as_weak();
    ui.on_payment_create_clicked(move || {
        if let Some(ui) = ui_weak_payment_create.upgrade() {
            reset_payment_form(&ui);
            ui.set_show_payment_form(true);
        }
    });

    let pool_pay_edit = pool.clone();
    let ui_weak_pay_edit = ui.as_weak();
    let active_company_id_pay_edit = active_company_id.clone();
    ui.on_payment_edit_clicked(move |id| {
        let pool = pool_pay_edit.clone();
        let ui_weak = ui_weak_pay_edit.clone();
        let cid = *active_company_id_pay_edit.lock().unwrap();
        let id_str = id.to_string();

        tokio::spawn(async move {
            let Ok(payment_id) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID платежу: {id_str}");
                return;
            };

            let counterparties = match db::counterparties::list(&pool, cid).await {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!("Помилка завантаження контрагентів для форми платежу: {e}");
                    return;
                }
            };

            match db::payments::get_by_id(&pool, payment_id).await {
                Ok(Some(payment)) => {
                    ui_weak
                        .upgrade_in_event_loop(move |ui| {
                            populate_payment_form(&ui, &counterparties, &payment);
                            ui.set_show_payment_form(true);
                        })
                        .ok();
                }
                Ok(None) => tracing::warn!("Платіж {payment_id} не знайдено."),
                Err(e) => tracing::error!("Помилка завантаження платежу для редагування: {e}"),
            }
        });
    });

    let pool_pay_delete = pool.clone();
    let ui_weak_pay_delete = ui.as_weak();
    let active_company_id_pay_delete = active_company_id.clone();
    let payment_state_delete = payment_state.clone();
    ui.on_payment_delete_clicked(move |id| {
        let pool = pool_pay_delete.clone();
        let ui_weak = ui_weak_pay_delete.clone();
        let cid = *active_company_id_pay_delete.lock().unwrap();
        let (query, direction) = {
            let state = payment_state_delete.lock().unwrap();
            (state.query.clone(), state.direction_filter.clone())
        };
        let id_str = id.to_string();

        tokio::spawn(async move {
            let Ok(payment_id) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID платежу: {id_str}");
                return;
            };

            match db::payments::delete(&pool, payment_id).await {
                Ok(()) => {
                    show_toast(ui_weak.clone(), "Платіж видалено".into(), false);
                    if let Err(e) = reload_payments(&pool, ui_weak, cid, direction, &query).await {
                        tracing::error!("Помилка оновлення списку платежів після видалення: {e}");
                    }
                }
                Err(e) => tracing::error!("Помилка видалення платежу: {e}"),
            }
        });
    });

    let pool_pay_save = pool.clone();
    let ui_weak_pay_save = ui.as_weak();
    let active_company_id_pay_save = active_company_id.clone();
    let payment_state_save = payment_state.clone();
    ui.on_payment_form_save(move |date, amount, direction, counterparty_id, bank_name, bank_ref, description| {
        let pool = pool_pay_save.clone();
        let ui_weak = ui_weak_pay_save.clone();
        let cid = *active_company_id_pay_save.lock().unwrap();
        let (query, direction_filter) = {
            let state = payment_state_save.lock().unwrap();
            (state.query.clone(), state.direction_filter.clone())
        };
        let date = date.to_string();
        let amount = amount.to_string();
        let counterparty_id = counterparty_id.to_string();
        let bank_name = bank_name.to_string();
        let bank_ref = bank_ref.to_string();
        let description = description.to_string();

        tokio::spawn(async move {
            let date = match NaiveDate::parse_from_str(&date, "%d.%m.%Y") {
                Ok(value) => value,
                Err(e) => {
                    tracing::error!("Невірний формат дати платежу: {e}");
                    show_toast(ui_weak, "Невірний формат дати".into(), true);
                    return;
                }
            };
            let amount = match amount.parse::<Decimal>() {
                Ok(value) => value,
                Err(e) => {
                    tracing::error!("Невірний формат суми платежу: {e}");
                    show_toast(ui_weak, "Невірний формат суми".into(), true);
                    return;
                }
            };
            let data = models::payment::NewPayment {
                company_id: cid,
                date,
                amount,
                direction: if direction == 0 {
                    models::payment::PaymentDirection::Income
                } else {
                    models::payment::PaymentDirection::Expense
                },
                counterparty_id: parse_optional_uuid(&counterparty_id),
                bank_name: optional_text(&bank_name),
                bank_ref: optional_text(&bank_ref),
                description: optional_text(&description),
            };

            match db::payments::create(&pool, data).await {
                Ok(payment) => {
                    show_toast(
                        ui_weak.clone(),
                        format!("Платіж на {:.2} збережено", payment.amount),
                        false,
                    );
                    if let Err(e) =
                        reload_payments(&pool, ui_weak, cid, direction_filter, &query).await
                    {
                        tracing::error!("Помилка оновлення платежів після створення: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("Помилка створення платежу: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });

    let pool_pay_update = pool.clone();
    let ui_weak_pay_update = ui.as_weak();
    let active_company_id_pay_update = active_company_id.clone();
    let payment_state_update = payment_state.clone();
    ui.on_payment_form_update(move |date, amount, direction, counterparty_id, bank_name, bank_ref, description| {
        let pool = pool_pay_update.clone();
        let ui_weak = ui_weak_pay_update.clone();
        let cid = *active_company_id_pay_update.lock().unwrap();
        let edit_id = ui_weak_pay_update
            .upgrade()
            .map(|ui| ui.get_payment_form_edit_id().to_string())
            .unwrap_or_default();
        let (query, direction_filter) = {
            let state = payment_state_update.lock().unwrap();
            (state.query.clone(), state.direction_filter.clone())
        };
        let date = date.to_string();
        let amount = amount.to_string();
        let counterparty_id = counterparty_id.to_string();
        let bank_name = bank_name.to_string();
        let bank_ref = bank_ref.to_string();
        let description = description.to_string();

        tokio::spawn(async move {
            let payment_id = match edit_id.parse::<uuid::Uuid>() {
                Ok(value) => value,
                Err(e) => {
                    tracing::error!("Некоректний UUID платежу для оновлення: {e}");
                    show_toast(ui_weak, "Не вдалося визначити платіж".into(), true);
                    return;
                }
            };
            let date = match NaiveDate::parse_from_str(&date, "%d.%m.%Y") {
                Ok(value) => value,
                Err(e) => {
                    tracing::error!("Невірний формат дати платежу: {e}");
                    show_toast(ui_weak, "Невірний формат дати".into(), true);
                    return;
                }
            };
            let amount = match amount.parse::<Decimal>() {
                Ok(value) => value,
                Err(e) => {
                    tracing::error!("Невірний формат суми платежу: {e}");
                    show_toast(ui_weak, "Невірний формат суми".into(), true);
                    return;
                }
            };
            let data = models::payment::UpdatePayment {
                date,
                amount,
                direction: if direction == 0 {
                    models::payment::PaymentDirection::Income
                } else {
                    models::payment::PaymentDirection::Expense
                },
                counterparty_id: parse_optional_uuid(&counterparty_id),
                bank_name: optional_text(&bank_name),
                bank_ref: optional_text(&bank_ref),
                description: optional_text(&description),
            };

            match db::payments::update(&pool, payment_id, data).await {
                Ok(Some(payment)) => {
                    show_toast(
                        ui_weak.clone(),
                        format!("Платіж на {:.2} оновлено", payment.amount),
                        false,
                    );
                    if let Err(e) =
                        reload_payments(&pool, ui_weak, cid, direction_filter, &query).await
                    {
                        tracing::error!("Помилка оновлення платежів після редагування: {e}");
                    }
                }
                Ok(None) => {
                    tracing::warn!("Платіж {payment_id} не знайдено під час оновлення.");
                    show_toast(ui_weak, "Платіж не знайдено".into(), true);
                }
                Err(e) => {
                    tracing::error!("Помилка оновлення платежу: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });

    let ui_weak_pay_cancel = ui.as_weak();
    ui.on_payment_form_cancel(move || {
        if let Some(ui) = ui_weak_pay_cancel.upgrade() {
            ui.set_show_payment_form(false);
        }
    });

    // ── Документи — колбеки ───────────────────────────────────────────────────

    // Зміна таба (Всі / Акти / Накладні)
    let pool_doc_tab = pool.clone();
    let ui_weak_doc_tab = ui.as_weak();
    let active_company_id_doc_tab = active_company_id.clone();
    let doc_state_tab = doc_state.clone();
    ui.on_doc_tab_changed(move |tab| {
        let pool = pool_doc_tab.clone();
        let ui_weak = ui_weak_doc_tab.clone();
        let cid = *active_company_id_doc_tab.lock().unwrap();
        let (query, direction, cp_id, df, dt) = {
            let mut s = doc_state_tab.lock().unwrap();
            s.tab = tab;
            (s.query.clone(), s.direction.clone(), s.counterparty_id, s.date_from, s.date_to)
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_doc_active_tab(tab);
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) = reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt).await {
                tracing::error!("Помилка фільтру документів за табом: {e}");
            }
        });
    });

    // Зміна напрямку (Вихідні / Вхідні)
    let pool_doc_direction = pool.clone();
    let ui_weak_doc_direction = ui.as_weak();
    let active_company_id_doc_direction = active_company_id.clone();
    let doc_state_direction = doc_state.clone();
    ui.on_doc_direction_changed(move |index| {
        let pool = pool_doc_direction.clone();
        let ui_weak = ui_weak_doc_direction.clone();
        let cid = *active_company_id_doc_direction.lock().unwrap();
        let (tab, direction, query, cp_id, df, dt) = {
            let mut s = doc_state_direction.lock().unwrap();
            s.direction = doc_direction_from_index(index).to_string();
            (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_from, s.date_to)
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_doc_direction_index(index);
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) = reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt).await {
                tracing::error!("Помилка фільтру документів за напрямком: {e}");
            }
        });
    });

    // Текстовий пошук
    let pool_doc_search = pool.clone();
    let ui_weak_doc_search = ui.as_weak();
    let active_company_id_doc_search = active_company_id.clone();
    let doc_state_search = doc_state.clone();
    ui.on_doc_search_changed(move |q| {
        let pool = pool_doc_search.clone();
        let ui_weak = ui_weak_doc_search.clone();
        let cid = *active_company_id_doc_search.lock().unwrap();
        let (tab, direction, query, cp_id, df, dt) = {
            let mut s = doc_state_search.lock().unwrap();
            s.query = q.to_string();
            (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_from, s.date_to)
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) = reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt).await {
                tracing::error!("Помилка пошуку документів: {e}");
            }
        });
    });

    // Новий акт
    let ui_weak_doc_new_act = ui.as_weak();
    ui.on_doc_new_act_clicked(move || {
        if let Some(ui) = ui_weak_doc_new_act.upgrade() {
            ui.invoke_act_create_clicked();
        }
    });

    // Нова накладна
    let ui_weak_doc_new_inv = ui.as_weak();
    ui.on_doc_new_invoice_clicked(move || {
        if let Some(ui) = ui_weak_doc_new_inv.upgrade() {
            ui.invoke_invoice_create_clicked();
        }
    });

    // Фільтр за контрагентом
    let pool_doc_cp = pool.clone();
    let ui_weak_doc_cp = ui.as_weak();
    let active_company_id_doc_cp = active_company_id.clone();
    let doc_state_cp = doc_state.clone();
    let doc_cp_ids_cp = doc_cp_ids.clone();
    ui.on_doc_cp_filter_changed(move |idx| {
        let pool = pool_doc_cp.clone();
        let ui_weak = ui_weak_doc_cp.clone();
        let cid = *active_company_id_doc_cp.lock().unwrap();
        let cp_id = if idx <= 0 {
            None
        } else {
            let ids = doc_cp_ids_cp.lock().unwrap();
            ids.get(idx as usize - 1).copied()
        };
        let (tab, direction, query, df, dt) = {
            let mut s = doc_state_cp.lock().unwrap();
            s.counterparty_id = cp_id;
            s.counterparty_index = idx;
            (s.tab, s.direction.clone(), s.query.clone(), s.date_from, s.date_to)
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_doc_filter_cp_index(idx);
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) = reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt).await {
                tracing::error!("Помилка фільтру документів за контрагентом: {e}");
            }
        });
    });

    // Фільтр за датою від
    let pool_doc_df = pool.clone();
    let ui_weak_doc_df = ui.as_weak();
    let active_company_id_doc_df = active_company_id.clone();
    let doc_state_df = doc_state.clone();
    ui.on_doc_date_from_changed(move |text| {
        // Парсимо лише якщо введено повну дату (10 символів)
        let df = if text.len() == 10 {
            chrono::NaiveDate::parse_from_str(text.as_str(), "%d.%m.%Y").ok()
        } else if text.is_empty() {
            None
        } else {
            return; // неповна дата — не реагуємо
        };
        let pool = pool_doc_df.clone();
        let ui_weak = ui_weak_doc_df.clone();
        let cid = *active_company_id_doc_df.lock().unwrap();
        let (tab, direction, query, cp_id, dt) = {
            let mut s = doc_state_df.lock().unwrap();
            s.date_from = df;
            (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_to)
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) = reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt).await {
                tracing::error!("Помилка фільтру документів за датою від: {e}");
            }
        });
    });

    // Фільтр за датою до
    let pool_doc_dt = pool.clone();
    let ui_weak_doc_dt = ui.as_weak();
    let active_company_id_doc_dt = active_company_id.clone();
    let doc_state_dt = doc_state.clone();
    ui.on_doc_date_to_changed(move |text| {
        let dt = if text.len() == 10 {
            chrono::NaiveDate::parse_from_str(text.as_str(), "%d.%m.%Y").ok()
        } else if text.is_empty() {
            None
        } else {
            return;
        };
        let pool = pool_doc_dt.clone();
        let ui_weak = ui_weak_doc_dt.clone();
        let cid = *active_company_id_doc_dt.lock().unwrap();
        let (tab, direction, query, cp_id, df) = {
            let mut s = doc_state_dt.lock().unwrap();
            s.date_to = dt;
            (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_from)
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) = reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt).await {
                tracing::error!("Помилка фільтру документів за датою до: {e}");
            }
        });
    });

    // Відкрити документ для редагування
    let ui_weak_doc_open = ui.as_weak();
    ui.on_doc_open_clicked(move |id| {
        let id_s = id.to_string();
        if let Some(ui) = ui_weak_doc_open.upgrade() {
            if let Some(act_uuid) = id_s.strip_prefix("act:") {
                ui.invoke_act_edit_clicked(SharedString::from(act_uuid));
            } else if let Some(inv_uuid) = id_s.strip_prefix("inv:") {
                ui.invoke_invoice_edit_clicked(SharedString::from(inv_uuid));
            } else {
                tracing::warn!("doc-open-clicked: невідомий префікс id='{id_s}'");
            }
        }
    });

    // Генерація PDF документу
    let ui_weak_doc_pdf = ui.as_weak();
    ui.on_doc_pdf_clicked(move |id| {
        let id_s = id.to_string();
        if let Some(ui) = ui_weak_doc_pdf.upgrade() {
            if let Some(act_uuid) = id_s.strip_prefix("act:") {
                ui.invoke_act_pdf_clicked(SharedString::from(act_uuid));
            } else {
                tracing::info!("PDF для накладних ще не реалізовано (id='{id_s}')");
            }
        }
    });

    // Видалення документу
    let pool_doc_del = pool.clone();
    let ui_weak_doc_del = ui.as_weak();
    let active_company_id_doc_del = active_company_id.clone();
    let doc_state_del = doc_state.clone();
    ui.on_doc_delete_clicked(move |id| {
        let pool = pool_doc_del.clone();
        let ui_weak = ui_weak_doc_del.clone();
        let cid = *active_company_id_doc_del.lock().unwrap();
        let id_s = id.to_string();
        let (tab, direction, query, cp_id, df, dt) = {
            let s = doc_state_del.lock().unwrap();
            (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_from, s.date_to)
        };
        tokio::spawn(async move {
            let result = if let Some(act_uuid_s) = id_s.strip_prefix("act:") {
                let Ok(uuid) = act_uuid_s.parse::<uuid::Uuid>() else {
                    tracing::error!("Невалідний UUID акту: {act_uuid_s}");
                    return;
                };
                db::acts::delete(&pool, uuid).await
            } else if let Some(inv_uuid_s) = id_s.strip_prefix("inv:") {
                let Ok(uuid) = inv_uuid_s.parse::<uuid::Uuid>() else {
                    tracing::error!("Невалідний UUID накладної: {inv_uuid_s}");
                    return;
                };
                db::invoices::delete(&pool, uuid).await
            } else {
                tracing::warn!("doc-delete-clicked: невідомий префікс id='{id_s}'");
                return;
            };
            match result {
                Ok(_) => {
                    tracing::info!("Документ '{id_s}' видалено.");
                    if let Err(e) = reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt).await {
                        tracing::error!("Помилка оновлення документів після видалення: {e}");
                    }
                }
                Err(e) => tracing::error!("Помилка видалення документу '{id_s}': {e}"),
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
    let doc_state_company_select = doc_state.clone();
    let doc_cp_ids_company_select = doc_cp_ids.clone();

    ui.on_company_select_clicked(move |id_str| {
        let pool = pool_company_select.clone();
        let ui_handle = ui_weak_company_select.clone();
        let active_company_id = active_company_id_select.clone();
        let counterparty_state = counterparty_state_company_select.clone();
        let act_state = act_state_company_select.clone();
        let doc_state = doc_state_company_select.clone();
        let doc_cp_ids = doc_cp_ids_company_select.clone();
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
                    {
                        let mut state = doc_state.lock().unwrap();
                        *state = DocListState::default();
                    }

                    ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.set_active_company_name(SharedString::from(name.as_str()));
                        ui.set_active_company_id(SharedString::from(id_str.as_str()));
                        ui.set_active_company_subtitle(SharedString::from(subtitle.as_str()));
                        ui.set_show_company_picker(false);
                        ui.set_show_cp_form(false);
                        ui.set_show_act_form(false);
                        ui.set_show_task_form(false);
                        ui.set_show_payment_form(false);
                        ui.set_show_counterparty_card(false);
                        ui.set_doc_direction_index(0);
                        ui.set_doc_active_tab(0);
                        ui.set_doc_filter_cp_index(0);
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

                    if let Err(e) = reload_payments(&pool, ui_handle.clone(), company_id, None, "").await {
                        tracing::error!("Помилка завантаження платежів після вибору компанії: {e}");
                    }

                    if let Err(e) = reload_doc_cp_filter(&pool, ui_handle.clone(), company_id, &doc_cp_ids).await {
                        tracing::error!("Помилка оновлення фільтру контрагентів після вибору компанії: {e}");
                    }
                    if let Err(e) = reload_documents(&pool, ui_handle.clone(), company_id, 0, "outgoing", "", None, None, None).await {
                        tracing::error!("Помилка завантаження документів після вибору компанії: {e}");
                    }
                    if let Err(e) = reload_settings(&pool, ui_handle.clone(), company_id).await {
                        tracing::error!("Помилка завантаження налаштувань після вибору компанії: {e}");
                    }
                    if let Err(e) = reload_payment_counterparty_options(&pool, ui_handle.clone(), company_id).await {
                        tracing::error!("Помилка оновлення контрагентів для форми платежу після вибору компанії: {e}");
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

    let pool_settings_edit_company = pool.clone();
    let ui_weak_settings_edit_company = ui.as_weak();
    let active_company_id_settings_edit_company = active_company_id.clone();
    ui.on_settings_edit_company_clicked(move || {
        let pool = pool_settings_edit_company.clone();
        let ui_handle = ui_weak_settings_edit_company.clone();
        let company_id = *active_company_id_settings_edit_company.lock().unwrap();

        tokio::spawn(async move {
            match db::companies::get_by_id(&pool, company_id).await {
                Ok(Some(c)) => {
                    ui_handle
                        .upgrade_in_event_loop(move |ui| {
                            ui.set_company_form_is_edit(true);
                            ui.set_company_form_edit_id(SharedString::from(c.id.to_string().as_str()));
                            ui.set_company_form_name(SharedString::from(c.name.as_str()));
                            ui.set_company_form_edrpou(SharedString::from(c.edrpou.as_deref().unwrap_or("")));
                            ui.set_company_form_iban(SharedString::from(c.iban.as_deref().unwrap_or("")));
                            ui.set_company_form_legal_address(SharedString::from(c.legal_address.as_deref().unwrap_or("")));
                            ui.set_company_form_director(SharedString::from(c.director_name.as_deref().unwrap_or("")));
                            ui.set_company_form_accountant(SharedString::from(c.accountant_name.as_deref().unwrap_or("")));
                            ui.set_company_form_is_vat(c.is_vat_payer);
                            ui.set_current_page(6);
                            ui.set_show_company_form(true);
                        })
                        .ok();
                }
                Ok(None) => tracing::warn!("Активну компанію {company_id} не знайдено."),
                Err(e) => tracing::error!("Помилка відкриття компанії з налаштувань: {e}"),
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

                    // Заповнюємо стандартні категорії доходів/витрат
                    if let Err(e) = db::categories::seed_defaults(&pool, c.id).await {
                        tracing::warn!("Не вдалось заповнити категорії для нової компанії: {e}");
                    }

                    let mut cfg = AppConfig::load();
                    cfg.last_company_id = Some(c.id);
                    cfg.save();

                    let active_id = c.id;
                    if let Err(e) = reload_companies(&pool, ui_weak.clone(), active_id).await {
                        tracing::error!("Помилка оновлення списку компаній: {e}");
                    }
                    if let Err(e) = reload_settings(&pool, ui_weak.clone(), c.id).await {
                        tracing::error!("Помилка оновлення налаштувань компанії після створення: {e}");
                    }
                    if let Err(e) = reload_payment_counterparty_options(&pool, ui_weak.clone(), c.id).await {
                        tracing::error!("Помилка оновлення контрагентів для форми платежу після створення компанії: {e}");
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

    ui.on_company_form_update(move |id, name, edrpou, iban, address, director, accountant, is_vat| {
        let pool = pool_company_update.clone();
        let ui_weak = ui_weak_company_update.clone();
        let active_company_id = active_company_id_company_update.clone();
        let edit_id = id.to_string();

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
                        if let Err(e) = reload_settings(&pool, ui_weak.clone(), c.id).await {
                            tracing::error!("Помилка оновлення налаштувань після редагування компанії: {e}");
                        }
                        if let Err(e) = reload_payment_counterparty_options(&pool, ui_weak.clone(), c.id).await {
                            tracing::error!("Помилка оновлення контрагентів для форми платежу після редагування компанії: {e}");
                        }
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

    // ── Dashboard callbacks ───────────────────────────────────────────────────

    // Оновити дані Dashboard (викликається при переключенні на сторінку 🏠)
    let pool_dash = pool.clone();
    let ui_weak_dash = ui.as_weak();
    let active_company_dash = active_company_id.clone();
    ui.on_dashboard_refresh(move || {
        let pool = pool_dash.clone();
        let ui_weak = ui_weak_dash.clone();
        let cid = *active_company_dash.lock().unwrap();
        tokio::spawn(async move {
            if let Err(e) = reload_dashboard(&pool, ui_weak, cid).await {
                tracing::error!("Dashboard refresh помилка: {e:#}");
            }
        });
    });

    // «+ Новий акт» на Dashboard → перейти до форми акту
    let ui_weak_dash_act = ui.as_weak();
    ui.on_dashboard_new_act_clicked(move || {
        if let Some(ui) = ui_weak_dash_act.upgrade() {
            ui.set_current_page(1);
            // Тригеримо створення акту (аналогічно кнопці в ActList)
            ui.invoke_act_create_clicked();
        }
    });

    // «Всі акти →» на Dashboard → відкрити список актів
    let ui_weak_dash_all = ui.as_weak();
    ui.on_dashboard_all_acts_clicked(move || {
        if let Some(ui) = ui_weak_dash_all.upgrade() {
            ui.set_current_page(1);
        }
    });

    // «Відкрити →» To-Do на Dashboard → перейти на сторінку To-Do
    let ui_weak_dash_todo = ui.as_weak();
    ui.on_dashboard_add_todo_clicked(move || {
        if let Some(ui) = ui_weak_dash_todo.upgrade() {
            ui.set_current_page(5);
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

// Перетворити список актів у Vec<ActRow> для Slint.
// Vec<ActRow> є Send, тому можна безпечно передати в upgrade_in_event_loop.
fn to_act_rows(acts: &[models::ActListRow]) -> Vec<ActRow> {
    acts.iter()
        .map(|a| ActRow {
            id: SharedString::from(a.id.to_string().as_str()),
            num: SharedString::from(a.number.as_str()),
            date: SharedString::from(a.date.format("%d.%m.%Y").to_string().as_str()),
            counterparty: SharedString::from(a.counterparty_name.as_str()),
            amount: SharedString::from(format_amount_ua(a.total_amount).as_str()),
            status_label: SharedString::from(a.status.label()),
            status: match a.status {
                ModelActStatus::Draft => ActStatus::Draft,
                ModelActStatus::Issued => ActStatus::Issued,
                ModelActStatus::Signed => ActStatus::Signed,
                ModelActStatus::Paid => ActStatus::Paid,
            },
        })
        .collect()
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
    let items_model = ui.get_act_form_items();
    (0..items_model.row_count())
        .filter_map(|i| {
            let item = items_model.row_data(i)?;
            let quantity = item.quantity.parse::<Decimal>().ok()?;
            let unit_price = item.price.parse::<Decimal>().ok()?;
            Some(NewActItem {
                description: item.description.to_string(),
                quantity,
                unit: item.unit.to_string(),
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
    status_filter: Option<ModelActStatus>,
    query: String,
    close_form: bool,
) -> Result<()> {
    // Три незалежних запити паралельно (урок: tokio::join!)
    let (acts_result, counts_result, kpi_result) = tokio::join!(
        db::acts::list_filtered(pool, company_id, status_filter, None, normalized_query(&query), None, None, None),
        db::acts::count_by_status(pool, company_id),
        db::acts::get_kpi(pool, company_id)
    );
    let acts = acts_result?;
    let counts = counts_result?;
    let kpi = kpi_result?;
    let act_rows = to_act_rows(&acts);

    let kpi_revenue = format_kpi_amount(kpi.revenue_this_month);
    let kpi_unpaid  = format_kpi_amount(kpi.unpaid_total);

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_act_rows(ModelRc::new(VecModel::from(act_rows)));
            ui.set_act_status_counts(ModelRc::new(VecModel::from(counts)));
            ui.set_act_kpi_acts_month(kpi.acts_this_month as i32);
            ui.set_act_kpi_revenue(SharedString::from(kpi_revenue.as_str()));
            ui.set_act_kpi_unpaid(SharedString::from(kpi_unpaid.as_str()));
            ui.set_act_kpi_overdue(kpi.overdue_count as i32);
            if close_form {
                ui.set_show_act_form(false);
            }
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

/// Завантажити всі дані Dashboard і оновити UI.
///
/// Три незалежних запити паралельно: KPI, бар-чарт, статуси, платежі, останні акти.
async fn reload_dashboard(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
) -> Result<()> {
    let (kpi_res, bars_res, status_res, payments_res, recent_res) = tokio::join!(
        db::dashboard::get_kpi_summary(pool, company_id),
        db::dashboard::revenue_by_month(pool, company_id, 6),
        db::dashboard::acts_status_distribution(pool, company_id),
        db::dashboard::upcoming_payments(pool, company_id, 3),
        db::dashboard::get_recent_acts(pool, company_id, 5),
    );

    let kpi      = kpi_res?;
    let bars     = bars_res?;
    let statuses = status_res?;
    let payments = payments_res?;
    let recent   = recent_res?;

    // ── KPI форматування ────────────────────────────────────────────────────
    let revenue_str = format_kpi_amount(kpi.revenue_this_month);
    let unpaid_str  = format_kpi_amount(kpi.unpaid_total);
    let acts_str    = kpi.acts_this_month.to_string();
    let cp_str      = kpi.active_counterparties.to_string();

    // ── Бар-чарт ───────────────────────────────────────────────────────────
    // Знайти максимум для нормалізації висот
    let max_amount = bars.iter().map(|b| b.amount).max().unwrap_or(Decimal::ONE);
    let max_f = if max_amount.is_zero() { Decimal::ONE } else { max_amount };

    let bar_labels:   Vec<SharedString> = bars.iter().map(|b| SharedString::from(b.month_label())).collect();
    let bar_val_lbls: Vec<SharedString> = bars.iter().map(|b| {
        if b.amount.is_zero() { SharedString::from("") } else { SharedString::from(format_kpi_amount(b.amount)) }
    }).collect();
    let bar_fractions: Vec<f32> = bars.iter().map(|b| {
        (b.amount / max_f).to_f64().unwrap_or(0.0) as f32
    }).collect();

    // ── Статуси ─────────────────────────────────────────────────────────────
    let get_count = |st: &str| -> i32 {
        statuses.iter().find(|s| s.status == st).map(|s| s.count as i32).unwrap_or(0)
    };
    let paid_count   = get_count("paid");
    let issued_count = get_count("issued");
    let signed_count = get_count("signed");
    let draft_count  = get_count("draft");

    // ── Останні акти ────────────────────────────────────────────────────────
    let recent_rows: Vec<DashboardActRow> = recent.iter().map(|a| DashboardActRow {
        num:        SharedString::from(a.num.as_str()),
        contractor: SharedString::from(a.contractor.as_str()),
        amount:     SharedString::from(format_amount_ua(a.amount).as_str()),
        status:     SharedString::from(a.status.as_str()),
        date:       SharedString::from(a.date.as_str()),
    }).collect();

    // ── Очікувані платежі ────────────────────────────────────────────────────
    let payment_rows: Vec<DashboardPaymentRow> = payments.iter().map(|p| DashboardPaymentRow {
        date_label:  SharedString::from(p.date_label.as_str()),
        contractor:  SharedString::from(p.contractor.as_str()),
        amount:      SharedString::from(format_amount_ua(p.amount).as_str()),
        is_overdue:  p.is_overdue,
    }).collect();

    // ── Підпис місяця ────────────────────────────────────────────────────────
    let now = Local::now();
    let month_ua = match now.month() {
        1 => "Січень", 2 => "Лютий", 3 => "Березень", 4 => "Квітень",
        5 => "Травень", 6 => "Червень", 7 => "Липень", 8 => "Серпень",
        9 => "Вересень", 10 => "Жовтень", 11 => "Листопад", 12 => "Грудень",
        _ => "",
    };
    let month_label = format!("{} {}", month_ua, now.year());

    // ── Передача в UI (upgrade_in_event_loop) ────────────────────────────────
    ui_weak.upgrade_in_event_loop(move |ui| {
        ui.set_dashboard_kpi_revenue(SharedString::from(revenue_str.as_str()));
        ui.set_dashboard_kpi_unpaid(SharedString::from(unpaid_str.as_str()));
        ui.set_dashboard_kpi_acts_month(SharedString::from(acts_str.as_str()));
        ui.set_dashboard_kpi_counterparties(SharedString::from(cp_str.as_str()));
        ui.set_dashboard_month_label(SharedString::from(month_label.as_str()));

        ui.set_dashboard_chart_bar_labels(ModelRc::new(VecModel::from(bar_labels)));
        ui.set_dashboard_chart_bar_value_labels(ModelRc::new(VecModel::from(bar_val_lbls)));
        ui.set_dashboard_chart_bar_fractions(ModelRc::new(VecModel::from(bar_fractions)));

        ui.set_dashboard_status_paid(paid_count);
        ui.set_dashboard_status_issued(issued_count);
        ui.set_dashboard_status_signed(signed_count);
        ui.set_dashboard_status_draft(draft_count);

        ui.set_dashboard_recent_acts(ModelRc::new(VecModel::from(recent_rows)));
        ui.set_dashboard_upcoming_payments(ModelRc::new(VecModel::from(payment_rows)));
    }).map_err(anyhow::Error::from)?;

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
    let invoices = db::invoices::list_filtered(pool, company_id, status_filter, None, normalized_query(&query), None, None, None).await?;
    let invoice_rows: Vec<InvoiceRow> = invoices
        .iter()
        .map(|inv| InvoiceRow {
            id: SharedString::from(inv.id.to_string().as_str()),
            num: SharedString::from(inv.number.as_str()),
            date: SharedString::from(inv.date.format("%d.%m.%Y").to_string().as_str()),
            counterparty: SharedString::from(inv.counterparty_name.as_str()),
            amount: SharedString::from(format_amount_ua(inv.total_amount).as_str()),
            status_label: SharedString::from(match inv.status {
                InvoiceStatus::Draft  => "Чернетка",
                InvoiceStatus::Issued => "Виставлено",
                InvoiceStatus::Signed => "Підписано",
                InvoiceStatus::Paid   => "Оплачено",
            }),
            status: SharedString::from(inv.status.as_str()),
        })
        .collect();

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_invoice_rows(ModelRc::new(VecModel::from(invoice_rows)));
            if close_form {
                ui.set_show_invoice_form(false);
            }
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

/// Завантажити список платежів та агрегати доходів/витрат.
async fn reload_payments(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    direction: Option<crate::models::payment::PaymentDirection>,
    query: &str,
) -> Result<()> {
    use rust_decimal::Decimal;
    let query_lower = query.trim().to_lowercase();
    let rows = db::payments::list(pool, company_id, direction)
        .await?
        .into_iter()
        .filter(|row| {
            if query_lower.is_empty() {
                return true;
            }
            let haystack = [
                row.date.as_str(),
                row.counterparty_name.as_deref().unwrap_or(""),
                row.description.as_deref().unwrap_or(""),
                row.bank_name.as_deref().unwrap_or(""),
            ]
            .join(" ")
            .to_lowercase();
            haystack.contains(query_lower.as_str())
        })
        .collect::<Vec<_>>();

    let mut total_income = Decimal::ZERO;
    let mut total_expense = Decimal::ZERO;
    for r in &rows {
        match r.direction {
            crate::models::payment::PaymentDirection::Income => total_income += r.amount,
            crate::models::payment::PaymentDirection::Expense => total_expense += r.amount,
        }
    }

    let payment_rows: Vec<PaymentRow> = rows.iter().map(|r| PaymentRow {
        id: SharedString::from(r.id.to_string().as_str()),
        date: SharedString::from(r.date.as_str()),
        counterparty: SharedString::from(r.counterparty_name.as_deref().unwrap_or("")),
        description: SharedString::from(r.description.as_deref().unwrap_or("")),
        bank: SharedString::from(r.bank_name.as_deref().unwrap_or("")),
        amount: SharedString::from(format!("{:.2}", r.amount).as_str()),
        direction: SharedString::from(r.direction.label()),
        reconciled: r.is_reconciled,
    }).collect();

    ui_weak.upgrade_in_event_loop(move |ui| {
        ui.set_payment_rows(ModelRc::new(VecModel::from(payment_rows)));
        ui.set_payment_total_income(SharedString::from(format!("{:.2}", total_income)));
        ui.set_payment_total_expense(SharedString::from(format!("{:.2}", total_expense)));
        ui.set_payments_loading(false);
        ui.set_show_payment_form(false);
    }).map_err(anyhow::Error::from)?;
    Ok(())
}

async fn reload_settings(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
) -> Result<()> {
    let company = db::companies::get_by_id(pool, company_id).await?;
    let categories = db::categories::list(pool, company_id).await?;

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            if let Some(company) = company {
                ui.set_settings_company_name(SharedString::from(company.name.as_str()));
                ui.set_settings_company_edrpou(SharedString::from(
                    company.edrpou.as_deref().unwrap_or(""),
                ));
                ui.set_settings_company_iban(SharedString::from(
                    company.iban.as_deref().unwrap_or(""),
                ));
                ui.set_settings_company_director(SharedString::from(
                    company.director_name.as_deref().unwrap_or(""),
                ));
                ui.set_settings_company_address(SharedString::from(
                    company.legal_address.as_deref().unwrap_or(""),
                ));
            }

            let rows = categories
                .iter()
                .map(|cat| SettingsCategoryRow {
                    name: SharedString::from(cat.name.as_str()),
                    kind: SharedString::from(cat.kind.as_str()),
                    depth: if cat.parent_id.is_some() { 1 } else { 0 },
                })
                .collect::<Vec<_>>();
            ui.set_settings_category_rows(ModelRc::new(VecModel::from(rows)));
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

async fn reload_payment_counterparty_options(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
) -> Result<()> {
    let counterparties = db::counterparties::list(pool, company_id).await?;
    let mut names = vec![SharedString::from("Без контрагента")];
    let mut ids = vec![SharedString::from("")];
    for cp in counterparties {
        names.push(SharedString::from(cp.name.as_str()));
        ids.push(SharedString::from(cp.id.to_string().as_str()));
    }
    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_payment_form_counterparty_names(ModelRc::new(VecModel::from(names)));
            ui.set_payment_form_counterparty_ids(ModelRc::new(VecModel::from(ids)));
            if !ui.get_payment_form_is_edit() {
                ui.set_payment_form_counterparty_index(0);
            }
        })
        .map_err(anyhow::Error::from)?;
    Ok(())
}

/// Завантажити дані акту та відкрити картку-overlay в UI.
///
/// Паралельно отримуємо акт+позиції та задачі (обидва залежать лише від act_id).
/// Після цього послідовно — контрагент (потребує act.counterparty_id).
async fn open_act_card(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    act_id: uuid::Uuid,
) -> Result<()> {
    // Акт+позиції та задачі — незалежні, беремо паралельно
    let (act_result, tasks_result) = tokio::join!(
        db::acts::get_by_id(pool, act_id),
        db::tasks::list_by_act(pool, act_id),
    );

    let (act, items) = act_result?
        .ok_or_else(|| anyhow::anyhow!("Акт {act_id} не знайдено"))?;
    let tasks = tasks_result?;

    // Контрагент — після отримання act.counterparty_id
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
            ui.set_act_card_date(SharedString::from(act.date.format("%d.%m.%Y").to_string()));
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

async fn open_counterparty_card(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    counterparty_id: uuid::Uuid,
) -> Result<()> {
    let counterparty = db::counterparties::get_by_id(pool, counterparty_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Контрагента не знайдено"))?;
    let acts = db::acts::list_filtered(pool, company_id, None, None, None, Some(counterparty_id), None, None).await?;
    let invoices =
        db::invoices::list_filtered(pool, company_id, None, None, None, Some(counterparty_id), None, None).await?;
    let payments = db::payments::list_by_counterparty(pool, company_id, counterparty_id).await?;
    let contracts = db::contracts::list_by_counterparty(pool, company_id, counterparty_id).await?;
    let tasks = db::tasks::list_by_counterparty(pool, counterparty_id).await?;

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_counterparty_card_id(SharedString::from(counterparty.id.to_string().as_str()));
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
            ui.set_counterparty_card_stat_acts(SharedString::from(acts.len().to_string().as_str()));
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
                        description: SharedString::from(row.description.as_deref().unwrap_or("")),
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
                tasks.iter()
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

/// Завантажити єдиний список документів (акти + накладні) і оновити UI.
///
/// tab: 0=Всі, 1=Акти, 2=Рахунки
async fn reload_documents(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    tab: i32,
    direction: &str,
    query: &str,
    counterparty_id: Option<uuid::Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
) -> Result<()> {
    let search = if query.trim().is_empty() { None } else { Some(query) };

    // Паралельне завантаження актів та накладних залежно від таба
    let (acts_res, invs_res) = tokio::join!(
        async {
            if tab != 2 {
                db::acts::list_filtered(
                    pool,
                    company_id,
                    None,
                    Some(direction),
                    search,
                    counterparty_id,
                    date_from,
                    date_to,
                )
                .await
            } else {
                Ok(vec![])
            }
        },
        async {
            if tab != 1 {
                db::invoices::list_filtered(
                    pool,
                    company_id,
                    None,
                    Some(direction),
                    search,
                    counterparty_id,
                    date_from,
                    date_to,
                )
                .await
            } else {
                Ok(vec![])
            }
        }
    );
    let acts = acts_res?;
    let invs = invs_res?;

    // Об'єднуємо в один вектор (date, DocRow) і сортуємо за датою DESC
    let mut combined: Vec<(chrono::NaiveDate, DocRow)> = Vec::with_capacity(acts.len() + invs.len());

    for a in &acts {
        combined.push((a.date, DocRow {
            id:           SharedString::from(format!("act:{}", a.id)),
            doc_type:     SharedString::from("АКТ"),
            number:       SharedString::from(a.number.as_str()),
            counterparty: SharedString::from(a.counterparty_name.as_str()),
            amount:       SharedString::from(format!("{} ₴", format_amount_ua(a.total_amount))),
            date:         SharedString::from(a.date.format("%d.%m.%Y").to_string()),
            status:       SharedString::from(match a.status {
                models::ActStatus::Draft  => "Чернетка",
                models::ActStatus::Issued => "Виставлений",
                models::ActStatus::Signed => "Підписаний",
                models::ActStatus::Paid   => "Оплачений",
            }),
        }));
    }

    for i in &invs {
        combined.push((i.date, DocRow {
            id:           SharedString::from(format!("inv:{}", i.id)),
            doc_type:     SharedString::from("НАК"),
            number:       SharedString::from(i.number.as_str()),
            counterparty: SharedString::from(i.counterparty_name.as_str()),
            amount:       SharedString::from(format!("{} ₴", format_amount_ua(i.total_amount))),
            date:         SharedString::from(i.date.format("%d.%m.%Y").to_string()),
            status:       SharedString::from(match i.status {
                models::InvoiceStatus::Draft  => "Чернетка",
                models::InvoiceStatus::Issued => "Виставлений",
                models::InvoiceStatus::Signed => "Підписаний",
                models::InvoiceStatus::Paid   => "Оплачений",
            }),
        }));
    }

    // Сортування за датою DESC
    combined.sort_by(|(da, _), (db, _)| db.cmp(da));
    let doc_rows: Vec<DocRow> = combined.into_iter().map(|(_, r)| r).collect();
    let direction_index = doc_direction_index(direction);

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_document_rows(ModelRc::new(VecModel::from(doc_rows)));
            ui.set_doc_active_tab(tab);
            ui.set_doc_direction_index(direction_index);
            ui.set_documents_loading(false);
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

/// Завантажити список контрагентів для фільтру в списку документів.
///
/// Оновлює `doc_cp_ids` (UUID-и без "Всі контрагенти") і встановлює
/// `doc-filter-cp-names` на UI (з "Всі контрагенти" на позиції 0).
async fn reload_doc_cp_filter(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    doc_cp_ids: &Mutex<Vec<uuid::Uuid>>,
) -> Result<()> {
    let cps = db::acts::counterparties_for_select(pool, company_id).await?;
    {
        let mut ids = doc_cp_ids.lock().unwrap();
        *ids = cps.iter().map(|(id, _)| *id).collect();
    }
    let mut names: Vec<slint::SharedString> = vec![slint::SharedString::from("Всі контрагенти")];
    names.extend(cps.iter().map(|(_, n)| slint::SharedString::from(n.as_str())));
    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_doc_filter_cp_names(ModelRc::new(VecModel::from(names)));
        })
        .map_err(anyhow::Error::from)?;
    Ok(())
}

/// Зібрати позиції накладної з UI-форми у Vec<NewInvoiceItem>.
fn collect_invoice_items_from_ui(ui_weak: &Weak<MainWindow>) -> Vec<NewInvoiceItem> {
    use slint::Model;
    let Some(ui) = ui_weak.upgrade() else { return vec![]; };
    let model = ui.get_invoice_form_items();
    let count = model.row_count();
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let row = model.row_data(i).unwrap_or_default();
        let quantity = row.quantity.to_string().parse::<Decimal>().unwrap_or(Decimal::ONE);
        let price = row.price.to_string().parse::<Decimal>().unwrap_or(Decimal::ZERO);
        let unit = row.unit.to_string();
        items.push(NewInvoiceItem {
            position: (i + 1) as i16,
            description: row.description.to_string(),
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
    let model = ui.get_invoice_form_items();
    let mut items: Vec<FormItemRow> = (0..model.row_count()).filter_map(|i| model.row_data(i)).collect();
    let mut total = Decimal::ZERO;
    for item in &mut items {
        let qty = item.quantity.to_string().parse::<Decimal>().unwrap_or(Decimal::ZERO);
        let price = item.price.to_string().parse::<Decimal>().unwrap_or(Decimal::ZERO);
        let amount = (qty * price).round_dp(2);
        item.amount = SharedString::from(format!("{:.2}", amount).as_str());
        total += amount;
    }
    ui.set_invoice_form_items(ModelRc::new(VecModel::from(items)));
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

fn act_status_from_ui(status: ActStatus) -> ModelActStatus {
    match status {
        ActStatus::Draft => ModelActStatus::Draft,
        ActStatus::Issued => ModelActStatus::Issued,
        ActStatus::Signed => ModelActStatus::Signed,
        ActStatus::Paid => ModelActStatus::Paid,
    }
}

fn format_company_total(amount: &Decimal) -> String {
    format!("{} грн", amount.round_dp(2))
}

fn doc_direction_from_index(index: i32) -> &'static str {
    if index == 1 { "incoming" } else { "outgoing" }
}

fn doc_direction_index(direction: &str) -> i32 {
    if direction == "incoming" { 1 } else { 0 }
}

/// Форматує суму в українському вигляді: "78\u{00A0}000,00 ₴".
/// Тисячі розділяються нерозривним пробілом, дробова частина через кому.
fn format_amount_ua(amount: Decimal) -> String {
    let s = format!("{:.2}", amount.abs());
    let (int_part, dec_part) = s.split_once('.').unwrap_or((&s, "00"));
    let len = int_part.len();
    let mut result = String::with_capacity(len + len / 3 + 8);
    for (i, ch) in int_part.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('\u{00A0}');
        }
        result.push(ch);
    }
    format!("{},{} ₴", result, dec_part)
}

/// Форматує грошову суму для KPI-картки: "78\u{00A0}000 ₴".
/// Тисячі розділяються нерозривним пробілом, копійки відкидаються.
fn format_kpi_amount(amount: Decimal) -> String {
    let s = amount.round().abs().to_string();
    let len = s.len();
    let mut result = String::with_capacity(len + len / 3 + 2);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('\u{00A0}');
        }
        result.push(ch);
    }
    format!("{} ₴", result)
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

fn reset_payment_form(ui: &MainWindow) {
    ui.set_payment_form_is_edit(false);
    ui.set_payment_form_edit_id(SharedString::from(""));
    ui.set_payment_form_date(SharedString::from(Local::now().format("%d.%m.%Y").to_string()));
    ui.set_payment_form_amount(SharedString::from(""));
    ui.set_payment_form_direction_index(0);
    ui.set_payment_form_counterparty_index(0);
    ui.set_payment_form_bank_name(SharedString::from(""));
    ui.set_payment_form_bank_ref(SharedString::from(""));
    ui.set_payment_form_description(SharedString::from(""));
}

fn optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_optional_uuid(value: &str) -> Option<uuid::Uuid> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        uuid::Uuid::parse_str(trimmed).ok()
    }
}

fn populate_payment_form(
    ui: &MainWindow,
    counterparties: &[models::counterparty::Counterparty],
    payment: &models::payment::Payment,
) {
    let mut names = vec![SharedString::from("Без контрагента")];
    let mut ids = vec![SharedString::from("")];
    let mut selected_index = 0_i32;

    for (index, cp) in counterparties.iter().enumerate() {
        names.push(SharedString::from(cp.name.as_str()));
        ids.push(SharedString::from(cp.id.to_string().as_str()));
        if payment.counterparty_id == Some(cp.id) {
            selected_index = index as i32 + 1;
        }
    }

    ui.set_payment_form_counterparty_names(ModelRc::new(VecModel::from(names)));
    ui.set_payment_form_counterparty_ids(ModelRc::new(VecModel::from(ids)));
    ui.set_payment_form_is_edit(true);
    ui.set_payment_form_edit_id(SharedString::from(payment.id.to_string().as_str()));
    ui.set_payment_form_date(SharedString::from(payment.date.format("%d.%m.%Y").to_string()));
    ui.set_payment_form_amount(SharedString::from(format!("{:.2}", payment.amount)));
    ui.set_payment_form_direction_index(match payment.direction {
        models::payment::PaymentDirection::Income => 0,
        models::payment::PaymentDirection::Expense => 1,
    });
    ui.set_payment_form_counterparty_index(selected_index);
    ui.set_payment_form_bank_name(SharedString::from(
        payment.bank_name.as_deref().unwrap_or(""),
    ));
    ui.set_payment_form_bank_ref(SharedString::from(
        payment.bank_ref.as_deref().unwrap_or(""),
    ));
    ui.set_payment_form_description(SharedString::from(
        payment.description.as_deref().unwrap_or(""),
    ));
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
    doc_state: Arc<Mutex<DocListState>>,
    company_id: uuid::Uuid,
    number: String,
    date_str: String,
    cp_id_str: String,
    notes: Option<String>,
    cat_id_str: String,
    con_id_str: String,
    exp_date_str: String,
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
                let (doc_tab, doc_direction, doc_query, doc_cp, doc_df, doc_dt) = {
                    let s = doc_state.lock().unwrap();
                    (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_from, s.date_to)
                };
                if let Err(e) = reload_documents(&pool, ui_weak.clone(), company_id, doc_tab, &doc_direction, &doc_query, doc_cp, doc_df, doc_dt).await {
                    tracing::error!("Помилка оновлення документів після збереження акту: {e}");
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
    doc_state: Arc<Mutex<DocListState>>,
    company_id: uuid::Uuid,
    number: String,
    date_str: String,
    cp_id_str: String,
    notes: Option<String>,
    cat_id_str: String,
    con_id_str: String,
    exp_date_str: String,
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
                show_toast(ui_weak.clone(), format!("Накладну '{}' збережено", inv.number), false);
                let (status_filter, query) = {
                    let state = inv_state.lock().unwrap();
                    (state.status_filter.clone(), state.query.clone())
                };
                if let Err(e) = reload_invoices(&pool, ui_weak.clone(), company_id, status_filter, query, true).await {
                    tracing::error!("Помилка оновлення списку накладних: {e}");
                }
                let (doc_tab, doc_direction, doc_query, doc_cp, doc_df, doc_dt) = {
                    let s = doc_state.lock().unwrap();
                    (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_from, s.date_to)
                };
                if let Err(e) = reload_documents(&pool, ui_weak, company_id, doc_tab, &doc_direction, &doc_query, doc_cp, doc_df, doc_dt).await {
                    tracing::error!("Помилка оновлення документів після збереження накладної: {e}");
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

fn to_task_rows(tasks: &[Task]) -> Vec<TaskRow> {
    tasks
        .iter()
        .map(|task| TaskRow {
            id: SharedString::from(task.id.to_string().as_str()),
            title: SharedString::from(task.title.as_str()),
            priority_label: SharedString::from(task.priority.label()),
            due_date: format_task_datetime(task.due_date),
            reminder: format_task_datetime(task.reminder_at),
            status_label: SharedString::from(task.status.label()),
            status: SharedString::from(task.status.as_str()),
            priority: SharedString::from(task.priority.as_str()),
        })
        .collect()
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
    let task_rows = to_task_rows(&filtered);

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_task_rows(ModelRc::new(VecModel::from(task_rows)));
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
    let task_rows = to_task_rows(&tasks);

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_act_task_rows(ModelRc::new(VecModel::from(task_rows)));
            ui.set_act_tasks_loading(false);
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

fn spawn_save_task(
    pool: sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    task_state: Arc<Mutex<TaskListState>>,
    company_id: uuid::Uuid,
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
            db::tasks::create(&pool, company_id, &task).await.map(Some)
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
    use std::sync::{Arc, Mutex};

    use chrono::{TimeZone, Utc};
    use rust_decimal::Decimal;
    use slint::{Model, SharedString};
    use uuid::Uuid;

    use crate::models::{
        ActListRow, ActStatus, Company, CompanySummary, Counterparty, Task, TaskPriority, TaskStatus,
    };

    use super::{
        MainWindow, build_models, company_display_name,
        company_initials, company_subtitle, format_amount_ua, format_company_total, format_kpi_amount,
        format_task_datetime, normalized_query, parse_task_datetime, task_matches_query,
        task_priority_from_index, task_priority_index, to_act_rows, to_table_data,
        to_task_rows,
    };

    fn sample_counterparty() -> Counterparty {
        Counterparty {
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
        }
    }

    fn sample_task() -> Task {
        Task {
            id: Uuid::new_v4(),
            title: "Перевірити оплату".to_string(),
            description: Some("До кінця дня".to_string()),
            status: TaskStatus::InProgress,
            priority: TaskPriority::Critical,
            due_date: Some(Utc.with_ymd_and_hms(2026, 4, 3, 18, 45, 0).unwrap()),
            reminder_at: None,
            counterparty_id: None,
            act_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

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
        let cp = sample_counterparty();

        let table = to_table_data(&[cp]);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0][0].as_str(), "ТОВ Приклад");
        assert_eq!(table.rows[0][1].as_str(), "—");
        assert_eq!(table.rows[0][2].as_str(), "—");
        assert_eq!(table.rows[0][3].as_str(), "—");
        assert_eq!(table.archived, vec![false]);
    }

    #[test]
    fn to_act_rows_formats_date_amount_and_status() {
        let act = ActListRow {
            id: Uuid::new_v4(),
            number: "АКТ-2026-007".to_string(),
            direction: "outgoing".to_string(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 4, 1).expect("valid date"),
            counterparty_name: "ФОП Іваненко".to_string(),
            total_amount: Decimal::new(12345, 2),
            status: ActStatus::Issued,
        };

        let rows = to_act_rows(&[act]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].num.as_str(), "АКТ-2026-007");
        assert_eq!(rows[0].date.as_str(), "01.04.2026");
        assert_eq!(rows[0].counterparty.as_str(), "ФОП Іваненко");
        assert_eq!(rows[0].amount.as_str(), "123,45 ₴");
        assert_eq!(rows[0].status_label.as_str(), "Виставлено");
        assert_eq!(rows[0].status.as_str(), "issued");
    }

    #[test]
    fn task_priority_index_roundtrip_works() {
        assert_eq!(task_priority_from_index(0), TaskPriority::Low);
        assert_eq!(task_priority_from_index(1), TaskPriority::Normal);
        assert_eq!(task_priority_from_index(2), TaskPriority::High);
        assert_eq!(task_priority_from_index(99), TaskPriority::Critical);

        assert_eq!(task_priority_index(&TaskPriority::Low), 0);
        assert_eq!(task_priority_index(&TaskPriority::Normal), 1);
        assert_eq!(task_priority_index(&TaskPriority::High), 2);
        assert_eq!(task_priority_index(&TaskPriority::Critical), 3);
    }

    #[test]
    fn format_task_datetime_and_parse_roundtrip_work() {
        let dt = Utc.with_ymd_and_hms(2026, 4, 2, 14, 30, 0).unwrap();
        assert_eq!(format_task_datetime(Some(dt)).as_str(), "02.04.2026 14:30");
        assert_eq!(format_task_datetime(None).as_str(), "—");

        assert_eq!(parse_task_datetime("   ").expect("empty is allowed"), None);
        assert_eq!(
            parse_task_datetime("02.04.2026 14:30")
                .expect("valid datetime")
                .expect("datetime exists"),
            dt
        );
        assert!(parse_task_datetime("2026-04-02 14:30").is_err());
    }

    #[test]
    fn task_matches_query_uses_title_and_description_case_insensitively() {
        let task = Task {
            title: "Підготувати акт".to_string(),
            description: Some("Узгодити з клієнтом фінальну версію".to_string()),
            ..sample_task()
        };

        assert!(task_matches_query(&task, None));
        assert!(task_matches_query(&task, Some("АКТ")));
        assert!(task_matches_query(&task, Some("клієнтом")));
        assert!(!task_matches_query(&task, Some("накладна")));
    }

    #[test]
    fn to_task_rows_formats_rows_and_metadata() {
        let task = sample_task();
        let rows = to_task_rows(&[task.clone()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title.as_str(), "Перевірити оплату");
        assert_eq!(rows[0].priority_label.as_str(), "Критичний");
        assert_eq!(rows[0].due_date.as_str(), "03.04.2026 18:45");
        assert_eq!(rows[0].reminder.as_str(), "—");
        assert_eq!(rows[0].status_label.as_str(), "В роботі");
        assert_eq!(rows[0].id.as_str(), task.id.to_string());
        assert_eq!(rows[0].status.as_str(), "in_progress");
        assert_eq!(rows[0].priority.as_str(), "critical");
    }

    #[test]
    fn build_models_helpers_create_expected_row_counts() {
        let counterparty_data = to_table_data(&[sample_counterparty()]);
        let (rows, ids, archived) = build_models(counterparty_data);
        assert_eq!(rows.row_count(), 1);
        assert_eq!(ids.row_count(), 1);
        assert_eq!(archived.row_count(), 1);

        let act_rows = to_act_rows(&[ActListRow {
            id: Uuid::new_v4(),
            number: "АКТ-1".to_string(),
            direction: "outgoing".to_string(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            counterparty_name: "ФОП".to_string(),
            total_amount: Decimal::new(1000, 2),
            status: ActStatus::Draft,
        }]);
        assert_eq!(act_rows.len(), 1);

        let task_rows = to_task_rows(&[sample_task()]);
        assert_eq!(task_rows.len(), 1);
    }

    #[test]
    fn company_helpers_format_text_for_ui() {
        let company = Company {
            id: Uuid::new_v4(),
            name: "Товариство Приклад".to_string(),
            short_name: Some("Приклад".to_string()),
            edrpou: Some("12345678".to_string()),
            ipn: None,
            iban: None,
            legal_address: None,
            actual_address: None,
            phone: None,
            email: None,
            director_name: None,
            accountant_name: None,
            tax_system: None,
            is_vat_payer: false,
            logo_path: None,
            notes: None,
            is_archived: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let summary = CompanySummary {
            id: company.id,
            name: company.name.clone(),
            short_name: company.short_name.clone(),
            edrpou: company.edrpou.clone(),
            is_vat_payer: false,
            act_count: 3,
            total_amount: Decimal::new(123456, 2),
        };

        assert_eq!(company_display_name(&company), "Приклад");
        assert_eq!(company_subtitle(&company), "ЄДРПОУ: 12345678");
        assert_eq!(company_initials(&summary), "П");
        assert_eq!(format_company_total(&Decimal::new(123456, 2)), "1234.56 грн");
        assert_eq!(format_kpi_amount(Decimal::new(78000, 0)), "78 000 ₴");
        assert_eq!(format_amount_ua(Decimal::new(12345, 2)), "123,45 ₴");
        assert_eq!(format_amount_ua(Decimal::new(7800000, 2)), "78 000,00 ₴");
        assert_eq!(format_amount_ua(Decimal::new(0, 2)), "0,00 ₴");
    }

    #[test]
    fn slint_callback_harness_covers_callbacks_and_properties() {
        let ui = MainWindow::new().expect("MainWindow should be constructible in tests");
        let received_query = Arc::new(Mutex::new(None::<String>));
        let query_capture = Arc::clone(&received_query);
        let call_count = Arc::new(Mutex::new(0usize));
        let count_capture = Arc::clone(&call_count);

        ui.on_task_search_changed(move |query| {
            *query_capture.lock().expect("mutex poisoned") = Some(query.to_string());
        });
        ui.on_company_add_clicked(move || {
            *count_capture.lock().expect("mutex poisoned") += 1;
        });

        ui.invoke_task_search_changed(SharedString::from("рахунок"));
        ui.invoke_company_add_clicked();
        ui.invoke_company_add_clicked();

        ui.set_current_page(6);
        ui.set_show_company_picker(true);
        ui.set_toast_message(SharedString::from("Збережено"));
        ui.set_task_form_title(SharedString::from("Передзвонити клієнту"));

        assert_eq!(
            received_query.lock().expect("mutex poisoned").as_deref(),
            Some("рахунок")
        );
        assert_eq!(*call_count.lock().expect("mutex poisoned"), 2);
        assert_eq!(ui.get_current_page(), 6);
        assert!(ui.get_show_company_picker());
        assert_eq!(ui.get_toast_message().as_str(), "Збережено");
        assert_eq!(ui.get_task_form_title().as_str(), "Передзвонити клієнту");
    }
}
