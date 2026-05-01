use std::path::Path;

use anyhow::Result;

pub fn read_pdf_text(_path: &Path) -> Result<String> {
    todo!()
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
