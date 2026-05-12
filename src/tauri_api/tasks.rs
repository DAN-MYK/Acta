use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app_ctx::AppCtx;
use crate::db;
use crate::models::task::{NewTask, Task, TaskPriority, TaskStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskItemDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub status_label: String,
    pub priority: String,
    pub priority_label: String,
    pub due_date: String,
    pub reminder_at: String,
    pub link_kind: String,
    pub link_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskDraftFormDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub due_date: String,
    pub reminder_at: String,
    pub status: String,
    pub counterparty_id: String,
    pub act_id: String,
    pub link_kind: String,
    pub link_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TasksScreenDto {
    pub items: Vec<TaskItemDto>,
    pub open_count: i32,
    pub done_count: i32,
    pub high_count: i32,
    pub today_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskEditorDto {
    pub title: String,
    pub form: TaskDraftFormDto,
    pub show_editor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TasksListRequest {
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskSaveRequest {
    pub form: TaskDraftFormDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskSaveResultDto {
    pub ok: bool,
    pub saved_id: String,
    pub message: String,
    pub updated_list: TasksScreenDto,
    pub updated_editor: Option<TaskEditorDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskMutationResultDto {
    pub ok: bool,
    pub task_id: String,
    pub message: String,
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_uuid_field(value: &str, field: &str) -> Result<Option<Uuid>> {
    let Some(value) = optional_string(value) else {
        return Ok(None);
    };

    Uuid::parse_str(&value)
        .with_context(|| format!("Некоректний ідентифікатор у полі {field}: {value}"))
        .map(Some)
}

fn parse_priority(value: &str) -> TaskPriority {
    match value {
        "low" => TaskPriority::Low,
        "high" => TaskPriority::High,
        "critical" => TaskPriority::Critical,
        _ => TaskPriority::Normal,
    }
}

fn parse_status(value: &str) -> TaskStatus {
    match value {
        "in_progress" => TaskStatus::InProgress,
        "done" => TaskStatus::Done,
        "cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Open,
    }
}

fn format_due_date(value: Option<DateTime<Utc>>) -> String {
    value
        .map(|date| date.with_timezone(&Local).format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn format_reminder_at(value: Option<DateTime<Utc>>) -> String {
    value
        .map(|date| {
            date.with_timezone(&Local)
                .format("%Y-%m-%dT%H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

fn parse_local_datetime(naive: NaiveDateTime, field: &str) -> Result<DateTime<Utc>> {
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(value) => Ok(value.with_timezone(&Utc)),
        LocalResult::Ambiguous(value, _) => Ok(value.with_timezone(&Utc)),
        LocalResult::None => Err(anyhow!("Не вдалося розпізнати дату/час у полі {field}")),
    }
}

fn parse_due_date(value: &str) -> Result<Option<DateTime<Utc>>> {
    let Some(value) = optional_string(value) else {
        return Ok(None);
    };

    let date = NaiveDate::parse_from_str(&value, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(&value, "%d.%m.%Y"))
        .with_context(|| format!("Некоректна дата дедлайну: {value}"))?;
    let naive = date
        .and_hms_opt(9, 0, 0)
        .ok_or_else(|| anyhow!("Не вдалося побудувати дату дедлайну"))?;

    parse_local_datetime(naive, "Дедлайн").map(Some)
}

fn parse_reminder_at(value: &str) -> Result<Option<DateTime<Utc>>> {
    let Some(value) = optional_string(value) else {
        return Ok(None);
    };

    let parsed = NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M"))
        .with_context(|| format!("Некоректна дата нагадування: {value}"))?;

    parse_local_datetime(parsed, "Нагадування").map(Some)
}

pub(crate) async fn resolve_link_label(ctx: &AppCtx, task: &Task) -> Result<(String, String)> {
    if let Some(act_id) = task.act_id {
        if let Some((act, _)) =
            db::acts::get_by_id_scoped(ctx.pool(), ctx.company_id(), act_id).await?
        {
            return Ok(("act".to_string(), format!("Акт {}", act.number)));
        }
    }

    if let Some(counterparty_id) = task.counterparty_id {
        if let Some(counterparty) =
            db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id).await?
        {
            return Ok(("counterparty".to_string(), counterparty.name));
        }
    }

    Ok((String::new(), String::new()))
}

async fn task_to_item(ctx: &AppCtx, task: &Task) -> Result<TaskItemDto> {
    let (link_kind, link_label) = resolve_link_label(ctx, task).await?;

    Ok(TaskItemDto {
        id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone().unwrap_or_default(),
        status: task.status.as_str().to_string(),
        status_label: task.status.label().to_string(),
        priority: task.priority.as_str().to_string(),
        priority_label: task.priority.label().to_string(),
        due_date: format_due_date(task.due_date),
        reminder_at: format_reminder_at(task.reminder_at),
        link_kind,
        link_label,
    })
}

async fn load_tasks_screen(ctx: &AppCtx, query: Option<&str>) -> Result<TasksScreenDto> {
    let query = query.map(str::trim).unwrap_or_default();
    let tasks = db::tasks::list_all(ctx.pool(), ctx.company_id(), query).await?;
    let today = Local::now().date_naive();

    let open_count = tasks
        .iter()
        .filter(|task| matches!(task.status, TaskStatus::Open | TaskStatus::InProgress))
        .count() as i32;
    let done_count = tasks
        .iter()
        .filter(|task| matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled))
        .count() as i32;
    let high_count = tasks
        .iter()
        .filter(|task| matches!(task.priority, TaskPriority::High | TaskPriority::Critical))
        .count() as i32;
    let today_count = tasks
        .iter()
        .filter(|task| {
            task.due_date
                .map(|date| date.with_timezone(&Local).date_naive() == today)
                .unwrap_or(false)
                || task
                    .reminder_at
                    .map(|date| date.with_timezone(&Local).date_naive() == today)
                    .unwrap_or(false)
        })
        .count() as i32;

    let mut items = Vec::with_capacity(tasks.len());
    for task in &tasks {
        items.push(task_to_item(ctx, task).await?);
    }

    Ok(TasksScreenDto {
        items,
        open_count,
        done_count,
        high_count,
        today_count,
    })
}

fn empty_task_form() -> TaskDraftFormDto {
    TaskDraftFormDto {
        id: String::new(),
        title: String::new(),
        description: String::new(),
        priority: "normal".to_string(),
        due_date: String::new(),
        reminder_at: String::new(),
        status: "open".to_string(),
        counterparty_id: String::new(),
        act_id: String::new(),
        link_kind: String::new(),
        link_label: String::new(),
    }
}

async fn editor_for_task(ctx: &AppCtx, task: &Task) -> Result<TaskEditorDto> {
    let (link_kind, link_label) = resolve_link_label(ctx, task).await?;

    Ok(TaskEditorDto {
        title: "Редагування завдання".to_string(),
        form: TaskDraftFormDto {
            id: task.id.to_string(),
            title: task.title.clone(),
            description: task.description.clone().unwrap_or_default(),
            priority: task.priority.as_str().to_string(),
            due_date: format_due_date(task.due_date),
            reminder_at: format_reminder_at(task.reminder_at),
            status: task.status.as_str().to_string(),
            counterparty_id: task
                .counterparty_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            act_id: task
                .act_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            link_kind,
            link_label,
        },
        show_editor: true,
    })
}

fn new_task_payload(form: &TaskDraftFormDto) -> Result<NewTask> {
    let title = form.title.trim();
    if title.is_empty() {
        return Err(anyhow!("Назва завдання є обов'язковою"));
    }

    Ok(NewTask {
        title: title.to_string(),
        description: optional_string(&form.description),
        priority: parse_priority(&form.priority),
        due_date: parse_due_date(&form.due_date)?,
        reminder_at: parse_reminder_at(&form.reminder_at)?,
        counterparty_id: parse_uuid_field(&form.counterparty_id, "Контрагент")?,
        act_id: parse_uuid_field(&form.act_id, "Акт")?,
    })
}

pub async fn tasks_list(ctx: &AppCtx, request: TasksListRequest) -> Result<TasksScreenDto> {
    load_tasks_screen(ctx, request.query.as_deref()).await
}

pub async fn task_open_editor(ctx: &AppCtx, task_id: Option<String>) -> Result<TaskEditorDto> {
    let Some(task_id) = task_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(TaskEditorDto {
            title: "Нове завдання".to_string(),
            form: empty_task_form(),
            show_editor: true,
        });
    };

    let task_id = Uuid::parse_str(&task_id)
        .with_context(|| format!("Некоректний ідентифікатор завдання: {task_id}"))?;
    let task = db::tasks::get_by_id_scoped(ctx.pool(), ctx.company_id(), task_id)
        .await?
        .ok_or_else(|| anyhow!("Завдання не знайдено"))?;

    editor_for_task(ctx, &task).await
}

pub async fn task_save(ctx: &AppCtx, request: TaskSaveRequest) -> Result<TaskSaveResultDto> {
    let payload = new_task_payload(&request.form)?;
    let desired_status = parse_status(&request.form.status);
    let maybe_id = optional_string(&request.form.id);

    let saved_id = if let Some(task_id) = maybe_id {
        let task_id = Uuid::parse_str(&task_id)
            .with_context(|| format!("Некоректний ідентифікатор завдання: {task_id}"))?;
        db::tasks::get_by_id_scoped(ctx.pool(), ctx.company_id(), task_id)
            .await?
            .ok_or_else(|| anyhow!("Завдання не знайдено"))?;

        let updated = db::tasks::update_scoped(ctx.pool(), ctx.company_id(), task_id, &payload)
            .await?
            .ok_or_else(|| anyhow!("Завдання не знайдено"))?;
        if updated.status != desired_status {
            db::tasks::set_status_scoped(ctx.pool(), ctx.company_id(), task_id, desired_status)
                .await?
                .ok_or_else(|| anyhow!("Завдання не знайдено"))?;
        }
        task_id
    } else {
        let created = db::tasks::create(ctx.pool(), ctx.company_id(), &payload).await?;
        if !matches!(desired_status, TaskStatus::Open) {
            db::tasks::set_status_scoped(ctx.pool(), ctx.company_id(), created.id, desired_status)
                .await?
                .ok_or_else(|| anyhow!("Завдання не знайдено"))?;
        }
        created.id
    };

    let updated_list = load_tasks_screen(ctx, None).await?;
    let updated_editor = Some(task_open_editor(ctx, Some(saved_id.to_string())).await?);

    Ok(TaskSaveResultDto {
        ok: true,
        saved_id: saved_id.to_string(),
        message: "Завдання збережено".to_string(),
        updated_list,
        updated_editor,
    })
}

pub async fn task_delete(ctx: &AppCtx, task_id: String) -> Result<TaskMutationResultDto> {
    let task_id = Uuid::parse_str(&task_id)
        .with_context(|| format!("Некоректний ідентифікатор завдання: {task_id}"))?;
    let deleted = db::tasks::delete_scoped(ctx.pool(), ctx.company_id(), task_id).await?;
    if !deleted {
        return Err(anyhow!("Завдання не знайдено"));
    }

    Ok(TaskMutationResultDto {
        ok: true,
        task_id: task_id.to_string(),
        message: "Завдання видалено".to_string(),
    })
}

pub async fn task_set_status(
    ctx: &AppCtx,
    task_id: String,
    status: String,
) -> Result<TaskMutationResultDto> {
    let task_id = Uuid::parse_str(&task_id)
        .with_context(|| format!("Некоректний ідентифікатор завдання: {task_id}"))?;
    let status = parse_status(&status);
    db::tasks::set_status_scoped(ctx.pool(), ctx.company_id(), task_id, status)
        .await?
        .ok_or_else(|| anyhow!("Завдання не знайдено"))?;

    Ok(TaskMutationResultDto {
        ok: true,
        task_id: task_id.to_string(),
        message: "Статус завдання оновлено".to_string(),
    })
}
