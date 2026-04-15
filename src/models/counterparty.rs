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

/// Перевіряє що ЄДРПОУ коректний: рівно 8 цифр.
pub fn is_valid_edrpou(s: &str) -> bool {
    s.len() == 8 && s.chars().all(|c| c.is_ascii_digit())
}

/// Перевіряє що ІПН/РНОКПП коректний: рівно 10 цифр.
pub fn is_valid_ipn(s: &str) -> bool {
    s.len() == 10 && s.chars().all(|c| c.is_ascii_digit())
}

/// Перевіряє що IBAN коректний для України: "UA" + рівно 27 цифр (29 символів разом).
pub fn is_valid_iban(s: &str) -> bool {
    s.len() == 29 && s.starts_with("UA") && s[2..].chars().all(|c| c.is_ascii_digit())
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
    use super::{is_valid_edrpou, is_valid_iban, is_valid_ipn};

    #[test]
    fn edrpou_valid() {
        assert!(is_valid_edrpou("12345678"));
        assert!(is_valid_edrpou("00000000"));
    }

    #[test]
    fn edrpou_invalid() {
        assert!(!is_valid_edrpou("1234567"));   // 7 цифр
        assert!(!is_valid_edrpou("123456789")); // 9 цифр
        assert!(!is_valid_edrpou("1234567a")); // буква
        assert!(!is_valid_edrpou(""));
    }

    #[test]
    fn ipn_valid() {
        assert!(is_valid_ipn("1234567890"));
        assert!(is_valid_ipn("0000000000"));
    }

    #[test]
    fn ipn_invalid() {
        assert!(!is_valid_ipn("123456789"));   // 9 цифр
        assert!(!is_valid_ipn("12345678901")); // 11 цифр
        assert!(!is_valid_ipn("123456789x")); // буква
        assert!(!is_valid_ipn(""));
    }

    #[test]
    fn iban_valid() {
        // UA + 27 цифр = 29 символів
        assert!(is_valid_iban("UA123456789012345678901234567"));
        assert!(is_valid_iban("UA000000000000000000000000000"));
    }

    #[test]
    fn iban_invalid() {
        assert!(!is_valid_iban("UA12345"));              // занадто короткий
        assert!(!is_valid_iban("DE123456789012345678901234567")); // не UA
        assert!(!is_valid_iban("UA12345678901234567890123456a")); // буква
        assert!(!is_valid_iban("UA123")); // 5 символів
        assert!(!is_valid_iban(""));
    }
}
