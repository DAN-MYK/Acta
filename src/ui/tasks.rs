use chrono::{DateTime, Local, NaiveDate, Timelike, Utc};
use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::ui::helpers::task_to_item;
use acta::app_ctx::{AppCtx, AppScreen};
use acta::db;
use acta::models::task::{NewTask, Task, TaskPriority, TaskStatus};

#[cfg(test)]
#[derive(Debug, PartialEq)]
pub enum TaskFilter {
    Open,
    Done,
    All,
}

#[cfg(test)]
pub fn filter_for_tab(tab: &str) -> TaskFilter {
    match tab {
        "done" => TaskFilter::Done,
        "all" => TaskFilter::All,
        _ => TaskFilter::Open,
    }
}

pub struct TasksData {
    pub open: Vec<crate::TaskItem>,
    pub done: Vec<crate::TaskItem>,
    pub all: Vec<crate::TaskItem>,
    pub day_events: Vec<crate::DayEvent>,
}

pub async fn prepare_tasks_data(pool: &PgPool, company_id: Uuid) -> TasksData {
    let (open_res, all_res) = tokio::join!(
        db::tasks::list_open(pool, company_id),
        db::tasks::list_all(pool, company_id),
    );

    let all_tasks = all_res.unwrap_or_default();
    let open_items: Vec<crate::TaskItem> = open_res
        .unwrap_or_default()
        .iter()
        .map(task_to_item)
        .collect();

    let done_items: Vec<crate::TaskItem> = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done || t.status == TaskStatus::Cancelled)
        .map(task_to_item)
        .collect();

    let all_items: Vec<crate::TaskItem> = all_tasks.iter().map(task_to_item).collect();

    TasksData {
        open: open_items,
        done: done_items,
        all: all_items,
        day_events: build_day_events(&all_tasks),
    }
}

pub fn apply_tasks_to_ui(ui: &crate::AppWindow, data: TasksData) {
    let open_count = data.open.len() as i32;
    let done_count = data.done.len() as i32;
    let high_count = data
        .open
        .iter()
        .filter(|t| t.priority == crate::Priority::High)
        .count() as i32;

    ui.set_tasks_screen(crate::TasksViewData {
        open: ModelRc::new(VecModel::from(data.open)),
        done: ModelRc::new(VecModel::from(data.done)),
        all: ModelRc::new(VecModel::from(data.all)),
        day_events: ModelRc::new(VecModel::from(data.day_events)),
        open_count,
        high_count,
        done_count,
        today_label: today_label(),
    });
}

fn parse_task_priority(value: &str) -> TaskPriority {
    match value {
        "low" => TaskPriority::Low,
        "high" => TaskPriority::High,
        "critical" => TaskPriority::Critical,
        _ => TaskPriority::Normal,
    }
}

fn parse_task_status(value: &str) -> Option<TaskStatus> {
    match value {
        "open" => Some(TaskStatus::Open),
        "in_progress" => Some(TaskStatus::InProgress),
        "done" => Some(TaskStatus::Done),
        "cancelled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

fn parse_optional_task_date(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let date = NaiveDate::parse_from_str(value, "%d.%m.%Y")
        .or_else(|_| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .ok()?;

    date.and_hms_opt(9, 0, 0)
        .map(|date_time| DateTime::<Utc>::from_naive_utc_and_offset(date_time, Utc))
}

fn today_label() -> slint::SharedString {
    use chrono::Datelike;
    let today = chrono::Local::now().naive_local().date();
    let day = today.day();
    let month = match today.month() {
        1 => "Січня",
        2 => "Лютого",
        3 => "Березня",
        4 => "Квітня",
        5 => "Травня",
        6 => "Червня",
        7 => "Липня",
        8 => "Серпня",
        9 => "Вересня",
        10 => "Жовтня",
        11 => "Листопада",
        12 => "Грудня",
        _ => "",
    };
    let weekday = match today.weekday() {
        chrono::Weekday::Mon => "Понеділок",
        chrono::Weekday::Tue => "Вівторок",
        chrono::Weekday::Wed => "Середа",
        chrono::Weekday::Thu => "Четвер",
        chrono::Weekday::Fri => "П'ятниця",
        chrono::Weekday::Sat => "Субота",
        chrono::Weekday::Sun => "Неділя",
    };
    format!("{weekday}, {day} {month}").into()
}

fn priority_label(priority: &TaskPriority) -> &'static str {
    match priority {
        TaskPriority::Critical => "Критичний пріоритет",
        TaskPriority::High => "Високий пріоритет",
        TaskPriority::Normal => "Звичайний пріоритет",
        TaskPriority::Low => "Низький пріоритет",
    }
}

fn build_day_events(tasks: &[Task]) -> Vec<crate::DayEvent> {
    let today = Local::now().date_naive();
    let mut timed_events: Vec<(i32, crate::DayEvent)> = Vec::new();

    for task in tasks {
        let reminder_local = task.reminder_at.map(|date| date.with_timezone(&Local));
        if let Some(reminder) = reminder_local {
            if reminder.date_naive() == today {
                timed_events.push((
                    reminder.hour() as i32,
                    crate::DayEvent {
                        hour: reminder.hour() as i32,
                        title: task.title.clone().into(),
                        subtitle: format!("Нагадування • {}", priority_label(&task.priority))
                            .into(),
                        is_primary: matches!(
                            task.priority,
                            TaskPriority::High | TaskPriority::Critical
                        ),
                    },
                ));
            }
        }

        let due_local = task.due_date.map(|date| date.with_timezone(&Local));
        if let Some(due) = due_local {
            if due.date_naive() == today {
                let reminder_same_hour = reminder_local
                    .map(|reminder| reminder.date_naive() == today && reminder.hour() == due.hour())
                    .unwrap_or(false);

                if !reminder_same_hour {
                    timed_events.push((
                        due.hour() as i32,
                        crate::DayEvent {
                            hour: due.hour() as i32,
                            title: task.title.clone().into(),
                            subtitle: "Дедлайн на сьогодні".into(),
                            is_primary: matches!(
                                task.priority,
                                TaskPriority::High | TaskPriority::Critical
                            ),
                        },
                    ));
                }
            }
        }
    }

    timed_events.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.title.as_str().cmp(b.1.title.as_str()))
    });

    let mut events = Vec::new();
    for hour in 8..=18 {
        let mut has_entries = false;
        for (_, event) in timed_events
            .iter()
            .filter(|(event_hour, _)| *event_hour == hour)
        {
            has_entries = true;
            events.push(event.clone());
        }

        if !has_entries {
            events.push(crate::DayEvent {
                hour,
                title: "".into(),
                subtitle: "".into(),
                is_primary: false,
            });
        }
    }

    events
}

/// Підписує всі task callbacks — виконує завдання (збереження, видалення, і т.д.).
/// Використовує AppCtx як єдине джерело стану (Epic 4).
pub fn wire_task_callbacks(ui: &crate::AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_task_toggled({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id, done| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id_str = id.to_string();
            tokio::spawn(async move {
                if let Ok(uuid) = Uuid::parse_str(&id_str) {
                    let new_status = if done {
                        TaskStatus::Done
                    } else {
                        TaskStatus::Open
                    };
                    let _ = db::tasks::set_status(ctx.pool(), uuid, new_status).await;
                }
                crate::bootstrap::refresh_current_screen(ui_weak, ctx).await;
            });
        }
    });

    // Epic 7: Фільтрація завдань — чисто клієнтська сторона Slint.
    // Компонент Tasks має три моделі (open/done/all), обирає потрібну за поточною вкладкою.
    // Rust-side оновлення state тут не потрібне — DB запит однаково повертає всі три групи.
    ui.on_task_filter_changed(|filter| {
        tracing::debug!(
            "task_filter_changed: {} — вибір списку відбувається у Slint-компоненті",
            filter
        );
    });

    ui.on_task_new({
        move || {
            // show-task-form is managed internally by the Tasks Slint component
        }
    });

    ui.on_task_save({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |title, priority, due_date, reminder_at| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let title = title.trim().to_string();
            let priority = priority.to_string();
            let due_date = due_date.to_string();
            let reminder_at = reminder_at.to_string();

            tokio::spawn(async move {
                if title.is_empty() {
                    tracing::warn!("task_save: порожню назву задачі пропущено");
                    return;
                }

                let task = NewTask {
                    title,
                    description: None,
                    priority: parse_task_priority(&priority),
                    due_date: parse_optional_task_date(&due_date),
                    reminder_at: parse_optional_task_date(&reminder_at),
                    counterparty_id: None,
                    act_id: None,
                };

                if let Err(error) = db::tasks::create(ctx.pool(), ctx.company_id(), &task).await {
                    tracing::error!("task_save: не вдалося створити задачу: {error}");
                    return;
                }

                crate::bootstrap::refresh_screen(ui_weak.clone(), ctx.clone(), AppScreen::Tasks)
                    .await;
                crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Dashboard).await;
            });
        }
    });

    ui.on_task_more(|id| {
        tracing::info!("task_more: відкрито деталі задачі {id}");
    });

    ui.on_task_status_set({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id, status| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id_str = id.to_string();
            let status_str = status.to_string();

            tokio::spawn(async move {
                let Ok(task_id) = Uuid::parse_str(&id_str) else {
                    tracing::warn!("task_status_set: некоректний id задачі {id_str}");
                    return;
                };

                let Some(task_status) = parse_task_status(&status_str) else {
                    tracing::warn!("task_status_set: невідомий статус {status_str}");
                    return;
                };

                match db::tasks::set_status(ctx.pool(), task_id, task_status).await {
                    Ok(Some(_)) => {
                        crate::bootstrap::refresh_screen(
                            ui_weak.clone(),
                            ctx.clone(),
                            AppScreen::Tasks,
                        )
                        .await;
                        crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Dashboard).await;
                    }
                    Ok(None) => {
                        tracing::warn!("task_status_set: задачу {task_id} не знайдено");
                    }
                    Err(error) => {
                        tracing::error!("task_status_set: не вдалося оновити статус: {error}");
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn filter_selects_correct_task_list() {
        assert_eq!(filter_for_tab("open"), TaskFilter::Open);
        assert_eq!(filter_for_tab("done"), TaskFilter::Done);
        assert_eq!(filter_for_tab("all"), TaskFilter::All);
        assert_eq!(filter_for_tab("unknown"), TaskFilter::Open);
    }

    #[test]
    fn task_priority_form_value_maps_to_model() {
        assert_eq!(parse_task_priority("low"), TaskPriority::Low);
        assert_eq!(parse_task_priority("normal"), TaskPriority::Normal);
        assert_eq!(parse_task_priority("high"), TaskPriority::High);
        assert_eq!(parse_task_priority("critical"), TaskPriority::Critical);
        assert_eq!(parse_task_priority("unknown"), TaskPriority::Normal);
    }

    #[test]
    fn task_status_parser_maps_known_values() {
        assert_eq!(parse_task_status("open"), Some(TaskStatus::Open));
        assert_eq!(parse_task_status("in_progress"), Some(TaskStatus::InProgress));
        assert_eq!(parse_task_status("done"), Some(TaskStatus::Done));
        assert_eq!(parse_task_status("cancelled"), Some(TaskStatus::Cancelled));
        assert_eq!(parse_task_status("unknown"), None);
    }

    #[test]
    fn task_date_parser_accepts_ua_and_iso_dates() {
        let ua = parse_optional_task_date("24.04.2026").expect("ua date");
        let iso = parse_optional_task_date("2026-04-24").expect("iso date");
        let expected = chrono::NaiveDate::from_ymd_opt(2026, 4, 24).unwrap();

        assert_eq!(ua.date_naive(), expected);
        assert_eq!(iso.date_naive(), expected);
        assert!(parse_optional_task_date("").is_none());
        assert!(parse_optional_task_date("не дата").is_none());
    }
    #[test]
    fn build_day_events_includes_today_slots_and_task_entries() {
        use acta::models::task::{Task, TaskPriority, TaskStatus};
        use chrono::{Duration, TimeZone};

        let now = Local::now();
        let today = now.date_naive();
        let due = Local
            .with_ymd_and_hms(today.year(), today.month(), today.day(), 11, 0, 0)
            .unwrap()
            .with_timezone(&Utc);

        let task = Task {
            id: Uuid::nil(),
            title: "Подзвонити клієнту".to_string(),
            description: None,
            status: TaskStatus::Open,
            priority: TaskPriority::High,
            due_date: Some(due),
            reminder_at: None,
            counterparty_id: None,
            act_id: None,
            created_at: Utc::now() - Duration::hours(1),
            updated_at: Utc::now(),
        };

        let events = build_day_events(&[task]);
        assert!(events
            .iter()
            .any(|event| event.hour == 11 && event.title.as_str() == "Подзвонити клієнту"));
        assert!(events
            .iter()
            .any(|event| event.hour == 8 && event.title.as_str().is_empty()));
    }
}
