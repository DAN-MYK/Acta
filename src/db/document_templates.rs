use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::document_template::{DocumentTemplate, NewDocumentTemplate, TemplateListRow, UpdateDocumentTemplate};

/// Всі шаблони компанії.
pub async fn list(pool: &PgPool, company_id: Uuid) -> Result<Vec<TemplateListRow>> {
    let rows = sqlx::query_as::<_, TemplateListRow>(
        r#"
        SELECT id, name, description, template_type, is_default
        FROM document_templates
        WHERE company_id = $1
        ORDER BY template_type, name
        "#,
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Один шаблон за ID.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<DocumentTemplate>> {
    let row = sqlx::query_as::<_, DocumentTemplate>(
        r#"
        SELECT id, company_id, name, description, template_type,
               template_path, is_default, created_at, updated_at
        FROM document_templates
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Створити новий шаблон.
/// Якщо is_default = true, попередній дефолтний того ж типу скидається.
pub async fn create(pool: &PgPool, company_id: Uuid, data: NewDocumentTemplate) -> Result<DocumentTemplate> {
    let mut tx = pool.begin().await?;

    if data.is_default {
        sqlx::query(
            r#"
            UPDATE document_templates
            SET is_default = FALSE, updated_at = NOW()
            WHERE company_id = $1 AND template_type = $2 AND is_default = TRUE
            "#,
        )
        .bind(company_id)
        .bind(&data.template_type)
        .execute(&mut *tx)
        .await?;
    }

    let row = sqlx::query_as::<_, DocumentTemplate>(
        r#"
        INSERT INTO document_templates (company_id, name, description, template_type, template_path, is_default)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, company_id, name, description, template_type,
                  template_path, is_default, created_at, updated_at
        "#,
    )
    .bind(company_id)
    .bind(&data.name)
    .bind(&data.description)
    .bind(&data.template_type)
    .bind(&data.template_path)
    .bind(data.is_default)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(row)
}

/// Оновити шаблон.
/// Якщо is_default = true, попередній дефолтний того ж типу скидається.
pub async fn update(pool: &PgPool, id: Uuid, data: UpdateDocumentTemplate) -> Result<DocumentTemplate> {
    let existing = get_by_id(pool, id).await?.ok_or_else(|| anyhow::anyhow!("Шаблон не знайдено"))?;

    let mut tx = pool.begin().await?;

    // Якщо встановлюється is_default=true — скинути інший
    if data.is_default == Some(true) {
        sqlx::query(
            r#"
            UPDATE document_templates
            SET is_default = FALSE, updated_at = NOW()
            WHERE company_id = $1 AND template_type = $2 AND is_default = TRUE AND id != $3
            "#,
        )
        .bind(existing.company_id)
        .bind(&existing.template_type)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }

    let name = data.name.as_ref().unwrap_or(&existing.name);
    let description = data.description.clone().or(existing.description.clone());
    let template_path = data.template_path.as_ref().unwrap_or(&existing.template_path);
    let is_default = data.is_default.unwrap_or(existing.is_default);

    let row = sqlx::query_as::<_, DocumentTemplate>(
        r#"
        UPDATE document_templates
        SET name = $1,
            description = $2,
            template_path = $3,
            is_default = $4,
            updated_at = NOW()
        WHERE id = $5
        RETURNING id, company_id, name, description, template_type,
                  template_path, is_default, created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(&description)
    .bind(template_path)
    .bind(is_default)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(row)
}

/// Видалити шаблон.
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM document_templates WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Дефолтний шаблон для типу ('act' або 'invoice').
pub async fn get_default(pool: &PgPool, company_id: Uuid, template_type: &str) -> Result<Option<DocumentTemplate>> {
    let row = sqlx::query_as::<_, DocumentTemplate>(
        r#"
        SELECT id, company_id, name, description, template_type,
               template_path, is_default, created_at, updated_at
        FROM document_templates
        WHERE company_id = $1 AND template_type = $2 AND is_default = TRUE
        "#,
    )
    .bind(company_id)
    .bind(template_type)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
