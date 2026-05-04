use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::import::bank_common::ParsedBankRow;
use crate::import::bank_csv::{
    BankStatementParser, MonobankCsvParser, OschadbankCsvParser, OtpBankCsvParser,
    PrivatBankCsvParser, PumbCsvParser, RaiffeisenCsvParser, SenseBankCsvParser,
    UkrgasbankCsvParser,
};
use crate::import::bank_xlsx::{is_xlsx_path, parse_xlsx_file};
use crate::models::payment::NewPayment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentImportAction {
    Create,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentImportPlanRow {
    pub bank_ref: Option<String>,
    pub description: String,
    pub action: PaymentImportAction,
    pub note: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PaymentImportReport {
    pub parsed: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub rows: Vec<PaymentImportPlanRow>,
}

pub fn bank_import_dir() -> PathBuf {
    PathBuf::from("storage/import/bank")
}

/// Шукає найновішу банківську виписку (CSV/XLSX/XLS) у `storage/import/bank/`.
///
/// Backward compatibility: поверне `.csv` якщо немає XLSX (старий flow).
pub async fn newest_statement_path() -> Result<PathBuf> {
    let dir = bank_import_dir();
    tokio::fs::create_dir_all(&dir).await?;
    let mut entries = tokio::fs::read_dir(&dir).await?;
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let Some(ext) = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_lowercase)
        else {
            continue;
        };
        if !matches!(ext.as_str(), "csv" | "xlsx" | "xls") {
            continue;
        }

        let modified = entry
            .metadata()
            .await
            .and_then(|metadata| metadata.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        match &newest {
            Some((current, _)) if modified <= *current => {}
            _ => newest = Some((modified, path)),
        }
    }

    newest.map(|(_, path)| path).ok_or_else(|| {
        anyhow!("У `storage/import/bank/` не знайдено CSV або XLSX для імпорту")
    })
}

/// Аліас для зворотної сумісності зі старим API.
#[deprecated(note = "Використовуйте `newest_statement_path()` — він підтримує і XLSX")]
pub async fn newest_payments_csv_path() -> Result<PathBuf> {
    newest_statement_path().await
}

pub async fn parse_payments_csv_file(path: &Path) -> Result<Vec<ParsedBankRow>> {
    let csv_text = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Не вдалося прочитати CSV файл {}", path.display()))?;
    parse_payments_csv_text(path, &csv_text)
}

pub fn parse_payments_csv_text(path: &Path, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
    let mut last_error = None;

    for parser in parser_candidates(path) {
        match parser.parse(csv_text) {
            Ok(rows) if !rows.is_empty() => return Ok(rows),
            Ok(_) => {
                last_error = Some(anyhow!("CSV не містить жодного рядка після парсингу"));
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Не вдалося розпізнати формат банківського CSV")))
}

/// Універсальний парсер виписки за path: CSV або XLSX/XLS.
///
/// Виявляє формат за extension і викликає відповідний бекенд-парсер.
pub async fn parse_payments_statement_file(path: &Path) -> Result<Vec<ParsedBankRow>> {
    if is_xlsx_path(path) {
        let bank_name = parser_for_path(path).bank_name();
        return parse_xlsx_file(bank_name, path).await;
    }

    parse_payments_csv_file(path).await
}

pub async fn import_payments_from_csv(
    pool: &PgPool,
    company_id: Uuid,
    path: &Path,
    dry_run: bool,
) -> Result<PaymentImportReport> {
    let rows = parse_payments_statement_file(path).await?;
    apply_imported_payments(pool, company_id, &rows, dry_run).await
}

/// Універсальний імпорт виписки (CSV або XLSX) з повним dry-run підтримкою.
pub async fn import_payments_from_statement(
    pool: &PgPool,
    company_id: Uuid,
    path: &Path,
    dry_run: bool,
) -> Result<PaymentImportReport> {
    let rows = parse_payments_statement_file(path).await?;
    apply_imported_payments(pool, company_id, &rows, dry_run).await
}

pub async fn apply_imported_payments(
    pool: &PgPool,
    company_id: Uuid,
    rows: &[ParsedBankRow],
    dry_run: bool,
) -> Result<PaymentImportReport> {
    let mut report = PaymentImportReport {
        parsed: rows.len(),
        ..PaymentImportReport::default()
    };

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
            report.skipped += 1;
            report.rows.push(PaymentImportPlanRow {
                bank_ref: row.bank_ref.clone(),
                description: row.description.clone(),
                action: PaymentImportAction::Skip,
                note: Some(build_duplicate_note(row)),
            });
            continue;
        }

        report.created += 1;
        report.rows.push(PaymentImportPlanRow {
            bank_ref: row.bank_ref.clone(),
            description: row.description.clone(),
            action: PaymentImportAction::Create,
            note: Some(build_create_note(row)),
        });

        if !dry_run {
            let payload = NewPayment {
                company_id,
                date: row.date,
                amount: row.amount,
                direction: row.direction.clone(),
                counterparty_id: None,
                bank_name: Some(row.bank_name.clone()),
                bank_ref: row.bank_ref.clone(),
                description: Some(row.description.clone()),
            };
            let _ = db::payments::create(pool, payload).await?;
        }
    }

    Ok(report)
}

fn build_duplicate_note(row: &ParsedBankRow) -> String {
    if row.bank_ref.is_some() {
        "skip: знайдено existing row за bank_ref".to_string()
    } else {
        "skip: знайдено existing row за exact description fallback".to_string()
    }
}

fn build_create_note(row: &ParsedBankRow) -> String {
    if row.bank_ref.is_some() {
        "create: bank_ref відсутній у БД".to_string()
    } else {
        "create: exact description fallback не знайшов дубліката".to_string()
    }
}

/// Підбирає bank-specific парсер за filename.
///
/// Розширений список: Ощадбанк, Sense, ПриватБанк, Monobank, Райффайзен,
/// OTP Bank, ПУМБ, Укргазбанк (default).
pub(crate) fn parser_for_path(path: &Path) -> Box<dyn BankStatementParser> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .unwrap_or_default();

    if contains_any(&name, &["ощад", "oschad"]) {
        Box::new(OschadbankCsvParser)
    } else if contains_any(&name, &["приват", "privat", "p24"]) {
        Box::new(PrivatBankCsvParser)
    } else if contains_any(&name, &["моно", "mono"]) {
        Box::new(MonobankCsvParser)
    } else if contains_any(&name, &["sense", "альфа", "alpha"]) {
        Box::new(SenseBankCsvParser)
    } else if contains_any(&name, &["райф", "raiff"]) {
        Box::new(RaiffeisenCsvParser)
    } else if contains_any(&name, &["otp"]) {
        Box::new(OtpBankCsvParser)
    } else if contains_any(&name, &["пумб", "pumb"]) {
        Box::new(PumbCsvParser)
    } else {
        Box::new(UkrgasbankCsvParser)
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn parser_candidates(path: &Path) -> Vec<Box<dyn BankStatementParser>> {
    let primary = parser_for_path(path);
    let primary_name = primary.bank_name();
    let mut parsers: Vec<Box<dyn BankStatementParser>> = vec![primary];

    let pool: Vec<Box<dyn BankStatementParser>> = vec![
        Box::new(OschadbankCsvParser),
        Box::new(SenseBankCsvParser),
        Box::new(PrivatBankCsvParser),
        Box::new(MonobankCsvParser),
        Box::new(RaiffeisenCsvParser),
        Box::new(OtpBankCsvParser),
        Box::new(PumbCsvParser),
        Box::new(UkrgasbankCsvParser),
    ];

    for parser in pool {
        if parser.bank_name() != primary_name {
            parsers.push(parser);
        }
    }

    parsers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::payment::PaymentDirection;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn parse_payments_csv_text_chooses_parser_by_filename() {
        let csv = "date,amount,description,direction\n\
                   01.04.2026,1000.00,Тест,income\n";
        let rows = parse_payments_csv_text(Path::new("sense_statement.csv"), csv)
            .expect("CSV має парситися");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bank_name, "Sense Bank");
    }

    #[test]
    fn parse_payments_csv_text_falls_back_to_other_bank_parser() {
        let csv = "date,amount,description,direction\n\
                   01.04.2026,1000.00,Тест,income\n";
        let rows = parse_payments_csv_text(Path::new("mystery_export.csv"), csv)
            .expect("CSV має парситися fallback-парсером");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bank_name, "Укргазбанк");
    }

    #[test]
    fn parser_for_path_routes_privat() {
        let parser = parser_for_path(Path::new("privat_p24_2026-04.csv"));
        assert_eq!(parser.bank_name(), "ПриватБанк");
    }

    #[test]
    fn parser_for_path_routes_mono() {
        let parser = parser_for_path(Path::new("mono-export.xlsx"));
        assert_eq!(parser.bank_name(), "Monobank");
    }

    #[test]
    fn parser_for_path_routes_raiffeisen() {
        let parser = parser_for_path(Path::new("Raiffeisen_2026.csv"));
        assert_eq!(parser.bank_name(), "Райффайзен Банк");
    }

    #[test]
    fn parser_for_path_routes_otp() {
        let parser = parser_for_path(Path::new("otp_business_2026.xlsx"));
        assert_eq!(parser.bank_name(), "OTP Bank");
    }

    #[test]
    fn parser_for_path_routes_pumb() {
        let parser = parser_for_path(Path::new("pumb_2026.csv"));
        assert_eq!(parser.bank_name(), "ПУМБ");
    }

    #[test]
    fn parser_for_path_falls_back_to_ukrgasbank() {
        let parser = parser_for_path(Path::new("unknown.csv"));
        assert_eq!(parser.bank_name(), "Укргазбанк");
    }

    #[test]
    fn parser_for_path_recognises_uk_privat() {
        let parser = parser_for_path(Path::new("приватбанк-виписка-травень.csv"));
        assert_eq!(parser.bank_name(), "ПриватБанк");
    }

    #[test]
    fn apply_imported_payments_report_defaults_to_create_shape() {
        let rows = vec![ParsedBankRow {
            date: NaiveDate::from_ymd_opt(2026, 4, 1).expect("валідна дата"),
            amount: dec!(1000.00),
            direction: PaymentDirection::Income,
            description: "Оплата".to_string(),
            bank_ref: Some("REF-001".to_string()),
            bank_name: "Тест Банк".to_string(),
            counterparty_name: None,
            counterparty_iban: None,
            currency: None,
        }];

        let report = PaymentImportReport {
            parsed: rows.len(),
            created: 1,
            updated: 0,
            skipped: 0,
            conflicts: 0,
            rows: vec![PaymentImportPlanRow {
                bank_ref: Some("REF-001".to_string()),
                description: "Оплата".to_string(),
                action: PaymentImportAction::Create,
                note: Some("create: bank_ref відсутній у БД".to_string()),
            }],
        };

        assert_eq!(report.parsed, 1);
        assert_eq!(report.created, 1);
    }

    #[test]
    fn payment_notes_depend_on_matching_strategy() {
        let with_ref = ParsedBankRow {
            date: NaiveDate::from_ymd_opt(2026, 4, 1).expect("валідна дата"),
            amount: dec!(1000.00),
            direction: PaymentDirection::Income,
            description: "Оплата".to_string(),
            bank_ref: Some("REF-001".to_string()),
            bank_name: "Тест Банк".to_string(),
            counterparty_name: None,
            counterparty_iban: None,
            currency: None,
        };
        let no_ref = ParsedBankRow {
            bank_ref: None,
            ..with_ref.clone()
        };

        assert!(build_duplicate_note(&with_ref).contains("bank_ref"));
        assert!(build_duplicate_note(&no_ref).contains("description fallback"));
        assert!(build_create_note(&with_ref).contains("bank_ref"));
        assert!(build_create_note(&no_ref).contains("description fallback"));
    }
}
