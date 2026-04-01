// CRUD операції для контрагентів
//
// `sqlx::query_as!` — макрос, що перевіряє SQL під час компіляції.
#![allow(dead_code)]
// Потребує DATABASE_URL у змінних середовища та запущеної БД при `cargo build`.
// Для офлайн-розробки запустити: cargo sqlx prepare

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Counterparty, NewCounterparty, UpdateCounterparty};

/// Отримати список усіх активних контрагентів (не архівованих), відсортованих за назвою.
pub async fn list(pool: &PgPool) -> Result<Vec<Counterparty>> {
    list_filtered(pool, None, false).await
}

/// Отримати одного контрагента за UUID.
/// Повертає `None` якщо не знайдено.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Counterparty>> {
    let row = sqlx::query_as!(
        Counterparty,
        r#"
        SELECT id, name, edrpou, iban, address, phone, email, notes,
               is_archived, bas_id, created_at, updated_at
        FROM counterparties
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Пошук контрагентів за назвою або ЄДРПОУ (часткове співпадіння, без урахування регістру).
pub async fn search(pool: &PgPool, query: &str) -> Result<Vec<Counterparty>> {
    list_filtered(pool, Some(query), false).await
}

/// Список контрагентів з опціональним текстовим пошуком та показом архіву.
pub async fn list_filtered(
    pool: &PgPool,
    query: Option<&str>,
    include_archived: bool,
) -> Result<Vec<Counterparty>> {
    let query = query.map(str::trim).filter(|q| !q.is_empty());

    let rows = match (query, include_archived) {
        (None, false) => {
            sqlx::query_as::<_, Counterparty>(
                r#"
                SELECT id, name, edrpou, iban, address, phone, email, notes,
                       is_archived, bas_id, created_at, updated_at
                FROM counterparties
                WHERE is_archived = FALSE
                ORDER BY name
                "#,
            )
            .fetch_all(pool)
            .await?
        }
        (None, true) => {
            sqlx::query_as::<_, Counterparty>(
                r#"
                SELECT id, name, edrpou, iban, address, phone, email, notes,
                       is_archived, bas_id, created_at, updated_at
                FROM counterparties
                ORDER BY name
                "#,
            )
            .fetch_all(pool)
            .await?
        }
        (Some(q), false) => {
            let pattern = format!("%{q}%");
            sqlx::query_as::<_, Counterparty>(
                r#"
                SELECT id, name, edrpou, iban, address, phone, email, notes,
                       is_archived, bas_id, created_at, updated_at
                FROM counterparties
                WHERE is_archived = FALSE
                  AND (name ILIKE $1 OR COALESCE(edrpou, '') ILIKE $1)
                ORDER BY name
                LIMIT 100
                "#,
            )
            .bind(pattern)
            .fetch_all(pool)
            .await?
        }
        (Some(q), true) => {
            let pattern = format!("%{q}%");
            sqlx::query_as::<_, Counterparty>(
                r#"
                SELECT id, name, edrpou, iban, address, phone, email, notes,
                       is_archived, bas_id, created_at, updated_at
                FROM counterparties
                WHERE name ILIKE $1 OR COALESCE(edrpou, '') ILIKE $1
                ORDER BY name
                LIMIT 100
                "#,
            )
            .bind(pattern)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows)
}

/// Створити нового контрагента. Повертає створений запис.
pub async fn create(pool: &PgPool, data: &NewCounterparty) -> Result<Counterparty> {
    // RETURNING * — PostgreSQL повертає щойно вставлений рядок
    let row = sqlx::query_as!(
        Counterparty,
        r#"
        INSERT INTO counterparties (name, edrpou, iban, address, phone, email, notes, bas_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, name, edrpou, iban, address, phone, email, notes,
                  is_archived, bas_id, created_at, updated_at
        "#,
        data.name,
        data.edrpou,
        data.iban,
        data.address,
        data.phone,
        data.email,
        data.notes,
        data.bas_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Оновити дані контрагента. Повертає оновлений запис або `None` якщо не знайдено.
pub async fn update(
    pool: &PgPool,
    id: Uuid,
    data: &UpdateCounterparty,
) -> Result<Option<Counterparty>> {
    let row = sqlx::query_as!(
        Counterparty,
        r#"
        UPDATE counterparties
        SET name       = $2,
            edrpou     = $3,
            iban       = $4,
            address    = $5,
            phone      = $6,
            email      = $7,
            notes      = $8,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, edrpou, iban, address, phone, email, notes,
                  is_archived, bas_id, created_at, updated_at
        "#,
        id,
        data.name,
        data.edrpou,
        data.iban,
        data.address,
        data.phone,
        data.email,
        data.notes,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Архівувати контрагента (м'яке видалення — `is_archived = TRUE`).
/// Повертає `true` якщо запис знайдено та оновлено.
pub async fn archive(pool: &PgPool, id: Uuid) -> Result<bool> {
    // execute() повертає PgQueryResult — rows_affected() показує скільки рядків змінено
    let affected = sqlx::query!(
        r#"
        UPDATE counterparties
        SET is_archived = TRUE, updated_at = NOW()
        WHERE id = $1
        "#,
        id
    )
    .execute(pool)
    .await?
    .rows_affected();

    // 0 означає "запис не знайдено"
    Ok(affected > 0)
}

/// Кількість архівованих контрагентів.
///
/// Використовує звичайний `sqlx::query_scalar` (без макросу) —
/// не потребує `cargo sqlx prepare`, перевіряється тільки в runtime.
pub async fn count_archived(pool: &PgPool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM counterparties WHERE is_archived = TRUE"
    )
    .fetch_one(pool)
    .await?;
    Ok(count)
}

/// Знайти контрагента за bas_id (для імпорту з BAS без дублів).
pub async fn find_by_bas_id(pool: &PgPool, bas_id: &str) -> Result<Option<Counterparty>> {
    let row = sqlx::query_as!(
        Counterparty,
        r#"
        SELECT id, name, edrpou, iban, address, phone, email, notes,
               is_archived, bas_id, created_at, updated_at
        FROM counterparties
        WHERE bas_id = $1
        "#,
        bas_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
