// Модель категорій доходів/витрат
//
// Використовується для класифікації актів, накладних та платежів.
// Підтримує ієрархію: parent_id вказує на батьківську категорію.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Запис категорії з БД.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Category {
    pub id:          Uuid,
    pub name:        String,
    /// "income" або "expense"
    pub kind:        String,
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
    pub kind:       String,
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
    pub kind:     String,
    pub depth:    u8,    // 0 — верхній рівень, 1 — підкатегорія
}
