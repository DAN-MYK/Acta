// Моделі платежів
//
// `Payment` — фактичний платіж (запис з банківської виписки або введений вручну).
// `PaymentSchedule` — запланований платіж (очікуваний).
// `PaymentAct` / `PaymentInvoice` — зв'язки платежів із документами.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

/// Напрямок платежу.
#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "payment_direction", rename_all = "lowercase")]
pub enum PaymentDirection {
    Income,
    Expense,
}

impl PaymentDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentDirection::Income => "income",
            PaymentDirection::Expense => "expense",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PaymentDirection::Income => "Надходження",
            PaymentDirection::Expense => "Витрата",
        }
    }
}

/// Повторюваність запланованого платежу.
#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "schedule_recurrence", rename_all = "lowercase")]
pub enum ScheduleRecurrence {
    None,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl ScheduleRecurrence {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleRecurrence::None => "none",
            ScheduleRecurrence::Weekly => "weekly",
            ScheduleRecurrence::Monthly => "monthly",
            ScheduleRecurrence::Quarterly => "quarterly",
            ScheduleRecurrence::Yearly => "yearly",
        }
    }
}

/// Фактичний платіж.
#[derive(Debug, Clone)]
pub struct Payment {
    pub id:              Uuid,
    pub company_id:      Uuid,
    pub date:            NaiveDate,
    pub amount:          Decimal,
    pub direction:       PaymentDirection,
    pub counterparty_id: Option<Uuid>,
    pub bank_name:       Option<String>,
    pub bank_ref:        Option<String>,
    pub description:     Option<String>,
    pub is_reconciled:   bool,
    pub bas_id:          Option<String>,
    pub created_at:      DateTime<Utc>,
    pub updated_at:      DateTime<Utc>,
}

/// Рядок списку платежів (з назвою контрагента).
#[derive(Debug, Clone)]
pub struct PaymentListRow {
    pub id:               Uuid,
    pub date:             String,       // ДД.ММ.РРРР
    pub amount:           Decimal,
    pub direction:        PaymentDirection,
    pub counterparty_id:  Option<Uuid>,
    pub counterparty_name: Option<String>,
    pub bank_name:        Option<String>,
    pub description:      Option<String>,
    pub is_reconciled:    bool,
}

/// Дані для створення платежу.
#[derive(Debug, Clone)]
pub struct NewPayment {
    pub company_id:      Uuid,
    pub date:            NaiveDate,
    pub amount:          Decimal,
    pub direction:       PaymentDirection,
    pub counterparty_id: Option<Uuid>,
    pub bank_name:       Option<String>,
    pub bank_ref:        Option<String>,
    pub description:     Option<String>,
}

/// Дані для оновлення платежу.
#[derive(Debug, Clone)]
pub struct UpdatePayment {
    pub date:            NaiveDate,
    pub amount:          Decimal,
    pub direction:       PaymentDirection,
    pub counterparty_id: Option<Uuid>,
    pub bank_name:       Option<String>,
    pub bank_ref:        Option<String>,
    pub description:     Option<String>,
}

/// Запланований платіж.
#[derive(Debug, Clone)]
pub struct PaymentSchedule {
    pub id:              Uuid,
    pub company_id:      Uuid,
    pub title:           String,
    pub amount:          Option<Decimal>,
    pub direction:       PaymentDirection,
    pub scheduled_date:  NaiveDate,
    pub recurrence:      ScheduleRecurrence,
    pub recurrence_end:  Option<NaiveDate>,
    pub counterparty_id: Option<Uuid>,
    pub notes:           Option<String>,
    pub is_completed:    bool,
    pub created_at:      DateTime<Utc>,
    pub updated_at:      DateTime<Utc>,
}

/// Дані для створення запланованого платежу.
#[derive(Debug, Clone)]
pub struct NewPaymentSchedule {
    pub company_id:      Uuid,
    pub title:           String,
    pub amount:          Option<Decimal>,
    pub direction:       PaymentDirection,
    pub scheduled_date:  NaiveDate,
    pub recurrence:      ScheduleRecurrence,
    pub recurrence_end:  Option<NaiveDate>,
    pub counterparty_id: Option<Uuid>,
    pub notes:           Option<String>,
}
