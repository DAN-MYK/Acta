use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Шаблон документа — зберігає metadata про .typ файл.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DocumentTemplate {
    pub id: Uuid,
    pub company_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub template_type: String,
    pub template_path: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Для створення нового шаблону.
#[derive(Debug)]
pub struct NewDocumentTemplate {
    pub name: String,
    pub description: Option<String>,
    pub template_type: String,
    pub template_path: String,
    pub is_default: bool,
}

/// Для оновлення існуючого шаблону.
#[derive(Debug)]
pub struct UpdateDocumentTemplate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub template_path: Option<String>,
    pub is_default: Option<bool>,
}

/// Скорочений рядок для відображення в списку.
#[derive(Debug, Clone, FromRow)]
pub struct TemplateListRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub template_type: String,
    pub is_default: bool,
}

impl DocumentTemplate {
    pub fn template_type_label(&self) -> &'static str {
        match self.template_type.as_str() {
            "act" => "Акт",
            "invoice" => "Накладна",
            _ => "Невідомо",
        }
    }
}
