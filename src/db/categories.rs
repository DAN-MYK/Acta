// CRUD для категорій доходів/витрат.
//
// Використовує runtime-style sqlx::query_as::<_, T>() без compile-time макросів,
// щоб не залежати від cargo sqlx prepare при нових таблицях.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::category::{
    Category, CategoryKind, CategorySelectItem, NewCategory, UpdateCategory,
};

/// Всі категорії компанії (включаючи архівовані).
pub async fn list(pool: &PgPool, company_id: Uuid) -> Result<Vec<Category>> {
    let rows = sqlx::query_as::<_, Category>(
        r#"
        SELECT id, name, kind, parent_id, company_id, is_archived,
               created_at, updated_at
        FROM categories
        WHERE company_id = $1
        ORDER BY kind ASC, parent_id NULLS FIRST, name ASC
        "#,
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Категорії за типом (income/expense) для ComboBox у формах.
/// Повертає лише не-архівовані записи.
pub async fn list_for_select(
    pool: &PgPool,
    company_id: Uuid,
    kind: CategoryKind,
) -> Result<Vec<CategorySelectItem>> {
    struct Row {
        id: Uuid,
        name: String,
        kind: CategoryKind,
        parent_id: Option<Uuid>,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row;
            Ok(Self {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                kind: row.try_get("kind")?,
                parent_id: row.try_get("parent_id")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT id, name, kind, parent_id
        FROM categories
        WHERE company_id = $1
          AND kind = $2
          AND is_archived = FALSE
        ORDER BY parent_id NULLS FIRST, name ASC
        "#,
    )
    .bind(company_id)
    .bind(kind)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| CategorySelectItem {
            id: r.id,
            name: r.name,
            kind: r.kind,
            depth: if r.parent_id.is_some() { 1 } else { 0 },
        })
        .collect())
}

/// Всі категорії для ComboBox (без фільтру по kind).
pub async fn list_all_for_select(
    pool: &PgPool,
    company_id: Uuid,
) -> Result<Vec<CategorySelectItem>> {
    struct Row {
        id: Uuid,
        name: String,
        kind: CategoryKind,
        parent_id: Option<Uuid>,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row;
            Ok(Self {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                kind: row.try_get("kind")?,
                parent_id: row.try_get("parent_id")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT id, name, kind, parent_id
        FROM categories
        WHERE company_id = $1
          AND is_archived = FALSE
        ORDER BY kind ASC, parent_id NULLS FIRST, name ASC
        "#,
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| CategorySelectItem {
            id: r.id,
            name: r.name,
            kind: r.kind,
            depth: if r.parent_id.is_some() { 1 } else { 0 },
        })
        .collect())
}

/// Створити нову категорію.
pub async fn create(pool: &PgPool, data: NewCategory) -> Result<Category> {
    let row = sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (name, kind, parent_id, company_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, kind, parent_id, company_id, is_archived,
                  created_at, updated_at
        "#,
    )
    .bind(&data.name)
    .bind(&data.kind)
    .bind(data.parent_id)
    .bind(data.company_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Оновити назву/батька категорії.
/// Архівувати категорію (soft delete).
pub async fn update_scoped(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
    data: UpdateCategory,
) -> Result<Option<Category>> {
    let row = sqlx::query_as::<_, Category>(
        r#"
        UPDATE categories
        SET name = $3, parent_id = $4, updated_at = NOW()
        WHERE id = $1 AND company_id = $2
        RETURNING id, name, kind, parent_id, company_id, is_archived,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(company_id)
    .bind(&data.name)
    .bind(data.parent_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn archive_scoped(pool: &PgPool, company_id: Uuid, id: Uuid) -> Result<bool> {
    let affected = sqlx::query(
        "UPDATE categories SET is_archived = TRUE, updated_at = NOW() WHERE id = $1 AND company_id = $2",
    )
    .bind(id)
    .bind(company_id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(affected > 0)
}

/// Заповнити стандартні категорії при створенні нової компанії.
///
/// Income: Розробка ПЗ, Консалтинг, Технічна підтримка, Навчання
/// Expense: Зарплата, Оренда, Маркетинг, Податки, Комунальні послуги
pub async fn seed_defaults(pool: &PgPool, company_id: Uuid) -> Result<()> {
    let income_cats = [
        "Розробка ПЗ",
        "Консалтинг",
        "Технічна підтримка",
        "Навчання",
    ];
    let expense_cats = [
        "Зарплата",
        "Оренда",
        "Маркетинг",
        "Податки",
        "Комунальні послуги",
    ];

    for name in income_cats {
        sqlx::query(
            r#"
            INSERT INTO categories (name, kind, company_id)
            SELECT $1, $3, $2
            WHERE NOT EXISTS (
                SELECT 1
                FROM categories
                WHERE company_id = $2
                  AND kind = $3
                  AND name = $1
            )
            "#,
        )
        .bind(name)
        .bind(company_id)
        .bind(CategoryKind::Income)
        .execute(pool)
        .await?;
    }

    for name in expense_cats {
        sqlx::query(
            r#"
            INSERT INTO categories (name, kind, company_id)
            SELECT $1, $3, $2
            WHERE NOT EXISTS (
                SELECT 1
                FROM categories
                WHERE company_id = $2
                  AND kind = $3
                  AND name = $1
            )
            "#,
        )
        .bind(name)
        .bind(company_id)
        .bind(CategoryKind::Expense)
        .execute(pool)
        .await?;
    }

    Ok(())
}
