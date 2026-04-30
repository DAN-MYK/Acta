use anyhow::{anyhow, bail, Result};
use chrono::NaiveDate;
use csv::StringRecord;
use rust_decimal::Decimal;

use crate::models::payment::PaymentDirection;

#[derive(Debug, Clone)]
pub struct ParsedBankRow {
    pub date: NaiveDate,
    pub amount: Decimal,
    pub direction: PaymentDirection,
    pub description: String,
    pub bank_ref: Option<String>,
    pub bank_name: String,
}

pub trait BankStatementParser {
    fn bank_name(&self) -> &'static str;
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>>;
}

#[derive(Debug, Clone, Default)]
struct HeaderLayout {
    date_idx: Option<usize>,
    amount_idx: Option<usize>,
    description_idx: Option<usize>,
    direction_idx: Option<usize>,
    reference_idx: Option<usize>,
    debit_idx: Option<usize>,
    credit_idx: Option<usize>,
}

fn parse_decimal(raw: &str) -> Result<Decimal> {
    let trimmed = raw.trim().trim_matches('"').replace('\u{00a0}', " ");
    if trimmed.is_empty() {
        bail!("Порожнє числове поле");
    }

    let mut normalized = trimmed.replace(' ', "").replace(',', ".");
    let negative = normalized.starts_with('-')
        || normalized.ends_with('-')
        || (normalized.starts_with('(') && normalized.ends_with(')'));
    normalized = normalized
        .trim_start_matches('-')
        .trim_end_matches('-')
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim_start_matches('+')
        .to_string();

    let mut value = normalized.parse::<Decimal>()?;
    if negative {
        value = -value;
    }
    Ok(value)
}

fn parse_date(raw: &str) -> Result<NaiveDate> {
    let trimmed = raw.trim().trim_matches('"');
    let date_only = trimmed.split(['T', ' ']).next().unwrap_or(trimmed).trim();

    for format in [
        "%d.%m.%Y", "%Y-%m-%d", "%d/%m/%Y", "%Y/%m/%d", "%Y%m%d", "%d-%m-%Y",
    ] {
        if let Ok(date) = NaiveDate::parse_from_str(date_only, format) {
            return Ok(date);
        }
    }

    Err(anyhow!("Не вдалося розібрати дату '{raw}'"))
}

fn preprocess_csv_text(csv_text: &str) -> &str {
    csv_text.trim_start_matches('\u{feff}')
}

fn detect_delimiter(csv_text: &str) -> u8 {
    let first_line = preprocess_csv_text(csv_text)
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();

    let candidates = [(b';', ';'), (b',', ','), (b'\t', '\t')];
    candidates
        .into_iter()
        .max_by_key(|(_, ch)| first_line.matches(*ch).count())
        .map(|(byte, _)| byte)
        .unwrap_or(b',')
}

fn normalize_header(raw: &str) -> String {
    raw.trim()
        .trim_matches('\u{feff}')
        .trim_matches('"')
        .to_lowercase()
        .chars()
        .filter(|ch| !matches!(ch, ' ' | '_' | '-' | '.' | ':' | '/' | '\\' | '(' | ')'))
        .collect()
}

fn field_at(record: &StringRecord, idx: Option<usize>) -> Option<&str> {
    idx.and_then(|value| record.get(value))
}

fn text_or_empty(record: &StringRecord, idx: Option<usize>) -> String {
    field_at(record, idx)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_default()
}

fn optional_text(record: &StringRecord, idx: Option<usize>) -> Option<String> {
    field_at(record, idx)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn find_header_index(headers: &StringRecord, aliases: &[&str]) -> Option<usize> {
    headers.iter().position(|value| {
        let normalized = normalize_header(value);
        aliases.iter().any(|alias| normalized == *alias)
    })
}

fn header_layout(headers: &StringRecord) -> HeaderLayout {
    HeaderLayout {
        date_idx: find_header_index(
            headers,
            &[
                "date",
                "operationdate",
                "documentdate",
                "valuedate",
                "posteddate",
                "дата",
                "датаоперації",
                "документдата",
                "датавалютування",
            ],
        ),
        amount_idx: find_header_index(headers, &["amount", "sum", "total", "сума"]),
        description_idx: find_header_index(
            headers,
            &[
                "description",
                "purpose",
                "details",
                "comment",
                "назначениеплатежа",
                "призначенняплатежу",
                "опис",
                "коментар",
                "деталі",
            ],
        ),
        direction_idx: find_header_index(
            headers,
            &[
                "direction",
                "type",
                "operationtype",
                "напрям",
                "тип",
                "типоперації",
            ],
        ),
        reference_idx: find_header_index(
            headers,
            &[
                "reference",
                "ref",
                "bankref",
                "docno",
                "documentno",
                "operationid",
                "референс",
                "номердокумента",
                "кодоперації",
            ],
        ),
        debit_idx: find_header_index(headers, &["debit", "debet", "видаток", "списання"]),
        credit_idx: find_header_index(headers, &["credit", "kredit", "надходження", "зарахування"]),
    }
}

fn parse_direction_text(raw: &str) -> Option<PaymentDirection> {
    let normalized = raw.trim().trim_matches('"').to_lowercase();
    match normalized.as_str() {
        "income" | "in" | "credit" | "надходження" | "зарахування" | "прихід" => {
            Some(PaymentDirection::Income)
        }
        "expense" | "out" | "debit" | "витрата" | "списання" | "видаток" => {
            Some(PaymentDirection::Expense)
        }
        _ => None,
    }
}

fn amount_and_direction(
    record: &StringRecord,
    layout: &HeaderLayout,
) -> Result<(Decimal, PaymentDirection)> {
    if let Some(direction_raw) = field_at(record, layout.direction_idx).map(str::trim) {
        if !direction_raw.is_empty() {
            let direction = parse_direction_text(direction_raw)
                .ok_or_else(|| anyhow!("Невідомий напрямок платежу: {direction_raw}"))?;
            let amount = parse_decimal(field_at(record, layout.amount_idx).unwrap_or(""))?;
            return Ok((amount.abs(), direction));
        }
    }

    if let Some(amount_raw) = field_at(record, layout.amount_idx).map(str::trim) {
        if !amount_raw.is_empty() {
            let amount = parse_decimal(amount_raw)?;
            if amount.is_sign_negative() {
                return Ok((amount.abs(), PaymentDirection::Expense));
            }
            return Ok((amount, PaymentDirection::Income));
        }
    }

    let debit = optional_text(record, layout.debit_idx)
        .map(|value| parse_decimal(&value))
        .transpose()?;
    let credit = optional_text(record, layout.credit_idx)
        .map(|value| parse_decimal(&value))
        .transpose()?;

    match (debit, credit) {
        (Some(debit), None) if !debit.is_zero() => Ok((debit.abs(), PaymentDirection::Expense)),
        (None, Some(credit)) if !credit.is_zero() => Ok((credit.abs(), PaymentDirection::Income)),
        (Some(debit), Some(credit)) if credit.is_zero() && !debit.is_zero() => {
            Ok((debit.abs(), PaymentDirection::Expense))
        }
        (Some(debit), Some(credit)) if debit.is_zero() && !credit.is_zero() => {
            Ok((credit.abs(), PaymentDirection::Income))
        }
        _ => bail!("Не вдалося визначити суму або напрямок платежу"),
    }
}

fn parse_generic_csv(bank_name: &str, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
    let delimiter = detect_delimiter(csv_text);
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(true)
        .from_reader(preprocess_csv_text(csv_text).as_bytes());

    let headers = reader.headers()?.clone();
    let layout = header_layout(&headers);
    let date_idx = layout
        .date_idx
        .ok_or_else(|| anyhow!("У банківському CSV не знайдено стовпець дати"))?;

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        if record.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        rows.push(parse_record(bank_name, &record, date_idx, &layout)?);
    }

    Ok(rows)
}

fn parse_record(
    bank_name: &str,
    record: &StringRecord,
    date_idx: usize,
    layout: &HeaderLayout,
) -> Result<ParsedBankRow> {
    let (amount, direction) = amount_and_direction(record, layout)?;

    Ok(ParsedBankRow {
        date: parse_date(record.get(date_idx).unwrap_or(""))?,
        amount,
        direction,
        description: text_or_empty(record, layout.description_idx),
        bank_ref: optional_text(record, layout.reference_idx),
        bank_name: bank_name.to_string(),
    })
}

pub struct UkrgasbankCsvParser;
pub struct OschadbankCsvParser;
pub struct SenseBankCsvParser;

impl BankStatementParser for UkrgasbankCsvParser {
    fn bank_name(&self) -> &'static str {
        "Укргазбанк"
    }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
        parse_generic_csv(self.bank_name(), csv_text)
    }
}

impl BankStatementParser for OschadbankCsvParser {
    fn bank_name(&self) -> &'static str {
        "Ощадбанк"
    }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
        parse_generic_csv(self.bank_name(), csv_text)
    }
}

impl BankStatementParser for SenseBankCsvParser {
    fn bank_name(&self) -> &'static str {
        "Sense Bank"
    }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
        parse_generic_csv(self.bank_name(), csv_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn parse_decimal_standard_dot() {
        assert_eq!(parse_decimal("1200.50").unwrap(), dec!(1200.50));
    }

    #[test]
    fn parse_decimal_european_space_comma() {
        assert_eq!(parse_decimal("1 200,50").unwrap(), dec!(1200.50));
    }

    #[test]
    fn parse_decimal_space_thousands_dot() {
        assert_eq!(parse_decimal("1 200.50").unwrap(), dec!(1200.50));
    }

    #[test]
    fn parse_decimal_parentheses_negative() {
        assert_eq!(parse_decimal("(100.00)").unwrap(), dec!(-100.00));
    }

    #[test]
    fn parse_decimal_trims_whitespace() {
        assert_eq!(parse_decimal("  500.00  ").unwrap(), dec!(500.00));
    }

    #[test]
    fn parse_decimal_invalid_returns_err() {
        assert!(parse_decimal("abc").is_err());
    }

    #[test]
    fn parse_date_dd_mm_yyyy() {
        let d = parse_date("03.04.2026").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 3).unwrap());
    }

    #[test]
    fn parse_date_iso_yyyy_mm_dd() {
        let d = parse_date("2026-04-15").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn parse_date_slash_format_supported() {
        let d = parse_date("03/04/2026").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 3).unwrap());
    }

    #[test]
    fn parse_date_datetime_supported() {
        let d = parse_date("2026-04-15T14:20:00").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn ukrgasbank_bank_name() {
        assert_eq!(UkrgasbankCsvParser.bank_name(), "Укргазбанк");
    }

    #[test]
    fn ukrgasbank_parses_full_row() {
        let csv = "date,amount,description,direction,reference\n\
                   15.04.2026,5000.00,Оплата послуг,income,REF-001\n";
        let rows = UkrgasbankCsvParser.parse(csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
        assert_eq!(rows[0].amount, dec!(5000.00));
        assert_eq!(rows[0].description, "Оплата послуг");
        assert_eq!(rows[0].direction, PaymentDirection::Income);
        assert_eq!(rows[0].bank_ref.as_deref(), Some("REF-001"));
        assert_eq!(rows[0].bank_name, "Укргазбанк");
    }

    #[test]
    fn generic_parser_supports_semicolon_and_ukrainian_headers() {
        let csv = "Дата операції;Сума;Призначення платежу;Напрям;Номер документа\n\
                   15.04.2026;5000,00;Оплата послуг;надходження;REF-001\n";
        let rows = UkrgasbankCsvParser.parse(csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount, dec!(5000.00));
        assert_eq!(rows[0].direction, PaymentDirection::Income);
        assert_eq!(rows[0].bank_ref.as_deref(), Some("REF-001"));
    }

    #[test]
    fn generic_parser_supports_debit_credit_columns() {
        let csv = "date,debit,credit,description,operation_id\n\
                   2026-04-15,0,1500.50,Надходження,REF-100\n\
                   2026-04-16,200.00,0,Списання,REF-101\n";
        let rows = SenseBankCsvParser.parse(csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].amount, dec!(1500.50));
        assert_eq!(rows[0].direction, PaymentDirection::Income);
        assert_eq!(rows[1].amount, dec!(200.00));
        assert_eq!(rows[1].direction, PaymentDirection::Expense);
    }

    #[test]
    fn generic_parser_inferrs_direction_from_signed_amount() {
        let csv = "date,amount,description,reference\n\
                   2026-04-15,-100.00,Комісія,REF-200\n";
        let rows = OschadbankCsvParser.parse(csv).unwrap();
        assert_eq!(rows[0].amount, dec!(100.00));
        assert_eq!(rows[0].direction, PaymentDirection::Expense);
    }

    #[test]
    fn generic_parser_strips_utf8_bom() {
        let csv = "\u{feff}date,amount,description,direction\n\
                   01.04.2026,1000.00,Тест,income\n";
        let rows = SenseBankCsvParser.parse(csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount, dec!(1000.00));
    }

    #[test]
    fn generic_parser_unknown_direction_returns_err() {
        let csv = "date,amount,description,direction\n\
                   01.04.2026,1000.00,Тест,unknown\n";
        assert!(UkrgasbankCsvParser.parse(csv).is_err());
    }

    #[test]
    fn generic_parser_case_insensitive_headers() {
        let csv = "Date,Amount,Description,Direction\n\
                   01.04.2026,800.00,Тест,Income\n";
        let rows = OschadbankCsvParser.parse(csv).unwrap();
        assert_eq!(rows[0].amount, dec!(800.00));
        assert_eq!(rows[0].direction, PaymentDirection::Income);
    }
}
