use std::path::PathBuf;

use crate::app_ctx::AppCtx;
use crate::models::payment::PaymentDirection;
use anyhow::{anyhow, Context, Result};
use chrono::{Local, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportsLoadRequest {
    pub tab: Option<String>,
    pub scope: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportsFilterDto {
    pub tab: String,
    pub scope: String,
    pub date_from: String,
    pub date_to: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportsSummaryDto {
    pub opening_balance_str: String,
    pub income_str: String,
    pub expense_str: String,
    pub closing_balance_str: String,
    pub receivables_total_str: String,
    pub payables_total_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BankReportRowDto {
    pub key: String,
    pub label: String,
    pub income_str: String,
    pub expense_str: String,
    pub net_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReceivableRowDto {
    pub doc_id: String,
    pub doc_type: String,
    pub doc_number: String,
    pub doc_date: String,
    pub company_name: String,
    pub counterparty: String,
    pub amount_str: String,
    pub expected_date: String,
    pub overdue_days: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PayableRowDto {
    pub id: String,
    pub title: String,
    pub company_name: String,
    pub counterparty: String,
    pub amount_str: String,
    pub due_date: String,
    pub overdue_days: i32,
    pub recurrence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportsScreenDto {
    pub filter: ReportsFilterDto,
    pub summary: ReportsSummaryDto,
    pub bank_rows: Vec<BankReportRowDto>,
    pub receivables_rows: Vec<ReceivableRowDto>,
    pub payables_rows: Vec<PayableRowDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportsExportRequest {
    pub tab: String,
    pub scope: String,
    pub date_from: String,
    pub date_to: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportsExportResultDto {
    pub ok: bool,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Copy)]
enum ReportsScope {
    Active,
    All,
}

struct ResolvedFilter {
    scope: ReportsScope,
    date_from: NaiveDate,
    date_to: NaiveDate,
    query: String,
}

struct BankAggregateRow {
    key: String,
    label: String,
    income: Decimal,
    expense: Decimal,
}

struct ReceivableRow {
    doc_id: String,
    doc_type: String,
    doc_number: String,
    doc_date: NaiveDate,
    company_name: String,
    counterparty: String,
    amount: Decimal,
    expected_date: Option<NaiveDate>,
    overdue_days: i32,
    status: String,
}

struct PayableRow {
    id: String,
    title: String,
    company_name: String,
    counterparty: String,
    amount: Decimal,
    due_date: NaiveDate,
    overdue_days: i32,
    recurrence: String,
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

fn parse_ui_date(value: Option<&str>, fallback: NaiveDate, field: &str) -> Result<NaiveDate> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(fallback);
    };

    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(value, "%d.%m.%Y"))
        .with_context(|| format!("Некоректна дата у полі {field}: {value}"))
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

fn reports_dir() -> PathBuf {
    PathBuf::from("storage/reports")
}

fn report_stamp() -> String {
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn resolve_filter(
    request: ReportsLoadRequest,
    today: NaiveDate,
) -> Result<(ResolvedFilter, ReportsFilterDto)> {
    let date_to = parse_ui_date(request.date_to.as_deref(), today, "Дата до")?;
    let default_from = date_to - chrono::Days::new(89);
    let date_from = parse_ui_date(request.date_from.as_deref(), default_from, "Дата від")?;
    if date_from > date_to {
        return Err(anyhow!("Дата від не може бути більшою за дату до"));
    }

    let tab = match request.tab.as_deref() {
        Some("receivables") => "receivables".to_string(),
        Some("payables") => "payables".to_string(),
        _ => "bank".to_string(),
    };

    let scope = match request.scope.as_deref() {
        Some("all") => ReportsScope::All,
        _ => ReportsScope::Active,
    };

    let query = request.query.unwrap_or_default().trim().to_string();

    Ok((
        ResolvedFilter {
            scope,
            date_from,
            date_to,
            query: query.clone(),
        },
        ReportsFilterDto {
            tab,
            scope: match scope {
                ReportsScope::Active => "active".to_string(),
                ReportsScope::All => "all".to_string(),
            },
            date_from: date_from.format("%Y-%m-%d").to_string(),
            date_to: date_to.format("%Y-%m-%d").to_string(),
            query,
        },
    ))
}

async fn load_bank_rows(ctx: &AppCtx, filter: &ResolvedFilter) -> Result<Vec<BankAggregateRow>> {
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
            GROUP BY c.id, c.name
            ORDER BY c.name
            "#,
        )
        .bind(filter.date_from)
        .bind(filter.date_to)
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
            GROUP BY cp.id, cp.name, p.bank_name
            ORDER BY label
            "#,
        )
        .bind(company_id.expect("active company scope"))
        .bind(filter.date_from)
        .bind(filter.date_to)
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

async fn load_receivables_rows(
    ctx: &AppCtx,
    filter: &ResolvedFilter,
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

        ORDER BY overdue_days DESC, doc_date ASC
        "#,
    )
    .bind(company_id)
    .bind(filter.date_to)
    .bind(filter.date_from)
    .bind(filter.date_to)
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

async fn load_payables_rows(ctx: &AppCtx, filter: &ResolvedFilter) -> Result<Vec<PayableRow>> {
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
        ORDER BY ps.scheduled_date ASC
        "#,
    )
    .bind(company_id)
    .bind(filter.date_to)
    .bind(PaymentDirection::Expense)
    .bind(filter.date_from)
    .bind(filter.date_to)
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

async fn compute_opening_balance(ctx: &AppCtx, filter: &ResolvedFilter) -> Result<Decimal> {
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

fn bank_rows_to_dto(rows: Vec<BankAggregateRow>) -> Vec<BankReportRowDto> {
    rows.into_iter()
        .map(|row| BankReportRowDto {
            key: row.key,
            label: row.label,
            income_str: format_money_ua(row.income),
            expense_str: format_money_ua(row.expense),
            net_str: format_money_ua(row.income - row.expense),
        })
        .collect()
}

fn receivables_to_dto(rows: Vec<ReceivableRow>) -> Vec<ReceivableRowDto> {
    rows.into_iter()
        .map(|row| ReceivableRowDto {
            doc_id: row.doc_id,
            doc_type: row.doc_type,
            doc_number: row.doc_number,
            doc_date: row.doc_date.format("%d.%m.%Y").to_string(),
            company_name: row.company_name,
            counterparty: row.counterparty,
            amount_str: format_money_ua(row.amount),
            expected_date: row
                .expected_date
                .map(|date| date.format("%d.%m.%Y").to_string())
                .unwrap_or_default(),
            overdue_days: row.overdue_days,
            status: row.status,
        })
        .collect()
}

fn payables_to_dto(rows: Vec<PayableRow>) -> Vec<PayableRowDto> {
    rows.into_iter()
        .map(|row| PayableRowDto {
            id: row.id,
            title: row.title,
            company_name: row.company_name,
            counterparty: row.counterparty,
            amount_str: format_money_ua(row.amount),
            due_date: row.due_date.format("%d.%m.%Y").to_string(),
            overdue_days: row.overdue_days,
            recurrence: row.recurrence,
        })
        .collect()
}

fn sum_receivables(rows: &[ReceivableRow]) -> Decimal {
    rows.iter().fold(Decimal::ZERO, |acc, row| acc + row.amount)
}

fn sum_payables(rows: &[PayableRow]) -> Decimal {
    rows.iter().fold(Decimal::ZERO, |acc, row| acc + row.amount)
}

async fn build_reports_screen(
    ctx: &AppCtx,
    filter: ResolvedFilter,
    filter_dto: ReportsFilterDto,
) -> Result<ReportsScreenDto> {
    let opening_balance = compute_opening_balance(ctx, &filter).await?;
    let bank_rows = load_bank_rows(ctx, &filter).await?;
    let receivables_rows = load_receivables_rows(ctx, &filter).await?;
    let payables_rows = load_payables_rows(ctx, &filter).await?;

    let income = bank_rows
        .iter()
        .fold(Decimal::ZERO, |acc, row| acc + row.income);
    let expense = bank_rows
        .iter()
        .fold(Decimal::ZERO, |acc, row| acc + row.expense);
    let closing_balance = opening_balance + income - expense;
    let receivables_total = sum_receivables(&receivables_rows);
    let payables_total = sum_payables(&payables_rows);

    Ok(ReportsScreenDto {
        filter: filter_dto,
        summary: ReportsSummaryDto {
            opening_balance_str: format_money_ua(opening_balance),
            income_str: format_money_ua(income),
            expense_str: format_money_ua(expense),
            closing_balance_str: format_money_ua(closing_balance),
            receivables_total_str: format_money_ua(receivables_total),
            payables_total_str: format_money_ua(payables_total),
        },
        bank_rows: bank_rows_to_dto(bank_rows),
        receivables_rows: receivables_to_dto(receivables_rows),
        payables_rows: payables_to_dto(payables_rows),
    })
}

fn export_csv_content(screen: &ReportsScreenDto) -> String {
    let mut out = String::new();
    out.push_str("section,key,value\n");
    out.push_str(&format!("filter,tab,{}\n", screen.filter.tab));
    out.push_str(&format!("filter,scope,{}\n", screen.filter.scope));
    out.push_str(&format!("filter,date_from,{}\n", screen.filter.date_from));
    out.push_str(&format!("filter,date_to,{}\n", screen.filter.date_to));
    out.push_str(&format!("filter,query,{}\n", screen.filter.query));
    out.push_str(&format!(
        "summary,opening_balance,{}\nsummary,income,{}\nsummary,expense,{}\nsummary,closing_balance,{}\nsummary,receivables_total,{}\nsummary,payables_total,{}\n",
        screen.summary.opening_balance_str,
        screen.summary.income_str,
        screen.summary.expense_str,
        screen.summary.closing_balance_str,
        screen.summary.receivables_total_str,
        screen.summary.payables_total_str
    ));

    out.push_str("\nbank,label,income,expense,net\n");
    for row in &screen.bank_rows {
        out.push_str(&format!(
            "bank,{},{},{},{}\n",
            row.label, row.income_str, row.expense_str, row.net_str
        ));
    }

    out.push_str("\nreceivables,number,type,date,company,counterparty,amount,expected_date,overdue_days,status\n");
    for row in &screen.receivables_rows {
        out.push_str(&format!(
            "receivable,{},{},{},{},{},{},{},{},{}\n",
            row.doc_number,
            row.doc_type,
            row.doc_date,
            row.company_name,
            row.counterparty,
            row.amount_str,
            row.expected_date,
            row.overdue_days,
            row.status
        ));
    }

    out.push_str("\npayables,title,company,counterparty,amount,due_date,overdue_days,recurrence\n");
    for row in &screen.payables_rows {
        out.push_str(&format!(
            "payable,{},{},{},{},{},{},{}\n",
            row.title,
            row.company_name,
            row.counterparty,
            row.amount_str,
            row.due_date,
            row.overdue_days,
            row.recurrence
        ));
    }

    out
}

pub async fn reports_load(ctx: &AppCtx, request: ReportsLoadRequest) -> Result<ReportsScreenDto> {
    let today = Local::now().date_naive();
    let (filter, filter_dto) = resolve_filter(request, today)?;
    build_reports_screen(ctx, filter, filter_dto).await
}

pub async fn reports_export_csv(
    ctx: &AppCtx,
    request: ReportsExportRequest,
) -> Result<ReportsExportResultDto> {
    let screen = reports_load(
        ctx,
        ReportsLoadRequest {
            tab: Some(request.tab),
            scope: Some(request.scope),
            date_from: Some(request.date_from),
            date_to: Some(request.date_to),
            query: Some(request.query),
        },
    )
    .await?;

    fs::create_dir_all(reports_dir()).await?;
    let path = reports_dir().join(format!("reports-{}.csv", report_stamp()));
    fs::write(&path, export_csv_content(&screen)).await?;

    Ok(ReportsExportResultDto {
        ok: true,
        path: path.to_string_lossy().into_owned(),
        message: "Звіт експортовано у CSV".to_string(),
    })
}
