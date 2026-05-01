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
}
