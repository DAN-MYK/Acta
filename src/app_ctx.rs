// app_ctx.rs — канонічний контейнер спільного стану програми.
//
// Передається через Arc<AppCtx> у всі модулі UI wiring.
// Всі accessor'и безпечні при отруєному mutex.

use sqlx::PgPool;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// List state structs — клонуються для кожного refresh
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct CounterpartyListState {
    pub query: String,
    pub include_archived: bool,
}

#[derive(Clone, Default)]
pub struct TaskListState {
    pub query: String,
    pub filter: String, // "open" | "done" | "all"
}

// ---------------------------------------------------------------------------
// AppCtx — єдине джерело спільного стану в межах одного процесу
// ---------------------------------------------------------------------------

pub struct AppCtx {
    pub pool: PgPool,
    active_company_id: Arc<Mutex<uuid::Uuid>>,
    pub counterparty_state: Arc<Mutex<CounterpartyListState>>,
    pub task_state: Arc<Mutex<TaskListState>>,
}

impl AppCtx {
    /// Створює новий контекст з початковим UUID компанії.
    pub fn new(pool: PgPool, initial_company_id: uuid::Uuid) -> Self {
        Self {
            pool,
            active_company_id: Arc::new(Mutex::new(initial_company_id)),
            counterparty_state: Arc::new(Mutex::new(CounterpartyListState::default())),
            task_state: Arc::new(Mutex::new(TaskListState::default())),
        }
    }

    // --- Безпечний доступ до pool ---

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // --- Безпечний доступ до активної компанії ---

    /// Повертає поточний UUID. Nil = компанія не обрана.
    pub fn company_id(&self) -> uuid::Uuid {
        *self.active_company_id.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Повертає `Some(id)` якщо компанія обрана (не nil), `None` інакше.
    pub fn company_id_opt(&self) -> Option<uuid::Uuid> {
        let id = self.company_id();
        if id.is_nil() { None } else { Some(id) }
    }

    /// Встановлює активну компанію.
    pub fn set_company_id(&self, id: uuid::Uuid) {
        *self.active_company_id.lock().unwrap_or_else(|e| e.into_inner()) = id;
    }

    // --- Зручний клон для callbacks ---

    pub fn company_id_arc(&self) -> Arc<Mutex<uuid::Uuid>> {
        self.active_company_id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_ctx_pool() -> PgPool {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            sqlx::PgPool::connect_lazy("postgres://test:test@localhost/test").unwrap()
        })
    }

    #[test]
    fn company_id_is_initial_value() {
        let pool = make_ctx_pool();
        let id = Uuid::new_v4();
        let ctx = AppCtx::new(pool, id);
        assert_eq!(ctx.company_id(), id);
    }

    #[test]
    fn company_id_roundtrip_through_set() {
        let pool = make_ctx_pool();
        let ctx = AppCtx::new(pool, Uuid::nil());
        let new_id = Uuid::new_v4();
        ctx.set_company_id(new_id);
        assert_eq!(ctx.company_id(), new_id);
    }

    #[test]
    fn company_id_opt_nil_returns_none() {
        let pool = make_ctx_pool();
        let ctx = AppCtx::new(pool, Uuid::nil());
        assert!(ctx.company_id_opt().is_none());
    }

    #[test]
    fn company_id_opt_set_returns_some() {
        let pool = make_ctx_pool();
        let ctx = AppCtx::new(pool, Uuid::nil());
        let id = Uuid::new_v4();
        ctx.set_company_id(id);
        assert_eq!(ctx.company_id_opt(), Some(id));
    }

    #[test]
    fn pool_accessor_returns_reference() {
        let pool = make_ctx_pool();
        let ctx = AppCtx::new(pool.clone(), Uuid::nil());
        let _pool_ref: &PgPool = ctx.pool();
    }
}
