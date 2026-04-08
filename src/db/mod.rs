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
pub mod invoices;
pub mod payments;
pub mod tasks;

#[cfg(test)]
mod tests {
    use super::{acts, categories, companies, contracts, counterparties, invoices, payments, tasks};

    #[test]
    fn db_submodules_are_available() {
        let _ = acts::list;
        let _ = categories::list;
        let _ = companies::list;
        let _ = contracts::list;
        let _ = counterparties::list;
        let _ = invoices::list;
        let _ = payments::list;
        let _ = tasks::list_open;
    }
}
