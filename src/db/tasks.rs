#![allow(dead_code)]

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{NewTask, Task, TaskStatus};

pub async fn list_open(pool: &PgPool) -> Result<Vec<Task>> {
    let rows = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, title, description,
               status, priority,
               due_date, reminder_at,
               counterparty_id, act_id,
               created_at, updated_at
        FROM tasks
        WHERE status IN ('open', 'in_progress')
        ORDER BY
            CASE priority
                WHEN 'critical' THEN 1
                WHEN 'high' THEN 2
                WHEN 'normal' THEN 3
                WHEN 'low' THEN 4
            END,
            due_date ASC NULLS LAST,
            created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_by_counterparty(pool: &PgPool, counterparty_id: Uuid) -> Result<Vec<Task>> {
    let rows = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, title, description,
               status, priority,
               due_date, reminder_at,
               counterparty_id, act_id,
               created_at, updated_at
        FROM tasks
        WHERE counterparty_id = $1
        ORDER BY due_date NULLS LAST, created_at DESC
        "#,
    )
    .bind(counterparty_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Task>> {
    let row = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, title, description,
               status, priority,
               due_date, reminder_at,
               counterparty_id, act_id,
               created_at, updated_at
        FROM tasks
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn list_by_act(pool: &PgPool, act_id: Uuid) -> Result<Vec<Task>> {
    let rows = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, title, description,
               status, priority,
               due_date, reminder_at,
               counterparty_id, act_id,
               created_at, updated_at
        FROM tasks
        WHERE act_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(act_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn create(pool: &PgPool, company_id: Uuid, task: &NewTask) -> Result<Task> {
    let row = sqlx::query_as::<_, Task>(
        r#"
        INSERT INTO tasks (
            company_id, title, description, status, priority,
            due_date, reminder_at, counterparty_id, act_id
        )
        VALUES ($1, $2, $3, 'open', $4, $5, $6, $7, $8)
        RETURNING id, title, description,
                  status, priority,
                  due_date, reminder_at,
                  counterparty_id, act_id,
                  created_at, updated_at
        "#,
    )
    .bind(company_id)
    .bind(&task.title)
    .bind(&task.description)
    .bind(task.priority.clone())
    .bind(task.due_date.clone())
    .bind(task.reminder_at.clone())
    .bind(task.counterparty_id.clone())
    .bind(task.act_id.clone())
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn update(pool: &PgPool, id: Uuid, task: &NewTask) -> Result<Option<Task>> {
    let row = sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET title = $2,
            description = $3,
            priority = $4,
            due_date = $5,
            reminder_at = $6,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, title, description,
                  status, priority,
                  due_date, reminder_at,
                  counterparty_id, act_id,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&task.title)
    .bind(&task.description)
    .bind(task.priority.clone())
    .bind(task.due_date.clone())
    .bind(task.reminder_at.clone())
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn set_status(pool: &PgPool, id: Uuid, status: TaskStatus) -> Result<Option<Task>> {
    let row = sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET status = $2,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, title, description,
                  status, priority,
                  due_date, reminder_at,
                  counterparty_id, act_id,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(status)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool> {
    let affected = sqlx::query(
        r#"
        DELETE FROM tasks
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(affected > 0)
}

pub async fn due_reminders(pool: &PgPool) -> Result<Vec<Task>> {
    let rows = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, title, description,
               status, priority,
               due_date, reminder_at,
               counterparty_id, act_id,
               created_at, updated_at
        FROM tasks
        WHERE reminder_at IS NOT NULL
          AND reminder_at <= NOW() + INTERVAL '1 minute'
          AND reminder_at > NOW() - INTERVAL '1 minute'
          AND status = 'open'
        ORDER BY reminder_at ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_tasks_public_api_is_exposed() {
        let _ = list_open;
        let _ = get_by_id;
        let _ = list_by_act;
        let _ = create;
        let _ = update;
        let _ = set_status;
        let _ = delete;
        let _ = due_reminders;
    }
}
