//! XLSX/XLS-парсер банківських виписок. Працює через `calamine` і повторно
//! використовує header-aliases та decimal/date нормалізатори з
//! [`crate::import::bank_common`].
//!
//! Підтримує:
//!   - звичайні текстові комірки з датою у форматах `dd.mm.yyyy`, `yyyy-mm-dd`, тощо
//!   - "real" Excel-дати (1900 epoch)
//!   - чисельні комірки для сум (Excel зберігає число, а не рядок)
//!   - debit/credit pair колонки як альтернативу signed amount
//!
//! Шукає header row автоматично — пропускає до 12 верхніх порожніх або
//! заголовкових рядків (звичайна виписка має стилі, рядок-summary тощо).

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use calamine::{open_workbook_auto, Data, Range, Reader as CalamineReader};
use chrono::{Datelike, NaiveDate};

use crate::import::bank_common::{
    amount_and_direction_from_strings, header_layout_from_strs, normalize_iban, parse_date,
    HeaderLayout, ParsedBankRow,
};

/// Скільки верхніх рядків переглядаємо у пошуках header row.
const HEADER_SCAN_LIMIT: usize = 12;
/// Мінімальна кількість розпізнаних колонок у рядку-кандидаті щоб вважати його header.
const HEADER_MIN_RECOGNIZED: usize = 3;

/// Парсить виписку з XLSX/XLS файлу.
///
/// Виконується у `tokio::task::spawn_blocking`, бо `calamine` синхронний.
pub async fn parse_xlsx_file(bank_name: &str, path: &Path) -> Result<Vec<ParsedBankRow>> {
    let path = path.to_path_buf();
    let bank_name = bank_name.to_string();

    tokio::task::spawn_blocking(move || parse_xlsx_path_blocking(&bank_name, &path))
        .await
        .context("XLSX-парсер виписки завершився помилкою")?
}

fn parse_xlsx_path_blocking(bank_name: &str, path: &Path) -> Result<Vec<ParsedBankRow>> {
    let mut workbook = open_workbook_auto(path)
        .with_context(|| format!("Не вдалося відкрити Excel файл {}", path.display()))?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("У XLSX файлі немає жодного аркуша"))?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .with_context(|| format!("Не вдалося прочитати аркуш '{sheet_name}'"))?;

    parse_xlsx_range(bank_name, &range)
}

fn parse_xlsx_range(bank_name: &str, range: &Range<Data>) -> Result<Vec<ParsedBankRow>> {
    let rows: Vec<Vec<&Data>> = range.rows().map(|row| row.iter().collect()).collect();
    let (header_row_idx, layout) = locate_header(&rows)
        .ok_or_else(|| anyhow!("У XLSX не знайдено рядок із заголовками банківської виписки"))?;
    let date_idx = layout
        .date_idx
        .ok_or_else(|| anyhow!("У XLSX не знайдено колонку з датою операції"))?;

    let mut parsed_rows = Vec::new();
    for row in rows.iter().skip(header_row_idx + 1) {
        if row.iter().all(|cell| is_cell_empty(cell)) {
            continue;
        }
        if let Some(parsed) = parse_data_row(bank_name, row, date_idx, &layout)? {
            parsed_rows.push(parsed);
        }
    }

    Ok(parsed_rows)
}

/// Шукає header row серед перших `HEADER_SCAN_LIMIT` рядків. Повертає
/// індекс header row + готовий `HeaderLayout`.
fn locate_header(rows: &[Vec<&Data>]) -> Option<(usize, HeaderLayout)> {
    for (idx, row) in rows.iter().take(HEADER_SCAN_LIMIT).enumerate() {
        let headers: Vec<String> = row.iter().map(|cell| cell_to_text(cell)).collect();
        let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
        let layout = header_layout_from_strs(&header_refs);

        if layout_recognized_count(&layout) >= HEADER_MIN_RECOGNIZED && layout.date_idx.is_some() {
            return Some((idx, layout));
        }
    }
    None
}

fn layout_recognized_count(layout: &HeaderLayout) -> usize {
    [
        layout.date_idx,
        layout.amount_idx,
        layout.description_idx,
        layout.direction_idx,
        layout.reference_idx,
        layout.counterparty_name_idx,
        layout.counterparty_iban_idx,
        layout.currency_idx,
        layout.debit_idx,
        layout.credit_idx,
    ]
    .into_iter()
    .filter(Option::is_some)
    .count()
}

fn parse_data_row(
    bank_name: &str,
    row: &[&Data],
    date_idx: usize,
    layout: &HeaderLayout,
) -> Result<Option<ParsedBankRow>> {
    let date = match cell_to_date(row.get(date_idx).copied()) {
        Some(value) => value,
        None => return Ok(None), // рядок без дати — пропускаємо (часто це summary-рядок)
    };

    let direction_text = layout
        .direction_idx
        .and_then(|idx| row.get(idx).copied())
        .map(cell_to_text);
    let amount_text = layout
        .amount_idx
        .and_then(|idx| row.get(idx).copied())
        .map(cell_to_amount_text);
    let debit_text = layout
        .debit_idx
        .and_then(|idx| row.get(idx).copied())
        .map(cell_to_amount_text);
    let credit_text = layout
        .credit_idx
        .and_then(|idx| row.get(idx).copied())
        .map(cell_to_amount_text);

    let (amount, direction) = amount_and_direction_from_strings(
        direction_text.as_deref(),
        amount_text.as_deref(),
        debit_text.as_deref(),
        credit_text.as_deref(),
    )?;

    Ok(Some(ParsedBankRow {
        date,
        amount,
        direction,
        description: cell_text_or_empty(row, layout.description_idx),
        bank_ref: cell_optional_text(row, layout.reference_idx),
        bank_name: bank_name.to_string(),
        counterparty_name: cell_optional_text(row, layout.counterparty_name_idx),
        counterparty_iban: cell_optional_text(row, layout.counterparty_iban_idx)
            .as_deref()
            .and_then(normalize_iban),
        currency: cell_optional_text(row, layout.currency_idx),
    }))
}

fn cell_text_or_empty(row: &[&Data], idx: Option<usize>) -> String {
    cell_optional_text(row, idx).unwrap_or_default()
}

fn cell_optional_text(row: &[&Data], idx: Option<usize>) -> Option<String> {
    let cell = idx.and_then(|index| row.get(index)).copied()?;
    let text = cell_to_text(cell);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_cell_empty(cell: &Data) -> bool {
    matches!(cell, Data::Empty) || cell_to_text(cell).trim().is_empty()
}

fn cell_to_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.clone(),
        Data::Bool(value) => value.to_string(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) => format_float_loseless(*value),
        Data::DateTime(value) => format_excel_datetime(*value),
        Data::DateTimeIso(value) | Data::DurationIso(value) => value.clone(),
        Data::Error(_) => String::new(),
    }
}

/// Перетворює числову Excel-комірку на рядок придатний для `parse_decimal`.
/// На відміну від `cell_to_text`, тут не використовуємо `Display` для float
/// напряму (бо Rust може дати `1.7000000000000002`), а форматуємо через
/// фіксовану точність.
fn cell_to_amount_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.clone(),
        Data::Bool(value) => value.to_string(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) => format_float_loseless(*value),
        Data::DateTime(value) => format!("{value}"),
        Data::DateTimeIso(value) | Data::DurationIso(value) => value.clone(),
        Data::Error(_) => String::new(),
    }
}

fn format_float_loseless(value: f64) -> String {
    // Excel зберігає копійки з плаваючою точкою. Округлюємо до 4-х знаків
    // (достатньо для будь-якої грошової суми) і прибираємо хвостові нулі.
    let formatted = format!("{:.4}", value);
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

fn format_excel_datetime(value: calamine::ExcelDateTime) -> String {
    if let Some(date) = excel_serial_to_date(value.as_f64()) {
        date.format("%Y-%m-%d").to_string()
    } else {
        format!("{}", value.as_f64())
    }
}

fn cell_to_date(cell: Option<&Data>) -> Option<NaiveDate> {
    let cell = cell?;
    match cell {
        Data::DateTime(value) => excel_serial_to_date(value.as_f64()),
        Data::DateTimeIso(value) => parse_date(value).ok(),
        Data::Float(value) => excel_serial_to_date(*value),
        Data::Int(value) => excel_serial_to_date(*value as f64),
        Data::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                parse_date(trimmed).ok()
            }
        }
        _ => None,
    }
}

/// Конвертує Excel-серіальне число в `NaiveDate`. Excel рахує дні від
/// 1899-12-30 (через bug сумісний з Lotus 1-2-3 — 1900 vs 1900-01-01).
fn excel_serial_to_date(serial: f64) -> Option<NaiveDate> {
    if !serial.is_finite() || serial < 0.0 {
        return None;
    }
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let days = serial.trunc() as i64;
    epoch.checked_add_signed(chrono::Duration::days(days))
}

#[allow(dead_code)]
fn naive_date_year(value: NaiveDate) -> i32 {
    value.year()
}

/// Виявляє чи виглядає файл як XLSX/XLS за розширенням.
pub fn is_xlsx_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()).map(str::to_lowercase),
        Some(ext) if ext == "xlsx" || ext == "xls"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::payment::PaymentDirection;
    use calamine::Range;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn make_range(rows: Vec<Vec<Data>>) -> Range<Data> {
        let total_rows = rows.len() as u32;
        let total_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0) as u32;
        let mut range = Range::new((0, 0), (total_rows.saturating_sub(1), total_cols.saturating_sub(1)));
        for (r, row) in rows.into_iter().enumerate() {
            for (c, cell) in row.into_iter().enumerate() {
                range.set_value((r as u32, c as u32), cell);
            }
        }
        range
    }

    #[test]
    fn excel_serial_to_date_known_anchor() {
        // 44197 = 2021-01-01 за Excel-епохою
        let date = excel_serial_to_date(44197.0).unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2021, 1, 1).unwrap());
    }

    #[test]
    fn parse_xlsx_range_basic_layout() {
        let rows = vec![
            vec![
                Data::String("Дата".to_string()),
                Data::String("Сума".to_string()),
                Data::String("Призначення".to_string()),
                Data::String("Напрям".to_string()),
                Data::String("Референс".to_string()),
            ],
            vec![
                Data::String("15.04.2026".to_string()),
                Data::Float(2500.50),
                Data::String("Оплата за послуги".to_string()),
                Data::String("надходження".to_string()),
                Data::String("REF-XLSX-1".to_string()),
            ],
            vec![
                Data::String("16.04.2026".to_string()),
                Data::Float(-300.00),
                Data::String("Комісія".to_string()),
                Data::Empty,
                Data::String("REF-XLSX-2".to_string()),
            ],
        ];
        let range = make_range(rows);
        let parsed = parse_xlsx_range("ПриватБанк", &range).expect("should parse");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].date, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
        assert_eq!(parsed[0].amount, dec!(2500.5));
        assert_eq!(parsed[0].direction, PaymentDirection::Income);
        assert_eq!(parsed[0].bank_ref.as_deref(), Some("REF-XLSX-1"));
        assert_eq!(parsed[0].bank_name, "ПриватБанк");
        assert_eq!(parsed[1].direction, PaymentDirection::Expense);
        assert_eq!(parsed[1].amount, dec!(300));
    }

    #[test]
    fn parse_xlsx_range_skips_summary_rows_above_header() {
        let rows = vec![
            vec![Data::String("Виписка по рахунку".to_string())],
            vec![Data::String("Період: 01.04.2026 — 30.04.2026".to_string())],
            vec![Data::Empty],
            vec![
                Data::String("date".to_string()),
                Data::String("amount".to_string()),
                Data::String("description".to_string()),
                Data::String("direction".to_string()),
            ],
            vec![
                Data::String("2026-04-15".to_string()),
                Data::Float(1000.00),
                Data::String("Тест".to_string()),
                Data::String("income".to_string()),
            ],
        ];
        let range = make_range(rows);
        let parsed = parse_xlsx_range("Тест Банк", &range).expect("should parse");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].amount, dec!(1000.00));
    }

    #[test]
    fn parse_xlsx_range_with_excel_date_serial() {
        let rows = vec![
            vec![
                Data::String("Дата".to_string()),
                Data::String("Сума".to_string()),
                Data::String("Призначення".to_string()),
                Data::String("Напрям".to_string()),
            ],
            vec![
                Data::Float(44197.0), // 2021-01-01
                Data::Float(500.00),
                Data::String("Тест".to_string()),
                Data::String("income".to_string()),
            ],
        ];
        let range = make_range(rows);
        let parsed = parse_xlsx_range("Тест Банк", &range).expect("should parse");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].date, NaiveDate::from_ymd_opt(2021, 1, 1).unwrap());
    }

    #[test]
    fn parse_xlsx_range_handles_debit_credit_columns() {
        let rows = vec![
            vec![
                Data::String("date".to_string()),
                Data::String("debit".to_string()),
                Data::String("credit".to_string()),
                Data::String("description".to_string()),
            ],
            vec![
                Data::String("2026-04-15".to_string()),
                Data::Float(0.0),
                Data::Float(750.25),
                Data::String("Надходження".to_string()),
            ],
            vec![
                Data::String("2026-04-16".to_string()),
                Data::Float(120.00),
                Data::Float(0.0),
                Data::String("Списання".to_string()),
            ],
        ];
        let range = make_range(rows);
        let parsed = parse_xlsx_range("Тест Банк", &range).expect("should parse");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].direction, PaymentDirection::Income);
        assert_eq!(parsed[0].amount, dec!(750.25));
        assert_eq!(parsed[1].direction, PaymentDirection::Expense);
        assert_eq!(parsed[1].amount, dec!(120));
    }

    #[test]
    fn is_xlsx_path_recognises_extensions() {
        assert!(is_xlsx_path(Path::new("statement.xlsx")));
        assert!(is_xlsx_path(Path::new("STATEMENT.XLS")));
        assert!(!is_xlsx_path(Path::new("statement.csv")));
        assert!(!is_xlsx_path(Path::new("statement")));
    }

    #[test]
    fn parse_xlsx_range_skips_rows_without_date() {
        let rows = vec![
            vec![
                Data::String("date".to_string()),
                Data::String("amount".to_string()),
                Data::String("description".to_string()),
                Data::String("direction".to_string()),
            ],
            vec![
                Data::String("2026-04-15".to_string()),
                Data::Float(100.0),
                Data::String("Норм".to_string()),
                Data::String("income".to_string()),
            ],
            vec![
                Data::Empty,
                Data::Float(0.0),
                Data::String("Підсумок".to_string()),
                Data::Empty,
            ],
        ];
        let range = make_range(rows);
        let parsed = parse_xlsx_range("Тест Банк", &range).expect("should parse");
        assert_eq!(parsed.len(), 1);
    }
}
