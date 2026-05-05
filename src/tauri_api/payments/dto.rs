use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MutationResultDto {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenTemplateResultDto {
    pub ok: bool,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyItemDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentItemDto {
    pub id: String,
    pub date: String,
    pub counterparty_id: String,
    pub counterparty: String,
    pub amount_str: String,
    pub direction: String,
    pub matched_doc: String,
    pub account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsKpiDto {
    pub incoming_str: String,
    pub outgoing_str: String,
    pub net_str: String,
    pub unmatched_str: String,
    pub incoming_sub: String,
    pub outgoing_sub: String,
    pub unmatched_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsScreenDto {
    pub items: Vec<PaymentItemDto>,
    pub counterparties: Vec<CounterpartyItemDto>,
    pub kpi: PaymentsKpiDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentCreateOrUpdateRequest {
    pub id: String,
    pub date: String,
    pub amount: String,
    pub direction: String,
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub bank_name: String,
    pub reference: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentReconcileRequest {
    pub payment_id: String,
    pub document_kind: String,
    pub document_id: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentReconcileSplitAllocationRequest {
    pub document_kind: String,
    pub document_id: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentReconcileSplitRequest {
    pub payment_id: String,
    pub allocations: Vec<PaymentReconcileSplitAllocationRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentReconcileSplitAllocationResultDto {
    pub document_id: String,
    pub document_kind: String,
    pub title: String,
    pub amount_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentReconcileSplitResultDto {
    pub ok: bool,
    pub message: String,
    pub payment_id: String,
    pub allocation_count: usize,
    pub total_allocated_str: String,
    pub allocations: Vec<PaymentReconcileSplitAllocationResultDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentUnreconcileRequest {
    pub payment_id: String,
    pub document_kind: String,
    pub document_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentUnreconcileAllRequest {
    pub payment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMatchPreviewRequest {
    pub payment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMatchApplyAutoRequest {
    pub payment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMatchManualCandidatesRequest {
    pub payment_id: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMatchCandidateDto {
    pub document_id: String,
    pub document_kind: String,
    pub title: String,
    pub open_amount_str: String,
    pub total_score: i32,
    pub same_iban: bool,
    pub reference_hit: bool,
    pub text_hits: i32,
    pub days_distance: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAutoMatchDto {
    pub document_id: String,
    pub document_kind: String,
    pub title: String,
    pub amount_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMatchPreviewDto {
    pub payment_id: String,
    pub is_reconciled: bool,
    pub decision_kind: String,
    pub candidates: Vec<PaymentMatchCandidateDto>,
    pub auto_match: Option<PaymentAutoMatchDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentManualMatchCandidatesDto {
    pub payment_id: String,
    pub query: String,
    pub candidates: Vec<PaymentMatchCandidateDto>,
}

// ─── Private helpers ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentCalendarMonthRequest {
    pub month: String,
    pub selected_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentCalendarEventDto {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub date: String,
    pub amount_str: String,
    pub direction: String,
    pub status_label: String,
    pub recurrence_label: String,
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub link_kind: String,
    pub link_id: String,
    pub note: String,
    pub actionable: bool,
    pub overdue: bool,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentCalendarDayDto {
    pub date: String,
    pub day_number: u32,
    pub weekday_short: String,
    pub in_current_month: bool,
    pub today: bool,
    pub selected: bool,
    pub has_overdue: bool,
    pub income_total_str: String,
    pub expense_total_str: String,
    pub event_count: usize,
    pub events: Vec<PaymentCalendarEventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentCalendarMonthDto {
    pub month: String,
    pub month_label: String,
    pub selected_date: String,
    pub today: String,
    pub days: Vec<PaymentCalendarDayDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentScheduleCompleteRequest {
    pub schedule_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentImportPreviewRowDto {
    pub action: String,
    pub bank_ref: String,
    pub description: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentImportPreviewDto {
    pub ok: bool,
    pub message: String,
    pub path: String,
    pub bank_name: String,
    pub parsed: i32,
    pub will_create: i32,
    pub will_skip: i32,
    pub conflicts: i32,
    pub rows: Vec<PaymentImportPreviewRowDto>,
    pub file_size: u64,
    pub file_mtime_secs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentImportCommitRequest {
    pub path: String,
    pub file_size: u64,
    pub file_mtime_secs: i64,
}
