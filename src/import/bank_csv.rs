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
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    // ─── parse_decimal ────────────────────────────────────────────────────────

    #[test]
    fn parse_decimal_standard_dot() {
        assert_eq!(parse_decimal("1200.50").unwrap(), dec!(1200.50));
    }

    #[test]
    fn parse_decimal_european_space_comma() {
        // Типовий банківський формат: "1 200,50" — пробіл-тисячник, кома-десяткова
        assert_eq!(parse_decimal("1 200,50").unwrap(), dec!(1200.50));
    }

    #[test]
    fn parse_decimal_space_thousands_dot() {
        assert_eq!(parse_decimal("1 200.50").unwrap(), dec!(1200.50));
    }

    #[test]
    fn parse_decimal_trims_whitespace() {
        assert_eq!(parse_decimal("  500.00  ").unwrap(), dec!(500.00));
    }

    #[test]
    fn parse_decimal_negative() {
        assert_eq!(parse_decimal("-100.00").unwrap(), dec!(-100.00));
    }

    #[test]
    fn parse_decimal_integer_only() {
        assert_eq!(parse_decimal("1000").unwrap(), dec!(1000));
    }

    #[test]
    fn parse_decimal_invalid_returns_err() {
        assert!(parse_decimal("abc").is_err());
    }

    #[test]
    fn parse_decimal_empty_returns_err() {
        assert!(parse_decimal("").is_err());
    }

    // ─── parse_date ───────────────────────────────────────────────────────────

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
    fn parse_date_trims_whitespace() {
        let d = parse_date("  03.04.2026  ").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 3).unwrap());
    }

    #[test]
    fn parse_date_invalid_returns_err() {
        assert!(parse_date("99.99.9999").is_err());
    }

    #[test]
    fn parse_date_empty_returns_err() {
        assert!(parse_date("").is_err());
    }

    #[test]
    fn parse_date_slash_format_not_supported() {
        // "03/04/2026" — формат не підтримується парсером
        assert!(parse_date("03/04/2026").is_err());
    }

    // ─── UkrgasbankCsvParser ──────────────────────────────────────────────────

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
    fn ukrgasbank_parses_european_amount_quoted() {
        // CSV-поле в лапках: "1 500,75" → 1500.75
        let csv = "date,amount,description,direction\n\
                   01.04.2026,\"1 500,75\",Витрати,expense\n";
        let rows = UkrgasbankCsvParser.parse(csv).unwrap();
        assert_eq!(rows[0].amount, dec!(1500.75));
        assert_eq!(rows[0].direction, PaymentDirection::Expense);
    }

    #[test]
    fn ukrgasbank_parses_multiple_rows() {
        let csv = "date,amount,description,direction\n\
                   01.04.2026,1000.00,Надходження,income\n\
                   02.04.2026,500.00,Витрата,expense\n";
        let rows = UkrgasbankCsvParser.parse(csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].direction, PaymentDirection::Income);
        assert_eq!(rows[1].direction, PaymentDirection::Expense);
    }

    #[test]
    fn ukrgasbank_unknown_direction_returns_err() {
        let csv = "date,amount,description,direction\n\
                   01.04.2026,1000.00,Тест,unknown\n";
        assert!(UkrgasbankCsvParser.parse(csv).is_err());
    }

    #[test]
    fn ukrgasbank_empty_reference_gives_none_bank_ref() {
        // reference стовпець є, але значення порожнє → bank_ref = None
        let csv = "date,amount,description,direction,reference\n\
                   03.04.2026,200.00,Тест,in,\n";
        let rows = UkrgasbankCsvParser.parse(csv).unwrap();
        assert!(rows[0].bank_ref.is_none());
    }

    // ─── OschadbankCsvParser ──────────────────────────────────────────────────

    #[test]
    fn oschadbank_bank_name() {
        assert_eq!(OschadbankCsvParser.bank_name(), "Ощадбанк");
    }

    #[test]
    fn oschadbank_parses_ukrainian_directions() {
        // Ощадбанк використовує українські назви напрямку платежу
        let csv = "date,amount,description,direction\n\
                   10.03.2026,3000.00,Зарахування,надходження\n\
                   11.03.2026,750.00,Списання,витрата\n";
        let rows = OschadbankCsvParser.parse(csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].direction, PaymentDirection::Income);
        assert_eq!(rows[1].direction, PaymentDirection::Expense);
    }

    #[test]
    fn oschadbank_no_reference_column_gives_none_bank_ref() {
        let csv = "date,amount,description,direction\n\
                   05.04.2026,2500.00,Тест,in\n";
        let rows = OschadbankCsvParser.parse(csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].bank_ref.is_none());
        assert_eq!(rows[0].bank_name, "Ощадбанк");
    }

    #[test]
    fn oschadbank_iso_date_format() {
        // Ощадбанк може видавати дати у форматі ISO 8601
        let csv = "date,amount,description,direction\n\
                   2026-04-15,100.00,Тест ISO,out\n";
        let rows = OschadbankCsvParser.parse(csv).unwrap();
        assert_eq!(rows[0].date, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
        assert_eq!(rows[0].direction, PaymentDirection::Expense);
    }

    #[test]
    fn oschadbank_headers_are_case_insensitive() {
        // CSV з великими буквами у заголовках
        let csv = "Date,Amount,Description,Direction\n\
                   01.04.2026,800.00,Тест,Income\n";
        let rows = OschadbankCsvParser.parse(csv).unwrap();
        assert_eq!(rows[0].amount, dec!(800.00));
        assert_eq!(rows[0].direction, PaymentDirection::Income);
    }

    // ─── SenseBankCsvParser (існуючий тест) ──────────────────────────────────

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
