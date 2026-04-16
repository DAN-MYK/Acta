// Acta — програма управлінського обліку
//
// Підключаємо Rust типи, згенеровані з .slint файлів.
// Після цього доступний MainWindow (та інші export компоненти).
// ВАЖЛИВО: має бути на рівні модуля — не всередині функції.
slint::include_modules!();

mod ui;

use acta::app_ctx::{AppCtx, ActListState, CounterpartyListState, DocListState, InvoiceListState, TaskListState, PaymentListState};
use acta::{config::AppConfig, db, notifications};
use anyhow::Result;
use slint::{ModelRc, SharedString, VecModel};
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};
use ui::{
    acts::{apply_acts_to_ui, prepare_acts_data},
    companies::{apply_settings_to_ui, prepare_settings_data, reload_companies, reload_settings},
    counterparties::{apply_counterparties_to_ui, prepare_counterparties_data, reload_counterparties},
    dashboard::reload_dashboard,
    documents::{apply_documents_to_ui, fetch_doc_cp_filter_data, prepare_documents_data, reload_documents, DocCpFilterData},
    helpers::{apply_company_rows, company_display_name, company_subtitle, reset_company_form},
    invoices::{apply_invoices_to_ui, prepare_invoices_data},
    payments::{apply_payments_to_ui, prepare_payments_data, prepare_payment_cp_options_data, reload_payments},
    tasks::{apply_tasks_to_ui, prepare_tasks_data, reload_tasks},
};

/// Розмір сторінки у списку контрагентів.
pub const COUNTERPARTY_PAGE_SIZE: usize = 10;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let _ = dotenvy::dotenv();

    // Tokio runtime — пул потоків окремо від головного потоку Slint.
    // _rt_guard «входить» в runtime для поточного (головного) потоку,
    // тому tokio::spawn всередині callbacks працює без змін.
    // Slint вимагає що ui.run() виконується у справжньому OS main thread —
    // ця схема гарантує це на всіх платформах.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _rt_guard = rt.enter();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL не задано. Перевір .env файл.");

    let pool = rt.block_on(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url),
    )?;

    rt.block_on(sqlx::migrate!("./migrations").run(&pool))?;
    tracing::info!("Міграції застосовано.");

    tokio::spawn(notifications::reminder_loop(Arc::new(pool.clone())));

    // ── Створення вікна ──────────────────────────────────────────────────────
    // MainWindow — тип згенерований з ui/app.slint
    let ui = MainWindow::new()?;
    ui.set_counterparty_show_archived(false);

    // Toast та current-page зараз синхронізуються напряму через MainWindow
    // properties, без старого Rust API до Slint global.
    let ui_weak = ui.as_weak();

    // ── Активна компанія та стани списків — спільні між усіма callbacks ──────
    // Nil UUID = компанія ще не обрана. DB-запити з nil UUID повернуть порожній результат.
    let active_company_id: Arc<Mutex<uuid::Uuid>> = Arc::new(Mutex::new(uuid::Uuid::nil()));
    let doc_cp_ids: Arc<Mutex<Vec<uuid::Uuid>>> = Arc::new(Mutex::new(vec![]));

    // ── Початкове завантаження компаній ─────────────────────────────────────
    // block_on виконується на головному потоці → Slint-виклики безпечні.
    rt.block_on(async {
        let config = AppConfig::load();
        let companies = db::companies::list(&pool).await.unwrap_or_default();
        let company_rows = db::companies::list_with_summary(&pool).await.unwrap_or_default();
        // Відображаємо список у UI (для сторінки Компанії)
        apply_company_rows(&ui, &company_rows, *active_company_id.lock().unwrap());
        ui.set_active_company_subtitle(SharedString::from("Оберіть компанію для роботи"));

        match companies.len() {
            0 => {
                // Немає жодної компанії → одразу на сторінку Компанії (створити)
                ui.set_current_page(6);
                ui.set_active_company_name(SharedString::from("Оберіть компанію"));
                ui.set_active_company_id(SharedString::from(""));
                ui.set_active_company_subtitle(SharedString::from("Створіть першу компанію"));
                reset_company_form(&ui);
                ui.set_show_company_form(true);
            }
            1 => {
                // Єдина компанія — обираємо автоматично
                let c = &companies[0];
                *active_company_id.lock().unwrap() = c.id;
                ui.set_active_company_name(SharedString::from(company_display_name(c).as_str()));
                ui.set_active_company_id(SharedString::from(c.id.to_string().as_str()));
                ui.set_active_company_subtitle(SharedString::from(company_subtitle(c).as_str()));
                tracing::info!("Активна компанія: '{}'", c.name);
            }
            _ => {
                // Кілька компаній — відновити останню або показати вибір
                let restored = config.last_company_id.and_then(|lid| {
                    companies.iter().find(|c| c.id == lid).cloned()
                });
                if let Some(c) = restored {
                    *active_company_id.lock().unwrap() = c.id;
                    ui.set_active_company_name(SharedString::from(company_display_name(&c).as_str()));
                    ui.set_active_company_id(SharedString::from(c.id.to_string().as_str()));
                    ui.set_active_company_subtitle(SharedString::from(company_subtitle(&c).as_str()));
                    tracing::info!("Відновлено останню компанію: '{}'", c.name);
                } else {
                    ui.set_show_company_picker(true);
                    ui.set_active_company_name(SharedString::from("Оберіть компанію"));
                    ui.set_active_company_id(SharedString::from(""));
                    ui.set_active_company_subtitle(SharedString::from("Доступно кілька компаній"));
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    })?;

    // ── Початкове завантаження (паралельно, без upgrade_in_event_loop) ──────────
    // Ми на main thread до ui.run() → ui.set_*() безпечно викликати напряму.
    // tokio::join! виконує всі запити до БД паралельно → швидший старт.
    // Після цього apply_*_to_ui встановлює дані напряму в UI:
    // вікно відкриється вже з даними (без "flash of empty content").
    //
    // Якщо компанія ще не обрана (nil UUID) — пропускаємо завантаження:
    // UI залишається порожнім, picker/форма компанії вже відображені.
    let cid = *active_company_id.lock().unwrap();
    if !cid.is_nil() {
        let (cp_data, acts_data, inv_data, tasks_data, pay_data, doc_cp_data, docs_data, settings_data, pay_cp_data) =
            rt.block_on(async {
                let (r0, r1, r2, r3, r4, r5, r6, r7, r8) = tokio::join!(
                    prepare_counterparties_data(&pool, cid, String::new(), false, 0),
                    prepare_acts_data(&pool, cid, None, String::new()),
                    prepare_invoices_data(&pool, cid, None, String::new()),
                    prepare_tasks_data(&pool, String::new()),
                    prepare_payments_data(&pool, cid, None, ""),
                    fetch_doc_cp_filter_data(&pool, cid),
                    prepare_documents_data(&pool, cid, 0, "outgoing", "", None, None, None),
                    prepare_settings_data(&pool, cid),
                    prepare_payment_cp_options_data(&pool, cid),
                );
                Ok::<_, anyhow::Error>((r0?, r1?, r2?, r3?, r4?, r5?, r6?, r7?, r8?))
            })?;

        // Оновлюємо doc_cp_ids перед застосуванням фільтру
        let DocCpFilterData { cp_ids: doc_cp_id_list, names: doc_cp_names } = doc_cp_data;
        {
            let mut ids = doc_cp_ids.lock().unwrap();
            *ids = doc_cp_id_list;
        }

        // Застосовуємо всі дані напряму (main thread — ui.set_*() без event loop)
        apply_counterparties_to_ui(&ui, cp_data, false);
        apply_acts_to_ui(&ui, acts_data, false);
        apply_invoices_to_ui(&ui, inv_data, false);
        apply_tasks_to_ui(&ui, tasks_data, false);
        apply_payments_to_ui(&ui, pay_data);
        ui.set_doc_filter_cp_names(ModelRc::new(VecModel::from(doc_cp_names)));
        apply_documents_to_ui(&ui, docs_data);
        apply_settings_to_ui(&ui, settings_data);
        {
            let d = pay_cp_data;
            ui.set_payment_form_counterparty_names(ModelRc::new(VecModel::from(d.names)));
            ui.set_payment_form_counterparty_ids(ModelRc::new(VecModel::from(d.ids)));
            ui.set_payment_form_counterparty_index(0);
        }
    }

    // ── Спільний контекст ────────────────────────────────────────────────────
    let ctx = Arc::new(AppCtx {
        pool: pool.clone(),
        active_company_id,
        doc_cp_ids,
        counterparty_state: Arc::new(Mutex::new(CounterpartyListState::default())),
        act_state: Arc::new(Mutex::new(ActListState::default())),
        invoice_state: Arc::new(Mutex::new(InvoiceListState::default())),
        doc_state: Arc::new(Mutex::new(DocListState::default())),
        task_state: Arc::new(Mutex::new(TaskListState::default())),
        payment_state: Arc::new(Mutex::new(PaymentListState::default())),
    });

    // ── navigate(feature, page) — навігація з перезавантаженням даних ───────────
    // Викликається з Slint коли користувач клікає NavItem у sidebar.
    // current-page зберігаємо напряму в MainWindow, а не через старий global API.
    // Реєструється ПІСЛЯ ctx — щоб мати доступ до pool та active_company_id.
    {
        let pool = ctx.pool.clone();
        let cid_arc = ctx.active_company_id.clone();
        let weak = ui_weak.clone();
        ui.on_navigate(move |feature, page| {
            if let Some(ui) = weak.upgrade() {
                ui.set_current_page(page);
            }
            let cid = *cid_arc.lock().unwrap();
            if cid.is_nil() {
                return; // компанія ще не обрана — нічого не завантажувати
            }
            let pool = pool.clone();
            let weak2 = weak.clone();
            let feat = feature.to_string();
            tokio::spawn(async move {
                let result = match feat.as_str() {
                    "dashboard" => {
                        reload_dashboard(&pool, weak2, cid).await
                    }
                    "counterparties" => {
                        reload_counterparties(&pool, weak2, cid, String::new(), false, 0, false).await
                    }
                    "documents" => {
                        reload_documents(&pool, weak2, cid, 0, "outgoing", "", None, None, None).await
                    }
                    "payments" => {
                        reload_payments(&pool, weak2, cid, None, "").await
                    }
                    "tasks" => {
                        reload_tasks(&pool, weak2, String::new(), false).await
                    }
                    "companies" => {
                        reload_companies(&pool, weak2, cid).await
                    }
                    "settings" => {
                        reload_settings(&pool, weak2, cid).await
                    }
                    // "reports", "calendar" — заглушки, даних нема
                    _ => Ok(()),
                };
                if let Err(e) = result {
                    tracing::error!("Помилка завантаження при навігації до '{feat}': {e:#}");
                }
            });
        });
    }

    // ── Реєстрація callbacks по модулях ──────────────────────────────────────
    ui::counterparties::setup(&ui, ctx.clone());
    ui::acts::setup(&ui, ctx.clone());
    ui::invoices::setup(&ui, ctx.clone());
    ui::tasks::setup(&ui, ctx.clone());
    ui::payments::setup(&ui, ctx.clone());
    ui::documents::setup(&ui, ctx.clone());
    ui::companies::setup(&ui, ctx.clone());
    ui::dashboard::setup(&ui, ctx.clone());

    // run() блокує до закриття вікна
    ui.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use chrono::{TimeZone, Utc};
    use rust_decimal::Decimal;
    use slint::{Model, SharedString};
    use uuid::Uuid;

    use acta::models::{
        ActListRow, ActStatus, Company, CompanySummary, Counterparty, Task, TaskPriority, TaskStatus,
    };

    use crate::{
        MainWindow,
        ui::helpers::{
            build_models, company_display_name, company_initials, company_subtitle,
            format_amount_ua, format_company_total, format_kpi_amount, format_task_datetime,
            normalized_query, parse_task_datetime, task_matches_query, task_priority_from_index,
            task_priority_index, to_act_rows, to_table_data, to_task_rows,
        },
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
        assert_eq!(rows[0].status, crate::ActStatus::Issued);
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
        assert_eq!(format_kpi_amount(Decimal::new(78000, 0)), "78\u{00A0}000 ₴");
        assert_eq!(format_amount_ua(Decimal::new(12345, 2)), "123,45 ₴");
        assert_eq!(format_amount_ua(Decimal::new(7800000, 2)), "78\u{00A0}000,00 ₴");
        assert_eq!(format_amount_ua(Decimal::new(0, 2)), "0,00 ₴");
    }

    #[test]
    fn slint_callback_harness_covers_callbacks_and_properties() {
        i_slint_backend_testing::init_no_event_loop();
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

    // ── Tokio runtime ─────────────────────────────────────────────────────────
    // Тестуємо що main() зможе збудувати runtime та паралельно виконувати задачі.
    // Ці тести не потребують БД і не відкривають вікно.

    #[test]
    fn tokio_runtime_multi_thread_builds_and_runs_async_tasks() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime повинен будуватись без помилок");

        // block_on виконує Future на головному потоці
        let result = rt.block_on(async { 6u32 + 7 });
        assert_eq!(result, 13);

        // tokio::spawn виконує задачу на пулі потоків
        let spawned = rt.block_on(async {
            tokio::spawn(async { "spawn_ok" })
                .await
                .expect("spawn не повинен панікувати")
        });
        assert_eq!(spawned, "spawn_ok");
    }

    #[test]
    fn tokio_runtime_join_runs_two_futures_in_parallel() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime");

        // tokio::join! — паттерн що використовується в main() для початкового завантаження
        let (a, b) = rt.block_on(async { tokio::join!(async { 1u32 }, async { 2u32 }) });
        assert_eq!(a + b, 3);
    }

    // ── Shared state: Arc<Mutex<Uuid>> ────────────────────────────────────────
    // active_company_id у main() — Arc<Mutex<Uuid>>. Nil UUID = компанія не обрана.

    #[test]
    fn active_company_id_starts_as_nil_and_updates_after_selection() {
        let company_id: Arc<Mutex<Uuid>> = Arc::new(Mutex::new(Uuid::nil()));

        // До вибору компанії — nil
        assert!(company_id.lock().unwrap().is_nil(), "до вибору — nil UUID");

        let selected = Uuid::new_v4();
        *company_id.lock().unwrap() = selected;

        assert_eq!(*company_id.lock().unwrap(), selected, "після вибору — реальний UUID");
        assert!(!company_id.lock().unwrap().is_nil());
    }

    #[test]
    fn active_company_id_clones_share_the_same_mutex() {
        // Arc::clone передає той самий Mutex між callbacks — зміна в одному видима в іншому
        let id: Arc<Mutex<Uuid>> = Arc::new(Mutex::new(Uuid::nil()));
        let id_in_callback = Arc::clone(&id);

        let new_id = Uuid::new_v4();
        *id_in_callback.lock().unwrap() = new_id;

        assert_eq!(*id.lock().unwrap(), new_id, "оригінал відображає зміну зробленою в клоні");
    }

    // ── DocListState defaults ─────────────────────────────────────────────────
    // DocListState::default() визначає початковий вигляд сторінки Документи.

    #[test]
    fn doc_list_state_default_is_outgoing_direction_and_all_docs_tab() {
        use acta::app_ctx::DocListState;
        let s = DocListState::default();

        assert_eq!(s.direction, "outgoing", "початковий напрям — вихідні документи");
        assert_eq!(s.tab, 0, "початкова вкладка — Всі (0=всі, 1=акти, 2=накладні)");
        assert_eq!(s.counterparty_index, 0, "0 = всі контрагенти");
        assert!(s.query.is_empty());
        assert!(s.counterparty_id.is_none());
        assert!(s.date_from.is_none());
        assert!(s.date_to.is_none());
    }

    // ── Slint: додаткові callback bindings ────────────────────────────────────
    // on_*(handler) реєструє обробник; invoke_*() доставляє подію синхронно.

    #[test]
    fn callback_counterparty_search_changed_captures_query_string() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = MainWindow::new().expect("MainWindow");
        let captured = Arc::new(Mutex::new(String::new()));
        let cap = Arc::clone(&captured);

        ui.on_counterparty_search_changed(move |q| *cap.lock().unwrap() = q.to_string());
        ui.invoke_counterparty_search_changed(SharedString::from("іваненко"));

        assert_eq!(captured.lock().unwrap().as_str(), "іваненко");
    }

    #[test]
    fn callback_act_status_filter_changed_captures_int_index() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = MainWindow::new().expect("MainWindow");
        let captured = Arc::new(Mutex::new(-1i32));
        let cap = Arc::clone(&captured);

        // 0=всі, 1=чернетка, 2=виставлено, 3=підписано, 4=оплачено
        ui.on_act_status_filter_changed(move |idx| *cap.lock().unwrap() = idx);
        ui.invoke_act_status_filter_changed(2);

        assert_eq!(*captured.lock().unwrap(), 2);
    }

    #[test]
    fn callback_payment_direction_filter_changed_captures_int_index() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = MainWindow::new().expect("MainWindow");
        let captured = Arc::new(Mutex::new(0i32));
        let cap = Arc::clone(&captured);

        // 0=всі, 1=incoming, 2=outgoing
        ui.on_payment_direction_filter_changed(move |idx| *cap.lock().unwrap() = idx);
        ui.invoke_payment_direction_filter_changed(1);

        assert_eq!(*captured.lock().unwrap(), 1);
    }

    #[test]
    fn callbacks_invoice_search_and_doc_search_fire_independently() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = MainWindow::new().expect("MainWindow");

        let inv_q = Arc::new(Mutex::new(String::new()));
        let doc_q = Arc::new(Mutex::new(String::new()));

        {
            let c = Arc::clone(&inv_q);
            ui.on_invoice_search_changed(move |q| *c.lock().unwrap() = q.to_string());
        }
        {
            let c = Arc::clone(&doc_q);
            ui.on_doc_search_changed(move |q| *c.lock().unwrap() = q.to_string());
        }

        ui.invoke_invoice_search_changed(SharedString::from("ФОП"));
        ui.invoke_doc_search_changed(SharedString::from("2026"));

        assert_eq!(inv_q.lock().unwrap().as_str(), "ФОП");
        assert_eq!(doc_q.lock().unwrap().as_str(), "2026");
        // Переконуємось що invoice callback не змінив doc і навпаки
        assert_ne!(inv_q.lock().unwrap().as_str(), doc_q.lock().unwrap().as_str());
    }

    #[test]
    fn callback_click_handlers_count_invocations_correctly() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = MainWindow::new().expect("MainWindow");

        let act_count = Arc::new(Mutex::new(0u32));
        let cp_count = Arc::new(Mutex::new(0u32));

        {
            let c = Arc::clone(&act_count);
            ui.on_act_create_clicked(move || *c.lock().unwrap() += 1);
        }
        {
            let c = Arc::clone(&cp_count);
            ui.on_counterparty_create_clicked(move || *c.lock().unwrap() += 1);
        }

        ui.invoke_act_create_clicked();
        ui.invoke_act_create_clicked();
        ui.invoke_act_create_clicked();
        ui.invoke_counterparty_create_clicked();

        assert_eq!(*act_count.lock().unwrap(), 3, "act-create-clicked спрацював 3 рази");
        assert_eq!(*cp_count.lock().unwrap(), 1, "counterparty-create-clicked спрацював 1 раз");
    }

    // ── Slint: property roundtrips ────────────────────────────────────────────

    #[test]
    fn properties_form_flags_toggle_correctly() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = MainWindow::new().expect("MainWindow");

        ui.set_show_act_form(true);
        assert!(ui.get_show_act_form());
        ui.set_show_act_form(false);
        assert!(!ui.get_show_act_form());

        ui.set_show_cp_form(true);
        assert!(ui.get_show_cp_form());

        ui.set_show_company_form(true);
        assert!(ui.get_show_company_form());
    }

    #[test]
    fn properties_active_company_fields_roundtrip() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = MainWindow::new().expect("MainWindow");

        let cid = Uuid::new_v4();
        ui.set_active_company_name(SharedString::from("ТОВ Приклад"));
        ui.set_active_company_id(SharedString::from(cid.to_string().as_str()));
        ui.set_active_company_subtitle(SharedString::from("ЄДРПОУ: 12345678"));

        assert_eq!(ui.get_active_company_name().as_str(), "ТОВ Приклад");
        assert_eq!(ui.get_active_company_id().as_str(), cid.to_string().as_str());
        assert_eq!(ui.get_active_company_subtitle().as_str(), "ЄДРПОУ: 12345678");
    }

    #[test]
    fn property_current_page_navigates_across_all_main_sections() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = MainWindow::new().expect("MainWindow");

        // Нумерація з app.slint: 0=Контрагенти, 1=Документи, 3=Платежі,
        //                          5=To-Do, 6=Компанії, 8=Dashboard
        for page in [0i32, 1, 3, 5, 6, 8] {
            ui.set_current_page(page);
            assert_eq!(ui.get_current_page(), page, "перехід на сторінку {page}");
        }
    }
}
