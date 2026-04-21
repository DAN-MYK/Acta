// Acta — програма управлінського обліку
//
// Підключаємо Rust типи, згенеровані з .slint файлів.
// Після цього доступний AppWindow (та інші export компоненти).
slint::include_modules!();

mod ui;

use anyhow::Result;
use acta::notifications;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let _ = dotenvy::dotenv();

    // Tokio runtime — пул потоків окремо від головного потоку Slint.
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

    // AppWindow — тип згенерований з ui-redesign/app.slint
    let ui = AppWindow::new()?;

    // ── Stub callbacks — TODO: реалізувати завантаження даних з БД ─────────────
    ui.on_nav_changed(|_| {});
    ui.on_doc_search_changed(|_| {});
    ui.on_doc_tab_changed(|_| {});
    ui.on_doc_toggled(|_, _| {});
    ui.on_doc_open(|_| {});
    ui.on_doc_send(|_| {});
    ui.on_doc_delete(|_| {});
    ui.on_doc_new(|| {});
    ui.on_doc_page_changed(|_| {});
    ui.on_cp_selected(|_| {});
    ui.on_cp_search_changed(|_| {});
    ui.on_cp_new(|| {});
    ui.on_cp_create_doc(|_| {});
    ui.on_cp_tab_changed(|_| {});
    ui.on_pay_import_csv(|| {});
    ui.on_pay_sync_bank(|| {});
    ui.on_pay_new(|| {});
    ui.on_pay_link(|_| {});
    ui.on_rep_period_changed(|_| {});
    ui.on_rep_category_drilled(|_| {});
    ui.on_rep_export_csv(|| {});
    ui.on_rep_export_pdf(|| {});
    ui.on_task_toggled(|_, _| {});
    ui.on_task_more(|_| {});
    ui.on_task_new(|| {});
    ui.on_task_filter_changed(|_| {});
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
