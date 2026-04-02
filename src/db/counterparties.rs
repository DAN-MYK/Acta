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

/// Отримати список усіх активних контрагентів компанії (не архівованих), відсортованих за назвою.
pub async fn list(pool: &PgPool, company_id: Uuid) -> Result<Vec<Counterparty>> {
    list_filtered(pool, company_id, None, false).await
}

/// Отримати одного контрагента за UUID.
/// Повертає `None` якщо не знайдено.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Counterparty>> {
    let row = sqlx::query_as::<_, Counterparty>(
        r#"
        SELECT id, name, edrpou, ipn, iban, address, phone, email, notes,
               is_archived, bas_id, created_at, updated_at
        FROM counterparties
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Пошук контрагентів за назвою або ЄДРПОУ (часткове співпадіння, без урахування регістру).
pub async fn search(pool: &PgPool, company_id: Uuid, query: &str) -> Result<Vec<Counterparty>> {
    list_filtered(pool, company_id, Some(query), false).await
}

/// Список контрагентів компанії з опціональним текстовим пошуком та показом архіву.
///
/// Всі 4 гілки фільтрують за `company_id` — ізоляція даних між компаніями.
pub async fn list_filtered(
    pool: &PgPool,
    company_id: Uuid,
    query: Option<&str>,
    include_archived: bool,
) -> Result<Vec<Counterparty>> {
    let query = query.map(str::trim).filter(|q| !q.is_empty());

    let rows = match (query, include_archived) {
        (None, false) => {
            // $1 = company_id
            sqlx::query_as::<_, Counterparty>(
                r#"
                SELECT id, name, edrpou, ipn, iban, address, phone, email, notes,
                       is_archived, bas_id, created_at, updated_at
                FROM counterparties
                WHERE is_archived = FALSE AND company_id = $1
                ORDER BY name
                "#,
            )
            .bind(company_id)
            .fetch_all(pool)
            .await?
        }
        (None, true) => {
            // $1 = company_id
            sqlx::query_as::<_, Counterparty>(
                r#"
                SELECT id, name, edrpou, ipn, iban, address, phone, email, notes,
                       is_archived, bas_id, created_at, updated_at
                FROM counterparties
                WHERE company_id = $1
                ORDER BY name
                "#,
            )
            .bind(company_id)
            .fetch_all(pool)
            .await?
        }
        (Some(q), false) => {
            // $1 = pattern, $2 = company_id
            let pattern = format!("%{q}%");
            sqlx::query_as::<_, Counterparty>(
                r#"
                SELECT id, name, edrpou, ipn, iban, address, phone, email, notes,
                       is_archived, bas_id, created_at, updated_at
                FROM counterparties
                WHERE is_archived = FALSE
                  AND (name ILIKE $1 OR COALESCE(edrpou, '') ILIKE $1)
                  AND company_id = $2
                ORDER BY name
                LIMIT 100
                "#,
            )
            .bind(pattern)
            .bind(company_id)
            .fetch_all(pool)
            .await?
        }
        (Some(q), true) => {
            // $1 = pattern, $2 = company_id
            let pattern = format!("%{q}%");
            sqlx::query_as::<_, Counterparty>(
                r#"
                SELECT id, name, edrpou, ipn, iban, address, phone, email, notes,
                       is_archived, bas_id, created_at, updated_at
                FROM counterparties
                WHERE (name ILIKE $1 OR COALESCE(edrpou, '') ILIKE $1)
                  AND company_id = $2
                ORDER BY name
                LIMIT 100
                "#,
            )
            .bind(pattern)
            .bind(company_id)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows)
}

/// Створити нового контрагента в межах вказаної компанії. Повертає створений запис.
///
/// Використовує runtime-style query_as щоб уникнути потреби в `cargo sqlx prepare`
/// при зміні сигнатури (додано company_id).
pub async fn create(pool: &PgPool, company_id: Uuid, data: &NewCounterparty) -> Result<Counterparty> {
    let row = sqlx::query_as::<_, Counterparty>(
        r#"INSERT INTO counterparties (company_id, name, edrpou, ipn, iban, address, phone, email, notes, bas_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, name, edrpou, ipn, iban, address, phone, email, notes,
                     is_archived, bas_id, created_at, updated_at"#
    )
    .bind(company_id)
    .bind(&data.name)
    .bind(&data.edrpou)
    .bind(&data.ipn)
    .bind(&data.iban)
    .bind(&data.address)
    .bind(&data.phone)
    .bind(&data.email)
    .bind(&data.notes)
    .bind(&data.bas_id)
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
    let row = sqlx::query_as::<_, Counterparty>(
        r#"
        UPDATE counterparties
        SET name       = $2,
            edrpou     = $3,
            ipn        = $4,
            iban       = $5,
            address    = $6,
            phone      = $7,
            email      = $8,
            notes      = $9,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, edrpou, ipn, iban, address, phone, email, notes,
                  is_archived, bas_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&data.name)
    .bind(&data.edrpou)
    .bind(&data.ipn)
    .bind(&data.iban)
    .bind(&data.address)
    .bind(&data.phone)
    .bind(&data.email)
    .bind(&data.notes)
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
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM counterparties WHERE is_archived = TRUE")
            .fetch_one(pool)
            .await?;
    Ok(count)
}

/// Знайти контрагента за bas_id (для імпорту з BAS без дублів).
pub async fn find_by_bas_id(pool: &PgPool, bas_id: &str) -> Result<Option<Counterparty>> {
    let row = sqlx::query_as::<_, Counterparty>(
        r#"
        SELECT id, name, edrpou, ipn, iban, address, phone, email, notes,
               is_archived, bas_id, created_at, updated_at
        FROM counterparties
        WHERE bas_id = $1
        "#,
    )
    .bind(bas_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_counterparties_public_api_is_exposed() {
        // Перевіряємо що всі публічні функції доступні та компілюються
        let _ = list;
        let _ = get_by_id;
        let _ = search;
        let _ = list_filtered;
        let _ = create;
        let _ = update;
        let _ = archive;
        let _ = count_archived;
        let _ = find_by_bas_id;
    }
}
