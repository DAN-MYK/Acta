use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use tokio::fs;
use uuid::Uuid;

use crate::app_ctx::AppCtx;
use crate::db;
use crate::import::bas_payments::{
    apply_imported_payments, bank_import_dir, import_payments_from_statement,
    newest_statement_path, parse_payments_statement_file, PaymentImportAction,
    PaymentImportPlanRow, PaymentImportReport,
};
use crate::models::payment::{NewPayment, PaymentDirection, UpdatePayment};
use crate::services::payment_matching::{
    choose_best_match, score_match_candidates, MatchDecision, MatchDocumentKind, MatchKind,
    PaymentMatchInput, ScoredMatchCandidate,
};

use super::dto::*;

mod calendar;
pub use calendar::{payment_schedule_complete, payments_calendar_load};

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
    let amount = normalized.parse::<Decimal>().map_err(|_| {
        anyhow!("Невірна сума платежу")
    })?;
    if amount <= Decimal::ZERO {
        return Err(anyhow!(
            "Сума платежу має бути більшою за нуль"
        ));
    }
    Ok(amount)
}

fn parse_payment_direction(value: &str) -> Result<PaymentDirection> {
    match value.trim() {
        "income" => Ok(PaymentDirection::Income),
        "expense" => Ok(PaymentDirection::Expense),
        other => Err(anyhow!(
            "Невідомий напрям платежу: {other}"
        )),
    }
}

fn parse_optional_counterparty_id(value: &str) -> Result<Option<Uuid>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Uuid::parse_str(trimmed).map(Some).map_err(|_| {
        anyhow!("Невалідний ідентифікатор контрагента")
    })
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
    Uuid::parse_str(value.trim())
        .map_err(|_| anyhow!("Невалідний ідентифікатор платежу"))
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
                .ok_or_else(|| {
                    anyhow!("Документ не знайдено в межах компанії")
                })?;
            Ok(format!("Акт {}", act.number))
        }
        "invoice" => {
            let (invoice, _) = db::invoices::get_by_id(ctx.pool(), document_id)
                .await?
                .ok_or_else(|| {
                    anyhow!("Документ не знайдено в межах компанії")
                })?;
            Ok(format!("Накладна {}", invoice.number))
        }
        other => Err(anyhow!(
            "Невідомий тип документу: {other}"
        )),
    }
}

async fn build_match_input(
    ctx: &AppCtx,
    payment_id: Uuid,
) -> Result<(crate::models::payment::Payment, PaymentMatchInput)> {
    let payment = db::payments::get_by_id_scoped(ctx.pool(), ctx.company_id(), payment_id)
        .await?
        .ok_or_else(|| {
            anyhow!("Платіж не знайдено в межах компанії")
        })?;

    let counterparty_iban = match payment.counterparty_id {
        Some(counterparty_id) => {
            db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id)
                .await?
                .and_then(|counterparty| counterparty.iban)
        }
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
) -> Result<(
    crate::models::payment::Payment,
    Vec<ScoredMatchCandidate>,
    MatchDecision,
)> {
    let (payment, input) = build_match_input(ctx, payment_id).await?;
    let candidates = db::payments::list_open_document_candidates(
        ctx.pool(),
        ctx.company_id(),
        payment.direction.clone(),
    )
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
        MatchDecision::Exact(_) | MatchDecision::Ambiguous(_) | MatchDecision::None => {
            scored_candidates
        }
    }
}

fn exact_recommendation(decision: &MatchDecision) -> Option<PaymentAutoMatchDto> {
    match decision {
        MatchDecision::Exact(candidate) => Some(PaymentAutoMatchDto {
            document_id: candidate.candidate.document_id.to_string(),
            document_kind: match_document_kind_to_str(candidate.candidate.document_kind)
                .to_string(),
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
    // Підтримуємо і CSV, і XLSX через `newest_statement_path`.
    let path = newest_statement_path().await?;
    let report = import_payments_from_statement(ctx.pool(), ctx.company_id(), &path, false).await?;
    Ok((report.created, path))
}

fn payment_action_to_str(action: &PaymentImportAction) -> &'static str {
    match action {
        PaymentImportAction::Create => "create",
        PaymentImportAction::Skip => "skip",
    }
}

/// Детермінований FNV-1a 64-bit хеш у hex.
/// Використовується для перевірки що файл виписки не змінився між preview і commit.
/// Не криптографічний — мета лише виявити модифікацію вмісту.
fn fnv1a_64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn build_import_preview_dto(
    path: &Path,
    bank_name: &str,
    report: PaymentImportReport,
    file_size: u64,
    file_mtime_secs: i64,
    file_hash: String,
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
        format!("Знайдено {parsed} рядків. Буде створено {will_create}, пропущено {will_skip}",)
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
        file_size,
        file_mtime_secs,
        file_hash,
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

// Public API

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
            incoming_sub: "поточний місяць"
                .to_string(),
            outgoing_sub: "поточний місяць"
                .to_string(),
            unmatched_count: kpi.unmatched_count as i32,
        },
    })
}

pub async fn payments_import_latest_csv(ctx: &AppCtx) -> Result<MutationResultDto> {
    let (imported, path) = run_bank_import(ctx).await?;
    Ok(MutationResultDto {
        ok: true,
        message: format!(
            "Імпортовано {imported} рядків з {}",
            path.display()
        ),
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

    let meta = fs::metadata(&path_buf).await?;
    let file_size = meta.len();
    let file_mtime_secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let bytes = fs::read(&path_buf).await?;
    let file_hash = fnv1a_64_hex(&bytes);
    drop(bytes);

    let parsed_rows = parse_payments_statement_file(&path_buf).await?;
    let bank_name = parsed_rows
        .first()
        .map(|row| row.bank_name.clone())
        .unwrap_or_else(|| {
            crate::import::bas_payments::parser_for_path(&path_buf)
                .bank_name()
                .to_string()
        });

    let report = apply_imported_payments(ctx.pool(), ctx.company_id(), &parsed_rows, true).await?;

    Ok(build_import_preview_dto(
        &path_buf,
        &bank_name,
        report,
        file_size,
        file_mtime_secs,
        file_hash,
    ))
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
    let meta = fs::metadata(&path).await.map_err(|_| {
        anyhow!(
            "Файл {} не знайдено або до нього немає доступу",
            path.display()
        )
    })?;

    let current_size = meta.len();
    let current_mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    if current_size != request.file_size || current_mtime != request.file_mtime_secs {
        return Err(anyhow!(
            "Файл виписки змінився після попереднього перегляду. Виберіть файл знову."
        ));
    }

    let bytes = fs::read(&path).await?;
    let current_hash = fnv1a_64_hex(&bytes);
    drop(bytes);

    if current_hash != request.file_hash {
        return Err(anyhow!(
            "Вміст файлу виписки змінився після попереднього перегляду. Виберіть файл знову."
        ));
    }

    let report = import_payments_from_statement(ctx.pool(), ctx.company_id(), &path, false).await?;

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
            "Р С›Р В±РЎР‚Р С•Р В±Р В»Р ВµР Р…Р С• РЎвЂћР В°Р в„–Р В» {}. Р СњР С•Р Р†Р С‘РЎвЂ¦ Р С—Р В»Р В°РЎвЂљР ВµР В¶РЎвЂ“Р Р†: {imported}",
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
        message: "Р РЃР В°Р В±Р В»Р С•Р Р… CSV Р Р†РЎвЂ“Р Т‘Р С”РЎР‚Р С‘РЎвЂљР С•".to_string(),
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
    let doc_id = Uuid::parse_str(&req.document_id).map_err(|_| {
        anyhow!("Невалідний ідентифікатор документу")
    })?;
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
            Uuid::parse_str(&allocation.document_id).map_err(|_| {
                anyhow!("Невалідний ідентифікатор документу")
            })?,
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
    let doc_id = Uuid::parse_str(&req.document_id).map_err(|_| {
        anyhow!("Невалідний ідентифікатор документу")
    })?;

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
        message: "Звірення платежу скасовано".to_string(),
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
        message: "Звірення платежу скасовано".to_string(),
    })
}

// РІвЂќР‚РІвЂќР‚РІвЂќР‚ Tests РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚РІвЂќР‚

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
            let allocation = db::payments::PaymentReconcileAllocation {
                document_kind: match_document_kind_to_str(candidate.candidate.document_kind)
                    .to_string(),
                document_id: candidate.candidate.document_id,
                amount: payment.amount,
            };

            db::payments::reconcile_split_scoped(
                ctx.pool(),
                ctx.company_id(),
                payment.id,
                &[allocation],
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
    let candidates = db::payments::list_open_document_candidates(
        ctx.pool(),
        ctx.company_id(),
        payment.direction,
    )
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
                    text_hits: score
                        .map(|value| value.text_hits as i32)
                        .unwrap_or_default(),
                    days_distance: score.map(|value| value.days_distance).unwrap_or(365),
                }
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests;
