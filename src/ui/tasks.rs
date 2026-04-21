use std::sync::Arc;

use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use acta::db;
use acta::models::task::TaskStatus;
use crate::ui::helpers::task_to_item;

#[derive(Debug, PartialEq)]
pub enum TaskFilter {
    Open,
    Done,
    All,
}

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
    ui.set_tasks_open_count(data.open.len() as i32);
    ui.set_tasks_done_count(data.done.len() as i32);
    ui.set_tasks_open(ModelRc::new(VecModel::from(data.open)));
    ui.set_tasks_done(ModelRc::new(VecModel::from(data.done)));
    ui.set_tasks_all(ModelRc::new(VecModel::from(data.all)));
}

pub fn wire_task_callbacks(ui: &crate::AppWindow, pool: &Arc<PgPool>) {
    ui.on_task_toggled({
        let pool = pool.clone();
        let ui_weak = ui.as_weak();
        move |id, done| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let id_str = id.to_string();
            tokio::spawn(async move {
                if let Ok(uuid) = Uuid::parse_str(&id_str) {
                    let new_status = if done {
                        TaskStatus::Done
                    } else {
                        TaskStatus::Open
                    };
                    let _ = db::tasks::set_status(&pool, uuid, new_status).await;
                }
                let data = prepare_tasks_data(&pool).await;
                let _ = ui_weak.upgrade_in_event_loop(move |ui| apply_tasks_to_ui(&ui, data));
            });
        }
    });

    ui.on_task_filter_changed(|_| {});
    ui.on_task_new(|| {});
    ui.on_task_more(|_| {});
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
