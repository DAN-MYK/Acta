use std::sync::Arc;

use anyhow::Result;
use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use tokio::runtime::Runtime;
use uuid::Uuid;

use acta::app_ctx::{AppCtx, AppScreen};

use crate::ui;
use crate::{AppWindow, ChainStep, NavScreen};

struct InitialUiData {
    dashboard: ui::dashboard::DashboardData,
    documents: ui::documents::DocumentsData,
    counterparties: ui::counterparties::CounterpartiesData,
    payments: ui::payments::PaymentsData,
    tasks: ui::tasks::TasksData,
    reports: ui::reports::ReportsData,
    settings: ui::settings::SettingsData,
}

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

fn map_nav_screen(screen: NavScreen) -> AppScreen {
    match screen {
        NavScreen::Dashboard => AppScreen::Dashboard,
        NavScreen::Documents => AppScreen::Documents,
        NavScreen::Counterparties => AppScreen::Counterparties,
        NavScreen::Payments => AppScreen::Payments,
        NavScreen::Reports => AppScreen::Reports,
        NavScreen::Tasks => AppScreen::Tasks,
        NavScreen::Settings => AppScreen::Settings,
    }
}

async fn load_initial_ui_data(ctx: &AppCtx) -> InitialUiData {
    let company_id = ctx.company_id();
    let documents_state = ctx.documents_state_snapshot();
    let counterparty_state = ctx.counterparty_state_snapshot();
    let reports_state = ctx.reports_state_snapshot();

    let (
        dashboard,
        documents,
        counterparties,
        payments,
        tasks,
        reports,
        settings,
    ) = tokio::join!(
        ui::dashboard::prepare_dashboard_data(ctx.pool(), company_id),
        ui::documents::prepare_documents_data(
            ctx.pool(),
            company_id,
            if documents_state.query.is_empty() {
                None
            } else {
                Some(documents_state.query.as_str())
            },
            if documents_state.tab == "all" {
                None
            } else {
                Some(documents_state.tab.as_str())
            },
        ),
        ui::counterparties::prepare_counterparties_data(
            ctx.pool(),
            company_id,
            if counterparty_state.query.is_empty() {
                None
            } else {
                Some(counterparty_state.query.as_str())
            },
        ),
        ui::payments::prepare_payments_data(ctx.pool(), company_id),
        ui::tasks::prepare_tasks_data(ctx.pool()),
        ui::reports::prepare_reports_data(
            ctx.pool(),
            company_id,
            reports_state.period,
            if reports_state.drill_category.is_empty() {
                None
            } else {
                Some(reports_state.drill_category.as_str())
            },
        ),
        ui::settings::prepare_settings_data(ctx.pool(), company_id),
    );

    InitialUiData {
        dashboard,
        documents,
        counterparties,
        payments,
        tasks,
        reports,
        settings,
    }
}

fn apply_initial_ui_data(ui: &AppWindow, data: InitialUiData) {
    ui::dashboard::apply_dashboard_to_ui(ui, data.dashboard);
    ui::documents::apply_documents_to_ui(ui, data.documents);
    ui::counterparties::apply_counterparties_to_ui(ui, data.counterparties);
    ui::payments::apply_payments_to_ui(ui, data.payments);
    ui::tasks::apply_tasks_to_ui(ui, data.tasks);
    ui::reports::apply_reports_to_ui(ui, data.reports);
    ui::settings::apply_settings_to_ui(ui, data.settings);
}

pub async fn refresh_screen(ui_weak: slint::Weak<AppWindow>, ctx: Arc<AppCtx>, screen: AppScreen) {
    let company_id = ctx.company_id();

    match screen {
        AppScreen::Dashboard => {
            let data = ui::dashboard::prepare_dashboard_data(ctx.pool(), company_id).await;
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui::dashboard::apply_dashboard_to_ui(&ui, data);
            });
        }
        AppScreen::Documents => {
            let state = ctx.documents_state_snapshot();
            let data = ui::documents::prepare_documents_data(
                ctx.pool(),
                company_id,
                if state.query.is_empty() {
                    None
                } else {
                    Some(state.query.as_str())
                },
                if state.tab == "all" {
                    None
                } else {
                    Some(state.tab.as_str())
                },
            )
            .await;
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui::documents::apply_documents_to_ui(&ui, data);
            });
        }
        AppScreen::Counterparties => {
            let state = ctx.counterparty_state_snapshot();
            let data = ui::counterparties::prepare_counterparties_data(
                ctx.pool(),
                company_id,
                if state.query.is_empty() {
                    None
                } else {
                    Some(state.query.as_str())
                },
            )
            .await;
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui::counterparties::apply_counterparties_to_ui(&ui, data);
            });
        }
        AppScreen::Payments => {
            let data = ui::payments::prepare_payments_data(ctx.pool(), company_id).await;
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui::payments::apply_payments_to_ui(&ui, data);
            });
        }
        AppScreen::Reports => {
            let state = ctx.reports_state_snapshot();
            let data = ui::reports::prepare_reports_data(
                ctx.pool(),
                company_id,
                state.period,
                if state.drill_category.is_empty() {
                    None
                } else {
                    Some(state.drill_category.as_str())
                },
            )
            .await;
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui::reports::apply_reports_to_ui(&ui, data);
            });
        }
        AppScreen::Tasks => {
            let data = ui::tasks::prepare_tasks_data(ctx.pool()).await;
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui::tasks::apply_tasks_to_ui(&ui, data);
            });
        }
        AppScreen::Settings => {
            let data = ui::settings::prepare_settings_data(ctx.pool(), company_id).await;
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui::settings::apply_settings_to_ui(&ui, data);
            });
        }
    }
}

pub async fn refresh_current_screen(ui_weak: slint::Weak<AppWindow>, ctx: Arc<AppCtx>) {
    let screen = ctx.active_screen();
    refresh_screen(ui_weak, ctx, screen).await;
}

pub fn spawn_refresh_screen(ui_weak: slint::Weak<AppWindow>, ctx: Arc<AppCtx>, screen: AppScreen) {
    tokio::spawn(async move {
        refresh_screen(ui_weak, ctx, screen).await;
    });
}

/// Створює вікно та наповнює його стартовими даними.
pub fn build_ui(rt: &Runtime, ctx: &Arc<AppCtx>) -> Result<AppWindow> {
    let ui = AppWindow::new()?;
    let data = rt.block_on(load_initial_ui_data(ctx));
    // apply_documents_to_ui всередині ініціалізує chain_steps/cp_doc_chains порожніми VecModel-ами.
    apply_initial_ui_data(&ui, data);

    ui.set_shell(crate::ShellChrome {
        company_name: "Acta".into(),
        user_name: "Адміністратор".into(),
        user_initials: "АД".into(),
        user_role: "Адміністратор".into(),
        documents_badge: 0,
        tasks_badge: 0,
    });

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
            let app_screen = map_nav_screen(screen);
            ctx.set_active_screen(app_screen);
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                refresh_current_screen(ui_weak, ctx).await;
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
                let documents = ui.get_documents();
                ui.set_documents(crate::DocumentsViewData {
                    items: documents.items,
                    selected_ids: documents.selected_ids,
                    total_count: documents.total_count,
                    page_count: documents.page_count,
                    chain_steps: ModelRc::new(VecModel::<ChainStep>::default()),
                    cp_doc_chains: documents.cp_doc_chains,
                });
            });
        }
    });
    ui.on_doc_chain_create(|doc_type, source_id| {
        tracing::info!("TODO: doc_chain_create(type={doc_type}, source={source_id})");
    });
    ui.on_palette_query_changed(|query| tracing::info!("TODO: palette_query_changed({query})"));
    ui.on_palette_item_activated(|item| tracing::info!("TODO: palette_item_activated({item})"));
}
