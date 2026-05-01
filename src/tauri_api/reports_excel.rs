use anyhow::{Context, Result};
use rust_decimal::Decimal;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, Worksheet};

use super::reports::{BankReportRowDto, PayableRowDto, ReceivableRowDto, ReportsScreenDto};

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
        .join(" ")
        .chars()
        .rev()
        .collect::<String>();

    format!("{sign}{grouped},{frac} грн")
}

fn parse_money_ua(value: &str) -> Result<Decimal> {
    let normalized = value
        .trim()
        .replace("грн", "")
        .replace('\u{00a0}', "")
        .replace(' ', "")
        .replace(',', ".");
    normalized
        .parse::<Decimal>()
        .with_context(|| format!("Не вдалося розпарсити суму: {value}"))
}

fn aging_bucket_label(overdue_days: i32) -> &'static str {
    match overdue_days {
        i32::MIN..=-1 => "Не прострочено",
        0..=7 => "0-7 днів",
        8..=30 => "8-30 днів",
        31..=60 => "31-60 днів",
        61..=90 => "61-90 днів",
        _ => "90+ днів",
    }
}

fn write_sheet_title(worksheet: &mut Worksheet, title: &str) -> Result<()> {
    let title_format = Format::new()
        .set_bold()
        .set_font_size(16.)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x16324F))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    worksheet.merge_range(0, 0, 0, 5, title, &title_format)?;
    worksheet.set_row_height(0, 26)?;
    Ok(())
}

fn apply_tabular_sheet_finish(
    worksheet: &mut Worksheet,
    last_row: u32,
    last_col: u16,
) -> Result<()> {
    if last_row >= 1 {
        worksheet.autofilter(1, 0, last_row, last_col)?;
    }
    Ok(())
}

fn write_summary_sheet(workbook: &mut Workbook, screen: &ReportsScreenDto) -> Result<()> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Summary")?;
    write_sheet_title(worksheet, "Зведення звітів")?;

    let meta_format = Format::new()
        .set_font_color(Color::RGB(0x4B5563))
        .set_align(FormatAlign::Left);
    let section_format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x36536B))
        .set_border(FormatBorder::Thin);
    let label_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xEEF2F7))
        .set_border(FormatBorder::Thin);
    let value_format = Format::new().set_border(FormatBorder::Thin);
    let kpi_label_format = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0x4B5563))
        .set_background_color(Color::RGB(0xF5F7FA))
        .set_border(FormatBorder::Thin);
    let kpi_value_primary = Format::new()
        .set_bold()
        .set_font_size(15.)
        .set_font_color(Color::RGB(0x16324F))
        .set_background_color(Color::RGB(0xDCEBFA))
        .set_border(FormatBorder::Thin);
    let kpi_value_positive = Format::new()
        .set_bold()
        .set_font_size(15.)
        .set_font_color(Color::RGB(0x0E5A3A))
        .set_background_color(Color::RGB(0xDFF5E8))
        .set_border(FormatBorder::Thin);
    let kpi_value_negative = Format::new()
        .set_bold()
        .set_font_size(15.)
        .set_font_color(Color::RGB(0x8A1C1C))
        .set_background_color(Color::RGB(0xFBE4E4))
        .set_border(FormatBorder::Thin);

    worksheet.write_string_with_format(
        1,
        0,
        &format!(
            "Період: {} → {} | Scope: {}",
            screen.filter.date_from, screen.filter.date_to, screen.filter.scope
        ),
        &meta_format,
    )?;
    worksheet.write_string_with_format(
        2,
        0,
        &format!("Пошук: {}", screen.filter.query),
        &meta_format,
    )?;

    worksheet.merge_range(4, 0, 4, 2, "Грошовий стан", &section_format)?;
    worksheet.merge_range(4, 3, 4, 5, "P&L", &section_format)?;

    worksheet.write_string_with_format(5, 0, "Залишок на початок", &kpi_label_format)?;
    worksheet.write_string_with_format(
        5,
        1,
        &screen.summary.opening_balance_str,
        &kpi_value_primary,
    )?;
    worksheet.write_string_with_format(5, 2, "Надходження", &kpi_label_format)?;
    worksheet.write_string_with_format(5, 3, &screen.summary.income_str, &kpi_value_positive)?;
    worksheet.write_string_with_format(5, 4, "Витрати", &kpi_label_format)?;
    worksheet.write_string_with_format(5, 5, &screen.summary.expense_str, &kpi_value_negative)?;

    worksheet.write_string_with_format(6, 0, "Залишок на кінець", &kpi_label_format)?;
    worksheet.write_string_with_format(
        6,
        1,
        &screen.summary.closing_balance_str,
        &kpi_value_primary,
    )?;
    worksheet.write_string_with_format(6, 2, "Дебіторка", &kpi_label_format)?;
    worksheet.write_string_with_format(
        6,
        3,
        &screen.summary.receivables_total_str,
        &kpi_value_positive,
    )?;
    worksheet.write_string_with_format(6, 4, "Кредиторка", &kpi_label_format)?;
    worksheet.write_string_with_format(
        6,
        5,
        &screen.summary.payables_total_str,
        &kpi_value_negative,
    )?;

    worksheet.write_string_with_format(8, 3, "P&L дохід", &kpi_label_format)?;
    worksheet.write_string_with_format(
        8,
        4,
        &screen.summary.pnl_income_str,
        &kpi_value_positive,
    )?;
    worksheet.write_string_with_format(9, 3, "P&L витрати", &kpi_label_format)?;
    worksheet.write_string_with_format(
        9,
        4,
        &screen.summary.pnl_expense_str,
        &kpi_value_negative,
    )?;
    worksheet.write_string_with_format(10, 3, "P&L фінрезультат", &kpi_label_format)?;
    worksheet.write_string_with_format(
        10,
        4,
        &screen.summary.pnl_net_result_str,
        &kpi_value_primary,
    )?;

    let rows = [
        ("Звіт", screen.filter.tab.as_str()),
        ("Період від", screen.filter.date_from.as_str()),
        ("Період до", screen.filter.date_to.as_str()),
        ("Scope", screen.filter.scope.as_str()),
        ("Пошук", screen.filter.query.as_str()),
        (
            "Залишок на початок",
            screen.summary.opening_balance_str.as_str(),
        ),
        ("Надходження", screen.summary.income_str.as_str()),
        ("Витрати", screen.summary.expense_str.as_str()),
        (
            "Залишок на кінець",
            screen.summary.closing_balance_str.as_str(),
        ),
        (
            "Дебіторська заборгованість",
            screen.summary.receivables_total_str.as_str(),
        ),
        (
            "Кредиторська заборгованість",
            screen.summary.payables_total_str.as_str(),
        ),
        ("P&L дохід", screen.summary.pnl_income_str.as_str()),
        ("P&L витрати", screen.summary.pnl_expense_str.as_str()),
        (
            "P&L фінрезультат",
            screen.summary.pnl_net_result_str.as_str(),
        ),
    ];

    worksheet.write_string_with_format(12, 0, "Показник", &label_format)?;
    worksheet.write_string_with_format(12, 1, "Значення", &label_format)?;
    for (index, (label, value)) in rows.into_iter().enumerate() {
        let row = (index + 13) as u32;
        worksheet.write_string_with_format(row, 0, label, &label_format)?;
        worksheet.write_string_with_format(row, 1, value, &value_format)?;
    }

    worksheet.set_freeze_panes(13, 0)?;
    worksheet.set_column_width(0, 24)?;
    worksheet.set_column_width(1, 22)?;
    worksheet.set_column_width(2, 18)?;
    worksheet.set_column_width(3, 18)?;
    worksheet.set_column_width(4, 22)?;
    worksheet.set_column_width(5, 18)?;
    apply_tabular_sheet_finish(worksheet, 26, 1)?;
    Ok(())
}

fn write_bank_like_sheet(
    workbook: &mut Workbook,
    name: &str,
    title: &str,
    rows: &[BankReportRowDto],
    first_column: &str,
) -> Result<()> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name(name)?;
    write_sheet_title(worksheet, title)?;
    let header_format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x36536B))
        .set_border(FormatBorder::Thin);
    let cell_format = Format::new().set_border(FormatBorder::Thin);
    worksheet.write_string_with_format(1, 0, first_column, &header_format)?;
    worksheet.write_string_with_format(1, 1, "Дохід", &header_format)?;
    worksheet.write_string_with_format(1, 2, "Витрати", &header_format)?;
    worksheet.write_string_with_format(1, 3, "Результат", &header_format)?;

    for (index, row) in rows.iter().enumerate() {
        let sheet_row = (index + 2) as u32;
        worksheet.write_string_with_format(sheet_row, 0, &row.label, &cell_format)?;
        worksheet.write_string_with_format(sheet_row, 1, &row.income_str, &cell_format)?;
        worksheet.write_string_with_format(sheet_row, 2, &row.expense_str, &cell_format)?;
        worksheet.write_string_with_format(sheet_row, 3, &row.net_str, &cell_format)?;
    }

    worksheet.set_freeze_panes(2, 0)?;
    worksheet.set_column_width(0, 30)?;
    worksheet.set_column_width(1, 18)?;
    worksheet.set_column_width(2, 18)?;
    worksheet.set_column_width(3, 18)?;
    apply_tabular_sheet_finish(worksheet, rows.len() as u32 + 1, 3)?;
    Ok(())
}

fn write_aging_sheet_from_rows(
    workbook: &mut Workbook,
    name: &str,
    title: &str,
    rows: &[(&str, i32)],
) -> Result<()> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name(name)?;
    write_sheet_title(worksheet, title)?;

    let header_format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x36536B))
        .set_border(FormatBorder::Thin);
    let cell_format = Format::new().set_border(FormatBorder::Thin);

    let bucket_order = [
        "Не прострочено",
        "0-7 днів",
        "8-30 днів",
        "31-60 днів",
        "61-90 днів",
        "90+ днів",
    ];
    let mut counts = std::collections::BTreeMap::<&'static str, usize>::new();
    let mut totals = std::collections::BTreeMap::<&'static str, Decimal>::new();

    for (amount_str, overdue_days) in rows {
        let bucket = aging_bucket_label(*overdue_days);
        *counts.entry(bucket).or_default() += 1;
        *totals.entry(bucket).or_default() += parse_money_ua(amount_str)?;
    }

    worksheet.write_string_with_format(1, 0, "Bucket", &header_format)?;
    worksheet.write_string_with_format(1, 1, "К-сть записів", &header_format)?;
    worksheet.write_string_with_format(1, 2, "Сума", &header_format)?;

    for (index, bucket) in bucket_order.into_iter().enumerate() {
        let row = (index + 2) as u32;
        let count = counts.get(bucket).copied().unwrap_or_default();
        let total = totals.get(bucket).cloned().unwrap_or(Decimal::ZERO);
        worksheet.write_string_with_format(row, 0, bucket, &cell_format)?;
        worksheet.write_number_with_format(row, 1, count as f64, &cell_format)?;
        worksheet.write_string_with_format(row, 2, &format_money_ua(total), &cell_format)?;
    }

    worksheet.set_freeze_panes(2, 0)?;
    worksheet.set_column_width(0, 18)?;
    worksheet.set_column_width(1, 14)?;
    worksheet.set_column_width(2, 18)?;
    apply_tabular_sheet_finish(worksheet, 7, 2)?;
    Ok(())
}

fn write_top_counterparties_sheet(
    workbook: &mut Workbook,
    screen: &ReportsScreenDto,
) -> Result<()> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Top Counterparties")?;
    write_sheet_title(worksheet, "Топ контрагенти")?;

    let header_format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x36536B))
        .set_border(FormatBorder::Thin);
    let cell_format = Format::new().set_border(FormatBorder::Thin);

    let mut totals = std::collections::BTreeMap::<String, (Decimal, usize)>::new();
    for row in &screen.receivables_rows {
        let amount = parse_money_ua(&row.amount_str)?;
        let entry = totals
            .entry(row.counterparty.clone())
            .or_insert((Decimal::ZERO, 0));
        entry.0 += amount;
        entry.1 += 1;
    }
    for row in &screen.payables_rows {
        let amount = parse_money_ua(&row.amount_str)?;
        let entry = totals
            .entry(row.counterparty.clone())
            .or_insert((Decimal::ZERO, 0));
        entry.0 += amount;
        entry.1 += 1;
    }

    let mut rows = totals.into_iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| right.1 .0.cmp(&left.1 .0));

    worksheet.write_string_with_format(1, 0, "Контрагент", &header_format)?;
    worksheet.write_string_with_format(1, 1, "Сума", &header_format)?;
    worksheet.write_string_with_format(1, 2, "Записів", &header_format)?;

    for (index, (counterparty, (amount, count))) in rows.into_iter().take(10).enumerate() {
        let sheet_row = (index + 2) as u32;
        worksheet.write_string_with_format(sheet_row, 0, &counterparty, &cell_format)?;
        worksheet.write_string_with_format(sheet_row, 1, &format_money_ua(amount), &cell_format)?;
        worksheet.write_number_with_format(sheet_row, 2, count as f64, &cell_format)?;
    }

    worksheet.set_freeze_panes(2, 0)?;
    worksheet.set_column_width(0, 28)?;
    worksheet.set_column_width(1, 18)?;
    worksheet.set_column_width(2, 12)?;
    apply_tabular_sheet_finish(worksheet, 11, 2)?;
    Ok(())
}

fn write_top_debtors_sheet(workbook: &mut Workbook, rows: &[ReceivableRowDto]) -> Result<()> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Top Debtors")?;
    write_sheet_title(worksheet, "Топ боржники")?;

    let header_format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x36536B))
        .set_border(FormatBorder::Thin);
    let cell_format = Format::new().set_border(FormatBorder::Thin);

    let mut totals = std::collections::BTreeMap::<String, Decimal>::new();
    for row in rows {
        *totals.entry(row.counterparty.clone()).or_default() += parse_money_ua(&row.amount_str)?;
    }
    let mut rows = totals.into_iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| right.1.cmp(&left.1));

    worksheet.write_string_with_format(1, 0, "Контрагент", &header_format)?;
    worksheet.write_string_with_format(1, 1, "До отримання", &header_format)?;

    for (index, (counterparty, amount)) in rows.into_iter().take(10).enumerate() {
        let sheet_row = (index + 2) as u32;
        worksheet.write_string_with_format(sheet_row, 0, &counterparty, &cell_format)?;
        worksheet.write_string_with_format(sheet_row, 1, &format_money_ua(amount), &cell_format)?;
    }

    worksheet.set_freeze_panes(2, 0)?;
    worksheet.set_column_width(0, 28)?;
    worksheet.set_column_width(1, 18)?;
    apply_tabular_sheet_finish(worksheet, 11, 1)?;
    Ok(())
}

fn write_receivables_sheet(workbook: &mut Workbook, rows: &[ReceivableRowDto]) -> Result<()> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Receivables")?;
    write_sheet_title(worksheet, "Дебіторська заборгованість")?;
    let header_format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x36536B))
        .set_border(FormatBorder::Thin);
    let cell_format = Format::new().set_border(FormatBorder::Thin);
    let overdue_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::RGB(0xFBE4E4))
        .set_font_color(Color::RGB(0x8A1C1C));
    let headers = [
        "Документ",
        "Тип",
        "Дата",
        "Компанія",
        "Контрагент",
        "Сума",
        "Очікувана дата",
        "Прострочка",
        "Статус",
    ];
    for (index, header) in headers.into_iter().enumerate() {
        worksheet.write_string_with_format(1, index as u16, header, &header_format)?;
    }
    for (index, row) in rows.iter().enumerate() {
        let sheet_row = (index + 2) as u32;
        let active_format = if row.overdue_days > 0 {
            &overdue_format
        } else {
            &cell_format
        };
        worksheet.write_string_with_format(sheet_row, 0, &row.doc_number, active_format)?;
        worksheet.write_string_with_format(sheet_row, 1, &row.doc_type, active_format)?;
        worksheet.write_string_with_format(sheet_row, 2, &row.doc_date, active_format)?;
        worksheet.write_string_with_format(sheet_row, 3, &row.company_name, active_format)?;
        worksheet.write_string_with_format(sheet_row, 4, &row.counterparty, active_format)?;
        worksheet.write_string_with_format(sheet_row, 5, &row.amount_str, active_format)?;
        worksheet.write_string_with_format(sheet_row, 6, &row.expected_date, active_format)?;
        worksheet.write_number_with_format(
            sheet_row,
            7,
            f64::from(row.overdue_days),
            active_format,
        )?;
        worksheet.write_string_with_format(sheet_row, 8, &row.status, active_format)?;
    }
    worksheet.set_freeze_panes(2, 0)?;
    worksheet.set_column_width(0, 18)?;
    worksheet.set_column_width(1, 14)?;
    worksheet.set_column_width(2, 14)?;
    worksheet.set_column_width(3, 20)?;
    worksheet.set_column_width(4, 24)?;
    worksheet.set_column_width(5, 16)?;
    worksheet.set_column_width(6, 16)?;
    worksheet.set_column_width(7, 12)?;
    worksheet.set_column_width(8, 18)?;
    apply_tabular_sheet_finish(worksheet, rows.len() as u32 + 1, 8)?;
    Ok(())
}

fn write_payables_sheet(workbook: &mut Workbook, rows: &[PayableRowDto]) -> Result<()> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Payables")?;
    write_sheet_title(worksheet, "Кредиторська заборгованість")?;
    let header_format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x36536B))
        .set_border(FormatBorder::Thin);
    let cell_format = Format::new().set_border(FormatBorder::Thin);
    let overdue_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::RGB(0xFBE4E4))
        .set_font_color(Color::RGB(0x8A1C1C));
    let headers = [
        "Назва",
        "Компанія",
        "Контрагент",
        "Сума",
        "Дата",
        "Прострочка",
        "Повтор",
    ];
    for (index, header) in headers.into_iter().enumerate() {
        worksheet.write_string_with_format(1, index as u16, header, &header_format)?;
    }
    for (index, row) in rows.iter().enumerate() {
        let sheet_row = (index + 2) as u32;
        let active_format = if row.overdue_days > 0 {
            &overdue_format
        } else {
            &cell_format
        };
        worksheet.write_string_with_format(sheet_row, 0, &row.title, active_format)?;
        worksheet.write_string_with_format(sheet_row, 1, &row.company_name, active_format)?;
        worksheet.write_string_with_format(sheet_row, 2, &row.counterparty, active_format)?;
        worksheet.write_string_with_format(sheet_row, 3, &row.amount_str, active_format)?;
        worksheet.write_string_with_format(sheet_row, 4, &row.due_date, active_format)?;
        worksheet.write_number_with_format(
            sheet_row,
            5,
            f64::from(row.overdue_days),
            active_format,
        )?;
        worksheet.write_string_with_format(sheet_row, 6, &row.recurrence, active_format)?;
    }
    worksheet.set_freeze_panes(2, 0)?;
    worksheet.set_column_width(0, 24)?;
    worksheet.set_column_width(1, 20)?;
    worksheet.set_column_width(2, 24)?;
    worksheet.set_column_width(3, 16)?;
    worksheet.set_column_width(4, 14)?;
    worksheet.set_column_width(5, 12)?;
    worksheet.set_column_width(6, 16)?;
    apply_tabular_sheet_finish(worksheet, rows.len() as u32 + 1, 6)?;
    Ok(())
}

pub fn export_excel_bytes(screen: &ReportsScreenDto) -> Result<Vec<u8>> {
    let mut workbook = Workbook::new();
    write_summary_sheet(&mut workbook, screen)?;
    write_bank_like_sheet(
        &mut workbook,
        "Cashflow",
        "Рух грошей",
        &screen.bank_rows,
        "Група",
    )?;
    write_bank_like_sheet(&mut workbook, "P&L", "P&L", &screen.pnl_rows, "Категорія")?;
    write_receivables_sheet(&mut workbook, &screen.receivables_rows)?;
    write_payables_sheet(&mut workbook, &screen.payables_rows)?;

    let receivables_aging_rows = screen
        .receivables_rows
        .iter()
        .map(|row| (row.amount_str.as_str(), row.overdue_days))
        .collect::<Vec<_>>();
    write_aging_sheet_from_rows(
        &mut workbook,
        "Aging Receivables",
        "Aging дебіторки",
        &receivables_aging_rows,
    )?;

    let payables_aging_rows = screen
        .payables_rows
        .iter()
        .map(|row| (row.amount_str.as_str(), row.overdue_days))
        .collect::<Vec<_>>();
    write_aging_sheet_from_rows(
        &mut workbook,
        "Aging Payables",
        "Aging кредиторки",
        &payables_aging_rows,
    )?;

    write_top_counterparties_sheet(&mut workbook, screen)?;
    write_top_debtors_sheet(&mut workbook, &screen.receivables_rows)?;
    workbook.save_to_buffer().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use calamine::{open_workbook_from_rs, Data, Reader, Xlsx};

    use super::*;
    use crate::tauri_api::reports::{
        BankReportRowDto, PayableRowDto, ReceivableRowDto, ReportsFilterDto, ReportsScreenDto,
        ReportsSummaryDto,
    };

    fn sample_screen() -> ReportsScreenDto {
        ReportsScreenDto {
            filter: ReportsFilterDto {
                tab: "pnl".to_string(),
                scope: "active".to_string(),
                date_from: "2026-02-01".to_string(),
                date_to: "2026-05-01".to_string(),
                query: "послуги".to_string(),
            },
            summary: ReportsSummaryDto {
                opening_balance_str: "125 000,00 грн".to_string(),
                income_str: "48 200,00 грн".to_string(),
                expense_str: "19 000,00 грн".to_string(),
                closing_balance_str: "154 200,00 грн".to_string(),
                receivables_total_str: "23 000,00 грн".to_string(),
                payables_total_str: "14 500,00 грн".to_string(),
                pnl_income_str: "62 000,00 грн".to_string(),
                pnl_expense_str: "21 400,00 грн".to_string(),
                pnl_net_result_str: "40 600,00 грн".to_string(),
            },
            bank_rows: vec![BankReportRowDto {
                key: "ops".to_string(),
                label: "Операційна діяльність".to_string(),
                income_str: "48 200,00 грн".to_string(),
                expense_str: "19 000,00 грн".to_string(),
                net_str: "29 200,00 грн".to_string(),
            }],
            pnl_rows: vec![BankReportRowDto {
                key: "services".to_string(),
                label: "Послуги".to_string(),
                income_str: "62 000,00 грн".to_string(),
                expense_str: "0,00 грн".to_string(),
                net_str: "62 000,00 грн".to_string(),
            }],
            receivables_rows: vec![ReceivableRowDto {
                doc_id: "doc-1".to_string(),
                doc_type: "invoice".to_string(),
                doc_number: "INV-42".to_string(),
                doc_date: "01.05.2026".to_string(),
                company_name: "ТОВ Акт".to_string(),
                counterparty: "ТОВ Ромашка".to_string(),
                amount_str: "48 200,00 грн".to_string(),
                expected_date: "05.05.2026".to_string(),
                overdue_days: 4,
                status: "Очікується".to_string(),
            }],
            payables_rows: vec![PayableRowDto {
                id: "pay-1".to_string(),
                title: "Оренда".to_string(),
                company_name: "ТОВ Акт".to_string(),
                counterparty: "ФОП Іваненко".to_string(),
                amount_str: "14 500,00 грн".to_string(),
                due_date: "03.05.2026".to_string(),
                overdue_days: 2,
                recurrence: "Щомісяця".to_string(),
            }],
        }
    }

    #[test]
    fn export_excel_bytes_creates_expected_sheets_and_values() {
        let bytes = export_excel_bytes(&sample_screen()).expect("xlsx buffer");
        let cursor = Cursor::new(bytes);
        let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor).expect("open workbook");

        let sheet_names = workbook.sheet_names().to_vec();
        assert_eq!(
            sheet_names,
            vec![
                "Summary",
                "Cashflow",
                "P&L",
                "Receivables",
                "Payables",
                "Aging Receivables",
                "Aging Payables",
                "Top Counterparties",
                "Top Debtors"
            ]
        );

        let summary = workbook.worksheet_range("Summary").expect("summary range");
        assert_eq!(
            summary.get_value((0, 0)),
            Some(&Data::String("Зведення звітів".to_string()))
        );
        assert_eq!(
            summary.get_value((26, 1)),
            Some(&Data::String("40 600,00 грн".to_string()))
        );

        let pnl = workbook.worksheet_range("P&L").expect("pnl range");
        assert_eq!(
            pnl.get_value((2, 0)),
            Some(&Data::String("Послуги".to_string()))
        );

        let receivables = workbook
            .worksheet_range("Receivables")
            .expect("receivables range");
        assert_eq!(receivables.get_value((2, 7)), Some(&Data::Float(4.0)));

        let aging = workbook
            .worksheet_range("Aging Receivables")
            .expect("aging range");
        assert_eq!(
            aging.get_value((3, 0)),
            Some(&Data::String("0-7 днів".to_string()))
        );
        assert_eq!(
            aging.get_value((3, 2)),
            Some(&Data::String("48 200,00 грн".to_string()))
        );

        let top_debtors = workbook
            .worksheet_range("Top Debtors")
            .expect("top debtors range");
        assert_eq!(
            top_debtors.get_value((2, 0)),
            Some(&Data::String("ТОВ Ромашка".to_string()))
        );
    }
}
