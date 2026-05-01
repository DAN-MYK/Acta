use std::path::PathBuf;

use crate::app_ctx::AppCtx;
use crate::db::reports::{
    compute_opening_balance, load_bank_rows, load_payables_rows, load_pnl_rows,
    load_receivables_rows, load_top_counterparties_bank, load_top_counterparties_payables,
    load_top_counterparties_pnl, load_top_counterparties_receivables,
};
use crate::models::reports::{
    BankAggregateRow, PayableRow, ReceivableRow, ReportsScope, ResolvedReportsFilter,
    TopCounterpartyRow,
};
use crate::tauri_api::reports_excel::export_excel_bytes;
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
    pub selected_counterparty_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportsFilterDto {
    pub tab: String,
    pub scope: String,
    pub date_from: String,
    pub date_to: String,
    pub query: String,
    pub selected_counterparty_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectedCounterpartyDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopCounterpartyRowDto {
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub primary_amount_str: String,
    pub secondary_label: String,
    pub secondary_value: String,
    pub share_percent: u8,
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
    pub pnl_income_str: String,
    pub pnl_expense_str: String,
    pub pnl_net_result_str: String,
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
    pub selected_counterparty: Option<SelectedCounterpartyDto>,
    pub top_counterparties: Vec<TopCounterpartyRowDto>,
    pub summary: ReportsSummaryDto,
    pub bank_rows: Vec<BankReportRowDto>,
    pub pnl_rows: Vec<BankReportRowDto>,
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
    pub selected_counterparty_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportsExportResultDto {
    pub ok: bool,
    pub path: String,
    pub message: String,
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

fn reports_dir() -> PathBuf {
    PathBuf::from("storage/reports")
}

fn report_stamp() -> String {
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn resolve_filter(
    request: ReportsLoadRequest,
    today: NaiveDate,
) -> Result<(ResolvedReportsFilter, ReportsFilterDto)> {
    let date_to = parse_ui_date(request.date_to.as_deref(), today, "Дата до")?;
    let default_from = date_to - chrono::Days::new(89);
    let date_from = parse_ui_date(request.date_from.as_deref(), default_from, "Дата від")?;
    if date_from > date_to {
        return Err(anyhow!("Дата від не може бути більшою за дату до"));
    }

    let tab = match request.tab.as_deref() {
        Some("pnl") => "pnl".to_string(),
        Some("receivables") => "receivables".to_string(),
        Some("payables") => "payables".to_string(),
        _ => "bank".to_string(),
    };

    let scope = match request.scope.as_deref() {
        Some("all") => ReportsScope::All,
        _ => ReportsScope::Active,
    };

    let query = request.query.unwrap_or_default().trim().to_string();
    let selected_counterparty_id = request
        .selected_counterparty_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Ok((
        ResolvedReportsFilter {
            scope,
            date_from,
            date_to,
            query: query.clone(),
            selected_counterparty_id: selected_counterparty_id.clone(),
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
            selected_counterparty_id,
        },
    ))
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

fn top_counterparties_to_dto(rows: Vec<TopCounterpartyRow>) -> Vec<TopCounterpartyRowDto> {
    rows.into_iter()
        .map(|row| TopCounterpartyRowDto {
            counterparty_id: row.counterparty_id,
            counterparty_name: row.counterparty_name,
            primary_amount_str: format_money_ua(row.primary_amount),
            secondary_label: row.secondary_label,
            secondary_value: row.secondary_value,
            share_percent: row.share_percent,
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
    filter: ResolvedReportsFilter,
    filter_dto: ReportsFilterDto,
) -> Result<ReportsScreenDto> {
    let (opening_balance, bank_rows, pnl_rows, receivables_rows, payables_rows) = tokio::try_join!(
        compute_opening_balance(ctx, &filter),
        load_bank_rows(ctx, &filter),
        load_pnl_rows(ctx, &filter),
        load_receivables_rows(ctx, &filter),
        load_payables_rows(ctx, &filter),
    )?;

    let top_counterparties = match filter_dto.tab.as_str() {
        "receivables" => load_top_counterparties_receivables(ctx, &filter).await?,
        "payables" => load_top_counterparties_payables(ctx, &filter).await?,
        "pnl" => load_top_counterparties_pnl(ctx, &filter).await?,
        _ => load_top_counterparties_bank(ctx, &filter).await?,
    };

    let selected_counterparty = filter.selected_counterparty_id.as_ref().and_then(|id| {
        top_counterparties
            .iter()
            .find(|r| &r.counterparty_id == id)
            .map(|r| SelectedCounterpartyDto {
                id: r.counterparty_id.clone(),
                name: r.counterparty_name.clone(),
            })
    });

    let income = bank_rows
        .iter()
        .fold(Decimal::ZERO, |acc, row| acc + row.income);
    let expense = bank_rows
        .iter()
        .fold(Decimal::ZERO, |acc, row| acc + row.expense);
    let closing_balance = opening_balance + income - expense;
    let pnl_income = pnl_rows
        .iter()
        .fold(Decimal::ZERO, |acc, row| acc + row.income);
    let pnl_expense = pnl_rows
        .iter()
        .fold(Decimal::ZERO, |acc, row| acc + row.expense);
    let pnl_net_result = pnl_income - pnl_expense;
    let receivables_total = sum_receivables(&receivables_rows);
    let payables_total = sum_payables(&payables_rows);

    Ok(ReportsScreenDto {
        filter: filter_dto,
        selected_counterparty,
        top_counterparties: top_counterparties_to_dto(top_counterparties),
        summary: ReportsSummaryDto {
            opening_balance_str: format_money_ua(opening_balance),
            income_str: format_money_ua(income),
            expense_str: format_money_ua(expense),
            closing_balance_str: format_money_ua(closing_balance),
            receivables_total_str: format_money_ua(receivables_total),
            payables_total_str: format_money_ua(payables_total),
            pnl_income_str: format_money_ua(pnl_income),
            pnl_expense_str: format_money_ua(pnl_expense),
            pnl_net_result_str: format_money_ua(pnl_net_result),
        },
        bank_rows: bank_rows_to_dto(bank_rows),
        pnl_rows: bank_rows_to_dto(pnl_rows),
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

    out.push_str("\npnl,label,income,expense,net\n");
    for row in &screen.pnl_rows {
        out.push_str(&format!(
            "pnl,{},{},{},{}\n",
            row.label, row.income_str, row.expense_str, row.net_str
        ));
    }

    out.push_str(
        "\nreceivables,number,type,date,company,counterparty,amount,expected_date,overdue_days,status\n",
    );
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
            selected_counterparty_id: request.selected_counterparty_id,
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

pub async fn reports_export_excel(
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
            selected_counterparty_id: request.selected_counterparty_id,
        },
    )
    .await?;

    fs::create_dir_all(reports_dir()).await?;
    let path = reports_dir().join(format!("reports-{}.xlsx", report_stamp()));
    fs::write(&path, export_excel_bytes(&screen)?).await?;

    Ok(ReportsExportResultDto {
        ok: true,
        path: path.to_string_lossy().into_owned(),
        message: "Звіт експортовано у Excel".to_string(),
    })
}

pub async fn reports_export_excel_and_open(
    ctx: &AppCtx,
    request: ReportsExportRequest,
) -> Result<ReportsExportResultDto> {
    let result = reports_export_excel(ctx, request).await?;
    let open_path = PathBuf::from(&result.path);
    if let Ok(Err(error)) = tokio::task::spawn_blocking(move || open::that(open_path)).await {
        tracing::warn!("reports: не вдалося відкрити Excel: {error}");
    }

    Ok(ReportsExportResultDto {
        ok: result.ok,
        path: result.path,
        message: "Звіт експортовано у Excel і відкрито".to_string(),
    })
}
