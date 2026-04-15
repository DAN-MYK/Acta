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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_template(template_type: &str) -> DocumentTemplate {
        DocumentTemplate {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            name: "Тест".into(),
            description: None,
            template_type: template_type.into(),
            template_path: "templates/test.typ".into(),
            is_default: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn act_повертає_акт() {
        assert_eq!(make_template("act").template_type_label(), "Акт");
    }

    #[test]
    fn invoice_повертає_накладна() {
        assert_eq!(make_template("invoice").template_type_label(), "Накладна");
    }

    #[test]
    fn невідомий_тип_повертає_невідомо() {
        assert_eq!(make_template("contract").template_type_label(), "Невідомо");
        assert_eq!(make_template("").template_type_label(), "Невідомо");
        assert_eq!(make_template("ACT").template_type_label(), "Невідомо"); // case-sensitive
    }
}
