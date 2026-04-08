// Dashboard DB запити — агрегація для головного екрану.
//
// Використовує runtime-style sqlx::query_as::<_, T>() без макросів
// щоб не залежати від cargo sqlx prepare при нових запитах.

use anyhow::Result;
use chrono::{Datelike, Local};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::dashboard::{
    KpiSummary, MonthRevenue, RecentAct, StatusSlice, UpcomingPayment,
};

/// Один SQL-запит з агрегатами для KPI-карток Dashboard.
///
/// Паралельно рахує:
/// - виручка поточного місяця (оплачені акти)
/// - загальний борг (виставлені + підписані)
/// - кількість актів за місяць
/// - активних контрагентів
pub async fn get_kpi_summary(pool: &PgPool, company_id: Uuid) -> Result<KpiSummary> {
    struct Row {
        revenue_this_month: Decimal,
        unpaid_total: Decimal,
        acts_this_month: i64,
        active_counterparties: i64,
    }

    let row = sqlx::query_as!(
        Row,
        r#"
        SELECT
            COALESCE(SUM(total_amount) FILTER (
                WHERE status = 'paid'
                  AND date_trunc('month', date) = date_trunc('month', CURRENT_DATE)
            ), 0) AS "revenue_this_month!: Decimal",

            COALESCE(SUM(total_amount) FILTER (
                WHERE status IN ('issued', 'signed')
            ), 0) AS "unpaid_total!: Decimal",

            COUNT(*) FILTER (
                WHERE date_trunc('month', date) = date_trunc('month', CURRENT_DATE)
            ) AS "acts_this_month!: i64",

            (SELECT COUNT(*) FROM counterparties
             WHERE company_id = $1 AND is_archived = FALSE
            ) AS "active_counterparties!: i64"

        FROM acts
        WHERE company_id = $1
        "#,
        company_id
    )
    .fetch_one(pool)
    .await?;

    Ok(KpiSummary {
        revenue_this_month: row.revenue_this_month,
        unpaid_total: row.unpaid_total,
        acts_this_month: row.acts_this_month,
        active_counterparties: row.active_counterparties,
    })
}

/// Виручка по місяцях за останні `months` місяців (оплачені акти).
///
/// Повертає рівно `months` записів, заповнюючи нулями відсутні місяці.
/// Відсортовано від найстарішого до поточного.
pub async fn revenue_by_month(
    pool: &PgPool,
    company_id: Uuid,
    months: u32,
) -> Result<Vec<MonthRevenue>> {
    struct Row {
        month_num: i32,
        year_num: i32,
        amount: Decimal,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"
        SELECT
            EXTRACT(MONTH FROM date_trunc('month', date))::int AS "month_num!: i32",
            EXTRACT(YEAR  FROM date_trunc('month', date))::int AS "year_num!: i32",
            COALESCE(SUM(total_amount) FILTER (WHERE status = 'paid'), 0)
                AS "amount!: Decimal"
        FROM acts
        WHERE company_id = $1
          AND date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'
        GROUP BY date_trunc('month', date)
        ORDER BY date_trunc('month', date) ASC
        "#,
        company_id,
        months as i32,
    )
    .fetch_all(pool)
    .await?;

    // Заповнюємо всі N місяців, вставляємо 0 для місяців без даних
    let today = Local::now().date_naive();
    let mut result: Vec<MonthRevenue> = Vec::with_capacity(months as usize);

    for i in (0..months).rev() {
        // i=0 — поточний місяць, i=months-1 — найстаріший
        let target_month = subtract_months(today, i);
        let found = rows.iter().find(|r| {
            r.month_num as u32 == target_month.month()
                && r.year_num == target_month.year()
        });
        result.push(MonthRevenue {
            month_num: target_month.month(),
            year: target_month.year(),
            amount: found.map(|r| r.amount).unwrap_or(Decimal::ZERO),
        });
    }

    // result[0] = найстаріший, result[months-1] = поточний
    result.reverse();
    Ok(result)
}

/// Розподіл актів за статусами за поточний місяць.
pub async fn acts_status_distribution(
    pool: &PgPool,
    company_id: Uuid,
) -> Result<Vec<StatusSlice>> {
    struct Row {
        status: String,
        count: i64,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"
        SELECT
            status::text AS "status!: String",
            COUNT(*)     AS "count!: i64"
        FROM acts
        WHERE company_id = $1
          AND date_trunc('month', date) = date_trunc('month', CURRENT_DATE)
        GROUP BY status
        "#,
        company_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| StatusSlice {
            status: r.status,
            count: r.count,
        })
        .collect())
}

/// Найближчі очікувані платежі (акти з expected_payment_date IS NOT NULL, статус ≠ paid/draft).
///
/// Прострочені (expected_payment_date <= сьогодні) йдуть першими.
pub async fn upcoming_payments(
    pool: &PgPool,
    company_id: Uuid,
    limit: i64,
) -> Result<Vec<UpcomingPayment>> {
    struct Row {
        date_day: i32,
        date_month: i32,
        contractor: String,
        amount: Decimal,
        is_overdue: bool,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"
        SELECT
            EXTRACT(DAY   FROM a.expected_payment_date)::int AS "date_day!: i32",
            EXTRACT(MONTH FROM a.expected_payment_date)::int AS "date_month!: i32",
            c.name                                           AS "contractor!: String",
            a.total_amount                                   AS "amount!: Decimal",
            a.expected_payment_date <= CURRENT_DATE          AS "is_overdue!: bool"
        FROM acts a
        JOIN counterparties c ON c.id = a.counterparty_id
        WHERE a.company_id = $1
          AND a.expected_payment_date IS NOT NULL
          AND a.status IN ('issued', 'signed')
        ORDER BY
            a.expected_payment_date <= CURRENT_DATE DESC,
            a.expected_payment_date ASC
        LIMIT $2
        "#,
        company_id,
        limit,
    )
    .fetch_all(pool)
    .await?;

    let month_abbr = |m: i32| -> &'static str {
        match m {
            1 => "Січ", 2 => "Лют", 3 => "Бер", 4 => "Кві",
            5 => "Тра", 6 => "Чер", 7 => "Лип", 8 => "Сер",
            9 => "Вер", 10 => "Жов", 11 => "Лис", 12 => "Гру",
            _ => "???",
        }
    };

    Ok(rows
        .into_iter()
        .map(|r| UpcomingPayment {
            date_label: format!("{:02} {}", r.date_day, month_abbr(r.date_month)),
            contractor: r.contractor,
            amount: r.amount,
            is_overdue: r.is_overdue,
        })
        .collect())
}

/// Останні `limit` актів для таблиці на Dashboard.
pub async fn get_recent_acts(
    pool: &PgPool,
    company_id: Uuid,
    limit: i64,
) -> Result<Vec<RecentAct>> {
    struct Row {
        num: String,
        contractor: String,
        amount: Decimal,
        status: String,
        date: String,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"
        SELECT
            a.number          AS "num!: String",
            c.name            AS "contractor!: String",
            a.total_amount    AS "amount!: Decimal",
            a.status::text    AS "status!: String",
            TO_CHAR(a.date, 'DD.MM.YYYY') AS "date!: String"
        FROM acts a
        JOIN counterparties c ON c.id = a.counterparty_id
        WHERE a.company_id = $1
        ORDER BY a.created_at DESC
        LIMIT $2
        "#,
        company_id,
        limit,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| RecentAct {
            num: r.num,
            contractor: r.contractor,
            amount: r.amount,
            status: r.status,
            date: r.date,
        })
        .collect())
}

// ── Допоміжна функція ────────────────────────────────────────────────────────

/// Відняти `months` місяців від дати (без зміщення по днях).
fn subtract_months(date: chrono::NaiveDate, months: u32) -> chrono::NaiveDate {
    let total_months = date.year() * 12 + date.month() as i32 - 1 - months as i32;
    let year = total_months / 12;
    let month = (total_months % 12 + 1) as u32;
    chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(date)
}
