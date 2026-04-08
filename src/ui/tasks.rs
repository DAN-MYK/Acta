// ui/tasks.rs — колбеки та дані для сторінки Задачі.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

use crate::{
    app_ctx::AppCtx,
    ui::helpers::*,
    MainWindow, TaskRow,
};
use acta::{db, models::{NewTask, TaskStatus}};

// ═══════════════════════════════════════════════════════════════════════════════
// ── Проміжні дані (Send-safe) ──────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct TasksUiData {
    pub task_rows: Vec<TaskRow>,
}

pub async fn prepare_tasks_data(
    pool: &sqlx::PgPool,
    query: String,
) -> Result<TasksUiData> {
    let tasks = db::tasks::list_open(pool).await?;
    let filtered: Vec<acta::models::Task> = tasks
        .into_iter()
        .filter(|task| task_matches_query(task, normalized_query(&query)))
        .collect();
    Ok(TasksUiData { task_rows: to_task_rows(&filtered) })
}

pub fn apply_tasks_to_ui(ui: &MainWindow, d: TasksUiData, close_form: bool) {
    ui.set_task_rows(ModelRc::new(VecModel::from(d.task_rows)));
    ui.set_tasks_loading(false);
    if close_form {
        ui.set_show_task_form(false);
        ui.set_current_page(ui.get_task_form_return_page());
    }
}

pub async fn reload_tasks(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    query: String,
    close_form: bool,
) -> Result<()> {
    let d = prepare_tasks_data(pool, query).await?;
    ui_weak
        .upgrade_in_event_loop(move |ui| apply_tasks_to_ui(&ui, d, close_form))
        .map_err(anyhow::Error::from)
}

pub async fn reload_act_tasks(
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

// ═══════════════════════════════════════════════════════════════════════════════
// ── spawn_save_task ────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn spawn_save_task(
    pool: sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    task_state: Arc<std::sync::Mutex<TaskListState>>,
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
                        .warn_if_terminated();
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

// ═══════════════════════════════════════════════════════════════════════════════
// ── setup — реєстрація колбеків ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn setup(ui: &MainWindow, ctx: Arc<AppCtx>) {
    let task_state = ctx.task_state.clone();

    // ── Пошук задач ───────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = task_state.clone();
    let search_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>> = Arc::new(Mutex::new(None));
    ui.on_task_search_changed(move |query| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let query_str = {
            let mut s = state.lock().unwrap();
            s.query = query.to_string();
            s.query.clone()
        };
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_tasks_loading(true);
        }
        let handle = tokio::spawn(async move {
            if let Err(e) = reload_tasks(&pool, ui_handle, query_str, false).await {
                tracing::error!("Помилка пошуку задач: {e}");
            }
        });
        if let Some(old) = search_task.lock().unwrap().replace(handle) {
            old.abort();
        }
    });

    ui.on_task_selected(|id| {
        tracing::debug!("Вибрано задачу: {id}");
    });

    // ── Створити задачу ───────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_task_create_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
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

    // ── Редагувати задачу ─────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    ui.on_task_edit_clicked(move |task_id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
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
                        .warn_if_terminated();
                }
                Ok(None) => tracing::warn!("Задачу {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка завантаження задачі: {e}"),
            }
        });
    });

    // ── Перемкнути статус задачі ──────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = task_state.clone();
    ui.on_task_toggle_status_clicked(move |task_id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let id_str = task_id.to_string();
        let task_state = state.clone();
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

    // ── Видалити задачу ───────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = task_state.clone();
    ui.on_task_delete_clicked(move |task_id| {
        let pool = pool.clone();
        let ui_handle = ui_weak.clone();
        let id_str = task_id.to_string();
        let task_state = state.clone();
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

    // ── Зберегти задачу ───────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = task_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_task_form_save(move |title, description, priority_idx, due_str, reminder_str| {
        let company_id = *company_id_arc.lock().unwrap();
        let act_id = ui_weak
            .upgrade()
            .map(|ui| ui.get_task_form_act_id().to_string())
            .unwrap_or_default();
        spawn_save_task(
            pool.clone(),
            ui_weak.clone(),
            state.clone(),
            company_id,
            None,
            title.to_string(),
            description.to_string(),
            priority_idx,
            due_str.to_string(),
            reminder_str.to_string(),
            act_id,
        );
    });

    // ── Оновити задачу ────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let state = task_state.clone();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_task_form_update(move |title, description, priority_idx, due_str, reminder_str| {
        let company_id = *company_id_arc.lock().unwrap();
        let edit_id = ui_weak
            .upgrade()
            .map(|ui| ui.get_task_form_edit_id().to_string())
            .unwrap_or_default();
        let act_id = ui_weak
            .upgrade()
            .map(|ui| ui.get_task_form_act_id().to_string())
            .unwrap_or_default();
        spawn_save_task(
            pool.clone(),
            ui_weak.clone(),
            state.clone(),
            company_id,
            Some(edit_id),
            title.to_string(),
            description.to_string(),
            priority_idx,
            due_str.to_string(),
            reminder_str.to_string(),
            act_id,
        );
    });

    // ── Скасувати форму задачі ────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_task_form_cancel(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_show_task_form(false);
            ui.set_current_page(ui.get_task_form_return_page());
        }
    });
}
