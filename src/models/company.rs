// Модель компанії — юридична особа, від імені якої ведеться облік
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Компанія — власник документів (акти, контрагенти тощо).
/// Один екземпляр програми може вести облік кількох компаній.
#[derive(Debug, Clone, FromRow)]
pub struct Company {
    pub id:              Uuid,
    pub name:            String,
    pub short_name:      Option<String>,
    pub edrpou:          Option<String>,
    pub ipn:             Option<String>,
    pub iban:            Option<String>,
    pub legal_address:   Option<String>,
    pub actual_address:  Option<String>,
    pub phone:           Option<String>,
    pub email:           Option<String>,
    pub director_name:   Option<String>,
    pub accountant_name: Option<String>,
    pub tax_system:      Option<String>,
    pub is_vat_payer:    bool,
    pub logo_path:       Option<String>,
    pub notes:           Option<String>,
    pub is_archived:     bool,
    pub created_at:      DateTime<Utc>,
    pub updated_at:      DateTime<Utc>,
}

/// Дані для створення нової компанії.
#[derive(Debug, Clone)]
pub struct NewCompany {
    pub name:            String,
    pub short_name:      Option<String>,
    pub edrpou:          Option<String>,
    pub ipn:             Option<String>,
    pub iban:            Option<String>,
    pub legal_address:   Option<String>,
    pub director_name:   Option<String>,
    pub tax_system:      Option<String>,
    pub is_vat_payer:    bool,
}

/// Дані для оновлення існуючої компанії.
#[derive(Debug, Clone)]
pub struct UpdateCompany {
    pub name:            String,
    pub short_name:      Option<String>,
    pub edrpou:          Option<String>,
    pub iban:            Option<String>,
    pub legal_address:   Option<String>,
    pub director_name:   Option<String>,
    pub accountant_name: Option<String>,
    pub tax_system:      Option<String>,
    pub is_vat_payer:    bool,
    pub logo_path:       Option<String>,
}
