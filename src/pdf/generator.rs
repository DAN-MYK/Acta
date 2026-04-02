// Генерація PDF-актів через Typst CLI
//
// Алгоритм: структури даних → serde_json → JSON рядок → typst compile --input data=...
#![allow(dead_code)]
// Typst читає sys.inputs["data"] і будує PDF з шаблону templates/act.typ.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use serde::Serialize;

// ── Структури даних для шаблону ───────────────────────────────────────────

/// Реквізити однієї сторони (виконавець або замовник).
#[derive(Debug, Serialize)]
pub struct PdfCompany {
    pub name: String,
    pub edrpou: String,
    pub iban: String,
    pub address: String,
}

/// Одна позиція акту — рядок таблиці у PDF.
#[derive(Debug, Serialize)]
pub struct PdfActItem {
    /// Порядковий номер (1, 2, …).
    pub num: u32,
    pub name: String,
    /// Кількість у форматі "1.0000".
    pub qty: String,
    /// Одиниця виміру: "послуга", "шт", "год" тощо.
    pub unit: String,
    /// Ціна за одиницю у форматі "45000.00".
    pub price: String,
    /// Сума = qty × price у форматі "45000.00".
    pub amount: String,
}

/// Усі дані, які передаються в Typst-шаблон через JSON.
#[derive(Debug, Serialize)]
pub struct PdfActData {
    /// Номер акту: "АКТ-2026-001".
    pub number: String,
    /// Дата у форматі ДД.ММ.РРРР: "28.03.2026".
    pub date: String,
    /// Виконавець (дані з конфігу програми).
    pub company: PdfCompany,
    /// Замовник (контрагент).
    pub client: PdfCompany,
    pub items: Vec<PdfActItem>,
    /// Загальна сума: "45000.00".
    pub total: String,
    /// Загальна сума прописом.
    pub total_words: String,
    /// Примітки (порожній рядок — не виводяться).
    pub notes: String,
}

// ── Публічні функції ──────────────────────────────────────────────────────

/// Генерує PDF акту з переданих даних.
///
/// Алгоритм:
/// 1. Серіалізує `data` у JSON рядок.
/// 2. Викликає `typst compile templates/act.typ <output_path> --input data=<json>`.
/// 3. Перевіряє успішність команди через `ensure!`.
pub fn generate_act_pdf(data: &PdfActData, output_path: &Path) -> Result<()> {
    let json = serde_json::to_string(data).context("Серіалізація PdfActData у JSON")?;

    let input_arg = format!("data={json}");

    let output = std::process::Command::new("typst")
        .args([
            "compile",
            "templates/act.typ",
            output_path
                .to_str()
                .context("Невалідний шлях до output PDF")?,
            "--input",
            &input_arg,
        ])
        .output()
        .context("Не вдалось запустити typst. Перевір чи встановлено: scoop install typst")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("typst завершився з помилкою:\n{stderr}");
    }

    tracing::info!(path = %output_path.display(), "PDF акту згенеровано");
    Ok(())
}

/// Створює директорію `storage/documents/acts/{рік}/` і повертає
/// повний шлях до файлу `{act_number}.pdf`.
///
/// `act_number` очищується від символу `/` щоб уникнути небажаних підкаталогів.
/// Приклад: "АКТ-2026-001" → `storage/documents/acts/2026/АКТ-2026-001.pdf`
pub fn ensure_output_dir(act_number: &str) -> Result<PathBuf> {
    // Рік витягуємо з другого сегменту номеру (АКТ-2026-001 → "2026").
    // Якщо формат несподіваний — кладемо в "misc".
    let year = act_number
        .split('-')
        .nth(1)
        .filter(|s| s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or("misc");

    let dir = PathBuf::from("storage/documents/acts").join(year);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Не вдалось створити директорію {}", dir.display()))?;

    // Замінюємо символи небезпечні для імені файлу.
    let safe_name = act_number.replace(['/', '\\', ':'], "_");
    let file_path = dir.join(format!("{safe_name}.pdf"));

    Ok(file_path)
}

// ── Конвертація суми прописом ─────────────────────────────────────────────

/// Повертає суму прописом українською мовою.
///
/// Приклад: `Decimal::from_str("45000.00")` → `"Сорок п'ять тисяч гривень 00 копійок"`
///
/// Підтримувані діапазони: 0 – 999 999 999 гривень.
pub fn amount_to_words(amount: &Decimal) -> String {
    let hryvnias = amount.trunc().mantissa().unsigned_abs() as u64;
    // Копійки: (45000.75 - 45000) * 100 → 75
    let kopecks = ((*amount - amount.trunc()) * Decimal::from(100))
        .round()
        .mantissa()
        .unsigned_abs() as u64;

    let hryvnia_words = integer_to_words(hryvnias, Gender::Feminine);
    let hryvnia_form = plural_form(hryvnias, "гривня", "гривні", "гривень");

    format!("{hryvnia_words} {hryvnia_form} {kopecks:02} копійок")
}

// ── Внутрішні допоміжні функції ────────────────────────────────────────────

/// Граматичний рід для числівників (впливає на форму "один/одна", "два/дві").
#[derive(Clone, Copy)]
enum Gender {
    Masculine,
    Feminine,
}

/// Повертає правильну форму іменника залежно від числа (1/2-4/5+).
fn plural_form<'a>(n: u64, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    let last_two = n % 100;
    let last_one = n % 10;
    if (11..=19).contains(&last_two) {
        many
    } else {
        match last_one {
            1 => one,
            2..=4 => few,
            _ => many,
        }
    }
}

/// Перетворює ціле число (0–999 999 999) у слова з урахуванням роду.
fn integer_to_words(n: u64, gender: Gender) -> String {
    if n == 0 {
        return "нуль".to_string();
    }

    let mut parts: Vec<String> = Vec::new();

    // Мільйони (чоловічий рід: один мільйон, два мільйони)
    if n >= 1_000_000 {
        let millions = n / 1_000_000;
        parts.push(hundreds_to_words(millions, Gender::Masculine));
        parts.push(plural_form(millions, "мільйон", "мільйони", "мільйонів").to_string());
    }

    // Тисячі (жіночий рід: одна тисяча, дві тисячі)
    let thousands = (n % 1_000_000) / 1_000;
    if thousands > 0 {
        parts.push(hundreds_to_words(thousands, Gender::Feminine));
        parts.push(plural_form(thousands, "тисяча", "тисячі", "тисяч").to_string());
    }

    // Залишок (менше тисячі), рід успадковується від головного іменника
    let remainder = n % 1_000;
    if remainder > 0 {
        parts.push(hundreds_to_words(remainder, gender));
    }

    parts.join(" ")
}

/// Перетворює число 1–999 у слова з урахуванням роду одиниць.
fn hundreds_to_words(n: u64, gender: Gender) -> String {
    let mut parts: Vec<&str> = Vec::new();

    // Сотні
    let h = n / 100;
    if h > 0 {
        parts.push(hundreds_word(h));
    }

    // Десятки і одиниці
    let rest = n % 100;
    if rest >= 20 {
        parts.push(tens_word(rest / 10));
        let ones = rest % 10;
        if ones > 0 {
            parts.push(ones_word(ones, gender));
        }
    } else if rest >= 11 {
        // 11–19: виключення — спеціальні слова (незмінні)
        parts.push(teens_word(rest));
    } else if rest >= 1 {
        parts.push(ones_word(rest, gender));
    }

    parts.join(" ")
}

fn hundreds_word(h: u64) -> &'static str {
    match h {
        1 => "сто",
        2 => "двісті",
        3 => "триста",
        4 => "чотириста",
        5 => "п'ятсот",
        6 => "шістсот",
        7 => "сімсот",
        8 => "вісімсот",
        9 => "дев'ятсот",
        _ => "",
    }
}

fn tens_word(t: u64) -> &'static str {
    match t {
        2 => "двадцять",
        3 => "тридцять",
        4 => "сорок",
        5 => "п'ятдесят",
        6 => "шістдесят",
        7 => "сімдесят",
        8 => "вісімдесят",
        9 => "дев'яносто",
        _ => "",
    }
}

/// 11–19 — виключення, незмінні
fn teens_word(n: u64) -> &'static str {
    match n {
        11 => "одинадцять",
        12 => "дванадцять",
        13 => "тринадцять",
        14 => "чотирнадцять",
        15 => "п'ятнадцять",
        16 => "шістнадцять",
        17 => "сімнадцять",
        18 => "вісімнадцять",
        19 => "дев'ятнадцять",
        _ => "десять",
    }
}

/// 1–9 з урахуванням роду (впливає лише на 1 і 2).
fn ones_word(n: u64, gender: Gender) -> &'static str {
    match (n, gender) {
        (1, Gender::Feminine) => "одна",
        (1, Gender::Masculine) => "один",
        (2, Gender::Feminine) => "дві",
        (2, Gender::Masculine) => "два",
        (3, _) => "три",
        (4, _) => "чотири",
        (5, _) => "п'ять",
        (6, _) => "шість",
        (7, _) => "сім",
        (8, _) => "вісім",
        (9, _) => "дев'ять",
        _ => "",
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ── Видаткові накладні ───────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════

/// Одна позиція накладної — рядок таблиці у PDF.
///
/// Поле `price` (не `unit_price`) — відповідає схемі БД invoice_items.
#[derive(Debug, Serialize)]
pub struct PdfInvoiceItem {
    pub num: u32,
    pub name: String,
    pub qty: String,
    pub unit: String,
    pub price: String,
    pub amount: String,
}

/// Усі дані для Typst-шаблону накладної.
///
/// `vat_amount` — рядок "0.00" якщо ФОП без ПДВ; шаблон приховує блок ПДВ якщо "0.00".
#[derive(Debug, Serialize)]
pub struct PdfInvoiceData {
    pub number: String,
    pub date: String,
    pub company: PdfCompany,
    pub client: PdfCompany,
    pub items: Vec<PdfInvoiceItem>,
    pub total: String,
    pub vat_amount: String,
    pub total_words: String,
    pub notes: String,
}

/// Генерує PDF видаткової накладної з переданих даних.
///
/// Шаблон: `templates/invoice.typ`.
/// Алгоритм аналогічний `generate_act_pdf`.
pub fn generate_invoice_pdf(data: &PdfInvoiceData, output_path: &Path) -> Result<()> {
    let json = serde_json::to_string(data).context("Серіалізація PdfInvoiceData у JSON")?;
    let input_arg = format!("data={json}");

    let output = std::process::Command::new("typst")
        .args([
            "compile",
            "templates/invoice.typ",
            output_path
                .to_str()
                .context("Невалідний шлях до output PDF")?,
            "--input",
            &input_arg,
        ])
        .output()
        .context("Не вдалось запустити typst. Перевір чи встановлено: scoop install typst")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("typst завершився з помилкою:\n{stderr}");
    }

    tracing::info!(path = %output_path.display(), "PDF накладної згенеровано");
    Ok(())
}

/// Створює `storage/documents/invoices/{рік}/` і повертає шлях до файлу.
///
/// Приклад: "НАК-2026-001" → `storage/documents/invoices/2026/НАК-2026-001.pdf`
pub fn ensure_invoice_output_dir(invoice_number: &str) -> Result<PathBuf> {
    let year = invoice_number
        .split('-')
        .nth(1)
        .filter(|s| s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or("misc");

    let dir = PathBuf::from("storage/documents/invoices").join(year);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Не вдалось створити директорію {}", dir.display()))?;

    let safe_name = invoice_number.replace(['/', '\\', ':'], "_");
    let file_path = dir.join(format!("{safe_name}.pdf"));

    Ok(file_path)
}

// ── Тести ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_amount_to_words_simple() {
        assert_eq!(amount_to_words(&dec!(100.00)), "сто гривень 00 копійок");
    }

    #[test]
    fn test_amount_to_words_thousands() {
        assert_eq!(
            amount_to_words(&dec!(45000.00)),
            "сорок п'ять тисяч гривень 00 копійок"
        );
    }

    #[test]
    fn test_amount_to_words_with_kopecks() {
        assert_eq!(
            amount_to_words(&dec!(1234.56)),
            "одна тисяча двісті тридцять чотири гривні 56 копійок"
        );
    }

    #[test]
    fn test_amount_to_words_one_hryvnia() {
        assert_eq!(amount_to_words(&dec!(1.00)), "одна гривня 00 копійок");
    }

    #[test]
    fn test_amount_to_words_two_hryvnias() {
        assert_eq!(amount_to_words(&dec!(2.00)), "дві гривні 00 копійок");
    }

    #[test]
    fn test_amount_to_words_millions() {
        assert_eq!(
            amount_to_words(&dec!(1000000.00)),
            "один мільйон гривень 00 копійок"
        );
    }

    #[test]
    fn test_ensure_output_dir() {
        let path = ensure_output_dir("АКТ-2026-001").unwrap();
        assert!(path.to_str().unwrap().contains("2026"));
        assert!(path.to_str().unwrap().ends_with(".pdf"));
    }

    #[test]
    fn test_ensure_invoice_output_dir() {
        let path = ensure_invoice_output_dir("НАК-2026-001").unwrap();
        assert!(path.to_str().unwrap().contains("invoices"));
        assert!(path.to_str().unwrap().contains("2026"));
        assert!(path.to_str().unwrap().ends_with(".pdf"));
    }

    #[test]
    fn pdf_invoice_data_serializes_to_json() {
        let data = PdfInvoiceData {
            number: "НАК-2026-001".into(),
            date: "01.04.2026".into(),
            company: PdfCompany { name: "ФОП Тест".into(), edrpou: "1234567890".into(), iban: "UA123".into(), address: "Київ".into() },
            client:  PdfCompany { name: "ТОВ Клієнт".into(), edrpou: "0987654321".into(), iban: "UA456".into(), address: String::new() },
            items: vec![PdfInvoiceItem { num: 1, name: "Товар".into(), qty: "2.0000".into(), unit: "шт".into(), price: "500.00".into(), amount: "1000.00".into() }],
            total: "1000.00".into(),
            vat_amount: "0.00".into(),
            total_words: "одна тисяча гривень 00 копійок".into(),
            notes: String::new(),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("НАК-2026-001"));
        assert!(json.contains("vat_amount"));
    }
}
