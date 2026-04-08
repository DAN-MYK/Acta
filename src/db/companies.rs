// CRUD операції для таблиці companies
//
// Використовує runtime-style sqlx::query_as::<_, Company>() — без макросу query_as!
// Тому не потребує `cargo sqlx prepare` після змін цього файлу.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::company::{Company, CompanySummary, NewCompany, UpdateCompany};


/// Всі активні (не архівовані) компанії, відсортовані за назвою.
/// Використовується для списку вибору активної компанії.
pub async fn list(pool: &PgPool) -> Result<Vec<Company>> {
    Ok(sqlx::query_as::<_, Company>(
        "SELECT id, name, short_name, edrpou, ipn, iban, legal_address, actual_address,
                phone, email, director_name, accountant_name, tax_system, is_vat_payer,
                logo_path, notes, is_archived, created_at, updated_at
         FROM companies WHERE is_archived = FALSE ORDER BY name"
    )
    .fetch_all(pool)
    .await?)
}

/// Активні компанії разом зі статистикою по актах для карток управління.
pub async fn list_with_summary(pool: &PgPool) -> Result<Vec<CompanySummary>> {
    Ok(sqlx::query_as::<_, CompanySummary>(
        r#"SELECT
                c.id,
                c.name,
                c.short_name,
                c.edrpou,
                c.is_vat_payer,
                COUNT(a.id)::BIGINT AS act_count,
                COALESCE(SUM(a.total_amount), 0)::DECIMAL(15,2) AS total_amount
           FROM companies c
           LEFT JOIN acts a ON a.company_id = c.id
           WHERE c.is_archived = FALSE
           GROUP BY c.id, c.name, c.short_name, c.edrpou, c.is_vat_payer
           ORDER BY c.name"#
    )
    .fetch_all(pool)
    .await?)
}

/// Всі компанії включно з архівованими (для адмін-перегляду).
pub async fn list_all(pool: &PgPool) -> Result<Vec<Company>> {
    Ok(sqlx::query_as::<_, Company>(
        "SELECT id, name, short_name, edrpou, ipn, iban, legal_address, actual_address,
                phone, email, director_name, accountant_name, tax_system, is_vat_payer,
                logo_path, notes, is_archived, created_at, updated_at
         FROM companies ORDER BY name"
    )
    .fetch_all(pool)
    .await?)
}

/// Отримати компанію за UUID. Повертає `None` якщо не знайдено.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Company>> {
    Ok(sqlx::query_as::<_, Company>(
        "SELECT id, name, short_name, edrpou, ipn, iban, legal_address, actual_address,
                phone, email, director_name, accountant_name, tax_system, is_vat_payer,
                logo_path, notes, is_archived, created_at, updated_at
         FROM companies WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

/// Створити нову компанію. Повертає щойно створений запис.
pub async fn create(pool: &PgPool, c: &NewCompany) -> Result<Company> {
    Ok(sqlx::query_as::<_, Company>(
        r#"INSERT INTO companies (name, short_name, edrpou, ipn, iban, legal_address, director_name, tax_system, is_vat_payer)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
           RETURNING id, name, short_name, edrpou, ipn, iban, legal_address, actual_address,
                     phone, email, director_name, accountant_name, tax_system, is_vat_payer,
                     logo_path, notes, is_archived, created_at, updated_at"#
    )
    .bind(&c.name)
    .bind(&c.short_name)
    .bind(&c.edrpou)
    .bind(&c.ipn)
    .bind(&c.iban)
    .bind(&c.legal_address)
    .bind(&c.director_name)
    .bind(&c.tax_system)
    .bind(c.is_vat_payer)
    .fetch_one(pool)
    .await?)
}

/// Оновити дані компанії. Повертає оновлений запис або `None` якщо не знайдено.
pub async fn update(pool: &PgPool, id: Uuid, c: &UpdateCompany) -> Result<Option<Company>> {
    Ok(sqlx::query_as::<_, Company>(
        r#"UPDATE companies
           SET name = $2, short_name = $3, edrpou = $4, iban = $5,
               legal_address = $6, director_name = $7, accountant_name = $8,
               tax_system = $9, is_vat_payer = $10, logo_path = $11, updated_at = NOW()
           WHERE id = $1
           RETURNING id, name, short_name, edrpou, ipn, iban, legal_address, actual_address,
                     phone, email, director_name, accountant_name, tax_system, is_vat_payer,
                     logo_path, notes, is_archived, created_at, updated_at"#
    )
    .bind(id)
    .bind(&c.name)
    .bind(&c.short_name)
    .bind(&c.edrpou)
    .bind(&c.iban)
    .bind(&c.legal_address)
    .bind(&c.director_name)
    .bind(&c.accountant_name)
    .bind(&c.tax_system)
    .bind(c.is_vat_payer)
    .bind(&c.logo_path)
    .fetch_optional(pool)
    .await?)
}

/// Архівувати компанію (м'яке видалення).
/// Повертає `true` якщо запис знайдено та оновлено.
pub async fn archive(pool: &PgPool, id: Uuid) -> Result<bool> {
    let affected = sqlx::query(
        "UPDATE companies SET is_archived = TRUE, updated_at = NOW() WHERE id = $1"
    )
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_companies_public_api_is_exposed() {
        let _ = list;
        let _ = list_with_summary;
        let _ = list_all;
        let _ = get_by_id;
        let _ = create;
        let _ = update;
        let _ = archive;
    }
}
