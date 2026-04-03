// CRUD для платежів.
//
// Використовує runtime-style sqlx::query_as::<_, T>() без compile-time макросів.
#![allow(dead_code)]

use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::payment::{
    NewPayment, NewPaymentSchedule, Payment, PaymentDirection, PaymentListRow, PaymentSchedule,
    UpdatePayment,
};

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Payment {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        Ok(Self {
            id:              row.try_get("id")?,
            company_id:      row.try_get("company_id")?,
            date:            row.try_get("date")?,
            amount:          row.try_get("amount")?,
            direction:       row.try_get("direction")?,
            counterparty_id: row.try_get("counterparty_id")?,
            bank_name:       row.try_get("bank_name")?,
            bank_ref:        row.try_get("bank_ref")?,
            description:     row.try_get("description")?,
            is_reconciled:   row.try_get("is_reconciled")?,
            bas_id:          row.try_get("bas_id")?,
            created_at:      row.try_get("created_at")?,
            updated_at:      row.try_get("updated_at")?,
        })
    }
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for PaymentSchedule {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        Ok(Self {
            id:              row.try_get("id")?,
            company_id:      row.try_get("company_id")?,
            title:           row.try_get("title")?,
            amount:          row.try_get("amount")?,
            direction:       row.try_get("direction")?,
            scheduled_date:  row.try_get("scheduled_date")?,
            recurrence:      row.try_get("recurrence")?,
            recurrence_end:  row.try_get("recurrence_end")?,
            counterparty_id: row.try_get("counterparty_id")?,
            notes:           row.try_get("notes")?,
            is_completed:    row.try_get("is_completed")?,
            created_at:      row.try_get("created_at")?,
            updated_at:      row.try_get("updated_at")?,
        })
    }
}

/// Список платежів компанії з опційним фільтром напрямку.
pub async fn list(
    pool: &PgPool,
    company_id: Uuid,
    direction: Option<PaymentDirection>,
) -> Result<Vec<PaymentListRow>> {
    struct Row {
        id:               Uuid,
        date:             String,
        amount:           rust_decimal::Decimal,
        direction:        PaymentDirection,
        counterparty_id:  Option<Uuid>,
        counterparty_name: Option<String>,
        bank_name:        Option<String>,
        description:      Option<String>,
        is_reconciled:    bool,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                id:                r.try_get("id")?,
                date:              r.try_get("date")?,
                amount:            r.try_get("amount")?,
                direction:         r.try_get("direction")?,
                counterparty_id:   r.try_get("counterparty_id")?,
                counterparty_name: r.try_get("counterparty_name")?,
                bank_name:         r.try_get("bank_name")?,
                description:       r.try_get("description")?,
                is_reconciled:     r.try_get("is_reconciled")?,
            })
        }
    }

    let dir_filter = direction.as_ref().map(|d| d.as_str());

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            p.id,
            TO_CHAR(p.date, 'DD.MM.YYYY')  AS date,
            p.amount,
            p.direction,
            p.counterparty_id,
            cp.name                         AS counterparty_name,
            p.bank_name,
            p.description,
            p.is_reconciled
        FROM payments p
        LEFT JOIN counterparties cp ON cp.id = p.counterparty_id
        WHERE p.company_id = $1
          AND ($2::payment_direction IS NULL OR p.direction = $2::payment_direction)
        ORDER BY p.date DESC, p.created_at DESC
        "#,
    )
    .bind(company_id)
    .bind(dir_filter)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PaymentListRow {
            id:               r.id,
            date:             r.date,
            amount:           r.amount,
            direction:        r.direction,
            counterparty_id:  r.counterparty_id,
            counterparty_name: r.counterparty_name,
            bank_name:        r.bank_name,
            description:      r.description,
            is_reconciled:    r.is_reconciled,
        })
        .collect())
}

/// Отримати платіж за ID.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Payment>> {
    let row = sqlx::query_as::<_, Payment>(
        r#"
        SELECT id, company_id, date, amount, direction, counterparty_id,
               bank_name, bank_ref, description, is_reconciled, bas_id,
               created_at, updated_at
        FROM payments
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Створити платіж.
pub async fn create(pool: &PgPool, data: NewPayment) -> Result<Payment> {
    let row = sqlx::query_as::<_, Payment>(
        r#"
        INSERT INTO payments
            (company_id, date, amount, direction, counterparty_id,
             bank_name, bank_ref, description)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, company_id, date, amount, direction, counterparty_id,
                  bank_name, bank_ref, description, is_reconciled, bas_id,
                  created_at, updated_at
        "#,
    )
    .bind(data.company_id)
    .bind(data.date)
    .bind(data.amount)
    .bind(data.direction.as_str())
    .bind(data.counterparty_id)
    .bind(data.bank_name)
    .bind(data.bank_ref)
    .bind(data.description)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Оновити платіж.
pub async fn update(pool: &PgPool, id: Uuid, data: UpdatePayment) -> Result<Option<Payment>> {
    let row = sqlx::query_as::<_, Payment>(
        r#"
        UPDATE payments
        SET date            = $2,
            amount          = $3,
            direction       = $4,
            counterparty_id = $5,
            bank_name       = $6,
            bank_ref        = $7,
            description     = $8,
            updated_at      = NOW()
        WHERE id = $1
        RETURNING id, company_id, date, amount, direction, counterparty_id,
                  bank_name, bank_ref, description, is_reconciled, bas_id,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(data.date)
    .bind(data.amount)
    .bind(data.direction.as_str())
    .bind(data.counterparty_id)
    .bind(data.bank_name)
    .bind(data.bank_ref)
    .bind(data.description)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Позначити платіж як зведений (is_reconciled = true).
pub async fn mark_reconciled(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE payments SET is_reconciled = TRUE, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Видалити платіж.
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM payments WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Прив'язати платіж до акту (часткова оплата).
pub async fn link_act(
    pool: &PgPool,
    payment_id: Uuid,
    act_id: Uuid,
    amount: rust_decimal::Decimal,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO payment_acts (payment_id, act_id, amount)
        VALUES ($1, $2, $3)
        ON CONFLICT (payment_id, act_id) DO UPDATE SET amount = $3
        "#,
    )
    .bind(payment_id)
    .bind(act_id)
    .bind(amount)
    .execute(pool)
    .await?;
    Ok(())
}

/// Прив'язати платіж до накладної.
pub async fn link_invoice(
    pool: &PgPool,
    payment_id: Uuid,
    invoice_id: Uuid,
    amount: rust_decimal::Decimal,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO payment_invoices (payment_id, invoice_id, amount)
        VALUES ($1, $2, $3)
        ON CONFLICT (payment_id, invoice_id) DO UPDATE SET amount = $3
        "#,
    )
    .bind(payment_id)
    .bind(invoice_id)
    .bind(amount)
    .execute(pool)
    .await?;
    Ok(())
}

/// Список запланованих платежів (невиконаних) для Dashboard.
pub async fn list_upcoming_schedule(
    pool: &PgPool,
    company_id: Uuid,
    limit: i64,
) -> Result<Vec<PaymentSchedule>> {
    let rows = sqlx::query_as::<_, PaymentSchedule>(
        r#"
        SELECT id, company_id, title, amount, direction, scheduled_date,
               recurrence, recurrence_end, counterparty_id, notes,
               is_completed, created_at, updated_at
        FROM payment_schedule
        WHERE company_id = $1
          AND is_completed = FALSE
          AND scheduled_date >= CURRENT_DATE
        ORDER BY scheduled_date ASC
        LIMIT $2
        "#,
    )
    .bind(company_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Створити запланований платіж.
pub async fn create_schedule(
    pool: &PgPool,
    data: NewPaymentSchedule,
) -> Result<PaymentSchedule> {
    let row = sqlx::query_as::<_, PaymentSchedule>(
        r#"
        INSERT INTO payment_schedule
            (company_id, title, amount, direction, scheduled_date,
             recurrence, recurrence_end, counterparty_id, notes)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, company_id, title, amount, direction, scheduled_date,
                  recurrence, recurrence_end, counterparty_id, notes,
                  is_completed, created_at, updated_at
        "#,
    )
    .bind(data.company_id)
    .bind(data.title)
    .bind(data.amount)
    .bind(data.direction.as_str())
    .bind(data.scheduled_date)
    .bind(data.recurrence.as_str())
    .bind(data.recurrence_end)
    .bind(data.counterparty_id)
    .bind(data.notes)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Позначити запланований платіж як виконаний.
pub async fn complete_schedule(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE payment_schedule SET is_completed = TRUE, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
