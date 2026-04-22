use std::sync::Arc;

use anyhow::Result;
use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use tokio::runtime::Runtime;
use uuid::Uuid;

use acta::app_ctx::AppCtx;

use crate::ui;
use crate::{AppWindow, ChainStep, DocChainGroup, NavScreen};

/// Повертає першу компанію або nil UUID, якщо компаній ще немає.
async fn get_first_company_id(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM companies ORDER BY created_at LIMIT 1")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(Uuid::nil)
}

/// Створює tokio runtime для desktop-застосунку.
pub fn build_runtime() -> Result<Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

/// Підключається до БД, застосовує міграції та створює AppCtx.
pub fn init_app_ctx(rt: &Runtime) -> Result<Arc<AppCtx>> {
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
    Ok(Arc::new(AppCtx::new(pool, company_id)))
}

/// Запускає фонові сервіси застосунку.
pub fn spawn_background_tasks(ctx: &Arc<AppCtx>) {
    let pool = Arc::new(ctx.pool().clone());
    tokio::spawn(acta::notifications::reminder_loop(pool));
}

/// Створює вікно та наповнює його стартовими даними.
pub fn build_ui(rt: &Runtime, ctx: &Arc<AppCtx>) -> Result<AppWindow> {
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
    ui.set_cp_doc_chains(ModelRc::new(VecModel::<DocChainGroup>::default()));
    ui.set_doc_chain_steps(ModelRc::new(VecModel::<ChainStep>::default()));

    ui.set_company_name("Acta".into());
    ui.set_user_name("Адміністратор".into());
    ui.set_user_initials("АД".into());

    Ok(ui)
}

/// Підписує callback-и root shell та feature screens.
pub fn wire_app(ui: &AppWindow, ctx: &Arc<AppCtx>) {
    wire_navigation(ui, ctx);
    ui::documents::wire_document_callbacks(ui, ctx);
    ui::counterparties::wire_counterparty_callbacks(ui, ctx);
    ui::payments::wire_payment_callbacks(ui, ctx);
    ui::tasks::wire_task_callbacks(ui, ctx);
    ui::reports::wire_reports_callbacks(ui, ctx);
    ui::settings::wire_settings_callbacks(ui, ctx);
    wire_stub_callbacks(ui);
}

fn wire_navigation(ui: &AppWindow, ctx: &Arc<AppCtx>) {
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
    ui.on_inbox_action(|id, kind| {
        tracing::info!("TODO: inbox_action(doc={id}, kind={kind})");
    });
    ui.on_doc_chain_load({
        let ui_weak = ui.as_weak();
        move |id| {
            tracing::info!("TODO: doc_chain_load({id})");
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.set_doc_chain_steps(ModelRc::new(VecModel::<ChainStep>::default()));
            });
        }
    });
    ui.on_doc_chain_create(|doc_type, source_id| {
        tracing::info!("TODO: doc_chain_create(type={doc_type}, source={source_id})");
    });
    ui.on_palette_query_changed(|query| tracing::info!("TODO: palette_query_changed({query})"));
    ui.on_palette_item_activated(|item| tracing::info!("TODO: palette_item_activated({item})"));
}
