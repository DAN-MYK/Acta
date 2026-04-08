// app_ctx.rs — спільний стан програми, що передається між модулями UI.
//
// AppCtx містить пул БД, активну компанію та стани фільтрів/пошуку.
// Передається через Arc<AppCtx> у кожен setup-модуль.

use sqlx::PgPool;
use std::sync::{Arc, Mutex};

/// UUID дефолтної компанії (з міграції 012) — використовується якщо ще не обрано іншу.
pub const DEFAULT_COMPANY_ID: uuid::Uuid =
    uuid::Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);

#[derive(Clone, Default)]
pub struct CounterpartyListState {
    pub query: String,
    pub include_archived: bool,
    pub page: usize,
}

#[derive(Clone, Default)]
pub struct ActListState {
    pub query: String,
    pub status_filter: Option<acta::models::ActStatus>,
}

#[derive(Clone, Default)]
pub struct InvoiceListState {
    pub query: String,
    pub status_filter: Option<acta::models::InvoiceStatus>,
}

#[derive(Clone)]
pub struct DocListState {
    pub tab: i32,               // 0=Всі, 1=Акти, 2=Накладні
    pub direction: String,      // "outgoing" | "incoming"
    pub counterparty_index: i32, // 0 = всі контрагенти
    pub query: String,
    pub counterparty_id: Option<uuid::Uuid>, // None = всі контрагенти
    pub date_from: Option<chrono::NaiveDate>,
    pub date_to: Option<chrono::NaiveDate>,
}

impl Default for DocListState {
    fn default() -> Self {
        Self {
            tab: 0,
            direction: "outgoing".to_string(),
            counterparty_index: 0,
            query: String::new(),
            counterparty_id: None,
            date_from: None,
            date_to: None,
        }
    }
}

#[derive(Clone, Default)]
pub struct TaskListState {
    pub query: String,
}

#[derive(Clone, Default)]
pub struct PaymentListState {
    pub query: String,
    pub direction_filter: Option<acta::models::payment::PaymentDirection>,
}

/// Спільний контекст додатку — передається через Arc у всі модулі UI.
pub struct AppCtx {
    pub pool: PgPool,
    pub active_company_id: Arc<Mutex<uuid::Uuid>>,
    /// UUID-и контрагентів для фільтру в списку документів.
    /// Індекс 0 = "Всі контрагенти" (None), індекс n = cp_ids[n-1].
    pub doc_cp_ids: Arc<Mutex<Vec<uuid::Uuid>>>,
    // Стани списків — спільні між модулями (companies.rs потребує їх при перемиканні компанії)
    pub counterparty_state: Arc<Mutex<CounterpartyListState>>,
    pub act_state: Arc<Mutex<ActListState>>,
    pub invoice_state: Arc<Mutex<InvoiceListState>>,
    pub doc_state: Arc<Mutex<DocListState>>,
}

impl AppCtx {
    /// Читає UUID активної компанії. Безпечний при отруєному mutex.
    pub fn company_id(&self) -> uuid::Uuid {
        *self.active_company_id.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Встановлює UUID активної компанії. Безпечний при отруєному mutex.
    pub fn set_company_id(&self, id: uuid::Uuid) {
        *self.active_company_id.lock().unwrap_or_else(|e| e.into_inner()) = id;
    }
}
