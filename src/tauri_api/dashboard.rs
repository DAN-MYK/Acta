use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::app_ctx::AppCtx;

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
    pub urgent_tasks: Vec<TaskItemDto>,
}

pub async fn dashboard_load(ctx: &AppCtx) -> Result<DashboardScreenDto> {
    let reports_request = ReportsLoadRequest {
        tab: Some("bank".to_string()),
        scope: Some("active".to_string()),
        date_from: None,
        date_to: None,
        query: None,
    };

    let (reports, documents, payments, tasks) = tokio::try_join!(
        reports::reports_load(ctx, reports_request),
        documents::documents_list(ctx, DocumentsListRequest::default()),
        payments::payments_list(ctx),
        tasks::tasks_list(ctx, TasksListRequest::default()),
    )?;

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
        urgent_tasks: tasks.items.into_iter().take(5).collect(),
    })
}
