// CRUD для платежів.
//
// Використовує runtime-style sqlx::query_as::<_, T>() без compile-time макросів.

use anyhow::Result;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::models::payment::{
    NewPayment, NewPaymentSchedule, Payment, PaymentDirection, PaymentListRow, PaymentSchedule,
    UpdatePayment,
};
use crate::services::payment_matching::MatchCandidate;

/// KPI-агрегати для верхньої смужки екрана платежів.
pub struct PaymentKpi {
    pub incoming_month: rust_decimal::Decimal,
    pub outgoing_month: rust_decimal::Decimal,
    pub unmatched_count: i64,
}

/// Один allocation для атомарного split reconcile.
pub struct PaymentReconcileAllocation {
    pub document_kind: String,
    pub document_id: Uuid,
    pub amount: rust_decimal::Decimal,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Payment {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            company_id: row.try_get("company_id")?,
            date: row.try_get("date")?,
            amount: row.try_get("amount")?,
            direction: row.try_get("direction")?,
            counterparty_id: row.try_get("counterparty_id")?,
            bank_name: row.try_get("bank_name")?,
            bank_ref: row.try_get("bank_ref")?,
            description: row.try_get("description")?,
            is_reconciled: row.try_get("is_reconciled")?,
            bas_id: row.try_get("bas_id")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for PaymentSchedule {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            company_id: row.try_get("company_id")?,
            title: row.try_get("title")?,
            amount: row.try_get("amount")?,
            direction: row.try_get("direction")?,
            scheduled_date: row.try_get("scheduled_date")?,
            recurrence: row.try_get("recurrence")?,
            recurrence_end: row.try_get("recurrence_end")?,
            counterparty_id: row.try_get("counterparty_id")?,
            notes: row.try_get("notes")?,
            is_completed: row.try_get("is_completed")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
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
        id: Uuid,
        date: String,
        amount: rust_decimal::Decimal,
        direction: PaymentDirection,
        counterparty_id: Option<Uuid>,
        counterparty_name: Option<String>,
        bank_name: Option<String>,
        description: Option<String>,
        is_reconciled: bool,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                id: r.try_get("id")?,
                date: r.try_get("date")?,
                amount: r.try_get("amount")?,
                direction: r.try_get("direction")?,
                counterparty_id: r.try_get("counterparty_id")?,
                counterparty_name: r.try_get("counterparty_name")?,
                bank_name: r.try_get("bank_name")?,
                description: r.try_get("description")?,
                is_reconciled: r.try_get("is_reconciled")?,
            })
        }
    }

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
    .bind(direction)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PaymentListRow {
            id: r.id,
            date: r.date,
            amount: r.amount,
            direction: r.direction,
            counterparty_id: r.counterparty_id,
            counterparty_name: r.counterparty_name,
            bank_name: r.bank_name,
            description: r.description,
            is_reconciled: r.is_reconciled,
        })
        .collect())
}

/// Агреговані KPI платежів за поточний місяць.
pub async fn payment_kpi(pool: &PgPool, company_id: Uuid) -> Result<PaymentKpi> {
    struct Row {
        incoming_month: rust_decimal::Decimal,
        outgoing_month: rust_decimal::Decimal,
        unmatched_count: i64,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                incoming_month: r.try_get("incoming_month")?,
                outgoing_month: r.try_get("outgoing_month")?,
                unmatched_count: r.try_get("unmatched_count")?,
            })
        }
    }

    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            COALESCE(SUM(amount) FILTER (
                WHERE direction = 'income'
                  AND date_trunc('month', date) = date_trunc('month', CURRENT_DATE)
            ), 0) AS incoming_month,
            COALESCE(SUM(amount) FILTER (
                WHERE direction = 'expense'
                  AND date_trunc('month', date) = date_trunc('month', CURRENT_DATE)
            ), 0) AS outgoing_month,
            COUNT(*) FILTER (WHERE is_reconciled = FALSE) AS unmatched_count
        FROM payments
        WHERE company_id = $1
        "#,
    )
    .bind(company_id)
    .fetch_one(pool)
    .await?;

    Ok(PaymentKpi {
        incoming_month: row.incoming_month,
        outgoing_month: row.outgoing_month,
        unmatched_count: row.unmatched_count,
    })
}

/// Перевіряє, чи платіж із таким підписом уже імпортовано.
pub async fn exists_imported_row(
    pool: &PgPool,
    company_id: Uuid,
    date: chrono::NaiveDate,
    amount: rust_decimal::Decimal,
    direction: PaymentDirection,
    bank_ref: Option<&str>,
    description: &str,
) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM payments
            WHERE company_id = $1
              AND date = $2
              AND amount = $3
              AND direction = $4
              AND (
                    ($5::text IS NOT NULL AND bank_ref = $5::text)
                 OR ($5::text IS NULL AND COALESCE(description, '') = $6)
              )
        )
        "#,
    )
    .bind(company_id)
    .bind(date)
    .bind(amount)
    .bind(direction)
    .bind(bank_ref)
    .bind(description)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

/// Отримати платіж за ID.
pub async fn list_by_counterparty(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
) -> Result<Vec<PaymentListRow>> {
    Ok(list(pool, company_id, None)
        .await?
        .into_iter()
        .filter(|row| row.counterparty_id == Some(counterparty_id))
        .collect())
}

/// Отримати платіж за ID у межах конкретної компанії.
pub async fn get_by_id_scoped(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
) -> Result<Option<Payment>> {
    let row = sqlx::query_as::<_, Payment>(
        r#"
        SELECT id, company_id, date, amount, direction, counterparty_id,
               bank_name, bank_ref, description, is_reconciled, bas_id,
               created_at, updated_at
        FROM payments
        WHERE id = $1 AND company_id = $2
        "#,
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Завантажити всі відкриті документи-кандидати для matching напряму платежу.
pub async fn list_open_document_candidates(
    pool: &PgPool,
    company_id: Uuid,
    direction: PaymentDirection,
) -> Result<Vec<MatchCandidate>> {
    let mut candidates =
        crate::db::acts::list_open_act_candidates(pool, company_id, direction.clone()).await?;
    candidates.extend(
        crate::db::invoices::list_open_invoice_candidates(pool, company_id, direction).await?,
    );
    Ok(candidates)
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
    .bind(data.direction)
    .bind(data.counterparty_id)
    .bind(data.bank_name)
    .bind(data.bank_ref)
    .bind(data.description)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Оновити платіж.
/// Оновити платіж лише в межах конкретної компанії.
pub async fn update_scoped(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
    data: UpdatePayment,
) -> Result<Option<Payment>> {
    let row = sqlx::query_as::<_, Payment>(
        r#"
        UPDATE payments
        SET date            = $3,
            amount          = $4,
            direction       = $5,
            counterparty_id = $6,
            bank_name       = $7,
            bank_ref        = $8,
            description     = $9,
            updated_at      = NOW()
        WHERE id = $1
          AND company_id = $2
        RETURNING id, company_id, date, amount, direction, counterparty_id,
                  bank_name, bank_ref, description, is_reconciled, bas_id,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(company_id)
    .bind(data.date)
    .bind(data.amount)
    .bind(data.direction)
    .bind(data.counterparty_id)
    .bind(data.bank_name)
    .bind(data.bank_ref)
    .bind(data.description)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Видалити платіж.
/// Видалити платіж лише в межах конкретної компанії.
pub async fn delete_scoped(pool: &PgPool, company_id: Uuid, id: Uuid) -> Result<bool> {
    let affected = sqlx::query("DELETE FROM payments WHERE id = $1 AND company_id = $2")
        .bind(id)
        .bind(company_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(affected > 0)
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

/// Прив'язати платіж до документа (акт або накладна) і позначити як звірений.
async fn ensure_act_in_company(pool: &PgPool, company_id: Uuid, act_id: Uuid) -> Result<()> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM acts WHERE id = $1 AND company_id = $2)",
    )
    .bind(act_id)
    .bind(company_id)
    .fetch_one(pool)
    .await?;
    anyhow::ensure!(exists, "Документ не знайдено в межах компанії");
    Ok(())
}

async fn ensure_act_in_company_tx(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    act_id: Uuid,
) -> Result<()> {
    let exists = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM acts WHERE id = $1 AND company_id = $2 FOR UPDATE",
    )
    .bind(act_id)
    .bind(company_id)
    .fetch_optional(&mut **tx)
    .await?;
    anyhow::ensure!(exists.is_some(), "Документ не знайдено в межах компанії");
    Ok(())
}

async fn ensure_invoice_in_company(
    pool: &PgPool,
    company_id: Uuid,
    invoice_id: Uuid,
) -> Result<()> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM invoices WHERE id = $1 AND company_id = $2)",
    )
    .bind(invoice_id)
    .bind(company_id)
    .fetch_one(pool)
    .await?;
    anyhow::ensure!(exists, "Документ не знайдено в межах компанії");
    Ok(())
}

async fn ensure_invoice_in_company_tx(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    invoice_id: Uuid,
) -> Result<()> {
    let exists = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM invoices WHERE id = $1 AND company_id = $2 FOR UPDATE",
    )
    .bind(invoice_id)
    .bind(company_id)
    .fetch_optional(&mut **tx)
    .await?;
    anyhow::ensure!(exists.is_some(), "Документ не знайдено в межах компанії");
    Ok(())
}

/// Read payment amount within a transaction with `FOR UPDATE` to lock the row,
/// preventing concurrent reconcile/split on the same payment.
async fn payment_amount_scoped_tx_locked(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    payment_id: Uuid,
) -> Result<rust_decimal::Decimal> {
    let amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payments WHERE id = $1 AND company_id = $2 FOR UPDATE",
    )
    .bind(payment_id)
    .bind(company_id)
    .fetch_optional(&mut **tx)
    .await?;
    amount.ok_or_else(|| anyhow::anyhow!("Платіж не знайдено в межах компанії"))
}

async fn act_available_amount_for_payment_tx(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    payment_id: Uuid,
    act_id: Uuid,
) -> Result<rust_decimal::Decimal> {
    let amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        r#"
        SELECT a.total_amount - COALESCE(SUM(pa.amount) FILTER (WHERE pa.payment_id <> $3), 0::numeric)
        FROM acts a
        LEFT JOIN payment_acts pa ON pa.act_id = a.id
        WHERE a.id = $1 AND a.company_id = $2
        GROUP BY a.id, a.total_amount
        "#,
    )
    .bind(act_id)
    .bind(company_id)
    .bind(payment_id)
    .fetch_optional(&mut **tx)
    .await?;
    amount.ok_or_else(|| anyhow::anyhow!("Документ не знайдено в межах компанії"))
}

async fn invoice_available_amount_for_payment_tx(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    payment_id: Uuid,
    invoice_id: Uuid,
) -> Result<rust_decimal::Decimal> {
    let amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        r#"
        SELECT i.total_amount - COALESCE(SUM(pi.amount) FILTER (WHERE pi.payment_id <> $3), 0::numeric)
        FROM invoices i
        LEFT JOIN payment_invoices pi ON pi.invoice_id = i.id
        WHERE i.id = $1 AND i.company_id = $2
        GROUP BY i.id, i.total_amount
        "#,
    )
    .bind(invoice_id)
    .bind(company_id)
    .bind(payment_id)
    .fetch_optional(&mut **tx)
    .await?;
    amount.ok_or_else(|| anyhow::anyhow!("Документ не знайдено в межах компанії"))
}

pub async fn reconcile_document_scoped(
    pool: &PgPool,
    company_id: Uuid,
    payment_id: Uuid,
    doc_kind: &str,
    doc_id: Uuid,
    amount: rust_decimal::Decimal,
) -> Result<()> {
    reconcile_split_scoped(
        pool,
        company_id,
        payment_id,
        &[PaymentReconcileAllocation {
            document_kind: doc_kind.to_string(),
            document_id: doc_id,
            amount,
        }],
    )
    .await
}

/// Від'єднати платіж від документа і перерахувати статус звірки.
pub async fn unreconcile_document_scoped(
    pool: &PgPool,
    company_id: Uuid,
    payment_id: Uuid,
    doc_kind: &str,
    doc_id: Uuid,
) -> Result<()> {
    let owns_payment = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM payments WHERE id = $1 AND company_id = $2)",
    )
    .bind(payment_id)
    .bind(company_id)
    .fetch_one(pool)
    .await?;
    anyhow::ensure!(owns_payment, "Платіж не знайдено в межах компанії");

    match doc_kind {
        "act" => {
            ensure_act_in_company(pool, company_id, doc_id).await?;
            sqlx::query("DELETE FROM payment_acts WHERE payment_id = $1 AND act_id = $2")
                .bind(payment_id)
                .bind(doc_id)
                .execute(pool)
                .await?;
        }
        "invoice" => {
            ensure_invoice_in_company(pool, company_id, doc_id).await?;
            sqlx::query("DELETE FROM payment_invoices WHERE payment_id = $1 AND invoice_id = $2")
                .bind(payment_id)
                .bind(doc_id)
                .execute(pool)
                .await?;
        }
        other => anyhow::bail!("Невідомий тип документу: {other}"),
    }

    refresh_reconciled_state(pool, payment_id).await?;

    Ok(())
}

/// Зняти всі зв'язки звірки з платежу в межах компанії та перерахувати derived state.
pub async fn unreconcile_all_scoped(
    pool: &PgPool,
    company_id: Uuid,
    payment_id: Uuid,
) -> Result<()> {
    let owns_payment = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM payments WHERE id = $1 AND company_id = $2)",
    )
    .bind(payment_id)
    .bind(company_id)
    .fetch_one(pool)
    .await?;
    anyhow::ensure!(owns_payment, "Платіж не знайдено в межах компанії");

    sqlx::query("DELETE FROM payment_acts WHERE payment_id = $1")
        .bind(payment_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM payment_invoices WHERE payment_id = $1")
        .bind(payment_id)
        .execute(pool)
        .await?;

    refresh_reconciled_state(pool, payment_id).await?;

    Ok(())
}

/// Позначити платіж як звірений без зміни allocation-зв'язків.
///
/// Використовується сумісним legacy surface, де звірка могла бути ручним toggle.
pub async fn mark_reconciled_scoped(
    pool: &PgPool,
    company_id: Uuid,
    payment_id: Uuid,
) -> Result<bool> {
    let affected = sqlx::query(
        r#"
        UPDATE payments
        SET is_reconciled = TRUE,
            updated_at = NOW()
        WHERE id = $1 AND company_id = $2
        "#,
    )
    .bind(payment_id)
    .bind(company_id)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(affected > 0)
}

/// Зняти ручну позначку звірки без зміни allocation-зв'язків.
pub async fn mark_unreconciled_scoped(
    pool: &PgPool,
    company_id: Uuid,
    payment_id: Uuid,
) -> Result<bool> {
    let affected = sqlx::query(
        r#"
        UPDATE payments
        SET is_reconciled = FALSE,
            updated_at = NOW()
        WHERE id = $1 AND company_id = $2
        "#,
    )
    .bind(payment_id)
    .bind(company_id)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(affected > 0)
}

/// Атомарно замінити всі links платежу на новий набір allocation-ів.
///
/// Уся валідація (існування платежу і документів, доступні залишки) виконується
/// всередині транзакції; рядок `payments` блокується через `SELECT … FOR UPDATE`.
/// Це усуває TOCTOU race з паралельними reconcile-операціями: snapshot available
/// amounts і запис у `payment_acts` / `payment_invoices` виконуються під одним
/// логічним lock-ом, тому один платіж не може бути over-allocated через гонку.
pub async fn reconcile_split_scoped(
    pool: &PgPool,
    company_id: Uuid,
    payment_id: Uuid,
    allocations: &[PaymentReconcileAllocation],
) -> Result<()> {
    anyhow::ensure!(
        !allocations.is_empty(),
        "Для split reconcile потрібен хоча б один allocation"
    );

    let mut tx = pool.begin().await?;

    let payment_amount = payment_amount_scoped_tx_locked(&mut tx, company_id, payment_id).await?;
    let total_allocated = allocations
        .iter()
        .fold(rust_decimal::Decimal::ZERO, |sum, allocation| {
            sum + allocation.amount
        });
    anyhow::ensure!(
        total_allocated <= payment_amount,
        "Сума звірки перевищує суму платежу"
    );

    // Видаляємо старі links перед перевіркою available amounts, щоб залишок
    // обчислювався без урахування поточних allocation-ів цього самого платежу.
    sqlx::query("DELETE FROM payment_acts WHERE payment_id = $1")
        .bind(payment_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM payment_invoices WHERE payment_id = $1")
        .bind(payment_id)
        .execute(&mut *tx)
        .await?;

    for allocation in allocations {
        match allocation.document_kind.as_str() {
            "act" => {
                ensure_act_in_company_tx(&mut tx, company_id, allocation.document_id).await?;
                let available = act_available_amount_for_payment_tx(
                    &mut tx,
                    company_id,
                    payment_id,
                    allocation.document_id,
                )
                .await?;
                anyhow::ensure!(
                    allocation.amount <= available,
                    "Сума звірки перевищує доступний залишок документа"
                );
                sqlx::query(
                    r#"
                    INSERT INTO payment_acts (payment_id, act_id, amount)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (payment_id, act_id) DO UPDATE SET amount = $3
                    "#,
                )
                .bind(payment_id)
                .bind(allocation.document_id)
                .bind(allocation.amount)
                .execute(&mut *tx)
                .await?;
            }
            "invoice" => {
                ensure_invoice_in_company_tx(&mut tx, company_id, allocation.document_id).await?;
                let available = invoice_available_amount_for_payment_tx(
                    &mut tx,
                    company_id,
                    payment_id,
                    allocation.document_id,
                )
                .await?;
                anyhow::ensure!(
                    allocation.amount <= available,
                    "Сума звірки перевищує доступний залишок документа"
                );
                sqlx::query(
                    r#"
                    INSERT INTO payment_invoices (payment_id, invoice_id, amount)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (payment_id, invoice_id) DO UPDATE SET amount = $3
                    "#,
                )
                .bind(payment_id)
                .bind(allocation.document_id)
                .bind(allocation.amount)
                .execute(&mut *tx)
                .await?;
            }
            other => anyhow::bail!("Невідомий тип документу: {other}"),
        }
    }

    refresh_reconciled_state_tx(&mut tx, payment_id).await?;
    tx.commit().await?;

    Ok(())
}

async fn refresh_reconciled_state(pool: &PgPool, payment_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE payments
        SET is_reconciled = (
                EXISTS(SELECT 1 FROM payment_acts WHERE payment_id = $1)
                OR EXISTS(SELECT 1 FROM payment_invoices WHERE payment_id = $1)
            ),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(payment_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn refresh_reconciled_state_tx(
    tx: &mut Transaction<'_, Postgres>,
    payment_id: Uuid,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE payments
        SET is_reconciled = (
                EXISTS(SELECT 1 FROM payment_acts WHERE payment_id = $1)
                OR EXISTS(SELECT 1 FROM payment_invoices WHERE payment_id = $1)
            ),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(payment_id)
    .execute(&mut **tx)
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

/// Список запланованих платежів у межах діапазону дат.
pub async fn list_schedule_in_range(
    pool: &PgPool,
    company_id: Uuid,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<Vec<PaymentSchedule>> {
    let rows = sqlx::query_as::<_, PaymentSchedule>(
        r#"
        SELECT id, company_id, title, amount, direction, scheduled_date,
               recurrence, recurrence_end, counterparty_id, notes,
               is_completed, created_at, updated_at
        FROM payment_schedule
        WHERE company_id = $1
          AND scheduled_date BETWEEN $2 AND $3
        ORDER BY scheduled_date ASC, created_at ASC
        "#,
    )
    .bind(company_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Створити запланований платіж.
pub async fn create_schedule(pool: &PgPool, data: NewPaymentSchedule) -> Result<PaymentSchedule> {
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
    .bind(data.direction)
    .bind(data.scheduled_date)
    .bind(data.recurrence)
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

/// Позначити запланований платіж як виконаний лише в межах конкретної компанії.
pub async fn complete_schedule_scoped(pool: &PgPool, company_id: Uuid, id: Uuid) -> Result<bool> {
    let affected = sqlx::query(
        r#"
        UPDATE payment_schedule
        SET is_completed = TRUE,
            updated_at = NOW()
        WHERE id = $1
          AND company_id = $2
        "#,
    )
    .bind(id)
    .bind(company_id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(dead_code)]
    fn all_functions_compile() {
        let _ = list;
        let _ = payment_kpi;
        let _ = exists_imported_row;
        let _ = list_by_counterparty;
        let _ = get_by_id_scoped;
        let _ = list_open_document_candidates;
        let _ = create;
        let _ = update_scoped;
        let _ = delete_scoped;
        let _ = link_act;
        let _ = link_invoice;
        let _ = reconcile_document_scoped;
        let _ = unreconcile_document_scoped;
        let _ = unreconcile_all_scoped;
        let _ = mark_reconciled_scoped;
        let _ = mark_unreconciled_scoped;
        let _ = reconcile_split_scoped;
        let _ = list_upcoming_schedule;
        let _ = list_schedule_in_range;
        let _ = create_schedule;
        let _ = complete_schedule;
        let _ = complete_schedule_scoped;
    }
}
