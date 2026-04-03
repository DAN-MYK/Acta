use anyhow::{Result, bail};
use chrono::NaiveDate;
use csv::StringRecord;
use rust_decimal::Decimal;

use crate::models::payment::PaymentDirection;

#[derive(Debug, Clone)]
pub struct ParsedBankRow {
    pub date:        NaiveDate,
    pub amount:      Decimal,
    pub direction:   PaymentDirection,
    pub description: String,
    pub bank_ref:    Option<String>,
    pub bank_name:   String,
}

pub trait BankStatementParser {
    fn bank_name(&self) -> &'static str;
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>>;
}

fn parse_decimal(raw: &str) -> Result<Decimal> {
    let normalized = raw.trim().replace(' ', "").replace(',', ".");
    Ok(normalized.parse::<Decimal>()?)
}

fn parse_date(raw: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(raw.trim(), "%d.%m.%Y")
        .or_else(|_| NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d"))
        .map_err(Into::into)
}

fn parse_generic_csv(bank_name: &str, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_text.as_bytes());

    let headers = reader.headers()?.clone();
    let date_idx = headers.iter().position(|v| v.eq_ignore_ascii_case("date")).unwrap_or(0);
    let amount_idx = headers.iter().position(|v| v.eq_ignore_ascii_case("amount")).unwrap_or(1);
    let desc_idx = headers
        .iter()
        .position(|v| v.eq_ignore_ascii_case("description"))
        .unwrap_or(2);
    let direction_idx = headers
        .iter()
        .position(|v| v.eq_ignore_ascii_case("direction"))
        .unwrap_or(3);
    let ref_idx = headers
        .iter()
        .position(|v| v.eq_ignore_ascii_case("reference"));

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        rows.push(parse_record(bank_name, &record, date_idx, amount_idx, desc_idx, direction_idx, ref_idx)?);
    }

    Ok(rows)
}

fn parse_record(
    bank_name: &str,
    record: &StringRecord,
    date_idx: usize,
    amount_idx: usize,
    desc_idx: usize,
    direction_idx: usize,
    ref_idx: Option<usize>,
) -> Result<ParsedBankRow> {
    let direction_raw = record.get(direction_idx).unwrap_or("").trim().to_lowercase();
    let direction = match direction_raw.as_str() {
        "income" | "in" | "надходження" => PaymentDirection::Income,
        "expense" | "out" | "витрата" => PaymentDirection::Expense,
        _ => bail!("Невідомий напрямок платежу: {direction_raw}"),
    };

    Ok(ParsedBankRow {
        date:        parse_date(record.get(date_idx).unwrap_or(""))?,
        amount:      parse_decimal(record.get(amount_idx).unwrap_or(""))?,
        direction,
        description: record.get(desc_idx).unwrap_or("").trim().to_string(),
        bank_ref:    ref_idx.and_then(|idx| record.get(idx)).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string),
        bank_name:   bank_name.to_string(),
    })
}

pub struct UkrgasbankCsvParser;
pub struct OschadbankCsvParser;
pub struct SenseBankCsvParser;

impl BankStatementParser for UkrgasbankCsvParser {
    fn bank_name(&self) -> &'static str { "Укргазбанк" }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> { parse_generic_csv(self.bank_name(), csv_text) }
}

impl BankStatementParser for OschadbankCsvParser {
    fn bank_name(&self) -> &'static str { "Ощадбанк" }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> { parse_generic_csv(self.bank_name(), csv_text) }
}

impl BankStatementParser for SenseBankCsvParser {
    fn bank_name(&self) -> &'static str { "Sense Bank" }
    fn parse(&self, csv_text: &str) -> Result<Vec<ParsedBankRow>> { parse_generic_csv(self.bank_name(), csv_text) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_parser_parses_minimal_statement() {
        let csv = "date,amount,description,direction,reference\n03.04.2026,1200.50,Оплата рахунку,income,REF-1\n";
        let rows = SenseBankCsvParser.parse(csv).expect("csv should parse");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bank_name, "Sense Bank");
        assert_eq!(rows[0].description, "Оплата рахунку");
        assert_eq!(rows[0].direction, PaymentDirection::Income);
    }
}
