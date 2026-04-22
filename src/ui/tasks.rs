use std::sync::Arc;
use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use acta::app_ctx::AppCtx;
use acta::db;
use acta::models::task::TaskStatus;
use crate::ui::helpers::task_to_item;

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
}

pub async fn prepare_tasks_data(pool: &PgPool) -> TasksData {
    let (open_res, all_res) = tokio::join!(
        db::tasks::list_open(pool),
        db::tasks::list_all(pool),
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
    }
}

pub fn apply_tasks_to_ui(ui: &crate::AppWindow, data: TasksData) {
    let high_count = data
        .open
        .iter()
        .filter(|t| t.priority == crate::Priority::High)
        .count() as i32;

    ui.set_tasks_open_count(data.open.len() as i32);
    ui.set_tasks_done_count(data.done.len() as i32);
    ui.set_tasks_high_count(high_count);
    ui.set_tasks_today_label(today_label());
    ui.set_tasks_open(ModelRc::new(VecModel::from(data.open)));
    ui.set_tasks_done(ModelRc::new(VecModel::from(data.done)));
    ui.set_tasks_all(ModelRc::new(VecModel::from(data.all)));
    ui.set_day_events(ModelRc::new(VecModel::<crate::DayEvent>::default()));
}

fn today_label() -> slint::SharedString {
    use chrono::Datelike;
    let today = chrono::Local::now().naive_local().date();
    let day = today.day();
    let month = match today.month() {
        1 => "Січня",  2 => "Лютого",   3 => "Березня",
        4 => "Квітня", 5 => "Травня",   6 => "Червня",
        7 => "Липня",  8 => "Серпня",   9 => "Вересня",
        10 => "Жовтня", 11 => "Листопада", 12 => "Грудня",
        _ => "",
    };
    let weekday = match today.weekday() {
        chrono::Weekday::Mon => "Понеділок", chrono::Weekday::Tue => "Вівторок",
        chrono::Weekday::Wed => "Середа",    chrono::Weekday::Thu => "Четвер",
        chrono::Weekday::Fri => "П'ятниця",  chrono::Weekday::Sat => "Субота",
        chrono::Weekday::Sun => "Неділя",
    };
    format!("{weekday}, {day} {month}").into()
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
                let data = prepare_tasks_data(ctx.pool()).await;
                let _ = ui_weak.upgrade_in_event_loop(move |ui| apply_tasks_to_ui(&ui, data));
            });
        }
    });

    // Epic 7: no-op → явний tracing
    ui.on_task_filter_changed(|f| tracing::info!("TODO: task_filter_changed: {}", f));
    ui.on_task_new(|| tracing::info!("TODO: створення нового завдання"));
    ui.on_task_more(|id| tracing::info!("TODO: показати більше про завдання {}", id));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_selects_correct_task_list() {
        assert_eq!(filter_for_tab("open"), TaskFilter::Open);
        assert_eq!(filter_for_tab("done"), TaskFilter::Done);
        assert_eq!(filter_for_tab("all"), TaskFilter::All);
        assert_eq!(filter_for_tab("unknown"), TaskFilter::Open);
    }
}
