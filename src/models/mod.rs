// Моделі даних — Rust структури, що відповідають таблицям БД
pub mod act;
pub mod company;
pub mod counterparty;
pub mod task;

#[allow(unused_imports)]
pub use act::{Act, ActItem, ActListRow, ActStatus, NewAct, NewActItem, UpdateAct};
#[allow(unused_imports)]
pub use company::{Company, NewCompany, UpdateCompany};
pub use counterparty::{Counterparty, NewCounterparty, UpdateCounterparty};
#[allow(unused_imports)]
pub use task::{NewTask, Task, TaskPriority, TaskStatus};

#[cfg(test)]
mod tests {
    use super::{ActStatus, NewCounterparty, TaskPriority, TaskStatus};

    #[test]
    fn reexports_are_available_for_consumers() {
        let status = ActStatus::Draft;
        assert_eq!(status.as_str(), "draft");

        assert_eq!(TaskStatus::Open.as_str(), "open");
        assert_eq!(TaskPriority::Critical.as_str(), "critical");

        let cp = NewCounterparty {
            name: "ТОВ Реекспорт".to_string(),
            edrpou: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: None,
        };
        assert_eq!(cp.name, "ТОВ Реекспорт");
    }
}
