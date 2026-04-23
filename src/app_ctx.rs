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
pub struct DocumentListState {
    pub query: String,
    pub tab: String, // "all" | "act" | "invoice" | "waybill"
}

#[derive(Clone, Default)]
pub struct ReportsState {
    pub period: i32,
    pub drill_category: String,
}

#[derive(Clone, Default)]
pub struct TaskListState {
    pub query: String,
    pub filter: String, // "open" | "done" | "all"
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppScreen {
    #[default]
    Dashboard,
    Documents,
    Counterparties,
    Payments,
    Reports,
    Tasks,
    Settings,
}

// ---------------------------------------------------------------------------
// AppCtx — єдине джерело спільного стану в межах одного процесу
// ---------------------------------------------------------------------------

pub struct AppCtx {
    pub pool: PgPool,
    active_company_id: Arc<Mutex<uuid::Uuid>>,
    active_screen: Arc<Mutex<AppScreen>>,
    documents_state: Arc<Mutex<DocumentListState>>,
    counterparty_state: Arc<Mutex<CounterpartyListState>>,
    reports_state: Arc<Mutex<ReportsState>>,
    task_state: Arc<Mutex<TaskListState>>,
}

impl AppCtx {
    /// Створює новий контекст з початковим UUID компанії.
    pub fn new(pool: PgPool, initial_company_id: uuid::Uuid) -> Self {
        Self {
            pool,
            active_company_id: Arc::new(Mutex::new(initial_company_id)),
            active_screen: Arc::new(Mutex::new(AppScreen::Dashboard)),
            documents_state: Arc::new(Mutex::new(DocumentListState {
                query: String::new(),
                tab: "all".to_string(),
            })),
            counterparty_state: Arc::new(Mutex::new(CounterpartyListState::default())),
            reports_state: Arc::new(Mutex::new(ReportsState {
                period: 1,
                drill_category: String::new(),
            })),
            task_state: Arc::new(Mutex::new(TaskListState {
                query: String::new(),
                filter: "open".to_string(),
            })),
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

    pub fn active_screen(&self) -> AppScreen {
        *self.active_screen.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_active_screen(&self, screen: AppScreen) {
        *self.active_screen.lock().unwrap_or_else(|e| e.into_inner()) = screen;
    }

    pub fn documents_state_snapshot(&self) -> DocumentListState {
        self.documents_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_documents_state(&self, state: DocumentListState) {
        *self.documents_state.lock().unwrap_or_else(|e| e.into_inner()) = state;
    }

    pub fn update_documents_state(
        &self,
        update: impl FnOnce(&mut DocumentListState),
    ) -> DocumentListState {
        let mut guard = self
            .documents_state
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        update(&mut guard);
        guard.clone()
    }

    pub fn counterparty_state_snapshot(&self) -> CounterpartyListState {
        self.counterparty_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_counterparty_state(&self, state: CounterpartyListState) {
        *self.counterparty_state.lock().unwrap_or_else(|e| e.into_inner()) = state;
    }

    pub fn update_counterparty_state(
        &self,
        update: impl FnOnce(&mut CounterpartyListState),
    ) -> CounterpartyListState {
        let mut guard = self
            .counterparty_state
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        update(&mut guard);
        guard.clone()
    }

    pub fn reports_state_snapshot(&self) -> ReportsState {
        self.reports_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_reports_state(&self, state: ReportsState) {
        *self.reports_state.lock().unwrap_or_else(|e| e.into_inner()) = state;
    }

    pub fn update_reports_state(&self, update: impl FnOnce(&mut ReportsState)) -> ReportsState {
        let mut guard = self.reports_state.lock().unwrap_or_else(|e| e.into_inner());
        update(&mut guard);
        guard.clone()
    }

    pub fn task_state_snapshot(&self) -> TaskListState {
        self.task_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_task_state(&self, state: TaskListState) {
        *self.task_state.lock().unwrap_or_else(|e| e.into_inner()) = state;
    }

    pub fn update_task_state(&self, update: impl FnOnce(&mut TaskListState)) -> TaskListState {
        let mut guard = self.task_state.lock().unwrap_or_else(|e| e.into_inner());
        update(&mut guard);
        guard.clone()
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

    #[test]
    fn active_screen_roundtrip_through_set() {
        let pool = make_ctx_pool();
        let ctx = AppCtx::new(pool, Uuid::nil());
        assert_eq!(ctx.active_screen(), AppScreen::Dashboard);
        ctx.set_active_screen(AppScreen::Reports);
        assert_eq!(ctx.active_screen(), AppScreen::Reports);
    }

    #[test]
    fn documents_state_snapshot_roundtrip() {
        let pool = make_ctx_pool();
        let ctx = AppCtx::new(pool, Uuid::nil());
        ctx.set_documents_state(DocumentListState {
            query: "акт".to_string(),
            tab: "act".to_string(),
        });

        let state = ctx.documents_state_snapshot();
        assert_eq!(state.query, "акт");
        assert_eq!(state.tab, "act");
    }

    #[test]
    fn reports_state_snapshot_roundtrip() {
        let pool = make_ctx_pool();
        let ctx = AppCtx::new(pool, Uuid::nil());
        ctx.set_reports_state(ReportsState {
            period: 2,
            drill_category: "Оренда".to_string(),
        });

        let state = ctx.reports_state_snapshot();
        assert_eq!(state.period, 2);
        assert_eq!(state.drill_category, "Оренда");
    }

    #[test]
    fn update_documents_state_mutates_in_place() {
        let pool = make_ctx_pool();
        let ctx = AppCtx::new(pool, Uuid::nil());

        let state = ctx.update_documents_state(|state| {
            state.query = "рахунок".to_string();
            state.tab = "invoice".to_string();
        });

        assert_eq!(state.query, "рахунок");
        assert_eq!(state.tab, "invoice");
        assert_eq!(ctx.documents_state_snapshot().query, "рахунок");
    }

    #[test]
    fn update_reports_state_preserves_existing_values() {
        let pool = make_ctx_pool();
        let ctx = AppCtx::new(pool, Uuid::nil());
        ctx.set_reports_state(ReportsState {
            period: 2,
            drill_category: String::new(),
        });

        let state = ctx.update_reports_state(|state| {
            state.drill_category = "Податки".to_string();
        });

        assert_eq!(state.period, 2);
        assert_eq!(state.drill_category, "Податки");
    }
}
