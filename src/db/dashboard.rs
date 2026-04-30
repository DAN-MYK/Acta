// Dashboard DB Р·Р°РїРёС‚Рё вЂ” Р°РіСЂРµРіР°С†С–СЏ РґР»СЏ РіРѕР»РѕРІРЅРѕРіРѕ РµРєСЂР°РЅСѓ.
//
// Р’РёРєРѕСЂРёСЃС‚РѕРІСѓС” runtime-style sqlx::query_as::<_, T>() Р±РµР· РјР°РєСЂРѕСЃС–РІ
// С‰РѕР± РЅРµ Р·Р°Р»РµР¶Р°С‚Рё РІС–Рґ cargo sqlx prepare РїСЂРё РЅРѕРІРёС… Р·Р°РїРёС‚Р°С….

use anyhow::Result;
use chrono::{Datelike, Local};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::dashboard::{
    CategoryRevenue, KpiSummary, MonthRevenue, RecentAct, StatusSlice, UpcomingPayment,
};

/// РћРґРёРЅ SQL-Р·Р°РїРёС‚ Р· Р°РіСЂРµРіР°С‚Р°РјРё РґР»СЏ KPI-РєР°СЂС‚РѕРє Dashboard.
///
/// РџР°СЂР°Р»РµР»СЊРЅРѕ СЂР°С…СѓС”:
/// - РІРёСЂСѓС‡РєР° РїРѕС‚РѕС‡РЅРѕРіРѕ РјС–СЃСЏС†СЏ (РѕРїР»Р°С‡РµРЅС– Р°РєС‚Рё)
/// - Р·Р°РіР°Р»СЊРЅРёР№ Р±РѕСЂРі (РІРёСЃС‚Р°РІР»РµРЅС– + РїС–РґРїРёСЃР°РЅС–)
/// - РєС–Р»СЊРєС–СЃС‚СЊ Р°РєС‚С–РІ Р·Р° РјС–СЃСЏС†СЊ
/// - Р°РєС‚РёРІРЅРёС… РєРѕРЅС‚СЂР°РіРµРЅС‚С–РІ
pub async fn get_kpi_summary(pool: &PgPool, company_id: Uuid) -> Result<KpiSummary> {
    struct Row {
        revenue_this_month: Decimal,
        unpaid_total: Decimal,
        acts_this_month: i64,
        active_counterparties: i64,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                revenue_this_month: r.try_get("revenue_this_month")?,
                unpaid_total: r.try_get("unpaid_total")?,
                acts_this_month: r.try_get("acts_this_month")?,
                active_counterparties: r.try_get("active_counterparties")?,
            })
        }
    }

    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            COALESCE(SUM(total_amount) FILTER (
                WHERE status = 'paid'
                  AND date_trunc('month', date) = date_trunc('month', CURRENT_DATE)
            ), 0) AS revenue_this_month,

            COALESCE(SUM(total_amount) FILTER (
                WHERE status IN ('issued', 'signed')
            ), 0) AS unpaid_total,

            COUNT(*) FILTER (
                WHERE date_trunc('month', date) = date_trunc('month', CURRENT_DATE)
            ) AS acts_this_month,

            (SELECT COUNT(*) FROM counterparties
             WHERE company_id = $1 AND is_archived = FALSE
            ) AS active_counterparties

        FROM acts
        WHERE company_id = $1
        "#,
    )
    .bind(company_id)
    .fetch_one(pool)
    .await?;

    Ok(KpiSummary {
        revenue_this_month: row.revenue_this_month,
        unpaid_total: row.unpaid_total,
        acts_this_month: row.acts_this_month,
        active_counterparties: row.active_counterparties,
    })
}

/// Р’РёСЂСѓС‡РєР° РїРѕ РјС–СЃСЏС†СЏС… Р·Р° РѕСЃС‚Р°РЅРЅС– `months` РјС–СЃСЏС†С–РІ (РѕРїР»Р°С‡РµРЅС– Р°РєС‚Рё).
///
/// РџРѕРІРµСЂС‚Р°С” СЂС–РІРЅРѕ `months` Р·Р°РїРёСЃС–РІ, Р·Р°РїРѕРІРЅСЋСЋС‡Рё РЅСѓР»СЏРјРё РІС–РґСЃСѓС‚РЅС– РјС–СЃСЏС†С–.
/// Р’С–РґСЃРѕСЂС‚РѕРІР°РЅРѕ РІС–Рґ РЅР°Р№СЃС‚Р°СЂС–С€РѕРіРѕ РґРѕ РїРѕС‚РѕС‡РЅРѕРіРѕ.
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

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                month_num: r.try_get("month_num")?,
                year_num: r.try_get("year_num")?,
                amount: r.try_get("amount")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            EXTRACT(MONTH FROM date_trunc('month', date))::int AS month_num,
            EXTRACT(YEAR  FROM date_trunc('month', date))::int AS year_num,
            COALESCE(SUM(total_amount) FILTER (WHERE status = 'paid'), 0)
                AS amount
        FROM acts
        WHERE company_id = $1
          AND date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'
        GROUP BY date_trunc('month', date)
        ORDER BY date_trunc('month', date) ASC
        "#,
    )
    .bind(company_id)
    .bind(months as i32)
    .fetch_all(pool)
    .await?;

    // Р—Р°РїРѕРІРЅСЋС”РјРѕ РІСЃС– N РјС–СЃСЏС†С–РІ, РІСЃС‚Р°РІР»СЏС”РјРѕ 0 РґР»СЏ РјС–СЃСЏС†С–РІ Р±РµР· РґР°РЅРёС…
    let today = Local::now().date_naive();
    let mut result: Vec<MonthRevenue> = Vec::with_capacity(months as usize);

    for i in (0..months).rev() {
        // i=0 вЂ” РїРѕС‚РѕС‡РЅРёР№ РјС–СЃСЏС†СЊ, i=months-1 вЂ” РЅР°Р№СЃС‚Р°СЂС–С€РёР№
        let target_month = subtract_months(today, i);
        let found = rows.iter().find(|r| {
            r.month_num as u32 == target_month.month() && r.year_num == target_month.year()
        });
        result.push(MonthRevenue {
            month_num: target_month.month(),
            year: target_month.year(),
            amount: found.map(|r| r.amount).unwrap_or(Decimal::ZERO),
        });
    }

    // result[0] = РЅР°Р№СЃС‚Р°СЂС–С€РёР№, result[months-1] = РїРѕС‚РѕС‡РЅРёР№
    result.reverse();
    Ok(result)
}

/// Р РѕР·РїРѕРґС–Р» Р°РєС‚С–РІ Р·Р° СЃС‚Р°С‚СѓСЃР°РјРё Р·Р° РїРѕС‚РѕС‡РЅРёР№ РјС–СЃСЏС†СЊ.
pub async fn acts_status_distribution(pool: &PgPool, company_id: Uuid) -> Result<Vec<StatusSlice>> {
    struct Row {
        status: String,
        count: i64,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                status: r.try_get("status")?,
                count: r.try_get("count")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            status::text AS status,
            COUNT(*)::bigint AS count
        FROM acts
        WHERE company_id = $1
          AND date_trunc('month', date) = date_trunc('month', CURRENT_DATE)
        GROUP BY status
        "#,
    )
    .bind(company_id)
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

/// РќР°Р№Р±Р»РёР¶С‡С– РЅРµР·РІС–СЂРµРЅС– РїР»Р°С‚РµР¶С– РґР»СЏ dashboard drill-in.
///
/// РџСЂРѕСЃС‚СЂРѕС‡РµРЅС– (date <= СЃСЊРѕРіРѕРґРЅС–) Р№РґСѓС‚СЊ РїРµСЂС€РёРјРё.
pub async fn upcoming_payments(
    pool: &PgPool,
    company_id: Uuid,
    limit: i64,
) -> Result<Vec<UpcomingPayment>> {
    struct Row {
        id: String,
        date_day: i32,
        date_month: i32,
        contractor: String,
        amount: Decimal,
        is_overdue: bool,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                id: r.try_get("id")?,
                date_day: r.try_get("date_day")?,
                date_month: r.try_get("date_month")?,
                contractor: r.try_get("contractor")?,
                amount: r.try_get("amount")?,
                is_overdue: r.try_get("is_overdue")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            a.id::text                                        AS id,
            EXTRACT(DAY   FROM a.expected_payment_date)::int   AS date_day,
            EXTRACT(MONTH FROM a.expected_payment_date)::int   AS date_month,
            COALESCE(c.name, ''Р‘РµР· РєРѕРЅС‚СЂР°РіРµРЅС‚Р°'')     AS contractor,
            a.total_amount                                    AS amount,
            a.expected_payment_date <= CURRENT_DATE           AS is_overdue
        FROM acts a
        LEFT JOIN counterparties c ON c.id = a.counterparty_id
        WHERE a.company_id = $1
          AND a.status IN (''issued'', ''signed'')
          AND a.expected_payment_date IS NOT NULL
        ORDER BY
            a.expected_payment_date <= CURRENT_DATE DESC,
            a.expected_payment_date ASC,
            a.created_at ASC
        LIMIT $2
        "#,
    )
    .bind(company_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let month_abbr = |m: i32| -> &'static str {
        match m {
            1 => "РЎС–С‡",
            2 => "Р›СЋС‚",
            3 => "Р‘РµСЂ",
            4 => "РљРІС–",
            5 => "РўСЂР°",
            6 => "Р§РµСЂ",
            7 => "Р›РёРї",
            8 => "РЎРµСЂ",
            9 => "Р’РµСЂ",
            10 => "Р–РѕРІ",
            11 => "Р›РёСЃ",
            12 => "Р“СЂСѓ",
            _ => "???",
        }
    };

    Ok(rows
        .into_iter()
        .map(|r| UpcomingPayment {
            id: r.id,
            date_label: format!("{:02} {}", r.date_day, month_abbr(r.date_month)),
            contractor: r.contractor,
            amount: r.amount,
            is_overdue: r.is_overdue,
        })
        .collect())
}

/// РћСЃС‚Р°РЅРЅС– `limit` Р°РєС‚С–РІ РґР»СЏ С‚Р°Р±Р»РёС†С– РЅР° Dashboard.
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

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                num: r.try_get("num")?,
                contractor: r.try_get("contractor")?,
                amount: r.try_get("amount")?,
                status: r.try_get("status")?,
                date: r.try_get("date")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            a.number          AS num,
            c.name            AS contractor,
            a.total_amount    AS amount,
            a.status::text    AS status,
            TO_CHAR(a.date, 'DD.MM.YYYY') AS date
        FROM acts a
        JOIN counterparties c ON c.id = a.counterparty_id
        WHERE a.company_id = $1
        ORDER BY a.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(company_id)
    .bind(limit)
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

// в”Ђв”Ђ Inbox вЂ” РґС–С—, С‰Рѕ РїРѕС‚СЂРµР±СѓСЋС‚СЊ СѓРІР°РіРё в”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђ

/// Р СЏРґРѕРє inbox: РїСЂРѕСЃС‚СЂРѕС‡РµРЅС– Р°РєС‚Рё С‚Р° РЅРµСѓР·РіРѕРґР¶РµРЅС– РїР»Р°С‚РµР¶С–.
pub struct InboxRow {
    pub doc_id: String,
    pub doc_number: String,
    pub counterparty: String,
    pub amount: Decimal,
    pub age_days: i32,
    pub kind: String,
    pub action_label: String,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for InboxRow {
    fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row as _;
        Ok(Self {
            doc_id: r.try_get("doc_id")?,
            doc_number: r.try_get("doc_number")?,
            counterparty: r.try_get("counterparty")?,
            amount: r.try_get("amount")?,
            age_days: r.try_get("age_days")?,
            kind: r.try_get("kind")?,
            action_label: r.try_get("action_label")?,
        })
    }
}

/// РџСЂРѕСЃС‚СЂРѕС‡РµРЅС– Р°РєС‚Рё (>14 РґРЅС–РІ Р±РµР· РѕРїР»Р°С‚Рё) С‚Р° РЅРµСѓР·РіРѕРґР¶РµРЅС– РїР»Р°С‚РµР¶С–.
/// Р’С–РґСЃРѕСЂС‚РѕРІР°РЅРѕ Р·Р° РєС–Р»СЊРєС–СЃС‚СЋ РґРЅС–РІ РѕС‡С–РєСѓРІР°РЅРЅСЏ (РЅР°Р№РґР°РІРЅС–С€С– вЂ” РїРµСЂС€РёРјРё).
/// РџРѕРІРµСЂС‚Р°С” РЅРµ Р±С–Р»СЊС€Рµ 20 Р·Р°РїРёСЃС–РІ.
pub async fn inbox_items(pool: &PgPool, company_id: Uuid) -> Result<Vec<InboxRow>> {
    sqlx::query_as::<_, InboxRow>(
        r#"
        SELECT
            'act:' || a.id::text          AS doc_id,
            a.number                       AS doc_number,
            c.name                         AS counterparty,
            a.total_amount                 AS amount,
            (CURRENT_DATE - a.date)::int   AS age_days,
            'overdue'::text                AS kind,
            'РќР°РіР°РґР°С‚Рё'::text               AS action_label
        FROM acts a
        JOIN counterparties c ON c.id = a.counterparty_id
        WHERE a.company_id = $1
          AND a.status::text = 'issued'
          AND a.date < CURRENT_DATE - INTERVAL '14 days'
        UNION ALL
        SELECT
            'pay:' || p.id::text,
            'РџР›Рў-' || LEFT(p.id::text, 8),
            COALESCE(c.name, 'вЂ”'),
            p.amount,
            (CURRENT_DATE - p.date)::int,
            'unmatched'::text,
            'РџРѕС”РґРЅР°С‚Рё'::text
        FROM payments p
        LEFT JOIN counterparties c ON c.id = p.counterparty_id
        WHERE p.company_id = $1
          AND p.is_reconciled = false
        ORDER BY age_days DESC
        LIMIT 20
        "#,
    )
    .bind(company_id)
    .fetch_all(pool)
    .await
    .map_err(anyhow::Error::from)
}

/// Р’РёС‚СЂР°С‚Рё РїРѕ РјС–СЃСЏС†СЏС… Р·Р° РєР°С‚РµРіРѕСЂС–СЏРјРё С‚РёРїСѓ `expense`.
pub async fn expenses_by_month(
    pool: &PgPool,
    company_id: Uuid,
    months: u32,
) -> Result<Vec<MonthRevenue>> {
    struct Row {
        month_num: i32,
        year_num: i32,
        amount: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                month_num: r.try_get("month_num")?,
                year_num: r.try_get("year_num")?,
                amount: r.try_get("amount")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        WITH expense_docs AS (
            SELECT a.date, a.total_amount AS amount
            FROM acts a
            JOIN categories c ON c.id = a.category_id
            WHERE a.company_id = $1
              AND c.kind = 'expense'
              AND a.date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'

            UNION ALL

            SELECT i.date, i.total_amount AS amount
            FROM invoices i
            JOIN categories c ON c.id = i.category_id
            WHERE i.company_id = $1
              AND c.kind = 'expense'
              AND i.date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'
        )
        SELECT
            EXTRACT(MONTH FROM date_trunc('month', date))::int AS month_num,
            EXTRACT(YEAR FROM date_trunc('month', date))::int AS year_num,
            COALESCE(SUM(amount), 0) AS amount
        FROM expense_docs
        GROUP BY date_trunc('month', date)
        ORDER BY date_trunc('month', date) ASC
        "#,
    )
    .bind(company_id)
    .bind(months as i32)
    .fetch_all(pool)
    .await?;

    let today = Local::now().date_naive();
    let mut result: Vec<MonthRevenue> = Vec::with_capacity(months as usize);

    for i in (0..months).rev() {
        let target_month = subtract_months(today, i);
        let found = rows.iter().find(|r| {
            r.month_num as u32 == target_month.month() && r.year_num == target_month.year()
        });
        result.push(MonthRevenue {
            month_num: target_month.month(),
            year: target_month.year(),
            amount: found.map(|r| r.amount).unwrap_or(Decimal::ZERO),
        });
    }

    result.reverse();
    Ok(result)
}

/// Р РѕР·РїРѕРґС–Р» РІРёС‚СЂР°С‚ Р·Р° РєР°С‚РµРіРѕСЂС–СЏРјРё РґР»СЏ РµРєСЂР°РЅР° Р·РІС–С‚С–РІ.
pub async fn category_breakdown(
    pool: &PgPool,
    company_id: Uuid,
    months: u32,
) -> Result<Vec<CategoryRevenue>> {
    struct Row {
        label: String,
        amount: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                label: r.try_get("label")?,
                amount: r.try_get("amount")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        WITH expense_docs AS (
            SELECT c.name AS label, a.total_amount AS amount
            FROM acts a
            JOIN categories c ON c.id = a.category_id
            WHERE a.company_id = $1
              AND c.kind = 'expense'
              AND a.date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'

            UNION ALL

            SELECT c.name AS label, i.total_amount AS amount
            FROM invoices i
            JOIN categories c ON c.id = i.category_id
            WHERE i.company_id = $1
              AND c.kind = 'expense'
              AND i.date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'
        )
        SELECT label, COALESCE(SUM(amount), 0) AS amount
        FROM expense_docs
        GROUP BY label
        ORDER BY amount DESC, label ASC
        "#,
    )
    .bind(company_id)
    .bind(months as i32)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| CategoryRevenue {
            label: row.label,
            amount: row.amount,
        })
        .collect())
}

// в”Ђв”Ђ Р”РѕРїРѕРјС–Р¶РЅР° С„СѓРЅРєС†С–СЏ в”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђ

/// Р’С–РґРЅСЏС‚Рё `months` РјС–СЃСЏС†С–РІ РІС–Рґ РґР°С‚Рё (Р±РµР· Р·РјС–С‰РµРЅРЅСЏ РїРѕ РґРЅСЏС…).
fn subtract_months(date: chrono::NaiveDate, months: u32) -> chrono::NaiveDate {
    let total_months = date.year() * 12 + date.month() as i32 - 1 - months as i32;
    let year = total_months / 12;
    let month = (total_months % 12 + 1) as u32;
    chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(date)
}
