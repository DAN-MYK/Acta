use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{NewTask, Task, TaskStatus};

fn normalized_parent_refs(task: &NewTask) -> (Option<Uuid>, Option<Uuid>) {
    match task.act_id {
        Some(act_id) => (None, Some(act_id)),
        None => (task.counterparty_id, None),
    }
}

pub async fn list_open(pool: &PgPool, company_id: Uuid, query: &str) -> Result<Vec<Task>> {
    let rows = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, title, description,
               status, priority,
               due_date, reminder_at,
               counterparty_id, act_id,
               created_at, updated_at
        FROM tasks
        WHERE company_id = $1
          AND status IN ('open', 'in_progress')
          AND (
              $2 = ''
              OR title ILIKE '%' || $2 || '%'
              OR COALESCE(description, '') ILIKE '%' || $2 || '%'
          )
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
    .bind(company_id)
    .bind(query)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_all(pool: &PgPool, company_id: Uuid, query: &str) -> Result<Vec<Task>> {
    let rows = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, title, description,
               status, priority,
               due_date, reminder_at,
               counterparty_id, act_id,
               created_at, updated_at
        FROM tasks
        WHERE company_id = $1
          AND (
              $2 = ''
              OR title ILIKE '%' || $2 || '%'
              OR COALESCE(description, '') ILIKE '%' || $2 || '%'
          )
        ORDER BY
            CASE status
                WHEN 'open' THEN 1
                WHEN 'in_progress' THEN 2
                WHEN 'done' THEN 3
                ELSE 4
            END,
            CASE priority
                WHEN 'critical' THEN 1
                WHEN 'high' THEN 2
                WHEN 'normal' THEN 3
                WHEN 'low' THEN 4
                ELSE 5
            END,
            due_date ASC NULLS LAST,
            created_at DESC
        "#,
    )
    .bind(company_id)
    .bind(query)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_due_today(pool: &PgPool, company_id: Uuid) -> Result<Vec<Task>> {
    let rows = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, title, description,
               status, priority,
               due_date, reminder_at,
               counterparty_id, act_id,
               created_at, updated_at
        FROM tasks
        WHERE company_id = $1
          AND due_date IS NOT NULL
          AND DATE(due_date AT TIME ZONE 'UTC') = CURRENT_DATE
          AND status IN ('open', 'in_progress')
        ORDER BY due_date ASC NULLS LAST, created_at ASC
        "#,
    )
    .bind(company_id)
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

pub async fn get_by_id_scoped(pool: &PgPool, company_id: Uuid, id: Uuid) -> Result<Option<Task>> {
    let row = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, title, description,
               status, priority,
               due_date, reminder_at,
               counterparty_id, act_id,
               created_at, updated_at
        FROM tasks
        WHERE id = $1 AND company_id = $2
        "#,
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn create(pool: &PgPool, company_id: Uuid, task: &NewTask) -> Result<Task> {
    let (counterparty_id, act_id) = normalized_parent_refs(task);

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
    .bind(counterparty_id)
    .bind(act_id)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn list_by_act_scoped(
    pool: &PgPool,
    company_id: Uuid,
    act_id: Uuid,
) -> Result<Vec<Task>> {
    let rows = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, title, description,
               status, priority,
               due_date, reminder_at,
               counterparty_id, act_id,
               created_at, updated_at
        FROM tasks
        WHERE act_id = $1 AND company_id = $2
        ORDER BY created_at DESC
        "#,
    )
    .bind(act_id)
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn update_scoped(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
    task: &NewTask,
) -> Result<Option<Task>> {
    let (counterparty_id, act_id) = normalized_parent_refs(task);

    let row = sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET title = $3,
            description = $4,
            priority = $5,
            due_date = $6,
            reminder_at = $7,
            counterparty_id = $8,
            act_id = $9,
            updated_at = NOW()
        WHERE id = $1 AND company_id = $2
        RETURNING id, title, description,
                  status, priority,
                  due_date, reminder_at,
                  counterparty_id, act_id,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(company_id)
    .bind(&task.title)
    .bind(&task.description)
    .bind(task.priority.clone())
    .bind(task.due_date.clone())
    .bind(task.reminder_at.clone())
    .bind(counterparty_id)
    .bind(act_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn set_status_scoped(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
    status: TaskStatus,
) -> Result<Option<Task>> {
    let row = sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET status = $3,
            updated_at = NOW()
        WHERE id = $1 AND company_id = $2
        RETURNING id, title, description,
                  status, priority,
                  due_date, reminder_at,
                  counterparty_id, act_id,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(company_id)
    .bind(status)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn delete_scoped(pool: &PgPool, company_id: Uuid, id: Uuid) -> Result<bool> {
    let affected = sqlx::query(
        r#"
        DELETE FROM tasks
        WHERE id = $1 AND company_id = $2
        "#,
    )
    .bind(id)
    .bind(company_id)
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
        let _ = list_all;
        let _ = list_due_today;
        let _ = get_by_id;
        let _ = list_by_act;
        let _ = create;
        let _ = update;
        let _ = set_status;
        let _ = delete;
        let _ = due_reminders;
    }
}
