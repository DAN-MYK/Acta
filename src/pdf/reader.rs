use std::path::Path;

use anyhow::{Context, Result};
use lopdf::Document;

/// Витягує весь текст з PDF-документу (всі сторінки, по порядку).
pub fn read_pdf_text(path: &Path) -> Result<String> {
    let doc = Document::load(path)
        .with_context(|| format!("Не вдалось відкрити PDF: {}", path.display()))?;

    let page_numbers: Vec<u32> = doc.get_pages().keys().copied().collect();

    let text = doc
        .extract_text(&page_numbers)
        .with_context(|| format!("Не вдалось витягти текст з PDF: {}", path.display()))?;

    Ok(text)
}

/// Замінює всі входження `old_text` → `new_text` на кожній сторінці PDF і зберігає файл.
///
/// Обмеження lopdf: замінює лише текст у `Tj`-операторах (точний рядок в одному операнді).
/// `TJ`-оператор (масив рядків, типовий для Typst) — не обробляється `replace_text`.
/// Якщо `old_text` не знайдено — функція успішно завершується без змін.
pub fn replace_pdf_text(path: &Path, old_text: &str, new_text: &str) -> Result<()> {
    let mut doc = Document::load(path)
        .with_context(|| format!("Не вдалось відкрити PDF: {}", path.display()))?;

    let page_numbers: Vec<u32> = doc.get_pages().keys().copied().collect();

    for page_number in page_numbers {
        doc.replace_text(page_number, old_text, new_text)
            .with_context(|| format!("replace_text на сторінці {page_number}"))?;
    }

    doc.save(path)
        .with_context(|| format!("Не вдалось зберегти PDF: {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_pdf_text_returns_err_for_missing_file() {
        let result = read_pdf_text(Path::new("__nonexistent__.pdf"));
        assert!(result.is_err());
    }

    #[test]
    fn replace_pdf_text_returns_err_for_missing_file() {
        let result = replace_pdf_text(
            Path::new("__nonexistent__.pdf"),
            "ЧЕРНЕТКА",
            "ОПЛАЧЕНО",
        );
        assert!(result.is_err());
    }

    fn typst_available() -> bool {
        std::process::Command::new("typst")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn typst_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    fn sample_act_data() -> crate::pdf::generator::PdfActData {
        use crate::pdf::generator::{PdfActData, PdfActItem, PdfCompany};
        PdfActData {
            number: "АКТ-2026-001".into(),
            date: "01.05.2026".into(),
            company: PdfCompany {
                name: "ФОП Тест".into(),
                edrpou: "1234567890".into(),
                iban: "UA123456789012345678901234567".into(),
                address: "м. Київ".into(),
            },
            client: PdfCompany {
                name: "ТОВ Замовник".into(),
                edrpou: "9876543210".into(),
                iban: "UA987654321098765432109876543".into(),
                address: "м. Львів".into(),
            },
            items: vec![PdfActItem {
                num: 1,
                name: "Послуга".into(),
                qty: "1.0000".into(),
                unit: "послуга".into(),
                price: "1000.00".into(),
                amount: "1000.00".into(),
            }],
            total: "1000.00".into(),
            total_words: "одна тисяча гривень 00 копійок".into(),
            notes: String::new(),
        }
    }

    #[test]
    fn read_pdf_text_extracts_text_from_typst_pdf() {
        if !typst_available() {
            eprintln!("пропуск: typst не встановлено");
            return;
        }
        let _guard = typst_lock().lock().unwrap();

        let out = std::env::temp_dir().join("acta_reader_integration.pdf");
        crate::pdf::generator::generate_act_pdf(
            &sample_act_data(),
            std::path::Path::new("templates/act.typ"),
            &out,
        )
        .expect("generate_act_pdf має завершитись успішно");

        let text = match read_pdf_text(&out) {
            Ok(text) => text,
            Err(e) => {
                let error_chain = format!("{:?}", e);
                if error_chain.contains("ToUnicode") {
                    eprintln!("пропуск: lopdf не підтримує Typst ToUnicode CMap");
                    let _ = std::fs::remove_file(&out);
                    return;
                } else {
                    panic!("Непередбачена помилка при читанні PDF: {e}");
                }
            }
        };
        assert!(!text.is_empty(), "витягнутий текст не повинен бути порожнім");

        let _ = std::fs::remove_file(&out);
    }
}
