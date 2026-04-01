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
    pub edrpou: Option<String>,   // ЄДРПОУ може бути відсутнім (ФОП без реєстрації)
    pub iban: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub is_archived: bool,
    pub bas_id: Option<String>,   // ID з BAS — для уникнення дублів при імпорті
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
    pub iban: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
}
