// Моделі актів виконаних робіт

use std::fmt;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Статус акту виконаних робіт.
///
/// `sqlx::Type` + `type_name = "act_status"` — зв'язує цей enum з PostgreSQL ENUM типом.
/// `rename_all = "lowercase"` — "Draft" в Rust → "draft" у БД.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "act_status", rename_all = "lowercase")]
pub enum ActStatus {
    Draft,
    Issued,
    Signed,
    Paid,
}

impl ActStatus {
    /// Наступний статус у циклі. `None` — якщо вже фінальний (Paid).
    pub fn next(&self) -> Option<ActStatus> {
        match self {
            ActStatus::Draft => Some(ActStatus::Issued),
            ActStatus::Issued => Some(ActStatus::Signed),
            ActStatus::Signed => Some(ActStatus::Paid),
            ActStatus::Paid => None,
        }
    }

    /// Назва статусу українською для відображення в UI.
    pub fn label(&self) -> &'static str {
        match self {
            ActStatus::Draft => "Чернетка",
            ActStatus::Issued => "Виставлено",
            ActStatus::Signed => "Підписано",
            ActStatus::Paid => "Оплачено",
        }
    }

    /// Рядкове представлення для передачі в SQL без явного cast.
    pub fn as_str(&self) -> &'static str {
        match self {
            ActStatus::Draft => "draft",
            ActStatus::Issued => "issued",
            ActStatus::Signed => "signed",
            ActStatus::Paid => "paid",
        }
    }
}

/// `Display` делегує до `label()` — виводить українську назву.
/// Дозволяє використовувати `format!("{}", status)`, `to_string()`,
/// а також `{}` у tracing-макросах без `.label()`.
impl fmt::Display for ActStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Акт виконаних робіт — головний документ.
///
/// `contract_id` — опціональний: акт може існувати без прив'язки до договору.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Act {
    pub id: Uuid,
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub date: NaiveDate,
    pub total_amount: Decimal,
    pub status: ActStatus,
    pub notes: Option<String>,
    pub bas_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Позиція акту — одна послуга або робота.
///
/// `amount` зберігається денормалізовано (= quantity × unit_price),
/// щоб уникнути перерахунку при кожному зчитуванні.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActItem {
    pub id: Uuid,
    pub act_id: Uuid,
    pub description: String,
    pub quantity: Decimal,
    pub unit: String,
    pub unit_price: Decimal,
    pub amount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Рядок акту для відображення в списку.
///
/// Окрема структура від `Act` — містить `counterparty_name` з JOIN,
/// але не містить усіх полів (оптимізація для списків).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ActListRow {
    pub id: Uuid,
    pub number: String,
    pub date: NaiveDate,
    pub counterparty_name: String,
    pub total_amount: Decimal,
    pub status: ActStatus,
}

/// Дані для створення нового акту разом з позиціями (одна транзакція).
pub struct NewAct {
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub date: NaiveDate,
    pub notes: Option<String>,
    pub bas_id: Option<String>,
    pub items: Vec<NewActItem>,
}

/// Дані для нової позиції акту. `amount` обчислюється в коді при вставці.
pub struct NewActItem {
    pub description: String,
    pub quantity: Decimal,
    pub unit: String,
    pub unit_price: Decimal,
}

/// Дані для оновлення заголовку акту (без позицій — MVP підхід).
pub struct UpdateAct {
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub date: NaiveDate,
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::ActStatus;

    #[test]
    fn act_status_next_moves_forward_only() {
        assert_eq!(ActStatus::Draft.next(), Some(ActStatus::Issued));
        assert_eq!(ActStatus::Issued.next(), Some(ActStatus::Signed));
        assert_eq!(ActStatus::Signed.next(), Some(ActStatus::Paid));
        assert_eq!(ActStatus::Paid.next(), None);
    }

    #[test]
    fn act_status_label_is_ukrainian_ui_text() {
        assert_eq!(ActStatus::Draft.label(), "Чернетка");
        assert_eq!(ActStatus::Issued.label(), "Виставлено");
        assert_eq!(ActStatus::Signed.label(), "Підписано");
        assert_eq!(ActStatus::Paid.label(), "Оплачено");
    }

    #[test]
    fn act_status_as_str_is_db_value() {
        assert_eq!(ActStatus::Draft.as_str(), "draft");
        assert_eq!(ActStatus::Issued.as_str(), "issued");
        assert_eq!(ActStatus::Signed.as_str(), "signed");
        assert_eq!(ActStatus::Paid.as_str(), "paid");
    }

    #[test]
    fn display_uses_label() {
        assert_eq!(ActStatus::Draft.to_string(), "Чернетка");
    }
}
