// Модуль роботи з базою даних
// Кожен файл — CRUD для однієї таблиці
pub mod acts;
pub mod companies;
pub mod counterparties;
pub mod tasks;

#[cfg(test)]
mod tests {
    use super::{acts, companies, counterparties, tasks};

    #[test]
    fn db_submodules_are_available() {
        let _ = acts::list;
        let _ = companies::list;
        let _ = counterparties::list;
        let _ = tasks::list_open;
    }
}
