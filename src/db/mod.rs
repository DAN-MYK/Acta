// Модуль роботи з базою даних
// Кожен файл — CRUD для однієї таблиці

/// Екранує спецсимволи ILIKE: `\` → `\\`, `%` → `\%`, `_` → `\_`.
/// Завжди використовувати разом з `ILIKE $n ESCAPE '\'` в SQL.
pub fn ilike_pattern(q: &str) -> String {
    let escaped = q
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}
pub mod acts;
pub mod categories;
pub mod companies;
pub mod contracts;
pub mod counterparties;
pub mod dashboard;
pub mod document_templates;
pub mod invoices;
pub mod payments;
pub mod tasks;

#[cfg(test)]
mod tests {
    use super::{acts, categories, companies, contracts, counterparties, dashboard, document_templates, invoices, ilike_pattern, payments, tasks};

    #[test]
    fn db_submodules_are_available() {
        let _ = acts::list;
        let _ = categories::list;
        let _ = companies::list;
        let _ = contracts::list;
        let _ = counterparties::list;
        let _ = dashboard::get_kpi_summary;
        let _ = document_templates::list;
        let _ = invoices::list;
        let _ = payments::list;
        let _ = tasks::list_open;
    }

    #[test]
    fn ilike_pattern_wraps_plain_text_with_wildcards() {
        assert_eq!(ilike_pattern("тест"), "%тест%");
        assert_eq!(ilike_pattern("ФОП Іваненко"), "%ФОП Іваненко%");
    }

    #[test]
    fn ilike_pattern_escapes_percent() {
        assert_eq!(ilike_pattern("100%"), "%100\\%%");
        assert_eq!(ilike_pattern("%start"), "%\\%start%");
    }

    #[test]
    fn ilike_pattern_escapes_underscore() {
        assert_eq!(ilike_pattern("foo_bar"), "%foo\\_bar%");
        assert_eq!(ilike_pattern("_leading"), "%\\_leading%");
    }

    #[test]
    fn ilike_pattern_escapes_backslash() {
        assert_eq!(ilike_pattern("C:\\path"), "%C:\\\\path%");
        assert_eq!(ilike_pattern("a\\b\\c"), "%a\\\\b\\\\c%");
    }

    #[test]
    fn ilike_pattern_empty_input_returns_match_all() {
        assert_eq!(ilike_pattern(""), "%%");
    }
}
