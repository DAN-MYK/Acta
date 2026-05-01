use std::path::Path;

use anyhow::{Context, Result};
use lopdf::Document;

pub fn read_pdf_text(path: &Path) -> Result<String> {
    let doc = Document::load(path)
        .with_context(|| format!("Не вдалось відкрити PDF: {}", path.display()))?;

    let page_numbers: Vec<u32> = doc.get_pages().keys().copied().collect();

    let text = doc
        .extract_text(&page_numbers)
        .with_context(|| format!("Не вдалось витягти текст з PDF: {}", path.display()))?;

    Ok(text)
}

pub fn replace_pdf_text(_path: &Path, _old_text: &str, _new_text: &str) -> Result<()> {
    todo!()
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
