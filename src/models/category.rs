// Модель категорій доходів/витрат
//
// Використовується для класифікації актів, накладних та платежів.
// Підтримує ієрархію: parent_id вказує на батьківську категорію.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Тип категорії: дохід або видаток.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum CategoryKind {
    #[sqlx(rename = "income")]
    Income,
    #[sqlx(rename = "expense")]
    Expense,
}

impl CategoryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Income => "Дохід",
            Self::Expense => "Видаток",
        }
    }
}

impl std::fmt::Display for CategoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl TryFrom<String> for CategoryKind {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "income" => Ok(Self::Income),
            "expense" => Ok(Self::Expense),
            _ => Err(format!("Unknown category kind: {}", s)),
        }
    }
}

/// Запис категорії з БД.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Category {
    pub id:          Uuid,
    pub name:        String,
    /// Тип: дохід або видаток.
    pub kind:        CategoryKind,
    pub parent_id:   Option<Uuid>,
    pub company_id:  Uuid,
    pub is_archived: bool,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

/// Дані для створення нової категорії.
#[derive(Debug, Clone)]
pub struct NewCategory {
    pub name:       String,
    pub kind:       CategoryKind,
    pub parent_id:  Option<Uuid>,
    pub company_id: Uuid,
}

/// Дані для оновлення категорії.
#[derive(Debug, Clone)]
pub struct UpdateCategory {
    pub name:      String,
    pub parent_id: Option<Uuid>,
}

/// Спрощений запис для ComboBox у формах.
#[derive(Debug, Clone)]
pub struct CategorySelectItem {
    pub id:       Uuid,
    pub name:     String,
    pub kind:     CategoryKind,
    pub depth:    u8,    // 0 — верхній рівень, 1 — підкатегорія
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_kind_as_str() {
        assert_eq!(CategoryKind::Income.as_str(), "income");
        assert_eq!(CategoryKind::Expense.as_str(), "expense");
    }

    #[test]
    fn test_category_kind_label() {
        assert_eq!(CategoryKind::Income.label(), "Дохід");
        assert_eq!(CategoryKind::Expense.label(), "Видаток");
    }

    #[test]
    fn test_category_kind_try_from() {
        assert_eq!(
            CategoryKind::try_from("income".to_string()),
            Ok(CategoryKind::Income)
        );
        assert_eq!(
            CategoryKind::try_from("expense".to_string()),
            Ok(CategoryKind::Expense)
        );
    }

    #[test]
    fn test_category_kind_display() {
        assert_eq!(CategoryKind::Income.to_string(), "Дохід");
        assert_eq!(CategoryKind::Expense.to_string(), "Видаток");
    }

    #[test]
    fn test_category_kind_try_from_invalid() {
        assert!(CategoryKind::try_from("invalid".to_string()).is_err());
    }
}
