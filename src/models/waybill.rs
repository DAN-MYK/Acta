// Моделі видаткових накладних (товарних накладних)

use std::fmt;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::DocumentDirection;

/// Статус видаткової накладної.
///
/// `sqlx::Type` + `type_name = "waybill_status"` — зв'язує з PostgreSQL ENUM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "waybill_status", rename_all = "lowercase")]
pub enum WaybillStatus {
    Draft,
    Issued,
    Signed,
    Delivered,
}

impl WaybillStatus {
    /// Чи дозволено перейти з поточного статусу до `next`.
    pub fn can_transition_to(&self, next: &WaybillStatus) -> bool {
        match (self, next) {
            (WaybillStatus::Draft, WaybillStatus::Issued) => true,
            (WaybillStatus::Issued, WaybillStatus::Signed) => true,
            (WaybillStatus::Signed, WaybillStatus::Delivered) => true,
            _ => false,
        }
    }

    /// Наступний статус у циклі. `None` — якщо вже фінальний (Delivered).
    pub fn next(&self) -> Option<WaybillStatus> {
        match self {
            WaybillStatus::Draft => Some(WaybillStatus::Issued),
            WaybillStatus::Issued => Some(WaybillStatus::Signed),
            WaybillStatus::Signed => Some(WaybillStatus::Delivered),
            WaybillStatus::Delivered => None,
        }
    }

    /// Назва статусу українською для відображення в UI.
    pub fn label(&self) -> &'static str {
        match self {
            WaybillStatus::Draft => "Чернетка",
            WaybillStatus::Issued => "Виставлена",
            WaybillStatus::Signed => "Підписана",
            WaybillStatus::Delivered => "Доставлено",
        }
    }

    /// Рядкове представлення для передачі в SQL без явного cast.
    pub fn as_str(&self) -> &'static str {
        match self {
            WaybillStatus::Draft => "draft",
            WaybillStatus::Issued => "issued",
            WaybillStatus::Signed => "signed",
            WaybillStatus::Delivered => "delivered",
        }
    }
}

impl fmt::Display for WaybillStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Видаткова накладна — документ передачі товарів.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Waybill {
    pub id: Uuid,
    pub company_id: Uuid,
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub direction: DocumentDirection,
    pub date: NaiveDate,
    pub total_amount: Decimal,
    pub status: WaybillStatus,
    pub notes: Option<String>,
    pub pdf_path: Option<String>,
    pub bas_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Позиція видаткової накладної — один товар.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WaybillItem {
    pub id: Uuid,
    pub waybill_id: Uuid,
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
pub struct WaybillListRow {
    pub id: Uuid,
    pub number: String,
    pub direction: DocumentDirection,
    pub date: NaiveDate,
    pub counterparty_name: String,
    pub total_amount: Decimal,
    pub status: WaybillStatus,
}

/// Дані для створення нової накладної разом з позиціями.
pub struct NewWaybill {
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub direction: DocumentDirection,
    pub date: NaiveDate,
    pub notes: Option<String>,
    pub bas_id: Option<String>,
    pub items: Vec<NewWaybillItem>,
}

/// Дані для нової позиції накладної.
pub struct NewWaybillItem {
    pub position: i16,
    pub description: String,
    pub unit: Option<String>,
    pub quantity: Decimal,
    pub price: Decimal,
}

/// Дані для оновлення заголовку накладної (позиції замінюються окремо).
pub struct UpdateWaybill {
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub date: NaiveDate,
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::WaybillStatus;

    #[test]
    fn waybill_status_next_moves_forward_only() {
        assert_eq!(WaybillStatus::Draft.next(), Some(WaybillStatus::Issued));
        assert_eq!(WaybillStatus::Issued.next(), Some(WaybillStatus::Signed));
        assert_eq!(WaybillStatus::Signed.next(), Some(WaybillStatus::Delivered));
        assert_eq!(WaybillStatus::Delivered.next(), None);
    }

    #[test]
    fn waybill_status_can_transition_to_allows_only_adjacent_forward_step() {
        assert!(WaybillStatus::Draft.can_transition_to(&WaybillStatus::Issued));
        assert!(WaybillStatus::Issued.can_transition_to(&WaybillStatus::Signed));
        assert!(WaybillStatus::Signed.can_transition_to(&WaybillStatus::Delivered));
    }

    #[test]
    fn waybill_status_can_transition_to_rejects_skips_backwards_and_same_state() {
        assert!(!WaybillStatus::Draft.can_transition_to(&WaybillStatus::Draft));
        assert!(!WaybillStatus::Draft.can_transition_to(&WaybillStatus::Signed));
        assert!(!WaybillStatus::Issued.can_transition_to(&WaybillStatus::Draft));
        assert!(!WaybillStatus::Signed.can_transition_to(&WaybillStatus::Issued));
    }

    #[test]
    fn waybill_status_delivered_is_terminal_state() {
        for next in [
            WaybillStatus::Draft,
            WaybillStatus::Issued,
            WaybillStatus::Signed,
            WaybillStatus::Delivered,
        ] {
            assert!(!WaybillStatus::Delivered.can_transition_to(&next));
        }
    }

    #[test]
    fn waybill_status_label_is_ukrainian() {
        assert_eq!(WaybillStatus::Draft.label(), "Чернетка");
        assert_eq!(WaybillStatus::Delivered.label(), "Доставлено");
    }

    #[test]
    fn waybill_status_as_str_matches_db_enum() {
        assert_eq!(WaybillStatus::Draft.as_str(), "draft");
        assert_eq!(WaybillStatus::Issued.as_str(), "issued");
        assert_eq!(WaybillStatus::Signed.as_str(), "signed");
        assert_eq!(WaybillStatus::Delivered.as_str(), "delivered");
    }

    #[test]
    fn display_uses_label() {
        assert_eq!(WaybillStatus::Draft.to_string(), "Чернетка");
        assert_eq!(WaybillStatus::Delivered.to_string(), "Доставлено");
    }
}
