use anyhow::Result;
use chrono::NaiveDate;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::app_ctx::AppCtx;
use crate::models::payment::PaymentDirection;
use crate::models::reports::{
    BankAggregateRow, PayableRow, ReceivableRow, ReportsScope, ResolvedReportsFilter,
    TopCounterpartyRow,
};

fn selected_counterparty_uuid(filter: &ResolvedReportsFilter) -> Option<uuid::Uuid> {
    filter
        .selected_counterparty_id
        .as_deref()
        .and_then(|v| uuid::Uuid::parse_str(v).ok())
}

fn query_matches(haystacks: &[&str], query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let query = query.to_lowercase();
    haystacks
        .iter()
        .any(|value| value.to_lowercase().contains(&query))
}

pub async fn load_bank_rows(
    ctx: &AppCtx,
    filter: &ResolvedReportsFilter,
) -> Result<Vec<BankAggregateRow>> {
    struct Row {
        key: String,
        label: String,
        income: Decimal,
        expense: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;

            Ok(Self {
                key: row.try_get("key")?,
                label: row.try_get("label")?,
                income: row.try_get("income")?,
                expense: row.try_get("expense")?,
            })
        }
    }

    let company_id = match filter.scope {
        ReportsScope::Active => Some(ctx.company_id()),
        ReportsScope::All => None,
    };

    let selected_cp_id = selected_counterparty_uuid(filter);

    let rows = if matches!(filter.scope, ReportsScope::All) {
        sqlx::query_as::<_, Row>(
            r#"
            SELECT
                c.id::text AS key,
                c.name AS label,
                COALESCE(SUM(CASE WHEN p.direction = 'income' THEN p.amount ELSE 0 END), 0) AS income,
                COALESCE(SUM(CASE WHEN p.direction = 'expense' THEN p.amount ELSE 0 END), 0) AS expense
            FROM companies c
            LEFT JOIN payments p
                ON p.company_id = c.id
               AND p.date BETWEEN $1 AND $2
               AND ($3::uuid IS NULL OR p.counterparty_id = $3::uuid)
            GROUP BY c.id, c.name
            ORDER BY c.name
            "#,
        )
        .bind(filter.date_from)
        .bind(filter.date_to)
        .bind(selected_cp_id)
        .fetch_all(ctx.pool())
        .await?
    } else {
        sqlx::query_as::<_, Row>(
            r#"
            SELECT
                COALESCE(cp.id::text, 'uncategorized') AS key,
                COALESCE(cp.name, COALESCE(p.bank_name, 'Без контрагента')) AS label,
                COALESCE(SUM(CASE WHEN p.direction = 'income' THEN p.amount ELSE 0 END), 0) AS income,
                COALESCE(SUM(CASE WHEN p.direction = 'expense' THEN p.amount ELSE 0 END), 0) AS expense
            FROM payments p
            LEFT JOIN counterparties cp ON cp.id = p.counterparty_id
            WHERE p.company_id = $1
              AND p.date BETWEEN $2 AND $3
              AND ($4::uuid IS NULL OR p.counterparty_id = $4::uuid)
            GROUP BY cp.id, cp.name, p.bank_name
            ORDER BY label
            "#,
        )
        .bind(company_id.ok_or_else(|| anyhow::anyhow!("active scope потребує активної компанії"))?)
        .bind(filter.date_from)
        .bind(filter.date_to)
        .bind(selected_cp_id)
        .fetch_all(ctx.pool())
        .await?
    };

    Ok(rows
        .into_iter()
        .filter(|row| query_matches(&[&row.label], &filter.query))
        .map(|row| BankAggregateRow {
            key: row.key,
            label: row.label,
            income: row.income,
            expense: row.expense,
        })
        .collect())
}

pub async fn load_pnl_rows(
    ctx: &AppCtx,
    filter: &ResolvedReportsFilter,
) -> Result<Vec<BankAggregateRow>> {
    struct Row {
        key: String,
        label: String,
        income: Decimal,
        expense: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;

            Ok(Self {
                key: row.try_get("key")?,
                label: row.try_get("label")?,
                income: row.try_get("income")?,
                expense: row.try_get("expense")?,
            })
        }
    }

    let company_id = match filter.scope {
        ReportsScope::Active => Some(ctx.company_id()),
        ReportsScope::All => None,
    };

    let selected_cp_id = selected_counterparty_uuid(filter);

    let rows = sqlx::query_as::<_, Row>(
        r#"
        WITH docs AS (
            SELECT
                COALESCE(cat.id::text, 'uncategorized') AS key,
                COALESCE(cat.name, 'Без категорії') AS label,
                COALESCE(cat.kind::text, 'income') AS category_kind,
                a.total_amount AS amount
            FROM acts a
            LEFT JOIN categories cat ON cat.id = a.category_id
            WHERE ($1::uuid IS NULL OR a.company_id = $1::uuid)
              AND a.date BETWEEN $2 AND $3
              AND a.status != 'draft'
              AND ($4::uuid IS NULL OR a.counterparty_id = $4::uuid)

            UNION ALL

            SELECT
                COALESCE(cat.id::text, 'uncategorized') AS key,
                COALESCE(cat.name, 'Без категорії') AS label,
                COALESCE(cat.kind::text, 'income') AS category_kind,
                i.total_amount AS amount
            FROM invoices i
            LEFT JOIN categories cat ON cat.id = i.category_id
            WHERE ($1::uuid IS NULL OR i.company_id = $1::uuid)
              AND i.date BETWEEN $2 AND $3
              AND i.status != 'draft'
              AND ($4::uuid IS NULL OR i.counterparty_id = $4::uuid)
        )
        SELECT
            key,
            label,
            COALESCE(SUM(CASE WHEN category_kind = 'expense' THEN 0 ELSE amount END), 0) AS income,
            COALESCE(SUM(CASE WHEN category_kind = 'expense' THEN amount ELSE 0 END), 0) AS expense
        FROM docs
        GROUP BY key, label
        ORDER BY label
        "#,
    )
    .bind(company_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .bind(selected_cp_id)
    .fetch_all(ctx.pool())
    .await?;

    Ok(rows
        .into_iter()
        .filter(|row| query_matches(&[&row.label], &filter.query))
        .map(|row| BankAggregateRow {
            key: row.key,
            label: row.label,
            income: row.income,
            expense: row.expense,
        })
        .collect())
}

pub async fn load_receivables_rows(
    ctx: &AppCtx,
    filter: &ResolvedReportsFilter,
) -> Result<Vec<ReceivableRow>> {
    struct Row {
        doc_id: String,
        doc_type: String,
        doc_number: String,
        doc_date: NaiveDate,
        company_name: String,
        counterparty: String,
        amount: Decimal,
        expected_date: Option<NaiveDate>,
        overdue_days: Option<i32>,
        status: String,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;

            Ok(Self {
                doc_id: row.try_get("doc_id")?,
                doc_type: row.try_get("doc_type")?,
                doc_number: row.try_get("doc_number")?,
                doc_date: row.try_get("doc_date")?,
                company_name: row.try_get("company_name")?,
                counterparty: row.try_get("counterparty")?,
                amount: row.try_get("amount")?,
                expected_date: row.try_get("expected_date")?,
                overdue_days: row.try_get("overdue_days")?,
                status: row.try_get("status")?,
            })
        }
    }

    let company_id = match filter.scope {
        ReportsScope::Active => Some(ctx.company_id()),
        ReportsScope::All => None,
    };

    let selected_cp_id = selected_counterparty_uuid(filter);

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            a.id::TEXT AS doc_id,
            'act'::TEXT AS doc_type,
            a.number AS doc_number,
            a.date AS doc_date,
            c.name AS company_name,
            cp.name AS counterparty,
            a.total_amount AS amount,
            a.expected_payment_date AS expected_date,
            CASE
                WHEN a.expected_payment_date IS NOT NULL
                THEN GREATEST(0, ($2::date - a.expected_payment_date))
                ELSE 0
            END::int AS overdue_days,
            a.status::TEXT AS status
        FROM acts a
        JOIN companies c ON c.id = a.company_id
        JOIN counterparties cp ON cp.id = a.counterparty_id
        WHERE ($1::uuid IS NULL OR a.company_id = $1::uuid)
          AND a.date BETWEEN $3 AND $4
          AND a.status NOT IN ('paid', 'draft')
          AND ($5::uuid IS NULL OR a.counterparty_id = $5::uuid)

        UNION ALL

        SELECT
            i.id::TEXT AS doc_id,
            'invoice'::TEXT AS doc_type,
            i.number AS doc_number,
            i.date AS doc_date,
            c.name AS company_name,
            cp.name AS counterparty,
            i.total_amount AS amount,
            i.expected_payment_date AS expected_date,
            CASE
                WHEN i.expected_payment_date IS NOT NULL
                THEN GREATEST(0, ($2::date - i.expected_payment_date))
                ELSE 0
            END::int AS overdue_days,
            i.status::TEXT AS status
        FROM invoices i
        JOIN companies c ON c.id = i.company_id
        JOIN counterparties cp ON cp.id = i.counterparty_id
        WHERE ($1::uuid IS NULL OR i.company_id = $1::uuid)
          AND i.date BETWEEN $3 AND $4
          AND i.status NOT IN ('paid', 'draft')
          AND ($5::uuid IS NULL OR i.counterparty_id = $5::uuid)

        ORDER BY overdue_days DESC, doc_date ASC
        "#,
    )
    .bind(company_id)
    .bind(filter.date_to)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .bind(selected_cp_id)
    .fetch_all(ctx.pool())
    .await?;

    Ok(rows
        .into_iter()
        .filter(|row| {
            query_matches(
                &[
                    &row.doc_number,
                    &row.counterparty,
                    &row.company_name,
                    &row.status,
                ],
                &filter.query,
            )
        })
        .map(|row| ReceivableRow {
            doc_id: row.doc_id,
            doc_type: row.doc_type,
            doc_number: row.doc_number,
            doc_date: row.doc_date,
            company_name: row.company_name,
            counterparty: row.counterparty,
            amount: row.amount,
            expected_date: row.expected_date,
            overdue_days: row.overdue_days.unwrap_or(0),
            status: row.status,
        })
        .collect())
}

pub async fn load_payables_rows(
    ctx: &AppCtx,
    filter: &ResolvedReportsFilter,
) -> Result<Vec<PayableRow>> {
    struct Row {
        id: String,
        title: String,
        company_name: String,
        counterparty: Option<String>,
        amount: Option<Decimal>,
        due_date: NaiveDate,
        overdue_days: i32,
        recurrence: String,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;

            Ok(Self {
                id: row.try_get("id")?,
                title: row.try_get("title")?,
                company_name: row.try_get("company_name")?,
                counterparty: row.try_get("counterparty")?,
                amount: row.try_get("amount")?,
                due_date: row.try_get("due_date")?,
                overdue_days: row.try_get("overdue_days")?,
                recurrence: row.try_get("recurrence")?,
            })
        }
    }

    let company_id = match filter.scope {
        ReportsScope::Active => Some(ctx.company_id()),
        ReportsScope::All => None,
    };

    let selected_cp_id = selected_counterparty_uuid(filter);

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            ps.id::TEXT AS id,
            ps.title AS title,
            c.name AS company_name,
            cp.name AS counterparty,
            ps.amount AS amount,
            ps.scheduled_date AS due_date,
            GREATEST(0, ($2::date - ps.scheduled_date))::int AS overdue_days,
            ps.recurrence::TEXT AS recurrence
        FROM payment_schedule ps
        JOIN companies c ON c.id = ps.company_id
        LEFT JOIN counterparties cp ON cp.id = ps.counterparty_id
        WHERE ($1::uuid IS NULL OR ps.company_id = $1::uuid)
          AND ps.direction = $3
          AND ps.is_completed = FALSE
          AND ps.scheduled_date BETWEEN $4 AND $5
          AND ($6::uuid IS NULL OR ps.counterparty_id = $6::uuid)
        ORDER BY ps.scheduled_date ASC
        "#,
    )
    .bind(company_id)
    .bind(filter.date_to)
    .bind(PaymentDirection::Expense)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .bind(selected_cp_id)
    .fetch_all(ctx.pool())
    .await?;

    Ok(rows
        .into_iter()
        .filter(|row| {
            query_matches(
                &[
                    &row.title,
                    &row.company_name,
                    row.counterparty.as_deref().unwrap_or(""),
                ],
                &filter.query,
            )
        })
        .map(|row| PayableRow {
            id: row.id,
            title: row.title,
            company_name: row.company_name,
            counterparty: row.counterparty.unwrap_or_default(),
            amount: row.amount.unwrap_or(Decimal::ZERO),
            due_date: row.due_date,
            overdue_days: row.overdue_days,
            recurrence: row.recurrence,
        })
        .collect())
}

pub async fn load_top_counterparties_bank(
    ctx: &AppCtx,
    filter: &ResolvedReportsFilter,
) -> Result<Vec<TopCounterpartyRow>> {
    struct Row {
        counterparty_id: String,
        counterparty_name: String,
        income: Decimal,
        expense: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;

            Ok(Self {
                counterparty_id: row.try_get("counterparty_id")?,
                counterparty_name: row.try_get("counterparty_name")?,
                income: row.try_get("income")?,
                expense: row.try_get("expense")?,
            })
        }
    }

    let company_id = match filter.scope {
        ReportsScope::Active => Some(ctx.company_id()),
        ReportsScope::All => None,
    };

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            cp.id::text AS counterparty_id,
            cp.name AS counterparty_name,
            COALESCE(SUM(CASE WHEN p.direction = 'income' THEN p.amount ELSE 0 END), 0) AS income,
            COALESCE(SUM(CASE WHEN p.direction = 'expense' THEN p.amount ELSE 0 END), 0) AS expense
        FROM payments p
        JOIN counterparties cp ON cp.id = p.counterparty_id
        WHERE ($1::uuid IS NULL OR p.company_id = $1::uuid)
          AND p.date BETWEEN $2 AND $3
        GROUP BY cp.id, cp.name
        ORDER BY (COALESCE(SUM(CASE WHEN p.direction = 'income' THEN p.amount ELSE 0 END), 0)
                + COALESCE(SUM(CASE WHEN p.direction = 'expense' THEN p.amount ELSE 0 END), 0)) DESC,
                 cp.name ASC
        LIMIT 8
        "#,
    )
    .bind(company_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .fetch_all(ctx.pool())
    .await?;

    let max_primary = rows
        .iter()
        .map(|r| r.income + r.expense)
        .max()
        .unwrap_or(Decimal::ZERO);

    Ok(rows
        .into_iter()
        .map(|row| {
            let primary_amount = row.income + row.expense;
            let share_percent = if max_primary.is_zero() {
                0u8
            } else {
                ((primary_amount / max_primary * Decimal::from(100u8)).round_dp(0))
                    .min(Decimal::from(100u8))
                    .to_u8()
                    .unwrap_or(100)
            };
            TopCounterpartyRow {
                counterparty_id: row.counterparty_id,
                counterparty_name: row.counterparty_name,
                primary_amount,
                secondary_label: "Чистий рух".to_string(),
                secondary_value: row.income - row.expense,
                share_percent,
            }
        })
        .collect())
}

pub async fn load_top_counterparties_receivables(
    ctx: &AppCtx,
    filter: &ResolvedReportsFilter,
) -> Result<Vec<TopCounterpartyRow>> {
    struct Row {
        counterparty_id: String,
        counterparty_name: String,
        total: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;

            Ok(Self {
                counterparty_id: row.try_get("counterparty_id")?,
                counterparty_name: row.try_get("counterparty_name")?,
                total: row.try_get("total")?,
            })
        }
    }

    let company_id = match filter.scope {
        ReportsScope::Active => Some(ctx.company_id()),
        ReportsScope::All => None,
    };

    let rows = sqlx::query_as::<_, Row>(
        r#"
        WITH docs AS (
            SELECT a.counterparty_id, a.total_amount AS amount
            FROM acts a
            WHERE ($1::uuid IS NULL OR a.company_id = $1::uuid)
              AND a.date BETWEEN $2 AND $3
              AND a.status NOT IN ('paid', 'draft')
            UNION ALL
            SELECT i.counterparty_id, i.total_amount AS amount
            FROM invoices i
            WHERE ($1::uuid IS NULL OR i.company_id = $1::uuid)
              AND i.date BETWEEN $2 AND $3
              AND i.status NOT IN ('paid', 'draft')
        )
        SELECT
            cp.id::text AS counterparty_id,
            cp.name AS counterparty_name,
            COALESCE(SUM(d.amount), 0) AS total
        FROM docs d
        JOIN counterparties cp ON cp.id = d.counterparty_id
        GROUP BY cp.id, cp.name
        ORDER BY total DESC, cp.name ASC
        LIMIT 8
        "#,
    )
    .bind(company_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .fetch_all(ctx.pool())
    .await?;

    let max_primary = rows
        .iter()
        .map(|r| r.total)
        .max()
        .unwrap_or(Decimal::ZERO);

    Ok(rows
        .into_iter()
        .map(|row| {
            let share_percent = if max_primary.is_zero() {
                0u8
            } else {
                ((row.total / max_primary * Decimal::from(100u8)).round_dp(0))
                    .min(Decimal::from(100u8))
                    .to_u8()
                    .unwrap_or(100)
            };
            TopCounterpartyRow {
                counterparty_id: row.counterparty_id,
                counterparty_name: row.counterparty_name,
                primary_amount: row.total,
                secondary_label: "Дебіторська заборгованість".to_string(),
                secondary_value: row.total,
                share_percent,
            }
        })
        .collect())
}

pub async fn load_top_counterparties_payables(
    ctx: &AppCtx,
    filter: &ResolvedReportsFilter,
) -> Result<Vec<TopCounterpartyRow>> {
    struct Row {
        counterparty_id: String,
        counterparty_name: String,
        total: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;

            Ok(Self {
                counterparty_id: row.try_get("counterparty_id")?,
                counterparty_name: row.try_get("counterparty_name")?,
                total: row.try_get("total")?,
            })
        }
    }

    let company_id = match filter.scope {
        ReportsScope::Active => Some(ctx.company_id()),
        ReportsScope::All => None,
    };

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            cp.id::text AS counterparty_id,
            cp.name AS counterparty_name,
            COALESCE(SUM(ps.amount), 0) AS total
        FROM payment_schedule ps
        JOIN counterparties cp ON cp.id = ps.counterparty_id
        WHERE ($1::uuid IS NULL OR ps.company_id = $1::uuid)
          AND ps.direction = 'expense'
          AND ps.is_completed = FALSE
          AND ps.scheduled_date BETWEEN $2 AND $3
        GROUP BY cp.id, cp.name
        ORDER BY total DESC, cp.name ASC
        LIMIT 8
        "#,
    )
    .bind(company_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .fetch_all(ctx.pool())
    .await?;

    let max_primary = rows
        .iter()
        .map(|r| r.total)
        .max()
        .unwrap_or(Decimal::ZERO);

    Ok(rows
        .into_iter()
        .map(|row| {
            let share_percent = if max_primary.is_zero() {
                0u8
            } else {
                ((row.total / max_primary * Decimal::from(100u8)).round_dp(0))
                    .min(Decimal::from(100u8))
                    .to_u8()
                    .unwrap_or(100)
            };
            TopCounterpartyRow {
                counterparty_id: row.counterparty_id,
                counterparty_name: row.counterparty_name,
                primary_amount: row.total,
                secondary_label: "Кредиторська заборгованість".to_string(),
                secondary_value: row.total,
                share_percent,
            }
        })
        .collect())
}

pub async fn load_top_counterparties_pnl(
    ctx: &AppCtx,
    filter: &ResolvedReportsFilter,
) -> Result<Vec<TopCounterpartyRow>> {
    struct Row {
        counterparty_id: String,
        counterparty_name: String,
        income: Decimal,
        expense: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;

            Ok(Self {
                counterparty_id: row.try_get("counterparty_id")?,
                counterparty_name: row.try_get("counterparty_name")?,
                income: row.try_get("income")?,
                expense: row.try_get("expense")?,
            })
        }
    }

    let company_id = match filter.scope {
        ReportsScope::Active => Some(ctx.company_id()),
        ReportsScope::All => None,
    };

    let rows = sqlx::query_as::<_, Row>(
        r#"
        WITH docs AS (
            SELECT
                a.counterparty_id,
                COALESCE(cat.kind::text, 'income') AS category_kind,
                a.total_amount AS amount
            FROM acts a
            LEFT JOIN categories cat ON cat.id = a.category_id
            WHERE ($1::uuid IS NULL OR a.company_id = $1::uuid)
              AND a.date BETWEEN $2 AND $3
              AND a.status != 'draft'

            UNION ALL

            SELECT
                i.counterparty_id,
                COALESCE(cat.kind::text, 'income') AS category_kind,
                i.total_amount AS amount
            FROM invoices i
            LEFT JOIN categories cat ON cat.id = i.category_id
            WHERE ($1::uuid IS NULL OR i.company_id = $1::uuid)
              AND i.date BETWEEN $2 AND $3
              AND i.status != 'draft'
        )
        SELECT
            cp.id::text AS counterparty_id,
            cp.name AS counterparty_name,
            COALESCE(SUM(CASE WHEN d.category_kind = 'expense' THEN 0 ELSE d.amount END), 0) AS income,
            COALESCE(SUM(CASE WHEN d.category_kind = 'expense' THEN d.amount ELSE 0 END), 0) AS expense
        FROM docs d
        JOIN counterparties cp ON cp.id = d.counterparty_id
        GROUP BY cp.id, cp.name
        ORDER BY (COALESCE(SUM(CASE WHEN d.category_kind = 'expense' THEN 0 ELSE d.amount END), 0)
                + COALESCE(SUM(CASE WHEN d.category_kind = 'expense' THEN d.amount ELSE 0 END), 0)) DESC,
                 cp.name ASC
        LIMIT 8
        "#,
    )
    .bind(company_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .fetch_all(ctx.pool())
    .await?;

    let max_primary = rows
        .iter()
        .map(|r| r.income + r.expense)
        .max()
        .unwrap_or(Decimal::ZERO);

    Ok(rows
        .into_iter()
        .map(|row| {
            let primary_amount = row.income + row.expense;
            let share_percent = if max_primary.is_zero() {
                0u8
            } else {
                ((primary_amount / max_primary * Decimal::from(100u8)).round_dp(0))
                    .min(Decimal::from(100u8))
                    .to_u8()
                    .unwrap_or(100)
            };
            TopCounterpartyRow {
                counterparty_id: row.counterparty_id,
                counterparty_name: row.counterparty_name,
                primary_amount,
                secondary_label: "Чистий результат".to_string(),
                secondary_value: row.income - row.expense,
                share_percent,
            }
        })
        .collect())
}

pub async fn compute_opening_balance(
    ctx: &AppCtx,
    filter: &ResolvedReportsFilter,
) -> Result<Decimal> {
    let company_id = match filter.scope {
        ReportsScope::Active => Some(ctx.company_id()),
        ReportsScope::All => None,
    };

    let value = sqlx::query_scalar::<_, Decimal>(
        r#"
        SELECT COALESCE(
            SUM(CASE WHEN direction = 'income' THEN amount ELSE -amount END),
            0
        ) AS balance
        FROM payments
        WHERE ($1::uuid IS NULL OR company_id = $1::uuid)
          AND date < $2
        "#,
    )
    .bind(company_id)
    .bind(filter.date_from)
    .fetch_one(ctx.pool())
    .await?;

    Ok(value)
}
