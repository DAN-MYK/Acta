// Моделі даних — Rust структури, що відповідають таблицям БД
pub mod act;
pub mod category;
pub mod company;
pub mod contract;
pub mod counterparty;
pub mod dashboard;
pub mod document_template;
pub mod invoice;
pub mod payment;
pub mod reports;
pub mod shared;
pub mod task;
pub mod adjustment_act;
pub mod waybill;

#[allow(unused_imports)]
pub use act::{Act, ActItem, ActListRow, ActStatus, NewAct, NewActItem, UpdateAct};
#[allow(unused_imports)]
pub use adjustment_act::{
    AdjustmentAct, AdjustmentActItem, AdjustmentActListRow, AdjustmentActStatus,
    NewAdjustmentActItem, UpdateAdjustmentAct,
};
#[allow(unused_imports)]
pub use category::{Category, CategoryKind, CategorySelectItem, NewCategory, UpdateCategory};
#[allow(unused_imports)]
pub use company::{Company, CompanySummary, NewCompany, UpdateCompany};
#[allow(unused_imports)]
pub use contract::{
    Contract, ContractListRow, ContractSelectItem, ContractStatus, NewContract, UpdateContract,
};
pub use counterparty::{Counterparty, NewCounterparty, UpdateCounterparty};
#[allow(unused_imports)]
pub use document_template::{
    DocumentTemplate, NewDocumentTemplate, TemplateListRow, UpdateDocumentTemplate,
};
#[allow(unused_imports)]
pub use invoice::{
    Invoice, InvoiceItem, InvoiceListRow, InvoiceStatus, NewInvoice, NewInvoiceItem, UpdateInvoice,
};
#[allow(unused_imports)]
pub use reports::{
    BankAggregateRow, PayableRow, PnlCategoryRow, ReceivableRow, ReportsScope,
    ResolvedReportsFilter,
};
#[allow(unused_imports)]
pub use shared::DocumentDirection;
#[allow(unused_imports)]
pub use task::{NewTask, Task, TaskPriority, TaskStatus};
#[allow(unused_imports)]
pub use waybill::{
    NewWaybill, NewWaybillItem, UpdateWaybill, Waybill, WaybillItem, WaybillListRow, WaybillStatus,
};

#[cfg(test)]
mod tests {
    use super::{
        ActStatus, AdjustmentActStatus, BankAggregateRow, CategoryKind, DocumentDirection,
        NewCounterparty, ReportsScope, ResolvedReportsFilter, TaskPriority, TaskStatus,
    };
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn reexports_are_available_for_consumers() {
        let status = ActStatus::Draft;
        assert_eq!(status.as_str(), "draft");
        assert_eq!(AdjustmentActStatus::Applied.as_str(), "applied");

        assert_eq!(TaskStatus::Open.as_str(), "open");
        assert_eq!(TaskPriority::Critical.as_str(), "critical");

        let cp = NewCounterparty {
            name: "ТОВ Реекспорт".to_string(),
            edrpou: None,
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: None,
        };
        assert_eq!(cp.name, "ТОВ Реекспорт");
    }

    #[test]
    fn category_kind_and_direction_are_reexported() {
        assert_eq!(CategoryKind::Income.as_str(), "income");
        assert_eq!(CategoryKind::Expense.as_str(), "expense");
        assert_eq!(DocumentDirection::Outgoing.as_str(), "outgoing");
    }

    #[test]
    fn reports_models_are_reexported() {
        let filter = ResolvedReportsFilter {
            scope: ReportsScope::Active,
            date_from: NaiveDate::from_ymd_opt(2026, 5, 1).expect("valid date"),
            date_to: NaiveDate::from_ymd_opt(2026, 5, 31).expect("valid date"),
            query: "ромашка".to_string(),
            selected_counterparty_id: None,
        };

        let row = BankAggregateRow {
            key: "ops".to_string(),
            label: "Операційна діяльність".to_string(),
            income: Decimal::new(100_00, 2),
            expense: Decimal::new(40_00, 2),
        };

        assert!(matches!(filter.scope, ReportsScope::Active));
        assert_eq!(filter.query, "ромашка");
        assert_eq!(row.label, "Операційна діяльність");
    }
}
