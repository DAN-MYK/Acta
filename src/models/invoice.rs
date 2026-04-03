// Моделі видаткових накладних

use std::fmt;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Статус видаткової накладної — ідентичний циклу актів.
///
/// `sqlx::Type` + `type_name = "invoice_status"` — зв'язує з PostgreSQL ENUM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "invoice_status", rename_all = "lowercase")]
pub enum InvoiceStatus {
    Draft,
    Issued,
    Signed,
    Paid,
}

impl InvoiceStatus {
    /// Наступний статус у циклі. `None` — якщо вже фінальний (Paid).
    pub fn next(&self) -> Option<InvoiceStatus> {
        match self {
            InvoiceStatus::Draft => Some(InvoiceStatus::Issued),
            InvoiceStatus::Issued => Some(InvoiceStatus::Signed),
            InvoiceStatus::Signed => Some(InvoiceStatus::Paid),
            InvoiceStatus::Paid => None,
        }
    }

    /// Назва статусу українською для відображення в UI.
    pub fn label(&self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "Чернетка",
            InvoiceStatus::Issued => "Виставлено",
            InvoiceStatus::Signed => "Підписано",
            InvoiceStatus::Paid => "Оплачено",
        }
    }

    /// Рядкове представлення для передачі в SQL без явного cast.
    pub fn as_str(&self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "draft",
            InvoiceStatus::Issued => "issued",
            InvoiceStatus::Signed => "signed",
            InvoiceStatus::Paid => "paid",
        }
    }
}

impl fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Видаткова накладна — документ передачі товарів або послуг.
///
/// `vat_amount` зберігається окремо — може бути 0 для ФОП без ПДВ.
/// `contract_id` опціональний — накладна може існувати без договору.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Invoice {
    pub id: Uuid,
    pub company_id: Uuid,
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub direction: String,
    pub date: NaiveDate,
    pub expected_payment_date: Option<NaiveDate>,
    pub total_amount: Decimal,
    pub vat_amount: Decimal,
    pub status: InvoiceStatus,
    pub notes: Option<String>,
    pub pdf_path: Option<String>,
    pub bas_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Позиція видаткової накладної — один товар або послуга.
///
/// `price` (не `unit_price` як в акті!) — уніфіковано зі схемою БД.
/// `position` визначає порядок відображення позицій у PDF.
/// `amount` денормалізовано = quantity × price.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InvoiceItem {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub position: i16,
    pub description: String,
    pub unit: Option<String>,
    pub quantity: Decimal,
    pub price: Decimal,
    pub amount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Рядок накладної для відображення в списку (JOIN з counterparties).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InvoiceListRow {
    pub id: Uuid,
    pub number: String,
    pub direction: String,
    pub date: NaiveDate,
    pub counterparty_name: String,
    pub total_amount: Decimal,
    pub status: InvoiceStatus,
}

/// Дані для створення нової накладної разом з позиціями.
pub struct NewInvoice {
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub direction: String,
    pub date: NaiveDate,
    pub expected_payment_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub bas_id: Option<String>,
    pub items: Vec<NewInvoiceItem>,
}

/// Дані для нової позиції накладної.
/// `amount` обчислюється в коді при вставці (quantity × price).
pub struct NewInvoiceItem {
    pub position: i16,
    pub description: String,
    pub unit: Option<String>,
    pub quantity: Decimal,
    pub price: Decimal,
}

/// Дані для оновлення заголовку накладної (позиції замінюються окремо).
pub struct UpdateInvoice {
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub date: NaiveDate,
    pub expected_payment_date: Option<NaiveDate>,
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::InvoiceStatus;

    #[test]
    fn invoice_status_next_moves_forward_only() {
        assert_eq!(InvoiceStatus::Draft.next(), Some(InvoiceStatus::Issued));
        assert_eq!(InvoiceStatus::Issued.next(), Some(InvoiceStatus::Signed));
        assert_eq!(InvoiceStatus::Signed.next(), Some(InvoiceStatus::Paid));
        assert_eq!(InvoiceStatus::Paid.next(), None);
    }

    #[test]
    fn invoice_status_label_is_ukrainian() {
        assert_eq!(InvoiceStatus::Draft.label(), "Чернетка");
        assert_eq!(InvoiceStatus::Paid.label(), "Оплачено");
    }

    #[test]
    fn invoice_status_as_str_matches_db_enum() {
        assert_eq!(InvoiceStatus::Draft.as_str(), "draft");
        assert_eq!(InvoiceStatus::Issued.as_str(), "issued");
        assert_eq!(InvoiceStatus::Signed.as_str(), "signed");
        assert_eq!(InvoiceStatus::Paid.as_str(), "paid");
    }

    #[test]
    fn display_uses_label() {
        assert_eq!(InvoiceStatus::Draft.to_string(), "Чернетка");
    }
}
