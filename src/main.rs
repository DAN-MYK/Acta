// Acta - програма управлінського обліку.
//
// main.rs містить лише bootstrap: ініціалізацію runtime, БД, AppCtx та запуск UI.
// Уся orchestration-логіка винесена в окремі wire_* модулі.

slint::include_modules!();

mod ui;

use anyhow::Result;
use slint::ComponentHandle;
use std::sync::Arc;
use uuid::Uuid;

/// Повертає першу компанію або nil UUID, якщо компаній ще немає.
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
    let _guard = rt.enter();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL не задано. Перевір .env файл.");

    let pool = rt.block_on(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url),
    )?;

    rt.block_on(sqlx::migrate!("./migrations").run(&pool))?;
    tracing::info!("Міграції застосовано.");

    let company_id = rt.block_on(get_first_company_id(&pool));
    let ctx = Arc::new(acta::app_ctx::AppCtx::new(pool, company_id));

    {
        let pool = Arc::new(ctx.pool().clone());
        tokio::spawn(acta::notifications::reminder_loop(pool));
    }

    let ui = AppWindow::new()?;

    let cid = ctx.company_id();
    let (dash, docs, counterparties, payments, tasks, reports, settings) = rt.block_on(async {
        tokio::join!(
            ui::dashboard::prepare_dashboard_data(ctx.pool(), cid),
            ui::documents::prepare_documents_data(ctx.pool(), cid, None, None),
            ui::counterparties::prepare_counterparties_data(ctx.pool(), cid, None),
            ui::payments::prepare_payments_data(ctx.pool(), cid),
            ui::tasks::prepare_tasks_data(ctx.pool()),
            ui::reports::prepare_reports_data(ctx.pool(), cid, 1, None),
            ui::settings::prepare_settings_data(ctx.pool(), cid),
        )
    });

    ui::dashboard::apply_dashboard_to_ui(&ui, dash);
    ui::documents::apply_documents_to_ui(&ui, docs);
    ui::counterparties::apply_counterparties_to_ui(&ui, counterparties);
    ui::payments::apply_payments_to_ui(&ui, payments);
    ui::tasks::apply_tasks_to_ui(&ui, tasks);
    ui::reports::apply_reports_to_ui(&ui, reports);
    ui::settings::apply_settings_to_ui(&ui, settings);

    ui.set_company_name("Acta".into());
    ui.set_user_name("Адміністратор".into());
    ui.set_user_initials("АД".into());

    wire_navigation(&ui, &ctx);
    ui::documents::wire_document_callbacks(&ui, &ctx);
    ui::counterparties::wire_counterparty_callbacks(&ui, &ctx);
    ui::payments::wire_payment_callbacks(&ui, &ctx);
    ui::tasks::wire_task_callbacks(&ui, &ctx);
    ui::reports::wire_reports_callbacks(&ui, &ctx);
    ui::settings::wire_settings_callbacks(&ui, &ctx);
    wire_stub_callbacks(&ui);

    ui.run()?;
    Ok(())
}

fn wire_navigation(ui: &AppWindow, ctx: &Arc<acta::app_ctx::AppCtx>) {
    ui.on_nav_changed({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |screen| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                let cid = ctx.company_id();
                match screen {
                    NavScreen::Dashboard => {
                        let data = ui::dashboard::prepare_dashboard_data(ctx.pool(), cid).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::dashboard::apply_dashboard_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Documents => {
                        let data =
                            ui::documents::prepare_documents_data(ctx.pool(), cid, None, None)
                                .await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::documents::apply_documents_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Counterparties => {
                        let data =
                            ui::counterparties::prepare_counterparties_data(ctx.pool(), cid, None)
                                .await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::counterparties::apply_counterparties_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Payments => {
                        let data = ui::payments::prepare_payments_data(ctx.pool(), cid).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::payments::apply_payments_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Reports => {
                        let data =
                            ui::reports::prepare_reports_data(ctx.pool(), cid, 1, None).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::reports::apply_reports_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Tasks => {
                        let data = ui::tasks::prepare_tasks_data(ctx.pool()).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::tasks::apply_tasks_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Settings => {
                        let data = ui::settings::prepare_settings_data(ctx.pool(), cid).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::settings::apply_settings_to_ui(&ui, data);
                        });
                    }
                }
            });
        }
    });
}

/// Явні TODO-маркери для ще не реалізованих сценаріїв.
fn wire_stub_callbacks(ui: &AppWindow) {
    ui.on_palette_query_changed(|query| tracing::info!("TODO: palette_query_changed({query})"));
    ui.on_palette_item_activated(|item| tracing::info!("TODO: palette_item_activated({item})"));
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
            .expect("tokio runtime повинен будуватися без помилок");

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
