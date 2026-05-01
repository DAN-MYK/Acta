use chrono::NaiveDate;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportsScope {
    Active,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReportsFilter {
    pub scope: ReportsScope,
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    pub query: String,
    pub selected_counterparty_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankAggregateRow {
    pub key: String,
    pub label: String,
    pub income: Decimal,
    pub expense: Decimal,
}

pub type PnlCategoryRow = BankAggregateRow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopCounterpartyRow {
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub primary_amount: Decimal,
    pub secondary_label: String,
    pub secondary_value: String,
    pub share_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivableRow {
    pub doc_id: String,
    pub doc_type: String,
    pub doc_number: String,
    pub doc_date: NaiveDate,
    pub company_name: String,
    pub counterparty: String,
    pub amount: Decimal,
    pub expected_date: Option<NaiveDate>,
    pub overdue_days: i32,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayableRow {
    pub id: String,
    pub title: String,
    pub company_name: String,
    pub counterparty: String,
    pub amount: Decimal,
    pub due_date: NaiveDate,
    pub overdue_days: i32,
    pub recurrence: String,
}
