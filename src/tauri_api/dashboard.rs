use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::app_ctx::AppCtx;
use crate::db;
use crate::models::payment::PaymentDirection;

use super::documents::{self, DocumentItemDto, DocumentsListRequest};
use super::payments;
use super::reports::{self, BankReportRowDto, ReportsLoadRequest};
use super::tasks::{self, TaskItemDto, TasksListRequest};

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

fn format_money_ua(value: rust_decimal::Decimal) -> String {
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

fn format_short_date_label(value: &str) -> String {
    let month_abbr = |month: u32| match month {
        1 => "січ",
        2 => "лют",
        3 => "бер",
        4 => "кві",
        5 => "тра",
        6 => "чер",
        7 => "лип",
        8 => "сер",
        9 => "вер",
        10 => "жов",
        11 => "лис",
        12 => "гру",
        _ => "???",
    };

    NaiveDate::parse_from_str(value, "%d.%m.%Y")
        .map(|date| format!("{:02} {}", date.day(), month_abbr(date.month())))
        .unwrap_or_else(|_| value.to_string())
}

pub async fn dashboard_load(ctx: &AppCtx) -> Result<DashboardScreenDto> {
    let reports_request = ReportsLoadRequest {
        tab: Some("bank".to_string()),
        scope: Some("active".to_string()),
        date_from: None,
        date_to: None,
        query: None,
        selected_counterparty_id: None,
    };

    let (reports, documents, payments, payment_rows, tasks) = tokio::try_join!(
        reports::reports_load(ctx, reports_request),
        documents::documents_list(ctx, DocumentsListRequest::default()),
        payments::payments_list(ctx),
        db::payments::list(ctx.pool(), ctx.company_id(), Some(PaymentDirection::Income)),
        tasks::tasks_list(ctx, TasksListRequest::default()),
    )?;

    let today = Local::now().date_naive();
    let mut upcoming_payments = payment_rows
        .into_iter()
        .filter(|payment| !payment.is_reconciled)
        .filter_map(|payment| {
            NaiveDate::parse_from_str(&payment.date, "%d.%m.%Y")
                .ok()
                .map(|date| (date, payment))
        })
        .collect::<Vec<_>>();

    upcoming_payments.sort_by_key(|(date, _)| (*date > today, *date));

    let kpis = vec![
        DashboardKpiDto {
            label: "Дохід за період".to_string(),
            value: reports.summary.income_str.clone(),
            detail: "За останні 90 днів".to_string(),
            tone: "positive".to_string(),
        },
        DashboardKpiDto {
            label: "Витрати за період".to_string(),
            value: reports.summary.expense_str.clone(),
            detail: format!("До сплати: {}", reports.summary.payables_total_str),
            tone: "warning".to_string(),
        },
        DashboardKpiDto {
            label: "Документи".to_string(),
            value: documents.total_count.to_string(),
            detail: format!("{} сторінок у поточній вибірці", documents.page_count),
            tone: "neutral".to_string(),
        },
        DashboardKpiDto {
            label: "Завдання".to_string(),
            value: tasks.open_count.to_string(),
            detail: format!(
                "{} сьогодні, {} високий пріоритет",
                tasks.today_count, tasks.high_count
            ),
            tone: "accent".to_string(),
        },
        DashboardKpiDto {
            label: "Нерознесені платежі".to_string(),
            value: payments.kpi.unmatched_str.clone(),
            detail: format!("{} платежів потребують уваги", payments.kpi.unmatched_count),
            tone: "danger".to_string(),
        },
        DashboardKpiDto {
            label: "Дебіторка".to_string(),
            value: reports.summary.receivables_total_str.clone(),
            detail: "Очікувані надходження".to_string(),
            tone: "positive".to_string(),
        },
    ];

    Ok(DashboardScreenDto {
        kpis,
        cashflow_rows: reports.bank_rows.into_iter().take(6).collect(),
        recent_documents: documents.items.into_iter().take(6).collect(),
        upcoming_payments: upcoming_payments
            .into_iter()
            .take(5)
            .map(|(date, payment)| DashboardUpcomingPaymentDto {
                id: payment.id.to_string(),
                date_label: format_short_date_label(&payment.date),
                contractor: payment
                    .counterparty_name
                    .unwrap_or_else(|| "Без контрагента".to_string()),
                amount_str: format_money_ua(payment.amount),
                is_overdue: date <= today,
            })
            .collect(),
        urgent_tasks: tasks.items.into_iter().take(5).collect(),
    })
}
