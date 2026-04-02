// Центральний стан застосунку — активна компанія + пул з'єднань
//
// AppState клонується (завдяки Arc) у всі колбеки UI без копіювання даних.
// RwLock дозволяє читати active_company з кількох місць одночасно,
// але запис (зміна активної компанії) є ексклюзивним.

use std::sync::{Arc, RwLock};
use uuid::Uuid;
use sqlx::PgPool;
use crate::models::company::Company;

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
    /// Використовується у всіх операціях що потребують company_id:
    /// ```rust
    /// let cid = state.company_id()?;
    /// db::acts::list_filtered(&state.pool, cid, None, None).await?;
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
