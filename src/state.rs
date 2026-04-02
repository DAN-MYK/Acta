use std::sync::{Arc, RwLock};

use uuid::Uuid;

use crate::models::company::Company;
use sqlx::PgPool;

/// Глобальний стан програми — клонується (Arc) в усі колбеки UI.
///
/// # Поля
/// - `pool` — пул з'єднань до PostgreSQL (вже є внутрішній Arc у sqlx)
/// - `active_company` — поточна компанія обліку (None до вибору при старті)
#[derive(Clone)]
pub struct AppState {
    pub pool:           PgPool,
    pub active_company: Arc<RwLock<Option<Company>>>,
}

impl AppState {
    /// Створити новий стан з пулом з'єднань.
    /// Активна компанія спочатку відсутня (None) — обирається при старті.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            active_company: Arc::new(RwLock::new(None)),
        }
    }

    /// Повертає UUID активної компанії або помилку якщо компанія не обрана.
    ///
    /// Використовується у всіх операціях що потребують `company_id`.
    ///
    /// ```ignore
    /// let cid = app_state.company_id()?;
    /// let acts = db::acts::list_filtered(&app_state.pool, cid, None, None).await?;
    /// ```
    pub fn company_id(&self) -> anyhow::Result<Uuid> {
        self.active_company
            .read()
            .unwrap()
            .as_ref()
            .map(|c| c.id)
            .ok_or_else(|| anyhow::anyhow!("Не обрано активну компанію"))
    }

    /// Встановити активну компанію (наприклад, після вибору в UI або завантаження з конфігу).
    pub fn set_active(&self, company: Company) {
        *self.active_company.write().unwrap() = Some(company);
    }

    /// Отримати копію активної компанії (або None якщо не обрано).
    pub fn active(&self) -> Option<Company> {
        self.active_company.read().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::postgres::PgPoolOptions;

    use super::*;

    fn lazy_pool() -> PgPool {
        PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/acta_test")
            .expect("lazy pool")
    }

    fn sample_company() -> Company {
        Company {
            id: Uuid::new_v4(),
            name: "ТОВ Тест".to_string(),
            short_name: Some("Тест".to_string()),
            edrpou: Some("12345678".to_string()),
            ipn: Some("1234567890".to_string()),
            iban: Some("UA123456789012345678901234567".to_string()),
            legal_address: Some("м. Київ".to_string()),
            actual_address: None,
            phone: None,
            email: None,
            director_name: Some("Директор".to_string()),
            accountant_name: None,
            tax_system: Some("general".to_string()),
            is_vat_payer: true,
            logo_path: None,
            notes: None,
            is_archived: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn new_state_has_no_active_company() {
        let state = AppState::new(lazy_pool());
        assert!(state.active().is_none());
    }

    #[tokio::test]
    async fn company_id_returns_error_when_company_is_missing() {
        let state = AppState::new(lazy_pool());
        let err = state.company_id().expect_err("company should be missing");
        assert!(err.to_string().contains("Не обрано активну компанію"));
    }

    #[tokio::test]
    async fn set_active_updates_current_company() {
        let state = AppState::new(lazy_pool());
        let company = sample_company();

        state.set_active(company.clone());

        assert_eq!(state.company_id().expect("company id"), company.id);
        assert_eq!(state.active().expect("active company").name, company.name);
    }
}

