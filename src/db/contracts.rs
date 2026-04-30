// CRUD для договорів.
//
// Використовує runtime-style sqlx::query_as::<_, T>() без compile-time макросів.

use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::contract::{
    Contract, ContractListRow, ContractSelectItem, ContractStatus, NewContract, UpdateContract,
};

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Contract {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            company_id: row.try_get("company_id")?,
            counterparty_id: row.try_get("counterparty_id")?,
            number: row.try_get("number")?,
            subject: row.try_get("subject")?,
            date: row.try_get("date")?,
            expires_at: row.try_get("expires_at")?,
            amount: row.try_get("amount")?,
            status: row.try_get("status")?,
            notes: row.try_get("notes")?,
            bas_id: row.try_get("bas_id")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Список договорів компанії з назвою контрагента.
pub async fn list(pool: &PgPool, company_id: Uuid) -> Result<Vec<ContractListRow>> {
    struct Row {
        id: Uuid,
        number: String,
        subject: Option<String>,
        counterparty_id: Uuid,
        counterparty_name: String,
        date: String,
        expires_at: Option<String>,
        amount: Option<rust_decimal::Decimal>,
        status: ContractStatus,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                id: r.try_get("id")?,
                number: r.try_get("number")?,
                subject: r.try_get("subject")?,
                counterparty_id: r.try_get("counterparty_id")?,
                counterparty_name: r.try_get("counterparty_name")?,
                date: r.try_get("date")?,
                expires_at: r.try_get("expires_at")?,
                amount: r.try_get("amount")?,
                status: r.try_get("status")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            c.id,
            c.number,
            c.subject,
            c.counterparty_id,
            cp.name                                AS counterparty_name,
            TO_CHAR(c.date, 'DD.MM.YYYY')          AS date,
            TO_CHAR(c.expires_at, 'DD.MM.YYYY')    AS expires_at,
            c.amount,
            c.status
        FROM contracts c
        JOIN counterparties cp ON cp.id = c.counterparty_id
        WHERE c.company_id = $1
        ORDER BY c.date DESC
        "#,
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ContractListRow {
            id: r.id,
            number: r.number,
            subject: r.subject,
            counterparty_id: r.counterparty_id,
            counterparty_name: r.counterparty_name,
            date: r.date,
            expires_at: r.expires_at,
            amount: r.amount,
            status: r.status,
        })
        .collect())
}

/// Список договорів конкретного контрагента.
pub async fn list_by_counterparty(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
) -> Result<Vec<ContractListRow>> {
    struct Row {
        id: Uuid,
        number: String,
        subject: Option<String>,
        counterparty_id: Uuid,
        counterparty_name: String,
        date: String,
        expires_at: Option<String>,
        amount: Option<rust_decimal::Decimal>,
        status: ContractStatus,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                id: r.try_get("id")?,
                number: r.try_get("number")?,
                subject: r.try_get("subject")?,
                counterparty_id: r.try_get("counterparty_id")?,
                counterparty_name: r.try_get("counterparty_name")?,
                date: r.try_get("date")?,
                expires_at: r.try_get("expires_at")?,
                amount: r.try_get("amount")?,
                status: r.try_get("status")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            c.id,
            c.number,
            c.subject,
            c.counterparty_id,
            cp.name                                AS counterparty_name,
            TO_CHAR(c.date, 'DD.MM.YYYY')          AS date,
            TO_CHAR(c.expires_at, 'DD.MM.YYYY')    AS expires_at,
            c.amount,
            c.status
        FROM contracts c
        JOIN counterparties cp ON cp.id = c.counterparty_id
        WHERE c.company_id = $1
          AND c.counterparty_id = $2
        ORDER BY c.date DESC
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ContractListRow {
            id: r.id,
            number: r.number,
            subject: r.subject,
            counterparty_id: r.counterparty_id,
            counterparty_name: r.counterparty_name,
            date: r.date,
            expires_at: r.expires_at,
            amount: r.amount,
            status: r.status,
        })
        .collect())
}

/// Для ComboBox у формах: активні договори контрагента.
pub async fn list_for_select(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
) -> Result<Vec<ContractSelectItem>> {
    struct Row {
        id: Uuid,
        number: String,
        subject: Option<String>,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                id: r.try_get("id")?,
                number: r.try_get("number")?,
                subject: r.try_get("subject")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT id, number, subject
        FROM contracts
        WHERE company_id = $1
          AND counterparty_id = $2
          AND status = 'active'
        ORDER BY date DESC
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ContractSelectItem {
            id: r.id,
            number: r.number,
            subject: r.subject,
        })
        .collect())
}

/// Отримати договір за ID.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Contract> {
    let row = sqlx::query_as::<_, Contract>(
        r#"
        SELECT id, company_id, counterparty_id, number, subject,
               date, expires_at, amount, notes, bas_id, status,
               created_at, updated_at
        FROM contracts
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Знайти договір за bas_id для ідемпотентного імпорту з BAS.
pub async fn find_by_bas_id(pool: &PgPool, bas_id: &str) -> Result<Option<Contract>> {
    let row = sqlx::query_as::<_, Contract>(
        r#"
        SELECT id, company_id, counterparty_id, number, subject,
               date, expires_at, amount, notes, bas_id, status,
               created_at, updated_at
        FROM contracts
        WHERE bas_id = $1
        "#,
    )
    .bind(bas_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Знайти договір за номером у межах компанії та контрагента.
pub async fn find_by_number(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    number: &str,
) -> Result<Option<Contract>> {
    let row = sqlx::query_as::<_, Contract>(
        r#"
        SELECT id, company_id, counterparty_id, number, subject,
               date, expires_at, amount, notes, bas_id, status,
               created_at, updated_at
        FROM contracts
        WHERE company_id = $1
          AND counterparty_id = $2
          AND lower(trim(number)) = lower(trim($3))
        ORDER BY date DESC
        LIMIT 1
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .bind(number)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Знайти всі договори за номером у межах компанії та контрагента.
pub async fn list_by_number_exact(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    number: &str,
) -> Result<Vec<Contract>> {
    let rows = sqlx::query_as::<_, Contract>(
        r#"
        SELECT id, company_id, counterparty_id, number, subject,
               date, expires_at, amount, notes, bas_id, status,
               created_at, updated_at
        FROM contracts
        WHERE company_id = $1
          AND counterparty_id = $2
          AND lower(trim(number)) = lower(trim($3))
        ORDER BY date DESC, created_at DESC
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .bind(number)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Створити новий договір.
pub async fn create(pool: &PgPool, data: NewContract) -> Result<Contract> {
    let row = sqlx::query_as::<_, Contract>(
        r#"
        INSERT INTO contracts
            (company_id, counterparty_id, number, subject, date, expires_at, amount)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, company_id, counterparty_id, number, subject,
                  date, expires_at, amount, notes, bas_id, status,
                  created_at, updated_at
        "#,
    )
    .bind(data.company_id)
    .bind(data.counterparty_id)
    .bind(&data.number)
    .bind(&data.subject)
    .bind(data.date)
    .bind(data.expires_at)
    .bind(data.amount)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Створити договір із повним payload для імпорту з BAS.
pub async fn create_imported(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    number: &str,
    subject: Option<&str>,
    date: chrono::NaiveDate,
    expires_at: Option<chrono::NaiveDate>,
    amount: Option<rust_decimal::Decimal>,
    notes: Option<&str>,
    bas_id: Option<&str>,
    status: ContractStatus,
) -> Result<Contract> {
    let row = sqlx::query_as::<_, Contract>(
        r#"
        INSERT INTO contracts
            (company_id, counterparty_id, number, subject, date, expires_at, amount, notes, bas_id, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, company_id, counterparty_id, number, subject,
                  date, expires_at, amount, notes, bas_id, status,
                  created_at, updated_at
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .bind(number)
    .bind(subject)
    .bind(date)
    .bind(expires_at)
    .bind(amount)
    .bind(notes)
    .bind(bas_id)
    .bind(status)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Оновити договір.
pub async fn update(pool: &PgPool, id: Uuid, data: UpdateContract) -> Result<Contract> {
    let row = sqlx::query_as::<_, Contract>(
        r#"
        UPDATE contracts
        SET number     = $2,
            subject    = $3,
            date       = $4,
            expires_at = $5,
            amount     = $6,
            status     = $7,
            notes      = $8,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, company_id, counterparty_id, number, subject,
                  date, expires_at, amount, notes, bas_id, status,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&data.number)
    .bind(&data.subject)
    .bind(data.date)
    .bind(data.expires_at)
    .bind(data.amount)
    .bind(data.status)
    .bind(&data.notes)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Оновити договір із повним payload під час імпорту з BAS.
pub async fn update_imported(
    pool: &PgPool,
    id: Uuid,
    number: &str,
    subject: Option<&str>,
    date: chrono::NaiveDate,
    expires_at: Option<chrono::NaiveDate>,
    amount: Option<rust_decimal::Decimal>,
    status: ContractStatus,
    notes: Option<&str>,
) -> Result<Contract> {
    let row = sqlx::query_as::<_, Contract>(
        r#"
        UPDATE contracts
        SET number     = $2,
            subject    = $3,
            date       = $4,
            expires_at = $5,
            amount     = $6,
            status     = $7,
            notes      = $8,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, company_id, counterparty_id, number, subject,
                  date, expires_at, amount, notes, bas_id, status,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(number)
    .bind(subject)
    .bind(date)
    .bind(expires_at)
    .bind(amount)
    .bind(status)
    .bind(notes)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Видалити договір.
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM contracts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_contracts_public_api_is_exposed() {
        let _ = list;
        let _ = list_by_counterparty;
        let _ = list_for_select;
        let _ = get_by_id;
        let _ = find_by_bas_id;
        let _ = find_by_number;
        let _ = list_by_number_exact;
        let _ = create;
        let _ = create_imported;
        let _ = update;
        let _ = update_imported;
        let _ = delete;
    }
}
