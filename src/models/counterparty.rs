// Модель контрагента
// Контрагент — це постачальник або покупець, з яким укладаються договори
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Контрагент — юридична або фізична особа-підприємець.
///
/// `sqlx::FromRow` — дозволяє sqlx автоматично перетворювати рядки БД у цю структуру.
/// Кожне поле відповідає стовпцю таблиці `counterparties`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Counterparty {
    pub id: Uuid,
    pub name: String,
    pub edrpou: Option<String>, // ЄДРПОУ може бути відсутнім (ФОП без реєстрації)
    pub ipn: Option<String>,    // ІПН / РНОКПП для ФОП або фізичних осіб
    pub iban: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub is_archived: bool,
    pub bas_id: Option<String>, // ID з BAS — для уникнення дублів при імпорті
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Дані для створення нового контрагента.
///
/// Окремий тип для вставки — щоб не передавати `id`, `created_at`, `updated_at`
/// (вони генеруються БД автоматично).
#[derive(Debug, Clone)]
pub struct NewCounterparty {
    pub name: String,
    pub edrpou: Option<String>,
    pub ipn: Option<String>,
    pub iban: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub bas_id: Option<String>,
}

/// Дані для оновлення контрагента.
///
/// `Option<Option<String>>` означає:
/// - `None` → поле не змінюємо
/// - `Some(None)` → очистити значення (записати NULL)
/// - `Some(Some("..."))` → записати нове значення
///
/// Для простоти MVP використовуємо плоску структуру — всі поля оновлюються разом.
#[derive(Debug, Clone)]
pub struct UpdateCounterparty {
    pub name: String,
    pub edrpou: Option<String>,
    pub ipn: Option<String>,
    pub iban: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{NewCounterparty, UpdateCounterparty};

    #[test]
    fn new_counterparty_can_hold_optional_fields() {
        let cp = NewCounterparty {
            name: "ТОВ Приклад".to_string(),
            edrpou: Some("12345678".to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: Some("+380501112233".to_string()),
            email: Some("mail@example.com".to_string()),
            notes: None,
            bas_id: Some("bas-42".to_string()),
        };

        assert_eq!(cp.name, "ТОВ Приклад");
        assert_eq!(cp.edrpou.as_deref(), Some("12345678"));
        assert_eq!(cp.bas_id.as_deref(), Some("bas-42"));
    }

    #[test]
    fn update_counterparty_clone_keeps_values() {
        let original = UpdateCounterparty {
            name: "ФОП Іваненко".to_string(),
            edrpou: None,
            ipn: Some("1234567890".to_string()),
            iban: Some("UA123".to_string()),
            address: Some("Київ".to_string()),
            phone: None,
            email: Some("fop@example.com".to_string()),
            notes: Some("оновити реквізити".to_string()),
        };

        let cloned = original.clone();
        assert_eq!(cloned.name, original.name);
        assert_eq!(cloned.ipn, original.ipn);
        assert_eq!(cloned.iban, original.iban);
        assert_eq!(cloned.notes, original.notes);
    }
}
