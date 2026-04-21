// Acta — програма управлінського обліку
//
// Підключаємо Rust типи, згенеровані з .slint файлів.
// Після цього доступний AppWindow (та інші export компоненти).
slint::include_modules!();

mod ui;

use anyhow::Result;
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

async fn get_first_company_id(pool: &sqlx::PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM companies ORDER BY created_at LIMIT 1")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(Uuid::nil)
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let _ = dotenvy::dotenv();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _rt_guard = rt.enter();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL не задано. Перевір .env файл.");

    let pool = rt.block_on(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url),
    )?;

    rt.block_on(sqlx::migrate!("./migrations").run(&pool))?;
    tracing::info!("Міграції застосовано.");

    let company_id = rt.block_on(get_first_company_id(&pool));
    let active_company_id: Arc<Mutex<Uuid>> = Arc::new(Mutex::new(company_id));

    let pool = Arc::new(pool);

    tokio::spawn(acta::notifications::reminder_loop(pool.clone()));

    let ui = AppWindow::new()?;

    // ── Початкове завантаження даних (паралельно) ────────────────────────────
    let (dash_data, doc_data, cp_data, pay_data, task_data) = rt.block_on(async {
        tokio::join!(
            ui::dashboard::prepare_dashboard_data(&pool, company_id),
            ui::documents::prepare_documents_data(&pool, company_id, None, None),
            ui::counterparties::prepare_counterparties_data(&pool, company_id, None),
            ui::payments::prepare_payments_data(&pool, company_id),
            ui::tasks::prepare_tasks_data(&pool),
        )
    });

    ui::dashboard::apply_dashboard_to_ui(&ui, dash_data);
    ui::documents::apply_documents_to_ui(&ui, doc_data);
    ui::counterparties::apply_counterparties_to_ui(&ui, cp_data);
    ui::payments::apply_payments_to_ui(&ui, pay_data);
    ui::tasks::apply_tasks_to_ui(&ui, task_data);
    ui::settings::apply_settings_to_ui(&ui);

    ui.set_company_name("Acta".into());
    ui.set_user_name("Адміністратор".into());
    ui.set_user_initials("АД".into());

    // ── Навігація ────────────────────────────────────────────────────────────
    ui.on_nav_changed({
        let pool = pool.clone();
        let ui_weak = ui.as_weak();
        let company_id = active_company_id.clone();
        move |screen| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let cid = *company_id.lock().unwrap();
            tokio::spawn(async move {
                match screen {
                    NavScreen::Dashboard => {
                        let data = ui::dashboard::prepare_dashboard_data(&pool, cid).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::dashboard::apply_dashboard_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Documents => {
                        let data =
                            ui::documents::prepare_documents_data(&pool, cid, None, None).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::documents::apply_documents_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Counterparties => {
                        let data =
                            ui::counterparties::prepare_counterparties_data(&pool, cid, None).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::counterparties::apply_counterparties_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Payments => {
                        let data = ui::payments::prepare_payments_data(&pool, cid).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::payments::apply_payments_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Tasks => {
                        let data = ui::tasks::prepare_tasks_data(&pool).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::tasks::apply_tasks_to_ui(&ui, data);
                        });
                    }
                    _ => {}
                }
            });
        }
    });

    // ── Документи ────────────────────────────────────────────────────────────
    ui::documents::wire_document_callbacks(&ui, &pool, &active_company_id);

    // ── Контрагенти ──────────────────────────────────────────────────────────
    ui::counterparties::wire_counterparty_callbacks(&ui, &pool, &active_company_id);

    // ── Завдання ─────────────────────────────────────────────────────────────
    ui::tasks::wire_task_callbacks(&ui, &pool);

    // ── Заглушки для нереалізованих callback'ів ──────────────────────────────
    ui.on_pay_import_csv(|| {});
    ui.on_pay_sync_bank(|| {});
    ui.on_pay_new(|| {});
    ui.on_pay_link(|_| {});
    ui.on_rep_period_changed(|_| {});
    ui.on_rep_category_drilled(|_| {});
    ui.on_rep_export_csv(|| {});
    ui.on_rep_export_pdf(|| {});
    ui.on_settings_section_changed(|_| {});
    ui.on_settings_dark_mode_toggled(|_| {});
    ui.on_settings_density_changed(|_| {});
    ui.on_settings_company_saved(|_| {});
    ui.on_settings_integration_configure(|_| {});
    ui.on_settings_team_invite(|| {});
    ui.on_settings_backup_now(|| {});
    ui.on_settings_backup_download(|| {});
    ui.on_palette_query_changed(|_| {});
    ui.on_palette_item_activated(|_| {});

    ui.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[test]
    fn tokio_runtime_multi_thread_builds_and_runs_async_tasks() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime повинен будуватись без помилок");

        let result = rt.block_on(async { 6u32 + 7 });
        assert_eq!(result, 13);

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

        let (a, b) = rt.block_on(async { tokio::join!(async { 1u32 }, async { 2u32 }) });
        assert_eq!(a + b, 3);
    }

    #[test]
    fn active_company_id_starts_as_nil_and_updates_after_selection() {
        let company_id: Arc<Mutex<Uuid>> = Arc::new(Mutex::new(Uuid::nil()));
        assert!(company_id.lock().unwrap().is_nil());

        let selected = Uuid::new_v4();
        *company_id.lock().unwrap() = selected;

        assert_eq!(*company_id.lock().unwrap(), selected);
        assert!(!company_id.lock().unwrap().is_nil());
    }

    #[test]
    fn active_company_id_clones_share_the_same_mutex() {
        let id: Arc<Mutex<Uuid>> = Arc::new(Mutex::new(Uuid::nil()));
        let id_in_callback = Arc::clone(&id);

        let new_id = Uuid::new_v4();
        *id_in_callback.lock().unwrap() = new_id;

        assert_eq!(*id.lock().unwrap(), new_id);
    }
}
