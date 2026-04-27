use std::sync::Arc;

use anyhow::Result;
use chrono::{Duration, Utc};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use sqlx::PgPool;
use tokio::runtime::Runtime;
use uuid::Uuid;

use acta::app_ctx::{AppCtx, AppScreen};
use acta::models::company::Company;
use acta::models::{NewTask, TaskPriority, TaskStatus};

use crate::ui;
use crate::ui::documents::load_document_chain;
use crate::{AppWindow, CompanySwitcherItem, NavScreen, PaletteItemData, ShellChrome};

struct InitialUiData {
    dashboard: ui::dashboard::DashboardData,
    documents: ui::documents::DocumentsData,
    counterparties: ui::counterparties::CounterpartiesData,
    payments: ui::payments::PaymentsData,
    tasks: ui::tasks::TasksData,
    reports: ui::reports::ReportsData,
    settings: ui::settings::SettingsData,
}

struct ShellState {
    chrome: ShellChrome,
    company_items: Vec<CompanySwitcherItem>,
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

    let (dashboard, documents, counterparties, payments, tasks, reports, settings) = tokio::join!(
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
        ui::tasks::prepare_tasks_data(ctx.pool(), company_id),
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

fn company_display_name(company: &Company) -> SharedString {
    company
        .short_name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| company.name.clone())
        .into()
}

fn company_switcher_initials(company: &Company) -> SharedString {
    let display_name = company_display_name(company).to_string();
    let mut initials = display_name
        .split(|ch: char| ch.is_whitespace() || ch == '«' || ch == '»' || ch == '"' || ch == '\'')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();

    if initials.is_empty() {
        initials = "A".to_string();
    }

    initials.into()
}

fn company_switcher_subtitle(company: &Company) -> SharedString {
    let mut parts = Vec::new();
    if let Some(edrpou) = company
        .edrpou
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("ЄДРПОУ {edrpou}"));
    }
    if company.is_vat_payer {
        parts.push("ПДВ".to_string());
    }

    if parts.is_empty() {
        "Без додаткових реквізитів".into()
    } else {
        parts.join(" · ").into()
    }
}

async fn load_shell_state(pool: &PgPool, active_company_id: Uuid) -> Result<ShellState> {
    let companies = acta::db::companies::list(pool).await?;
    let company_items = companies
        .iter()
        .map(|company| CompanySwitcherItem {
            id: company.id.to_string().into(),
            name: company_display_name(company),
            subtitle: company_switcher_subtitle(company),
            initials: company_switcher_initials(company),
            badge: if company.id == active_company_id {
                "Активна".into()
            } else {
                SharedString::default()
            },
            active: company.id == active_company_id,
        })
        .collect::<Vec<_>>();

    let company_name = companies
        .iter()
        .find(|company| company.id == active_company_id)
        .map(company_display_name)
        .or_else(|| companies.first().map(company_display_name))
        .unwrap_or_else(|| "Acta".into());

    Ok(ShellState {
        chrome: ShellChrome {
            company_name,
            user_name: "Адміністратор".into(),
            user_initials: "АД".into(),
            user_role: "Адміністратор".into(),
            documents_badge: 0,
            tasks_badge: 0,
        },
        company_items,
    })
}

fn apply_initial_ui_data(ui: &AppWindow, data: InitialUiData) {
    ui::dashboard::apply_dashboard_to_ui(ui, data.dashboard);
    ui::documents::apply_documents_to_ui(ui, data.documents);
    ui::counterparties::apply_counterparties_to_ui(ui, data.counterparties);
    ui::counterparties::apply_counterparty_detail_to_ui(
        ui,
        ui::counterparties::empty_counterparty_detail(),
    );
    ui.set_cp_selected_id("".into());
    ui::payments::apply_payments_to_ui(ui, data.payments);
    ui::tasks::apply_tasks_to_ui(ui, data.tasks);
    ui::reports::apply_reports_to_ui(ui, data.reports);
    ui::settings::apply_settings_to_ui(ui, data.settings);
}

fn apply_shell_state(ui: &AppWindow, shell_state: ShellState) {
    ui.set_shell(shell_state.chrome);
    ui.set_shell_company_items(ModelRc::new(VecModel::from(shell_state.company_items)));
}

pub async fn refresh_all_ui(ui_weak: slint::Weak<AppWindow>, ctx: Arc<AppCtx>) -> Result<()> {
    let company_id = ctx.company_id();
    let refresh_epoch = ctx.refresh_epoch();
    let data = load_initial_ui_data(&ctx).await;
    let shell_state = load_shell_state(ctx.pool(), company_id).await?;

    if is_refresh_stale(&ctx, company_id, refresh_epoch) {
        tracing::debug!("refresh_all_ui: пропускаємо застарілий refresh після switch компанії");
        return Ok(());
    }

    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        apply_initial_ui_data(&ui, data);
        apply_shell_state(&ui, shell_state);
    });

    Ok(())
}

fn is_refresh_stale(ctx: &AppCtx, company_id: Uuid, refresh_epoch: u64) -> bool {
    ctx.company_id() != company_id || ctx.refresh_epoch() != refresh_epoch
}

pub async fn refresh_screen(ui_weak: slint::Weak<AppWindow>, ctx: Arc<AppCtx>, screen: AppScreen) {
    let company_id = ctx.company_id();
    let refresh_epoch = ctx.refresh_epoch();

    match screen {
        AppScreen::Dashboard => {
            let data = ui::dashboard::prepare_dashboard_data(ctx.pool(), company_id).await;
            if is_refresh_stale(&ctx, company_id, refresh_epoch) {
                return;
            }
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
            if is_refresh_stale(&ctx, company_id, refresh_epoch) {
                return;
            }
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
            if is_refresh_stale(&ctx, company_id, refresh_epoch) {
                return;
            }
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui::counterparties::apply_counterparties_to_ui(&ui, data);
            });
        }
        AppScreen::Payments => {
            let data = ui::payments::prepare_payments_data(ctx.pool(), company_id).await;
            if is_refresh_stale(&ctx, company_id, refresh_epoch) {
                return;
            }
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
            if is_refresh_stale(&ctx, company_id, refresh_epoch) {
                return;
            }
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui::reports::apply_reports_to_ui(&ui, data);
            });
        }
        AppScreen::Tasks => {
            let data = ui::tasks::prepare_tasks_data(ctx.pool(), company_id).await;
            if is_refresh_stale(&ctx, company_id, refresh_epoch) {
                return;
            }
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui::tasks::apply_tasks_to_ui(&ui, data);
            });
        }
        AppScreen::Settings => {
            let data = ui::settings::prepare_settings_data(ctx.pool(), company_id).await;
            if is_refresh_stale(&ctx, company_id, refresh_epoch) {
                return;
            }
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
    let (data, shell_state) = rt.block_on(async {
        let data = load_initial_ui_data(ctx).await;
        let shell_state = load_shell_state(ctx.pool(), ctx.company_id()).await?;
        Ok::<_, anyhow::Error>((data, shell_state))
    })?;
    let company_name = shell_state.chrome.company_name.clone();
    // apply_documents_to_ui всередині ініціалізує chain_steps/cp_doc_chains порожніми VecModel-ами.
    apply_initial_ui_data(&ui, data);

    ui.set_shell(crate::ShellChrome {
        company_name,
        user_name: "Адміністратор".into(),
        user_initials: "АД".into(),
        user_role: "Адміністратор".into(),
        documents_badge: 0,
        tasks_badge: 0,
    });

    apply_shell_state(&ui, shell_state);
    Ok(ui)
}

/// Підписує callback-и root shell та feature screens.
pub fn wire_app(ui: &AppWindow, ctx: &Arc<AppCtx>) {
    wire_navigation(ui, ctx);
    wire_company_switcher(ui, ctx);
    ui::documents::wire_document_callbacks(ui, ctx);
    ui::counterparties::wire_counterparty_callbacks(ui, ctx);
    ui::payments::wire_payment_callbacks(ui, ctx);
    ui::tasks::wire_task_callbacks(ui, ctx);
    ui::reports::wire_reports_callbacks(ui, ctx);
    ui::settings::wire_settings_callbacks(ui, ctx);
    wire_inbox_callbacks(ui, ctx);
    wire_palette_callbacks(ui, ctx);
    wire_stub_callbacks(ui, ctx);
}

fn wire_company_switcher(ui: &AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_company_selected({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |company_id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let company_id = company_id.to_string();

            tokio::spawn(async move {
                let Ok(company_id) = Uuid::parse_str(&company_id) else {
                    tracing::warn!("company_switcher: некоректний UUID компанії: {company_id}");
                    return;
                };

                if ctx.company_id() == company_id {
                    return;
                }

                ctx.set_company_id(company_id);

                if let Err(error) = refresh_all_ui(ui_weak, ctx).await {
                    tracing::error!("company_switcher: не вдалося оновити UI: {error}");
                }
            });
        }
    });
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
fn prefixed_uuid(id: &str, prefix: &str) -> Option<Uuid> {
    id.strip_prefix(prefix)
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn nav_screen_from_search_id(id: &str) -> Option<NavScreen> {
    match id {
        "dashboard" => Some(NavScreen::Dashboard),
        "documents" | "acts" | "invoices" | "waybills" => Some(NavScreen::Documents),
        "counterparties" => Some(NavScreen::Counterparties),
        "payments" => Some(NavScreen::Payments),
        "reports" => Some(NavScreen::Reports),
        "tasks" => Some(NavScreen::Tasks),
        "settings" => Some(NavScreen::Settings),
        _ => None,
    }
}

fn palette_item_from_search(item: acta::db::search::SearchResultItem) -> PaletteItemData {
    let payload = if item.action.is_empty() {
        String::new()
    } else {
        format!("{}:{}", item.action, item.id)
    };

    PaletteItemData {
        kind: item.kind.into(),
        title: item.title.into(),
        subtitle: item.subtitle.into(),
        shortcut: item.shortcut.into(),
        payload: payload.into(),
    }
}

async fn create_overdue_act_reminder(ctx: &AppCtx, act_id: Uuid) -> Result<()> {
    let Some((act, _items)) = acta::db::acts::get_by_id(ctx.pool(), act_id).await? else {
        tracing::warn!("inbox_action: act {act_id} не знайдено для нагадування");
        return Ok(());
    };

    let title = format!("Нагадати про оплату акту {}", act.number);
    let already_open = acta::db::tasks::list_by_act(ctx.pool(), act_id)
        .await?
        .into_iter()
        .any(|task| {
            matches!(task.status, TaskStatus::Open | TaskStatus::InProgress) && task.title == title
        });

    if already_open {
        tracing::info!("inbox_action: задача-нагадування для акту {act_id} вже існує");
        return Ok(());
    }

    let reminder_at = Utc::now() + Duration::hours(2);
    let task = NewTask {
        title,
        description: Some("Створено з Inbox на головному екрані.".to_string()),
        priority: TaskPriority::High,
        due_date: Some(reminder_at),
        reminder_at: Some(reminder_at),
        counterparty_id: None,
        act_id: Some(act_id),
    };

    acta::db::tasks::create(ctx.pool(), ctx.company_id(), &task).await?;
    Ok(())
}

fn wire_inbox_callbacks(ui: &AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_inbox_action({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id, kind| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            let kind = kind.to_string();

            tokio::spawn(async move {
                match kind.as_str() {
                    "overdue" => {
                        if let Some(act_id) = prefixed_uuid(&id, "act:") {
                            if let Err(err) = create_overdue_act_reminder(&ctx, act_id).await {
                                tracing::error!(
                                    "inbox_action: не вдалося створити нагадування: {err}"
                                );
                            }
                            refresh_screen(ui_weak.clone(), ctx.clone(), AppScreen::Dashboard)
                                .await;
                            refresh_screen(ui_weak, ctx, AppScreen::Tasks).await;
                        } else {
                            tracing::warn!("inbox_action: некоректний id простроченого акту: {id}");
                        }
                    }
                    "unmatched" => {
                        ctx.set_active_screen(AppScreen::Payments);
                        let _ = ui_weak.upgrade_in_event_loop(|ui| {
                            ui.set_current_screen(NavScreen::Payments);
                        });
                        refresh_screen(ui_weak, ctx, AppScreen::Payments).await;
                    }
                    other => {
                        tracing::info!("inbox_action: дія '{other}' для {id} ще не підтримується");
                    }
                }
            });
        }
    });
}

fn wire_palette_callbacks(ui: &AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_palette_query_changed({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |query| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let query = query.to_string();

            tokio::spawn(async move {
                match acta::db::search::search(ctx.pool(), ctx.company_id(), &query).await {
                    Ok(items) => {
                        let items = items
                            .into_iter()
                            .map(palette_item_from_search)
                            .collect::<Vec<_>>();
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            let model: ModelRc<PaletteItemData> =
                                ModelRc::new(VecModel::from(items));
                            ui.set_palette_items(model);
                        });
                    }
                    Err(error) => {
                        tracing::error!("palette: search failed: {error}");
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_palette_items(ModelRc::new(
                                VecModel::<PaletteItemData>::default(),
                            ));
                        });
                    }
                }
            });
        }
    });

    ui.on_palette_item_activated({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |payload| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let payload = payload.to_string();

            tokio::spawn(async move {
                let Some((action, id)) = payload.split_once(':') else {
                    tracing::warn!("palette: некоректний payload '{payload}'");
                    return;
                };

                match action {
                    "navigate" => {
                        if let Some(screen) = nav_screen_from_search_id(id) {
                            let app_screen = map_nav_screen(screen);
                            ctx.set_active_screen(app_screen);
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                ui.set_current_screen(screen);
                            });
                            refresh_screen(ui_weak, ctx, app_screen).await;
                        }
                    }
                    "open_cp" => {
                        if let Ok(cp_id) = Uuid::parse_str(id) {
                            let company_id = ctx.company_id();
                            let refresh_epoch = ctx.refresh_epoch();
                            ctx.set_active_screen(AppScreen::Counterparties);
                            let data = ui::counterparties::prepare_counterparty_detail(
                                ctx.pool(),
                                company_id,
                                cp_id,
                            )
                            .await;
                            if is_refresh_stale(&ctx, company_id, refresh_epoch) {
                                return;
                            }
                            let id = id.to_string();
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                ui.set_current_screen(NavScreen::Counterparties);
                                ui.set_cp_selected_id(id.into());
                                if let Some(data) = data {
                                    ui::counterparties::apply_counterparty_detail_to_ui(&ui, data);
                                }
                            });
                        }
                    }
                    "open_doc" => {
                        ctx.set_active_screen(AppScreen::Documents);
                        let id = id.to_string();
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_current_screen(NavScreen::Documents);
                            ui.invoke_doc_open(id.into());
                        });
                        refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
                    }
                    "create" => {
                        let screen = if id == "counterparty" {
                            NavScreen::Counterparties
                        } else {
                            NavScreen::Documents
                        };
                        let app_screen = map_nav_screen(screen);
                        ctx.set_active_screen(app_screen);
                        let create_kind = id.to_string();
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_current_screen(screen);
                            if create_kind == "counterparty" {
                                ui.invoke_cp_new();
                            } else {
                                ui.invoke_doc_new();
                            }
                        });
                        refresh_screen(ui_weak, ctx, app_screen).await;
                        tracing::info!("palette: create('{id}') очікує повного create-flow");
                    }
                    other => tracing::warn!("palette: невідома дія '{other}'"),
                }
            });
        }
    });
}

fn wire_stub_callbacks(ui: &AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_doc_chain_load({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                let Some(doc_ref) = crate::ui::documents::parse_document_ref(&id) else {
                    tracing::error!("documents: invalid document ref for chain_load: {id}");
                    return;
                };

                match load_document_chain(ctx.pool(), ctx.company_id(), doc_ref).await {
                    Ok(steps) => {
                        let step_count = steps.len();
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            let chain_steps_model =
                                slint::ModelRc::new(slint::VecModel::from(steps));
                            let documents = ui.get_documents();
                            ui.set_documents(crate::DocumentsViewData {
                                items: documents.items,
                                invoice_items: documents.invoice_items,
                                act_items: documents.act_items,
                                waybill_items: documents.waybill_items,
                                selected_ids: documents.selected_ids,
                                total_count: documents.total_count,
                                page_count: documents.page_count,
                                chain_steps: chain_steps_model,
                                cp_doc_chains: documents.cp_doc_chains,
                            });
                        });

                        tracing::info!("documents: loaded chain for {id} with {step_count} steps");
                    }
                    Err(error) => {
                        tracing::error!("documents: chain_load failed for {id}: {error}");
                    }
                }
            });
        }
    });
    ui.on_doc_chain_create(|doc_type, source_id| {
        tracing::info!("TODO: doc_chain_create(type={doc_type}, source={source_id})");
    });
}
