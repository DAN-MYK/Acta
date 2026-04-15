// Модель договорів
//
// Договір між компанією та контрагентом.
// Прив'язується до актів та накладних через contract_id.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

/// Статус договору.
#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "contract_status", rename_all = "snake_case")]
pub enum ContractStatus {
    Active,
    Expired,
    Terminated,
}

impl ContractStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContractStatus::Active     => "active",
            ContractStatus::Expired    => "expired",
            ContractStatus::Terminated => "terminated",
        }
    }

    pub fn label_ua(&self) -> &'static str {
        match self {
            ContractStatus::Active     => "Активний",
            ContractStatus::Expired    => "Прострочений",
            ContractStatus::Terminated => "Розірваний",
        }
    }
}

/// Договір з БД (повний запис).
#[derive(Debug, Clone)]
pub struct Contract {
    pub id:              Uuid,
    pub company_id:      Uuid,
    pub counterparty_id: Uuid,
    pub number:          String,
    pub subject:         Option<String>,
    pub date:            NaiveDate,
    pub expires_at:      Option<NaiveDate>,
    pub amount:          Option<Decimal>,
    pub status:          ContractStatus,
    pub notes:           Option<String>,
    pub bas_id:          Option<String>,
    pub created_at:      DateTime<Utc>,
    pub updated_at:      DateTime<Utc>,
}

/// Договір для відображення у списку (з назвою контрагента).
#[derive(Debug, Clone)]
pub struct ContractListRow {
    pub id:               Uuid,
    pub number:           String,
    pub subject:          Option<String>,
    pub counterparty_id:  Uuid,
    pub counterparty_name: String,
    pub date:             String,    // "ДД.ММ.РРРР"
    pub expires_at:       Option<String>,
    pub amount:           Option<Decimal>,
    pub status:           ContractStatus,
}

/// Дані для створення нового договору.
#[derive(Debug, Clone)]
pub struct NewContract {
    pub company_id:      Uuid,
    pub counterparty_id: Uuid,
    pub number:          String,
    pub subject:         Option<String>,
    pub date:            NaiveDate,
    pub expires_at:      Option<NaiveDate>,
    pub amount:          Option<Decimal>,
}

/// Дані для оновлення договору.
#[derive(Debug, Clone)]
pub struct UpdateContract {
    pub number:     String,
    pub subject:    Option<String>,
    pub date:       NaiveDate,
    pub expires_at: Option<NaiveDate>,
    pub amount:     Option<Decimal>,
    pub status:     ContractStatus,
    pub notes:      Option<String>,
}

/// Спрощений запис для ComboBox у формах.
#[derive(Debug, Clone)]
pub struct ContractSelectItem {
    pub id:     Uuid,
    pub number: String,
    pub subject: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::ContractStatus;

    #[test]
    fn contract_status_as_str_matches_db_values() {
        assert_eq!(ContractStatus::Active.as_str(),     "active");
        assert_eq!(ContractStatus::Expired.as_str(),    "expired");
        assert_eq!(ContractStatus::Terminated.as_str(), "terminated");
    }

    #[test]
    fn contract_status_label_ua_is_ukrainian() {
        assert_eq!(ContractStatus::Active.label_ua(),     "Активний");
        assert_eq!(ContractStatus::Expired.label_ua(),    "Прострочений");
        assert_eq!(ContractStatus::Terminated.label_ua(), "Розірваний");
    }

    #[test]
    fn contract_status_as_str_and_label_ua_cover_all_variants() {
        // Якщо додадуть новий варіант — компілятор вимагатиме оновити match,
        // а цей тест нагадає дописати перевірку.
        let variants = [
            ContractStatus::Active,
            ContractStatus::Expired,
            ContractStatus::Terminated,
        ];
        for v in &variants {
            assert!(!v.as_str().is_empty(),   "{v:?}: as_str() не має бути порожнім");
            assert!(!v.label_ua().is_empty(), "{v:?}: label_ua() не має бути порожнім");
        }
    }

    #[test]
    fn contract_status_as_str_is_lowercase_ascii() {
        for v in [ContractStatus::Active, ContractStatus::Expired, ContractStatus::Terminated] {
            let s = v.as_str();
            assert!(s.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{v:?}: as_str() повинен бути snake_case ASCII, отримано '{s}'");
        }
    }
}
