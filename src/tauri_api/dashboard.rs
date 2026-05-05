use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app_ctx::AppCtx;
use crate::db;
use crate::models::task::{TaskPriority, TaskStatus};

use super::documents::{DocumentItemDto, DocumentKindDto, DocumentStatusDto};
use super::reports::BankReportRowDto;
use super::tasks::TaskItemDto;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardKpiDto {
    pub label: String,
    pub value: String,
    pub detail: String,
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardScreenDto {
    pub kpis: Vec<DashboardKpiDto>,
    pub cashflow_rows: Vec<BankReportRowDto>,
    pub recent_documents: Vec<DocumentItemDto>,
    pub upcoming_payments: Vec<DashboardUpcomingPaymentDto>,
    pub urgent_tasks: Vec<TaskItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardUpcomingPaymentDto {
    pub id: String,
    pub date_label: String,
    pub contractor: String,
    pub amount_str: String,
    pub is_overdue: bool,
}

struct DashboardKpiRaw {
    income_period: Decimal,
    expense_period: Decimal,
    receivables_total: Decimal,
    payables_total: Decimal,
    documents_total: i64,
    unmatched_count: i64,
}

struct TaskSummaryRaw {
    open_count: i64,
    today_count: i64,
    high_count: i64,
}

fn format_money_ua(value: Decimal) -> String {
    let normalized = format!("{:.2}", value.round_dp(2)).replace('.', ",");
    let (sign, digits) = normalized
        .strip_prefix('-')
        .map_or(("", normalized.as_str()), |rest| ("-", rest));
    let (whole, frac) = digits.split_once(',').unwrap_or((digits, "00"));
    let grouped = whole
        .chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .rev()
        .collect::<String>();

    format!("{sign}{grouped},{frac} грн")
}

fn format_date_ua(value: NaiveDate) -> String {
    value.format("%d.%m.%Y").to_string()
}

fn format_task_datetime(value: Option<DateTime<Utc>>) -> String {
    value
        .map(|date| {
            date.with_timezone(&Local)
                .format("%d.%m.%Y %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

fn document_status(kind: &str, status: &str) -> (DocumentStatusDto, String) {
    match (kind, status) {
        ("act", "draft") | ("invoice", "draft") | ("waybill", "draft") => {
            (DocumentStatusDto::Draft, "Чернетка".to_string())
        }
        ("act", "issued") | ("invoice", "issued") => {
            (DocumentStatusDto::Issued, "Виставлено".to_string())
        }
        ("waybill", "issued") => (DocumentStatusDto::Issued, "Виставлена".to_string()),
        ("act", "signed") | ("invoice", "signed") => {
            (DocumentStatusDto::Signed, "Підписано".to_string())
        }
        ("waybill", "signed") => (DocumentStatusDto::Signed, "Підписана".to_string()),
        ("act", "paid") | ("invoice", "paid") => (DocumentStatusDto::Paid, "Оплачено".to_string()),
        ("waybill", "delivered") => (DocumentStatusDto::Delivered, "Доставлено".to_string()),
        _ => (DocumentStatusDto::Draft, status.to_string()),
    }
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for DashboardKpiRaw {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        Ok(Self {
            income_period: row.try_get("income_period")?,
            expense_period: row.try_get("expense_period")?,
            receivables_total: row.try_get("receivables_total")?,
            payables_total: row.try_get("payables_total")?,
            documents_total: row.try_get("documents_total")?,
            unmatched_count: row.try_get("unmatched_count")?,
        })
    }
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for TaskSummaryRaw {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        Ok(Self {
            open_count: row.try_get("open_count")?,
            today_count: row.try_get("today_count")?,
            high_count: row.try_get("high_count")?,
        })
    }
}

async fn dashboard_kpis(
    pool: &PgPool,
    company_id: Uuid,
    date_from: NaiveDate,
    date_to: NaiveDate,
) -> Result<DashboardKpiRaw> {
    sqlx::query_as::<_, DashboardKpiRaw>(
        r#"
        WITH receivables AS (
            SELECT total_amount AS amount
            FROM acts
            WHERE company_id = $1
              AND date BETWEEN $2 AND $3
              AND status NOT IN ('paid', 'draft')
            UNION ALL
            SELECT total_amount AS amount
            FROM invoices
            WHERE company_id = $1
              AND date BETWEEN $2 AND $3
              AND status NOT IN ('paid', 'draft')
        )
        SELECT
            COALESCE((
                SELECT SUM(amount)
                FROM payments
                WHERE company_id = $1
                  AND direction = 'income'
                  AND date BETWEEN $2 AND $3
            ), 0) AS income_period,
            COALESCE((
                SELECT SUM(amount)
                FROM payments
                WHERE company_id = $1
                  AND direction = 'expense'
                  AND date BETWEEN $2 AND $3
            ), 0) AS expense_period,
            COALESCE((SELECT SUM(amount) FROM receivables), 0) AS receivables_total,
            COALESCE((
                SELECT SUM(amount)
                FROM payment_schedule
                WHERE company_id = $1
                  AND direction = 'expense'
                  AND is_completed = FALSE
                  AND scheduled_date BETWEEN $2 AND $3
            ), 0) AS payables_total,
            (
                (SELECT COUNT(*) FROM acts WHERE company_id = $1)
              + (SELECT COUNT(*) FROM invoices WHERE company_id = $1)
              + (SELECT COUNT(*) FROM waybills WHERE company_id = $1)
            )::bigint AS documents_total,
            (
                SELECT COUNT(*)
                FROM payments
                WHERE company_id = $1
                  AND is_reconciled = FALSE
            )::bigint AS unmatched_count
        "#,
    )
    .bind(company_id)
    .bind(date_from)
    .bind(date_to)
    .fetch_one(pool)
    .await
    .map_err(anyhow::Error::from)
}

async fn dashboard_cashflow_rows(
    pool: &PgPool,
    company_id: Uuid,
    date_from: NaiveDate,
    date_to: NaiveDate,
    limit: i64,
) -> Result<Vec<BankReportRowDto>> {
    struct RowDto {
        key: String,
        label: String,
        income: Decimal,
        expense: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for RowDto {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                key: row.try_get("key")?,
                label: row.try_get("label")?,
                income: row.try_get("income")?,
                expense: row.try_get("expense")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, RowDto>(
        r#"
        SELECT
            date::text AS key,
            TO_CHAR(date, 'DD.MM.YYYY') AS label,
            COALESCE(SUM(amount) FILTER (WHERE direction = 'income'), 0) AS income,
            COALESCE(SUM(amount) FILTER (WHERE direction = 'expense'), 0) AS expense
        FROM payments
        WHERE company_id = $1
          AND date BETWEEN $2 AND $3
        GROUP BY date
        ORDER BY date DESC
        LIMIT $4
        "#,
    )
    .bind(company_id)
    .bind(date_from)
    .bind(date_to)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| BankReportRowDto {
            key: row.key,
            label: row.label,
            income_str: format_money_ua(row.income),
            expense_str: format_money_ua(row.expense),
            net_str: format_money_ua(row.income - row.expense),
        })
        .collect())
}

async fn dashboard_recent_documents(
    pool: &PgPool,
    company_id: Uuid,
    limit: i64,
) -> Result<Vec<DocumentItemDto>> {
    struct RowDto {
        id: String,
        kind: String,
        number: String,
        date: NaiveDate,
        counterparty: String,
        amount: Decimal,
        status: String,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for RowDto {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                id: row.try_get("id")?,
                kind: row.try_get("kind")?,
                number: row.try_get("number")?,
                date: row.try_get("date")?,
                counterparty: row.try_get("counterparty")?,
                amount: row.try_get("amount")?,
                status: row.try_get("status")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, RowDto>(
        r#"
        SELECT * FROM (
            SELECT
                'act:' || a.id::text AS id,
                'act'::text AS kind,
                a.number,
                a.date,
                cp.name AS counterparty,
                a.total_amount AS amount,
                a.status::text AS status,
                a.created_at
            FROM acts a
            JOIN counterparties cp ON cp.id = a.counterparty_id
            WHERE a.company_id = $1
            UNION ALL
            SELECT
                'inv:' || i.id::text AS id,
                'invoice'::text AS kind,
                i.number,
                i.date,
                cp.name AS counterparty,
                i.total_amount AS amount,
                i.status::text AS status,
                i.created_at
            FROM invoices i
            JOIN counterparties cp ON cp.id = i.counterparty_id
            WHERE i.company_id = $1
            UNION ALL
            SELECT
                'wbl:' || w.id::text AS id,
                'waybill'::text AS kind,
                w.number,
                w.date,
                cp.name AS counterparty,
                w.total_amount AS amount,
                w.status::text AS status,
                w.created_at
            FROM waybills w
            JOIN counterparties cp ON cp.id = w.counterparty_id
            WHERE w.company_id = $1
        ) docs
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(company_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            let kind = match row.kind.as_str() {
                "act" => DocumentKindDto::Act,
                "invoice" => DocumentKindDto::Invoice,
                "waybill" => DocumentKindDto::Waybill,
                other => return Err(anyhow!("Невідомий тип документа dashboard: {other}")),
            };
            let (status, status_label) = document_status(&row.kind, &row.status);
            Ok(DocumentItemDto {
                id: row.id,
                kind,
                number: row.number,
                date: format_date_ua(row.date),
                counterparty: row.counterparty,
                amount_str: format_money_ua(row.amount),
                status,
                status_label,
                linked_id: String::new(),
            })
        })
        .collect()
}

async fn dashboard_task_summary(
    pool: &PgPool,
    company_id: Uuid,
    today: NaiveDate,
) -> Result<TaskSummaryRaw> {
    sqlx::query_as::<_, TaskSummaryRaw>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE status IN ('open', 'in_progress'))::bigint AS open_count,
            COUNT(*) FILTER (
                WHERE due_date::date = $2 OR reminder_at::date = $2
            )::bigint AS today_count,
            COUNT(*) FILTER (WHERE priority IN ('high', 'critical'))::bigint AS high_count
        FROM tasks
        WHERE company_id = $1
        "#,
    )
    .bind(company_id)
    .bind(today)
    .fetch_one(pool)
    .await
    .map_err(anyhow::Error::from)
}

async fn dashboard_urgent_tasks(
    pool: &PgPool,
    company_id: Uuid,
    limit: i64,
) -> Result<Vec<TaskItemDto>> {
    struct RowDto {
        id: Uuid,
        title: String,
        description: Option<String>,
        status: TaskStatus,
        priority: TaskPriority,
        due_date: Option<DateTime<Utc>>,
        reminder_at: Option<DateTime<Utc>>,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for RowDto {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                id: row.try_get("id")?,
                title: row.try_get("title")?,
                description: row.try_get("description")?,
                status: row.try_get("status")?,
                priority: row.try_get("priority")?,
                due_date: row.try_get("due_date")?,
                reminder_at: row.try_get("reminder_at")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, RowDto>(
        r#"
        SELECT id, title, description, status, priority, due_date, reminder_at
        FROM tasks
        WHERE company_id = $1
          AND status IN ('open', 'in_progress')
        ORDER BY
            CASE priority
                WHEN 'critical' THEN 1
                WHEN 'high' THEN 2
                WHEN 'normal' THEN 3
                WHEN 'low' THEN 4
            END,
            due_date ASC NULLS LAST,
            created_at DESC
        LIMIT $2
        "#,
    )
    .bind(company_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|task| TaskItemDto {
            id: task.id.to_string(),
            title: task.title,
            description: task.description.unwrap_or_default(),
            status: task.status.as_str().to_string(),
            status_label: task.status.label().to_string(),
            priority: task.priority.as_str().to_string(),
            priority_label: task.priority.label().to_string(),
            due_date: format_task_datetime(task.due_date),
            reminder_at: format_task_datetime(task.reminder_at),
            link_kind: String::new(),
            link_label: String::new(),
        })
        .collect())
}

pub async fn dashboard_load(ctx: &AppCtx) -> Result<DashboardScreenDto> {
    let today = Local::now().date_naive();
    let date_from = today
        .checked_sub_days(chrono::Days::new(89))
        .ok_or_else(|| anyhow!("Не вдалося розрахувати період dashboard"))?;
    let company_id = ctx.company_id();

    let (kpi, cashflow_rows, recent_documents, upcoming_payments, task_summary, urgent_tasks) = tokio::try_join!(
        dashboard_kpis(ctx.pool(), company_id, date_from, today),
        dashboard_cashflow_rows(ctx.pool(), company_id, date_from, today, 6),
        dashboard_recent_documents(ctx.pool(), company_id, 6),
        db::dashboard::upcoming_payments(ctx.pool(), company_id, 5),
        dashboard_task_summary(ctx.pool(), company_id, today),
        dashboard_urgent_tasks(ctx.pool(), company_id, 5),
    )?;

    let kpis = vec![
        DashboardKpiDto {
            label: "Дохід за період".to_string(),
            value: format_money_ua(kpi.income_period),
            detail: "За останні 90 днів".to_string(),
            tone: "positive".to_string(),
        },
        DashboardKpiDto {
            label: "Витрати за період".to_string(),
            value: format_money_ua(kpi.expense_period),
            detail: format!("До сплати: {}", format_money_ua(kpi.payables_total)),
            tone: "warning".to_string(),
        },
        DashboardKpiDto {
            label: "Документи".to_string(),
            value: kpi.documents_total.to_string(),
            detail: "У поточній компанії".to_string(),
            tone: "neutral".to_string(),
        },
        DashboardKpiDto {
            label: "Завдання".to_string(),
            value: task_summary.open_count.to_string(),
            detail: format!(
                "{} сьогодні, {} високий пріоритет",
                task_summary.today_count, task_summary.high_count
            ),
            tone: "accent".to_string(),
        },
        DashboardKpiDto {
            label: "Нерознесені платежі".to_string(),
            value: kpi.unmatched_count.to_string(),
            detail: format!("{} платежів потребують уваги", kpi.unmatched_count),
            tone: "danger".to_string(),
        },
        DashboardKpiDto {
            label: "Дебіторка".to_string(),
            value: format_money_ua(kpi.receivables_total),
            detail: "Очікувані надходження".to_string(),
            tone: "positive".to_string(),
        },
    ];

    Ok(DashboardScreenDto {
        kpis,
        cashflow_rows,
        recent_documents,
        upcoming_payments: upcoming_payments
            .into_iter()
            .map(|payment| DashboardUpcomingPaymentDto {
                id: payment.id,
                date_label: payment.date_label,
                contractor: payment.contractor,
                amount_str: format_money_ua(payment.amount),
                is_overdue: payment.is_overdue,
            })
            .collect(),
        urgent_tasks,
    })
}
