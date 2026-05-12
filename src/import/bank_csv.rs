//! CSV-парсер банківських виписок. Використовує спільну інфраструктуру з
//! [`crate::import::bank_common`] — header-aliases, decimal/date normalizers,
//! `HeaderLayout`. Працює з UTF-8 текстом (BOM stripping вбудовано).

use anyhow::{anyhow, Result};
use csv::StringRecord;

use crate::import::bank_common::{
    amount_and_direction_from_strings, header_layout_from_strs, normalize_iban, parse_date,
    preprocess_csv_text, HeaderLayout, ParsedBankRow,
};

pub use crate::import::bank_common::ParsedBankRow as BankRow;

/// Trait, який дозволяє підставляти різні bank-specific парсери.
pub trait BankStatementParser {
    fn bank_name(&self) -> &'static str;
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>>;
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

fn header_layout(headers: &StringRecord) -> HeaderLayout {
    let raw: Vec<&str> = headers.iter().collect();
    header_layout_from_strs(&raw)
}

fn parse_record(
    bank_name: &str,
    record: &StringRecord,
    date_idx: usize,
    layout: &HeaderLayout,
) -> Result<ParsedBankRow> {
    let direction_field = field_at(record, layout.direction_idx);
    let amount_field = field_at(record, layout.amount_idx);
    let debit_field = field_at(record, layout.debit_idx);
    let credit_field = field_at(record, layout.credit_idx);

    let (amount, direction) = amount_and_direction_from_strings(
        direction_field,
        amount_field,
        debit_field,
        credit_field,
    )?;

    Ok(ParsedBankRow {
        date: parse_date(record.get(date_idx).unwrap_or(""))?,
        amount,
        direction,
        description: text_or_empty(record, layout.description_idx),
        bank_ref: optional_text(record, layout.reference_idx),
        bank_name: bank_name.to_string(),
        counterparty_name: optional_text(record, layout.counterparty_name_idx),
        counterparty_iban: field_at(record, layout.counterparty_iban_idx).and_then(normalize_iban),
        currency: optional_text(record, layout.currency_idx),
    })
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

/// Узагальнена функція парсингу CSV для будь-якого банку.
pub fn parse_csv(bank_name: &str, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
    parse_generic_csv(bank_name, csv_text)
}

pub struct UkrgasbankCsvParser;
pub struct OschadbankCsvParser;
pub struct SenseBankCsvParser;
pub struct PrivatBankCsvParser;
pub struct MonobankCsvParser;
pub struct RaiffeisenCsvParser;
pub struct OtpBankCsvParser;
pub struct PumbCsvParser;

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

impl BankStatementParser for PrivatBankCsvParser {
    fn bank_name(&self) -> &'static str {
        "ПриватБанк"
    }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
        parse_generic_csv(self.bank_name(), csv_text)
    }
}

impl BankStatementParser for MonobankCsvParser {
    fn bank_name(&self) -> &'static str {
        "Monobank"
    }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
        parse_generic_csv(self.bank_name(), csv_text)
    }
}

impl BankStatementParser for RaiffeisenCsvParser {
    fn bank_name(&self) -> &'static str {
        "Райффайзен Банк"
    }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
        parse_generic_csv(self.bank_name(), csv_text)
    }
}

impl BankStatementParser for OtpBankCsvParser {
    fn bank_name(&self) -> &'static str {
        "OTP Bank"
    }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
        parse_generic_csv(self.bank_name(), csv_text)
    }
}

impl BankStatementParser for PumbCsvParser {
    fn bank_name(&self) -> &'static str {
        "ПУМБ"
    }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
        parse_generic_csv(self.bank_name(), csv_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::payment::PaymentDirection;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

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
        assert_eq!(rows[0].counterparty_iban, None);
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

    #[test]
    fn privat_parser_returns_correct_bank_name() {
        let csv = "date,amount,description,direction\n\
                   01.04.2026,500.00,Тест,income\n";
        let rows = PrivatBankCsvParser.parse(csv).unwrap();
        assert_eq!(rows[0].bank_name, "ПриватБанк");
    }

    #[test]
    fn mono_parser_handles_extended_uk_headers() {
        let csv = "Дата проведення;Сума;Призначення платежу;Контрагент;Рахунок отримувача\n\
                   2026-04-15;1\u{00a0}200,50;Платіж постачальнику;ТОВ Ромашка;UA12 305299 0000026000123456\n";
        let rows = MonobankCsvParser.parse(csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount, dec!(1200.50));
        assert_eq!(rows[0].counterparty_name.as_deref(), Some("ТОВ Ромашка"));
        assert_eq!(
            rows[0].counterparty_iban.as_deref(),
            Some("UA123052990000026000123456")
        );
        assert_eq!(rows[0].bank_name, "Monobank");
    }
}
