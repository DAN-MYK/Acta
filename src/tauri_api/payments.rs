use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use crate::app_ctx::AppCtx;
use crate::db;
use crate::import::bank_csv::{
    BankStatementParser, OschadbankCsvParser, ParsedBankRow, SenseBankCsvParser,
    UkrgasbankCsvParser,
};
use crate::models::payment::{NewPayment, PaymentDirection, UpdatePayment};

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationResultDto {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTemplateResultDto {
    pub ok: bool,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyItemDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentItemDto {
    pub id: String,
    pub date: String,
    pub counterparty: String,
    pub amount_str: String,
    pub direction: String,
    pub matched_doc: String,
    pub account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsScreenDto {
    pub items: Vec<PaymentItemDto>,
    pub counterparties: Vec<CounterpartyItemDto>,
    pub kpi: PaymentsKpiDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ─── Private helpers ──────────────────────────────────────────────────────────

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
    let normalized = value.trim().replace(' ', "").replace(',', ".");
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

fn bank_import_dir() -> PathBuf {
    PathBuf::from("storage/import/bank")
}

async fn newest_csv_path() -> Result<PathBuf> {
    let dir = bank_import_dir();
    fs::create_dir_all(&dir).await?;
    let mut entries = fs::read_dir(&dir).await?;
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !ext.eq_ignore_ascii_case("csv") {
            continue;
        }
        let modified = entry
            .metadata()
            .await
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        match &newest {
            Some((cur, _)) if modified <= *cur => {}
            _ => newest = Some((modified, path)),
        }
    }
    newest
        .map(|(_, p)| p)
        .ok_or_else(|| anyhow!("У `storage/import/bank/` не знайдено CSV для імпорту"))
}

fn parser_candidates(path: &Path) -> Vec<Box<dyn BankStatementParser>> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if name.contains("oschad") || name.contains("ощад") {
        return vec![Box::new(OschadbankCsvParser)];
    }
    if name.contains("sense") {
        return vec![Box::new(SenseBankCsvParser)];
    }
    if name.contains("ukrgas") || name.contains("укргаз") {
        return vec![Box::new(UkrgasbankCsvParser)];
    }
    vec![
        Box::new(UkrgasbankCsvParser),
        Box::new(OschadbankCsvParser),
        Box::new(SenseBankCsvParser),
    ]
}

fn parse_bank_rows(path: &Path, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
    let mut last_error: Option<anyhow::Error> = None;
    for parser in parser_candidates(path) {
        match parser.parse(csv_text) {
            Ok(rows) if !rows.is_empty() => return Ok(rows),
            Ok(_) => last_error = Some(anyhow!("CSV не містить жодного рядка")),
            Err(e) => last_error = Some(e),
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("Не вдалося розпізнати формат банківського CSV")))
}

async fn import_bank_rows(
    pool: &sqlx::PgPool,
    company_id: Uuid,
    rows: Vec<ParsedBankRow>,
) -> Result<usize> {
    let mut imported = 0usize;
    for row in rows {
        let exists = db::payments::exists_imported_row(
            pool,
            company_id,
            row.date,
            row.amount,
            row.direction.clone(),
            row.bank_ref.as_deref(),
            &row.description,
        )
        .await?;
        if exists {
            continue;
        }
        db::payments::create(
            pool,
            NewPayment {
                company_id,
                date: row.date,
                amount: row.amount,
                direction: row.direction,
                counterparty_id: None,
                bank_name: Some(row.bank_name),
                bank_ref: row.bank_ref,
                description: Some(row.description),
            },
        )
        .await?;
        imported += 1;
    }
    Ok(imported)
}

async fn run_bank_import(ctx: &AppCtx) -> Result<(usize, PathBuf)> {
    let csv_path = newest_csv_path().await?;
    let csv_text = fs::read_to_string(&csv_path)
        .await
        .with_context(|| format!("Не вдалося прочитати {}", csv_path.display()))?;
    let rows = parse_bank_rows(&csv_path, &csv_text)?;
    let imported = import_bank_rows(ctx.pool(), ctx.company_id(), rows).await?;
    Ok((imported, csv_path))
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
            counterparty: r.counterparty_name.as_deref().unwrap_or("").to_string(),
            amount_str: format_money_ua(r.amount),
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
            incoming_str: format_money_ua(kpi.incoming_month),
            outgoing_str: format_money_ua(kpi.outgoing_month),
            net_str: format_money_ua(net),
            unmatched_str: kpi.unmatched_count.to_string(),
            incoming_sub: "поточний місяць".to_string(),
            outgoing_sub: "поточний місяць".to_string(),
            unmatched_count: kpi.unmatched_count as i32,
        },
    })
}

pub async fn payments_import_latest_csv(ctx: &AppCtx) -> Result<MutationResultDto> {
    let (imported, path) = run_bank_import(ctx).await?;
    Ok(MutationResultDto {
        ok: true,
        message: format!("Імпортовано {imported} рядків з {}", path.display()),
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
    let _ = tokio::task::spawn_blocking(move || open::that(open_path)).await;
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

pub async fn payment_reconcile(ctx: &AppCtx, payment_id: String) -> Result<MutationResultDto> {
    let id =
        Uuid::parse_str(&payment_id).map_err(|_| anyhow!("Невалідний ідентифікатор платежу"))?;
    let changed = db::payments::mark_reconciled_scoped(ctx.pool(), ctx.company_id(), id).await?;
    if !changed {
        return Err(anyhow!(
            "Платіж не знайдено або він не належить поточній компанії"
        ));
    }
    Ok(MutationResultDto {
        ok: true,
        message: "Платіж позначено як звірений".to_string(),
    })
}

pub async fn payment_unreconcile(ctx: &AppCtx, payment_id: String) -> Result<MutationResultDto> {
    let id =
        Uuid::parse_str(&payment_id).map_err(|_| anyhow!("Невалідний ідентифікатор платежу"))?;
    let changed = db::payments::mark_unreconciled_scoped(ctx.pool(), ctx.company_id(), id).await?;
    if !changed {
        return Err(anyhow!(
            "Платіж не знайдено або він не належить поточній компанії"
        ));
    }
    Ok(MutationResultDto {
        ok: true,
        message: "Звірку платежу скасовано".to_string(),
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

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
    fn format_money_ua_basic() {
        assert_eq!(format_money_ua(dec!(1234.56)), "1\u{00a0}234,56");
    }

    #[test]
    fn format_money_ua_small() {
        assert_eq!(format_money_ua(dec!(5.00)), "5,00");
    }

    #[test]
    fn format_money_ua_negative() {
        assert_eq!(format_money_ua(dec!(-1234.56)), "-1\u{00a0}234,56");
    }

    #[test]
    fn format_money_ua_zero() {
        assert_eq!(format_money_ua(dec!(0)), "0,00");
    }

    #[test]
    fn compile_check_public_function_signatures() {
        let _ = payments_list;
        let _ = payments_import_latest_csv;
        let _ = payments_sync_bank;
        let _ = payments_open_manual_template;
        let _ = payment_create_or_update;
        let _ = payment_reconcile;
        let _ = payment_unreconcile;
    }
}
