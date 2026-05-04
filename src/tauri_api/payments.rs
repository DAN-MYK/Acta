use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use crate::app_ctx::AppCtx;
use crate::db;
use crate::import::bas_payments::{
    apply_imported_payments, bank_import_dir, import_payments_from_csv,
    import_payments_from_statement, newest_payments_csv_path, newest_statement_path,
    parse_payments_statement_file, PaymentImportAction, PaymentImportPlanRow,
    PaymentImportReport,
};
use crate::models::payment::{NewPayment, PaymentDirection, UpdatePayment};
use crate::services::payment_matching::{
    choose_best_match, score_match_candidates, MatchDecision, MatchDocumentKind, MatchKind,
    PaymentMatchInput, ScoredMatchCandidate,
};

// ─── DTOs ────────────────────────────────────────────────────────────────────

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentImportCommitRequest {
    pub path: String,
}

fn format_decimal_ua(value: Decimal) -> String {
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
        .join("\u{00a0}")
        .chars()
        .rev()
        .collect::<String>();
    format!("{sign}{grouped},{frac}")
}

fn direction_to_str(dir: &PaymentDirection) -> &'static str {
    match dir {
        PaymentDirection::Income => "in",
        PaymentDirection::Expense => "out",
    }
}

fn parse_payment_date(value: &str) -> Result<NaiveDate> {
    let trimmed = value.trim();
    NaiveDate::parse_from_str(trimmed, "%d.%m.%Y")
        .or_else(|_| NaiveDate::parse_from_str(trimmed, "%Y-%m-%d"))
        .map_err(|_| anyhow!("Невірна дата. Використовуйте дд.мм.рррр або yyyy-mm-dd"))
}

fn parse_payment_amount(value: &str) -> Result<Decimal> {
    let normalized = value
        .trim()
        .replace('\u{00A0}', "")
        .replace(' ', "")
        .replace(',', ".");
    let amount = normalized
        .parse::<Decimal>()
        .map_err(|_| anyhow!("Невірна сума платежу"))?;
    if amount <= Decimal::ZERO {
        return Err(anyhow!("Сума платежу має бути більшою за нуль"));
    }
    Ok(amount)
}

fn parse_payment_direction(value: &str) -> Result<PaymentDirection> {
    match value.trim() {
        "income" => Ok(PaymentDirection::Income),
        "expense" => Ok(PaymentDirection::Expense),
        other => Err(anyhow!("Невідомий напрям платежу: {other}")),
    }
}

fn parse_optional_counterparty_id(value: &str) -> Result<Option<Uuid>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Uuid::parse_str(trimmed)
        .map(Some)
        .map_err(|_| anyhow!("Невалідний ідентифікатор контрагента"))
}

fn parse_optional_payment_id(value: &str) -> Result<Option<Uuid>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Uuid::parse_str(trimmed)
        .map(Some)
        .map_err(|_| anyhow!("Невалідний ідентифікатор платежу"))
}

fn trimmed_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_payment_uuid(value: &str) -> Result<Uuid> {
    Uuid::parse_str(value.trim()).map_err(|_| anyhow!("Невалідний ідентифікатор платежу"))
}

fn parse_calendar_month(value: &str) -> Result<NaiveDate> {
    let month_value = format!("{}-01", value.trim());
    NaiveDate::parse_from_str(&month_value, "%Y-%m-%d")
        .map_err(|_| anyhow!("Невірний місяць. Використовуйте формат yyyy-mm"))
}

fn parse_calendar_date(value: &str, field: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| anyhow!("Невірна дата у полі {field}. Використовуйте формат yyyy-mm-dd"))
}

fn format_date_iso(value: NaiveDate) -> String {
    value.format("%Y-%m-%d").to_string()
}

fn weekday_short_label(value: Weekday) -> &'static str {
    match value {
        Weekday::Mon => "Пн",
        Weekday::Tue => "Вт",
        Weekday::Wed => "Ср",
        Weekday::Thu => "Чт",
        Weekday::Fri => "Пт",
        Weekday::Sat => "Сб",
        Weekday::Sun => "Нд",
    }
}

fn month_label_uk(value: NaiveDate) -> String {
    let month = match value.month() {
        1 => "Січень",
        2 => "Лютий",
        3 => "Березень",
        4 => "Квітень",
        5 => "Травень",
        6 => "Червень",
        7 => "Липень",
        8 => "Серпень",
        9 => "Вересень",
        10 => "Жовтень",
        11 => "Листопад",
        12 => "Грудень",
        _ => "Місяць",
    };
    format!("{month} {}", value.year())
}

fn recurrence_label(value: &crate::models::payment::ScheduleRecurrence) -> &'static str {
    match value {
        crate::models::payment::ScheduleRecurrence::None => "Разово",
        crate::models::payment::ScheduleRecurrence::Weekly => "Щотижня",
        crate::models::payment::ScheduleRecurrence::Monthly => "Щомісяця",
        crate::models::payment::ScheduleRecurrence::Quarterly => "Щокварталу",
        crate::models::payment::ScheduleRecurrence::Yearly => "Щороку",
    }
}

fn schedule_status_label(is_completed: bool) -> &'static str {
    if is_completed {
        "Виконано"
    } else {
        "Заплановано"
    }
}

fn link_label_if_present(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "Без прив'язки"
    } else {
        trimmed
    }
}

fn calendar_event_sort_key(event: &PaymentCalendarEventDto) -> (u8, String, String) {
    let kind_weight = match event.kind.as_str() {
        "schedule" => 0,
        "task" => 1,
        _ => 9,
    };
    (kind_weight, event.title.clone(), event.id.clone())
}

fn parse_event_amount(value: &str) -> Decimal {
    let normalized = value
        .replace('\u{00a0}', "")
        .replace(' ', "")
        .replace(',', ".");
    normalized.parse::<Decimal>().unwrap_or(Decimal::ZERO)
}

fn calendar_grid_bounds(anchor: NaiveDate) -> Result<(NaiveDate, NaiveDate)> {
    let month_start = anchor
        .with_day(1)
        .ok_or_else(|| anyhow!("Не вдалося визначити початок місяця"))?;
    let next_month = if month_start.month() == 12 {
        NaiveDate::from_ymd_opt(month_start.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(month_start.year(), month_start.month() + 1, 1)
    }
    .ok_or_else(|| anyhow!("Не вдалося визначити наступний місяць"))?;
    let month_end = next_month - Duration::days(1);
    let grid_start =
        month_start - Duration::days(month_start.weekday().num_days_from_monday() as i64);
    let grid_end =
        month_end + Duration::days((6 - month_end.weekday().num_days_from_monday()) as i64);
    Ok((grid_start, grid_end))
}

fn build_calendar_month(
    anchor: NaiveDate,
    selected: NaiveDate,
    events: Vec<PaymentCalendarEventDto>,
) -> Result<PaymentCalendarMonthDto> {
    let (grid_start, grid_end) = calendar_grid_bounds(anchor)?;
    let today = Local::now().date_naive();
    let month_start = anchor
        .with_day(1)
        .ok_or_else(|| anyhow!("Не вдалося визначити початок місяця"))?;

    let mut events_by_date: BTreeMap<String, Vec<PaymentCalendarEventDto>> = BTreeMap::new();
    for event in events {
        events_by_date.entry(event.date.clone()).or_default().push(event);
    }

    let mut days = Vec::new();
    let mut cursor = grid_start;
    while cursor <= grid_end {
        let key = format_date_iso(cursor);
        let mut day_events = events_by_date.remove(&key).unwrap_or_default();
        day_events.sort_by_key(calendar_event_sort_key);

        let income_total = day_events
            .iter()
            .filter(|event| event.kind == "schedule" && event.direction == "income")
            .fold(Decimal::ZERO, |sum, event| sum + parse_event_amount(&event.amount_str));
        let expense_total = day_events
            .iter()
            .filter(|event| event.kind == "schedule" && event.direction == "expense")
            .fold(Decimal::ZERO, |sum, event| sum + parse_event_amount(&event.amount_str));

        days.push(PaymentCalendarDayDto {
            date: key,
            day_number: cursor.day(),
            weekday_short: weekday_short_label(cursor.weekday()).to_string(),
            in_current_month: cursor.month() == month_start.month() && cursor.year() == month_start.year(),
            today: cursor == today,
            selected: cursor == selected,
            has_overdue: day_events.iter().any(|event| event.overdue),
            income_total_str: if income_total > Decimal::ZERO {
                format_decimal_ua(income_total)
            } else {
                String::new()
            },
            expense_total_str: if expense_total > Decimal::ZERO {
                format_decimal_ua(expense_total)
            } else {
                String::new()
            },
            event_count: day_events.len(),
            events: day_events,
        });

        cursor += Duration::days(1);
    }

    Ok(PaymentCalendarMonthDto {
        month: month_start.format("%Y-%m").to_string(),
        month_label: month_label_uk(month_start),
        selected_date: format_date_iso(selected),
        today: format_date_iso(today),
        days,
    })
}

fn match_document_kind_to_str(kind: MatchDocumentKind) -> &'static str {
    match kind {
        MatchDocumentKind::Act => "act",
        MatchDocumentKind::Invoice => "invoice",
    }
}

fn match_kind_to_str(kind: MatchKind) -> &'static str {
    match kind {
        MatchKind::Exact => "exact",
        MatchKind::Ambiguous => "ambiguous",
        MatchKind::Split => "split",
        MatchKind::None => "none",
    }
}

async fn load_split_allocation_title(
    ctx: &AppCtx,
    document_kind: &str,
    document_id: Uuid,
) -> Result<String> {
    match document_kind {
        "act" => {
            let (act, _) = db::acts::get_by_id(ctx.pool(), document_id)
                .await?
                .ok_or_else(|| anyhow!("Документ не знайдено в межах компанії"))?;
            Ok(format!("Акт {}", act.number))
        }
        "invoice" => {
            let (invoice, _) = db::invoices::get_by_id(ctx.pool(), document_id)
                .await?
                .ok_or_else(|| anyhow!("Документ не знайдено в межах компанії"))?;
            Ok(format!("Накладна {}", invoice.number))
        }
        other => Err(anyhow!("Невідомий тип документу: {other}")),
    }
}

async fn build_match_input(
    ctx: &AppCtx,
    payment_id: Uuid,
) -> Result<(crate::models::payment::Payment, PaymentMatchInput)> {
    let payment = db::payments::get_by_id_scoped(ctx.pool(), ctx.company_id(), payment_id)
        .await?
        .ok_or_else(|| anyhow!("Платіж не знайдено в межах компанії"))?;

    let counterparty_iban = match payment.counterparty_id {
        Some(counterparty_id) => db::counterparties::get_by_id(
            ctx.pool(),
            ctx.company_id(),
            counterparty_id,
        )
        .await?
        .and_then(|counterparty| counterparty.iban),
        None => None,
    };

    Ok((
        payment.clone(),
        PaymentMatchInput {
            amount: payment.amount,
            date: payment.date,
            counterparty_iban,
            description: payment.description.unwrap_or_default(),
            bank_ref: payment.bank_ref,
        },
    ))
}

async fn compute_match_preview(
    ctx: &AppCtx,
    payment_id: Uuid,
) -> Result<(crate::models::payment::Payment, Vec<ScoredMatchCandidate>, MatchDecision)> {
    let (payment, input) = build_match_input(ctx, payment_id).await?;
    let candidates =
        db::payments::list_open_document_candidates(ctx.pool(), ctx.company_id(), payment.direction.clone())
            .await?;
    let scored_candidates = score_match_candidates(&input, &candidates);
    let decision = choose_best_match(&input, &candidates);
    Ok((payment, scored_candidates, decision))
}

fn scored_candidate_to_dto(candidate: ScoredMatchCandidate) -> PaymentMatchCandidateDto {
    PaymentMatchCandidateDto {
        document_id: candidate.candidate.document_id.to_string(),
        document_kind: match_document_kind_to_str(candidate.candidate.document_kind).to_string(),
        title: candidate.candidate.title,
        open_amount_str: format_decimal_ua(candidate.candidate.open_amount),
        total_score: candidate.score.total,
        same_iban: candidate.score.same_iban,
        reference_hit: candidate.score.reference_hit,
        text_hits: candidate.score.text_hits as i32,
        days_distance: candidate.score.days_distance,
    }
}

fn preview_candidates_for_decision(
    decision: &MatchDecision,
    scored_candidates: Vec<ScoredMatchCandidate>,
) -> Vec<ScoredMatchCandidate> {
    match decision {
        MatchDecision::Split(candidates) => candidates.clone(),
        MatchDecision::Exact(_) | MatchDecision::Ambiguous(_) | MatchDecision::None => scored_candidates,
    }
}

fn exact_recommendation(decision: &MatchDecision) -> Option<PaymentAutoMatchDto> {
    match decision {
        MatchDecision::Exact(candidate) => Some(PaymentAutoMatchDto {
            document_id: candidate.candidate.document_id.to_string(),
            document_kind: match_document_kind_to_str(candidate.candidate.document_kind).to_string(),
            title: candidate.candidate.title.clone(),
            amount_str: format_decimal_ua(candidate.candidate.open_amount),
        }),
        MatchDecision::Ambiguous(_) | MatchDecision::Split(_) | MatchDecision::None => None,
    }
}

fn normalize_search_text(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect()
}

fn manual_candidate_matches_query(
    candidate: &crate::services::payment_matching::MatchCandidate,
    query: &str,
) -> bool {
    let normalized_query = normalize_search_text(query);
    let tokens: Vec<_> = normalized_query
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect();

    if tokens.is_empty() {
        return true;
    }

    let haystack = normalize_search_text(&format!(
        "{} {} {}",
        candidate.title,
        candidate.reference_text.clone().unwrap_or_default(),
        candidate.match_text.clone().unwrap_or_default()
    ));

    tokens.iter().all(|token| haystack.contains(token))
}

async fn run_bank_import(ctx: &AppCtx) -> Result<(usize, PathBuf)> {
    // Backward-compat: для legacy "Імпорт виписки" з `storage/import/bank/`.
    // Підтримує і CSV, і XLSX через `newest_statement_path`.
    let path = newest_statement_path().await?;
    let report =
        import_payments_from_statement(ctx.pool(), ctx.company_id(), &path, false).await?;
    Ok((report.created, path))
}

fn payment_action_to_str(action: &PaymentImportAction) -> &'static str {
    match action {
        PaymentImportAction::Create => "create",
        PaymentImportAction::Skip => "skip",
    }
}

fn build_import_preview_dto(
    path: &Path,
    bank_name: &str,
    report: PaymentImportReport,
) -> PaymentImportPreviewDto {
    let parsed = report.parsed as i32;
    let will_create = report.created as i32;
    let will_skip = report.skipped as i32;
    let conflicts = report.conflicts as i32;

    let rows = report
        .rows
        .into_iter()
        .map(|row: PaymentImportPlanRow| PaymentImportPreviewRowDto {
            action: payment_action_to_str(&row.action).to_string(),
            bank_ref: row.bank_ref.unwrap_or_default(),
            description: row.description,
            note: row.note.unwrap_or_default(),
        })
        .collect();

    let message = if parsed == 0 {
        "У файлі не знайдено жодного рядка виписки".to_string()
    } else if will_create == 0 {
        format!("У файлі {parsed} рядків, але всі вже імпортовано раніше")
    } else {
        format!(
            "Знайдено {parsed} рядків. Буде створено {will_create}, пропущено {will_skip}",
        )
    };

    PaymentImportPreviewDto {
        ok: true,
        message,
        path: path.to_string_lossy().into_owned(),
        bank_name: bank_name.to_string(),
        parsed,
        will_create,
        will_skip,
        conflicts,
        rows,
    }
}

async fn ensure_manual_import_template() -> Result<PathBuf> {
    let dir = bank_import_dir();
    fs::create_dir_all(&dir).await?;
    let path = dir.join("manual-payment-template.csv");
    if fs::metadata(&path).await.is_err() {
        let template = concat!(
            "date,amount,description,direction,reference\n",
            "2026-04-22,1500.00,Ручне надходження,income,MANUAL-001\n",
        );
        fs::write(&path, template).await?;
    }
    Ok(path)
}

// ─── Public API ───────────────────────────────────────────────────────────────

pub async fn payments_list(ctx: &AppCtx) -> Result<PaymentsScreenDto> {
    let company_id = ctx.company_id();
    let (rows_res, counterparties_res, kpi_res) = tokio::join!(
        db::payments::list(ctx.pool(), company_id, None),
        db::counterparties::list(ctx.pool(), company_id),
        db::payments::payment_kpi(ctx.pool(), company_id),
    );

    let rows = rows_res?;
    let counterparties = counterparties_res?;
    let kpi = kpi_res?;
    let net = kpi.incoming_month - kpi.outgoing_month;

    let items = rows
        .iter()
        .map(|r| PaymentItemDto {
            id: r.id.to_string(),
            date: r.date.clone(),
            counterparty_id: r
                .counterparty_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            counterparty: r.counterparty_name.as_deref().unwrap_or("").to_string(),
            amount_str: format_decimal_ua(r.amount),
            direction: direction_to_str(&r.direction).to_string(),
            matched_doc: String::new(),
            account: r.bank_name.as_deref().unwrap_or("").to_string(),
        })
        .collect();

    let counterparty_items = counterparties
        .iter()
        .map(|c| CounterpartyItemDto {
            id: c.id.to_string(),
            name: c.name.clone(),
        })
        .collect();

    Ok(PaymentsScreenDto {
        items,
        counterparties: counterparty_items,
        kpi: PaymentsKpiDto {
            incoming_str: format_decimal_ua(kpi.incoming_month),
            outgoing_str: format_decimal_ua(kpi.outgoing_month),
            net_str: format_decimal_ua(net),
            unmatched_str: kpi.unmatched_count.to_string(),
            incoming_sub: "поточний місяць".to_string(),
            outgoing_sub: "поточний місяць".to_string(),
            unmatched_count: kpi.unmatched_count as i32,
        },
    })
}

pub async fn payments_calendar_load(
    ctx: &AppCtx,
    request: PaymentCalendarMonthRequest,
) -> Result<PaymentCalendarMonthDto> {
    let anchor = parse_calendar_month(&request.month)?;
    let month_start = anchor
        .with_day(1)
        .ok_or_else(|| anyhow!("Не вдалося визначити початок місяця"))?;
    let (grid_start, grid_end) = calendar_grid_bounds(anchor)?;
    let today = Local::now().date_naive();
    let selected = match request.selected_date.as_deref() {
        Some(value) => parse_calendar_date(value, "selectedDate")?,
        None if today.month() == month_start.month() && today.year() == month_start.year() => today,
        None => month_start,
    };

    let schedules = db::payments::list_schedule_in_range(
        ctx.pool(),
        ctx.company_id(),
        grid_start,
        grid_end,
    )
    .await?;
    let tasks = db::tasks::list_all(ctx.pool(), ctx.company_id()).await?;

    let mut events = Vec::new();

    for schedule in schedules {
        let (counterparty_id, counterparty_name) = match schedule.counterparty_id {
            Some(counterparty_id) => db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id)
                .await?
                .map(|item| (item.id.to_string(), item.name))
                .unwrap_or_default(),
            None => (String::new(), String::new()),
        };

        events.push(PaymentCalendarEventDto {
            id: schedule.id.to_string(),
            kind: "schedule".to_string(),
            title: schedule.title,
            subtitle: if counterparty_name.is_empty() {
                "Плановий платіж".to_string()
            } else {
                counterparty_name.clone()
            },
            date: format_date_iso(schedule.scheduled_date),
            amount_str: schedule.amount.map(format_decimal_ua).unwrap_or_default(),
            direction: schedule.direction.as_str().to_string(),
            status_label: schedule_status_label(schedule.is_completed).to_string(),
            recurrence_label: recurrence_label(&schedule.recurrence).to_string(),
            counterparty_id,
            counterparty_name,
            link_kind: "schedule".to_string(),
            link_id: schedule.id.to_string(),
            note: schedule.notes.unwrap_or_default(),
            actionable: !schedule.is_completed,
            overdue: !schedule.is_completed && schedule.scheduled_date < today,
            done: schedule.is_completed,
        });
    }

    for task in tasks {
        let Some(due_date) = task.due_date.map(|value| value.with_timezone(&Local).date_naive()) else {
            continue;
        };
        if due_date < grid_start || due_date > grid_end {
            continue;
        }

        let (link_kind, link_label) = crate::tauri_api::tasks::resolve_link_label(ctx, &task).await?;
        let (counterparty_id, counterparty_name) = match task.counterparty_id {
            Some(counterparty_id) => db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id)
                .await?
                .map(|item| (item.id.to_string(), item.name))
                .unwrap_or_default(),
            None => (String::new(), String::new()),
        };

        let is_done = matches!(
            task.status,
            crate::models::task::TaskStatus::Done | crate::models::task::TaskStatus::Cancelled
        );

        events.push(PaymentCalendarEventDto {
            id: task.id.to_string(),
            kind: "task".to_string(),
            title: task.title.clone(),
            subtitle: format!("{} · {}", task.priority.label(), link_label_if_present(&link_label)),
            date: format_date_iso(due_date),
            amount_str: String::new(),
            direction: String::new(),
            status_label: task.status.label().to_string(),
            recurrence_label: String::new(),
            counterparty_id,
            counterparty_name,
            link_kind: if link_kind.is_empty() { "task".to_string() } else { link_kind },
            link_id: task.id.to_string(),
            note: task.description.unwrap_or_default(),
            actionable: !is_done,
            overdue: !is_done && due_date < today,
            done: is_done,
        });
    }

    build_calendar_month(anchor, selected, events)
}

pub async fn payment_schedule_complete(
    ctx: &AppCtx,
    request: PaymentScheduleCompleteRequest,
) -> Result<MutationResultDto> {
    let schedule_id = Uuid::parse_str(request.schedule_id.trim())
        .map_err(|_| anyhow!("Невалідний ідентифікатор запланованого платежу"))?;
    let updated =
        db::payments::complete_schedule_scoped(ctx.pool(), ctx.company_id(), schedule_id).await?;
    anyhow::ensure!(
        updated,
        "Запланований платіж не знайдено в межах активної компанії"
    );

    Ok(MutationResultDto {
        ok: true,
        message: "Запланований платіж позначено як виконаний".to_string(),
    })
}

pub async fn payments_import_latest_csv(ctx: &AppCtx) -> Result<MutationResultDto> {
    let (imported, path) = run_bank_import(ctx).await?;
    Ok(MutationResultDto {
        ok: true,
        message: format!("Імпортовано {imported} рядків з {}", path.display()),
    })
}

/// Робить dry-run попереднього перегляду виписки за вказаним шляхом.
/// Не вносить змін у БД, лише повертає підрахунок та плановані рядки.
/// Використовується після вибору файлу через нативний file picker.
pub async fn payments_import_preview(
    ctx: &AppCtx,
    path: String,
) -> Result<PaymentImportPreviewDto> {
    let trimmed = path.trim();
    anyhow::ensure!(
        !trimmed.is_empty(),
        "Не вказано шляху до файлу виписки для попереднього перегляду"
    );
    let path_buf = PathBuf::from(trimmed);
    if fs::metadata(&path_buf).await.is_err() {
        return Err(anyhow!(
            "Файл {} не знайдено або до нього немає доступу",
            path_buf.display()
        ));
    }

    let parsed_rows = parse_payments_statement_file(&path_buf).await?;
    let bank_name = parsed_rows
        .first()
        .map(|row| row.bank_name.clone())
        .unwrap_or_else(|| {
            crate::import::bas_payments::parser_for_path(&path_buf)
                .bank_name()
                .to_string()
        });

    let report =
        apply_imported_payments(ctx.pool(), ctx.company_id(), &parsed_rows, true).await?;

    Ok(build_import_preview_dto(&path_buf, &bank_name, report))
}

/// Підтверджує імпорт після `payments_import_preview` — фактично виконує запис у БД.
pub async fn payments_import_commit(
    ctx: &AppCtx,
    request: PaymentImportCommitRequest,
) -> Result<MutationResultDto> {
    let trimmed = request.path.trim();
    anyhow::ensure!(
        !trimmed.is_empty(),
        "Не вказано шляху до файлу виписки для імпорту"
    );

    let path = PathBuf::from(trimmed);
    if fs::metadata(&path).await.is_err() {
        return Err(anyhow!(
            "Файл {} не знайдено або до нього немає доступу",
            path.display()
        ));
    }

    let report =
        import_payments_from_statement(ctx.pool(), ctx.company_id(), &path, false).await?;

    Ok(MutationResultDto {
        ok: true,
        message: format!(
            "Імпортовано {} нових платежів з {} (пропущено {})",
            report.created,
            path.display(),
            report.skipped,
        ),
    })
}

pub async fn payments_sync_bank(ctx: &AppCtx) -> Result<MutationResultDto> {
    let (imported, path) = run_bank_import(ctx).await?;
    Ok(MutationResultDto {
        ok: true,
        message: format!(
            "Оброблено файл {}. Нових платежів: {imported}",
            path.display()
        ),
    })
}

pub async fn payments_open_manual_template(_ctx: &AppCtx) -> Result<OpenTemplateResultDto> {
    let path = ensure_manual_import_template().await?;
    let open_path = path.clone();
    if let Ok(Err(e)) = tokio::task::spawn_blocking(move || open::that(open_path)).await {
        tracing::warn!("payments: не вдалося відкрити шаблон CSV: {e}");
    }
    Ok(OpenTemplateResultDto {
        ok: true,
        path: path.to_string_lossy().into_owned(),
        message: "Шаблон CSV відкрито".to_string(),
    })
}

pub async fn payment_create_or_update(
    ctx: &AppCtx,
    request: PaymentCreateOrUpdateRequest,
) -> Result<MutationResultDto> {
    let date = parse_payment_date(&request.date)?;
    let amount = parse_payment_amount(&request.amount)?;
    let direction = parse_payment_direction(&request.direction)?;
    let counterparty_id = parse_optional_counterparty_id(&request.counterparty_id)?;
    let bank_name = trimmed_option(&request.bank_name);
    let bank_ref = trimmed_option(&request.reference);
    let description = trimmed_option(&request.description);

    if let Some(id) = parse_optional_payment_id(&request.id)? {
        db::payments::update_scoped(
            ctx.pool(),
            ctx.company_id(),
            id,
            UpdatePayment {
                date,
                amount,
                direction,
                counterparty_id,
                bank_name,
                bank_ref,
                description,
            },
        )
        .await?
        .ok_or_else(|| anyhow!("Платіж не знайдено або він не належить поточній компанії"))?;
        Ok(MutationResultDto {
            ok: true,
            message: "Платіж оновлено".to_string(),
        })
    } else {
        db::payments::create(
            ctx.pool(),
            NewPayment {
                company_id: ctx.company_id(),
                date,
                amount,
                direction,
                counterparty_id,
                bank_name,
                bank_ref,
                description,
            },
        )
        .await?;
        Ok(MutationResultDto {
            ok: true,
            message: "Платіж створено".to_string(),
        })
    }
}

pub async fn payment_reconcile(
    ctx: &AppCtx,
    req: PaymentReconcileRequest,
) -> Result<MutationResultDto> {
    let payment_id = Uuid::parse_str(&req.payment_id)
        .map_err(|_| anyhow!("Невалідний ідентифікатор платежу"))?;
    let doc_id = Uuid::parse_str(&req.document_id)
        .map_err(|_| anyhow!("Невалідний ідентифікатор документу"))?;
    let amount = parse_payment_amount(&req.amount)?;

    db::payments::reconcile_document_scoped(
        ctx.pool(),
        ctx.company_id(),
        payment_id,
        &req.document_kind,
        doc_id,
        amount,
    )
    .await?;

    Ok(MutationResultDto {
        ok: true,
        message: "Платіж зведено з документом".to_string(),
    })
}

pub async fn payment_reconcile_split(
    ctx: &AppCtx,
    req: PaymentReconcileSplitRequest,
) -> Result<PaymentReconcileSplitResultDto> {
    let payment_id = parse_payment_uuid(&req.payment_id)?;
    anyhow::ensure!(
        !req.allocations.is_empty(),
        "Для розподілу платежу потрібен хоча б один документ"
    );

    let mut unique_documents = HashSet::new();
    let mut allocation_results = Vec::new();
    let mut total_allocated = Decimal::ZERO;
    let allocations = req
        .allocations
        .into_iter()
        .map(|allocation| {
            let document_id = Uuid::parse_str(&allocation.document_id)
                .map_err(|_| anyhow!("Невалідний ідентифікатор документу"))?;
            let amount = parse_payment_amount(&allocation.amount)?;
            anyhow::ensure!(
                unique_documents.insert((allocation.document_kind.clone(), document_id)),
                "Один і той самий документ не можна передати двічі в split reconcile"
            );

            total_allocated += amount;
            allocation_results.push(PaymentReconcileSplitAllocationResultDto {
                document_id: document_id.to_string(),
                document_kind: allocation.document_kind.clone(),
                title: String::new(),
                amount_str: format_decimal_ua(amount),
            });

            Ok(db::payments::PaymentReconcileAllocation {
                document_kind: allocation.document_kind,
                document_id,
                amount,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    db::payments::reconcile_split_scoped(ctx.pool(), ctx.company_id(), payment_id, &allocations)
        .await?;

    for allocation in &mut allocation_results {
        allocation.title = load_split_allocation_title(
            ctx,
            &allocation.document_kind,
            Uuid::parse_str(&allocation.document_id)
                .map_err(|_| anyhow!("Невалідний ідентифікатор документу"))?,
        )
        .await?;
    }

    Ok(PaymentReconcileSplitResultDto {
        ok: true,
        message: "Розподіл платежу підтверджено".to_string(),
        payment_id: payment_id.to_string(),
        allocation_count: allocation_results.len(),
        total_allocated_str: format_decimal_ua(total_allocated),
        allocations: allocation_results,
    })
}

pub async fn payment_unreconcile(
    ctx: &AppCtx,
    req: PaymentUnreconcileRequest,
) -> Result<MutationResultDto> {
    let payment_id = Uuid::parse_str(&req.payment_id)
        .map_err(|_| anyhow!("Невалідний ідентифікатор платежу"))?;
    let doc_id = Uuid::parse_str(&req.document_id)
        .map_err(|_| anyhow!("Невалідний ідентифікатор документу"))?;

    db::payments::unreconcile_document_scoped(
        ctx.pool(),
        ctx.company_id(),
        payment_id,
        &req.document_kind,
        doc_id,
    )
    .await?;

    Ok(MutationResultDto {
        ok: true,
        message: "Зведення платежу скасовано".to_string(),
    })
}

pub async fn payment_unreconcile_all(
    ctx: &AppCtx,
    req: PaymentUnreconcileAllRequest,
) -> Result<MutationResultDto> {
    let payment_id = parse_payment_uuid(&req.payment_id)?;
    db::payments::unreconcile_all_scoped(ctx.pool(), ctx.company_id(), payment_id).await?;

    Ok(MutationResultDto {
        ok: true,
        message: "Зведення платежу скасовано".to_string(),
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

pub async fn payment_match_preview(
    ctx: &AppCtx,
    req: PaymentMatchPreviewRequest,
) -> Result<PaymentMatchPreviewDto> {
    let payment_id = parse_payment_uuid(&req.payment_id)?;
    let (payment, scored_candidates, decision) = compute_match_preview(ctx, payment_id).await?;
    let preview_candidates = preview_candidates_for_decision(&decision, scored_candidates);

    Ok(PaymentMatchPreviewDto {
        payment_id: payment.id.to_string(),
        is_reconciled: payment.is_reconciled,
        decision_kind: match_kind_to_str(decision.kind()).to_string(),
        candidates: preview_candidates
            .into_iter()
            .map(scored_candidate_to_dto)
            .collect(),
        auto_match: exact_recommendation(&decision),
    })
}

pub async fn payment_match_apply_auto(
    ctx: &AppCtx,
    req: PaymentMatchApplyAutoRequest,
) -> Result<MutationResultDto> {
    let payment_id = parse_payment_uuid(&req.payment_id)?;
    let (payment, _scored_candidates, decision) = compute_match_preview(ctx, payment_id).await?;

    anyhow::ensure!(
        !payment.is_reconciled,
        "Платіж уже звірено. Спершу скасуйте поточне звірення"
    );

    match decision {
        MatchDecision::Exact(candidate) => {
            db::payments::reconcile_document_scoped(
                ctx.pool(),
                ctx.company_id(),
                payment.id,
                match_document_kind_to_str(candidate.candidate.document_kind),
                candidate.candidate.document_id,
                payment.amount,
            )
            .await?;

            Ok(MutationResultDto {
                ok: true,
                message: "Автозіставлення платежу застосовано".to_string(),
            })
        }
        MatchDecision::Split(_) => Err(anyhow!(
            "Автозіставлення неможливе: знайдено розподіл платежу між кількома документами"
        )),
        MatchDecision::Ambiguous(_) => Err(anyhow!(
            "Автозіставлення неможливе: знайдено неоднозначне зіставлення з кількома рівноцінними кандидатами"
        )),
        MatchDecision::None => Err(anyhow!(
            "Автозіставлення неможливе: точний кандидат для платежу не знайдено"
        )),
    }
}

pub async fn payment_match_manual_candidates(
    ctx: &AppCtx,
    req: PaymentMatchManualCandidatesRequest,
) -> Result<PaymentManualMatchCandidatesDto> {
    let payment_id = parse_payment_uuid(&req.payment_id)?;
    let (payment, input) = build_match_input(ctx, payment_id).await?;
    let candidates =
        db::payments::list_open_document_candidates(ctx.pool(), ctx.company_id(), payment.direction)
            .await?;

    let mut exact_scores = std::collections::BTreeMap::new();
    for scored in score_match_candidates(&input, &candidates) {
        exact_scores.insert(scored.candidate.document_id, scored.score);
    }

    let mut filtered: Vec<_> = candidates
        .into_iter()
        .filter(|candidate| manual_candidate_matches_query(candidate, &req.query))
        .collect();

    filtered.sort_by(|left, right| {
        let left_score = exact_scores
            .get(&left.document_id)
            .map(|score| score.total)
            .unwrap_or_default();
        let right_score = exact_scores
            .get(&right.document_id)
            .map(|score| score.total)
            .unwrap_or_default();

        right_score
            .cmp(&left_score)
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| left.document_id.cmp(&right.document_id))
    });

    Ok(PaymentManualMatchCandidatesDto {
        payment_id: payment_id.to_string(),
        query: req.query.trim().to_string(),
        candidates: filtered
            .into_iter()
            .take(25)
            .map(|candidate| {
                let score = exact_scores.get(&candidate.document_id);
                PaymentMatchCandidateDto {
                    document_id: candidate.document_id.to_string(),
                    document_kind: match_document_kind_to_str(candidate.document_kind).to_string(),
                    title: candidate.title,
                    open_amount_str: format_decimal_ua(candidate.open_amount),
                    total_score: score.map(|value| value.total).unwrap_or_default(),
                    same_iban: score.map(|value| value.same_iban).unwrap_or(false),
                    reference_hit: score.map(|value| value.reference_hit).unwrap_or(false),
                    text_hits: score.map(|value| value.text_hits as i32).unwrap_or_default(),
                    days_distance: score.map(|value| value.days_distance).unwrap_or(365),
                }
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use crate::services::payment_matching::{MatchCandidate, MatchScore};

    #[test]
    fn parse_payment_date_dd_mm_yyyy() {
        let d = parse_payment_date("15.04.2026").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn parse_payment_date_iso_format() {
        let d = parse_payment_date("2026-04-15").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn parse_payment_date_trims_whitespace() {
        let d = parse_payment_date("  15.04.2026  ").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn parse_payment_date_invalid_returns_err() {
        assert!(parse_payment_date("not-a-date").is_err());
        assert!(parse_payment_date("32.01.2026").is_err());
    }

    #[test]
    fn parse_payment_amount_standard_dot() {
        assert_eq!(parse_payment_amount("1500.00").unwrap(), dec!(1500.00));
    }

    #[test]
    fn parse_payment_amount_comma_separator() {
        assert_eq!(parse_payment_amount("1500,50").unwrap(), dec!(1500.50));
    }

    #[test]
    fn parse_payment_amount_zero_returns_err() {
        assert!(parse_payment_amount("0").is_err());
        assert!(parse_payment_amount("0.00").is_err());
    }

    #[test]
    fn parse_payment_amount_negative_returns_err() {
        assert!(parse_payment_amount("-100.00").is_err());
    }

    #[test]
    fn parse_payment_amount_invalid_returns_err() {
        assert!(parse_payment_amount("abc").is_err());
    }

    #[test]
    fn parse_payment_direction_income() {
        assert!(matches!(
            parse_payment_direction("income").unwrap(),
            PaymentDirection::Income
        ));
    }

    #[test]
    fn parse_payment_direction_expense() {
        assert!(matches!(
            parse_payment_direction("expense").unwrap(),
            PaymentDirection::Expense
        ));
    }

    #[test]
    fn parse_payment_direction_invalid_returns_err() {
        assert!(parse_payment_direction("credit").is_err());
        assert!(parse_payment_direction("in").is_err());
        assert!(parse_payment_direction("").is_err());
    }

    #[test]
    fn parse_optional_counterparty_id_empty_is_none() {
        assert_eq!(parse_optional_counterparty_id("").unwrap(), None);
        assert_eq!(parse_optional_counterparty_id("  ").unwrap(), None);
    }

    #[test]
    fn parse_optional_counterparty_id_valid_uuid() {
        let id = Uuid::new_v4();
        assert_eq!(
            parse_optional_counterparty_id(&id.to_string()).unwrap(),
            Some(id)
        );
    }

    #[test]
    fn parse_optional_counterparty_id_invalid_returns_err() {
        assert!(parse_optional_counterparty_id("not-a-uuid").is_err());
    }

    #[test]
    fn parse_optional_payment_id_empty_is_none() {
        assert_eq!(parse_optional_payment_id("").unwrap(), None);
    }

    #[test]
    fn parse_optional_payment_id_valid_uuid() {
        let id = Uuid::new_v4();
        assert_eq!(
            parse_optional_payment_id(&id.to_string()).unwrap(),
            Some(id)
        );
    }

    #[test]
    fn direction_to_str_maps_correctly() {
        assert_eq!(direction_to_str(&PaymentDirection::Income), "in");
        assert_eq!(direction_to_str(&PaymentDirection::Expense), "out");
    }

    #[test]
    fn trimmed_option_empty_is_none() {
        assert_eq!(trimmed_option(""), None);
        assert_eq!(trimmed_option("   "), None);
    }

    #[test]
    fn trimmed_option_non_empty_is_some() {
        assert_eq!(trimmed_option("  hello  "), Some("hello".to_string()));
    }

    #[test]
    fn format_decimal_ua_basic() {
        assert_eq!(format_decimal_ua(dec!(1234.56)), "1\u{00a0}234,56");
    }

    #[test]
    fn format_decimal_ua_small() {
        assert_eq!(format_decimal_ua(dec!(5.00)), "5,00");
    }

    #[test]
    fn format_decimal_ua_negative() {
        assert_eq!(format_decimal_ua(dec!(-1234.56)), "-1\u{00a0}234,56");
    }

    #[test]
    fn format_decimal_ua_zero() {
        assert_eq!(format_decimal_ua(dec!(0)), "0,00");
    }

    #[test]
    fn payment_match_preview_helpers_map_exact_decision() {
        let decision = MatchDecision::Exact(ScoredMatchCandidate {
            candidate: MatchCandidate::act(
                Uuid::new_v4(),
                dec!(1250.00),
                Some("UA123".to_string()),
                "Акт №42",
                "ACT-42",
                "Оплата акту №42",
                Some(NaiveDate::from_ymd_opt(2026, 5, 1).expect("валідна дата")),
            ),
            score: MatchScore {
                total: 170,
                amount_fits: true,
                exact_amount: true,
                same_iban: true,
                reference_hit: true,
                text_hits: 2,
                days_distance: 0,
            },
        });

        let recommendation =
            exact_recommendation(&decision).expect("exact decision має повертати recommendation");

        assert_eq!(match_kind_to_str(decision.kind()), "exact");
        assert_eq!(recommendation.document_kind, "act");
        assert_eq!(recommendation.title, "Акт №42");
        assert_eq!(recommendation.amount_str, "1\u{00a0}250,00");
    }

    #[test]
    fn payment_match_preview_helpers_map_candidate_scores() {
        let dto = scored_candidate_to_dto(ScoredMatchCandidate {
            candidate: MatchCandidate::invoice(
                Uuid::new_v4(),
                dec!(980.00),
                None,
                "Рахунок №7",
                "INV-7",
                "Оплата послуг",
                Some(NaiveDate::from_ymd_opt(2026, 5, 3).expect("валідна дата")),
            ),
            score: MatchScore {
                total: 130,
                amount_fits: true,
                exact_amount: true,
                same_iban: false,
                reference_hit: false,
                text_hits: 1,
                days_distance: 2,
            },
        });

        assert_eq!(dto.document_kind, "invoice");
        assert_eq!(dto.open_amount_str, "980,00");
        assert_eq!(dto.total_score, 130);
        assert_eq!(dto.text_hits, 1);
        assert_eq!(dto.days_distance, 2);
    }

    #[test]
    fn payment_match_preview_helpers_map_split_decision_kind() {
        let decision = MatchDecision::Split(vec![ScoredMatchCandidate {
            candidate: MatchCandidate::invoice(
                Uuid::new_v4(),
                dec!(1500.00),
                None,
                "Накладна INV-007",
                "INV-7",
                "Оплата накладної",
                Some(NaiveDate::from_ymd_opt(2026, 5, 3).expect("валідна дата")),
            ),
            score: MatchScore {
                total: 88,
                amount_fits: true,
                exact_amount: false,
                same_iban: true,
                reference_hit: false,
                text_hits: 1,
                days_distance: 2,
            },
        }]);

        assert_eq!(match_kind_to_str(decision.kind()), "split");
        assert!(exact_recommendation(&decision).is_none());
    }

    #[test]
    fn compile_check_public_function_signatures() {
        let _ = payments_list;
        let _ = payments_import_latest_csv;
        let _ = payments_sync_bank;
        let _ = payments_open_manual_template;
        let _ = payment_create_or_update;
        let _ = payment_reconcile;
        let _ = payment_reconcile_split;
        let _ = payment_unreconcile;
        let _ = payment_match_preview;
        let _ = payment_match_apply_auto;
    }
}
