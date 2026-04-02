use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "task_status", rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    InProgress,
    Done,
    Cancelled,
}

#[allow(dead_code)]
impl TaskStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Open => "Відкрите",
            Self::InProgress => "В роботі",
            Self::Done => "Виконано",
            Self::Cancelled => "Скасовано",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "task_priority", rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[allow(dead_code)]
impl TaskPriority {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Low => "Низький",
            Self::Normal => "Звичайний",
            Self::High => "Високий",
            Self::Critical => "Критичний",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Utc>>,
    pub reminder_at: Option<DateTime<Utc>>,
    pub counterparty_id: Option<Uuid>,
    pub act_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewTask {
    pub title: String,
    pub description: Option<String>,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Utc>>,
    pub reminder_at: Option<DateTime<Utc>>,
    pub counterparty_id: Option<Uuid>,
    pub act_id: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::{NewTask, TaskPriority, TaskStatus};

    #[test]
    fn task_status_labels_are_ukrainian() {
        assert_eq!(TaskStatus::Open.label(), "Відкрите");
        assert_eq!(TaskStatus::InProgress.label(), "В роботі");
        assert_eq!(TaskStatus::Done.label(), "Виконано");
        assert_eq!(TaskStatus::Cancelled.label(), "Скасовано");
    }

    #[test]
    fn task_priority_strings_match_db_values() {
        assert_eq!(TaskPriority::Low.as_str(), "low");
        assert_eq!(TaskPriority::Normal.as_str(), "normal");
        assert_eq!(TaskPriority::High.as_str(), "high");
        assert_eq!(TaskPriority::Critical.as_str(), "critical");
    }

    #[test]
    fn task_status_strings_match_db_values() {
        assert_eq!(TaskStatus::Open.as_str(), "open");
        assert_eq!(TaskStatus::InProgress.as_str(), "in_progress");
        assert_eq!(TaskStatus::Done.as_str(), "done");
        assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn task_priority_labels_are_ukrainian() {
        assert_eq!(TaskPriority::Low.label(), "Низький");
        assert_eq!(TaskPriority::Normal.label(), "Звичайний");
        assert_eq!(TaskPriority::High.label(), "Високий");
        assert_eq!(TaskPriority::Critical.label(), "Критичний");
    }

    #[test]
    fn new_task_clone_preserves_fields() {
        let task = NewTask {
            title: "Перевірити акт".to_string(),
            description: Some("Перед відправкою клієнту".to_string()),
            priority: TaskPriority::High,
            due_date: Some(Utc::now()),
            reminder_at: None,
            counterparty_id: Some(Uuid::new_v4()),
            act_id: Some(Uuid::new_v4()),
        };

        let cloned = task.clone();
        assert_eq!(cloned.title, task.title);
        assert_eq!(cloned.description, task.description);
        assert_eq!(cloned.priority, task.priority);
        assert_eq!(cloned.counterparty_id, task.counterparty_id);
        assert_eq!(cloned.act_id, task.act_id);
    }
}
