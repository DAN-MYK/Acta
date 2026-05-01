use std::sync::Arc;

use anyhow::Result;
use sqlx::PgPool;
use tokio::runtime::{Handle, Runtime};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::app_ctx::AppCtx;
use crate::config::AppConfig;

/// Повертає першу компанію або nil UUID, якщо компаній ще немає.
pub async fn get_first_company_id(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM companies ORDER BY created_at LIMIT 1")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(Uuid::nil)
}

/// Створює tokio runtime для desktop-застосунку.
pub fn build_runtime() -> Result<Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

/// Підключається до БД за `DATABASE_URL`.
pub async fn connect_pool() -> Result<PgPool> {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL не задано. Перевір .env файл.");

    Ok(sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?)
}

/// Застосовує всі міграції.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    tracing::info!("Міграції застосовано.");
    Ok(())
}

/// Ініціалізує спільний AppCtx для Slint/Tauri runtime.
pub async fn init_app_ctx() -> Result<Arc<AppCtx>> {
    let pool = connect_pool().await?;
    run_migrations(&pool).await?;
    let config = AppConfig::load();
    let company_id = match config.last_company_id {
        Some(company_id)
            if crate::db::companies::get_by_id(&pool, company_id)
                .await?
                .is_some() =>
        {
            company_id
        }
        _ => get_first_company_id(&pool).await,
    };
    Ok(Arc::new(AppCtx::new(pool, company_id)))
}

/// Ініціалізує AppCtx у вже створеному Runtime.
pub fn init_app_ctx_blocking(rt: &Runtime) -> Result<Arc<AppCtx>> {
    rt.block_on(init_app_ctx())
}

/// Запускає фонові сервіси застосунку.
pub fn spawn_background_tasks(ctx: &Arc<AppCtx>, handle: &Handle) -> JoinHandle<()> {
    let pool = Arc::new(ctx.pool().clone());
    handle.spawn(crate::notifications::reminder_loop(pool))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    use super::spawn_background_tasks;
    use crate::app_ctx::AppCtx;

    fn fake_ctx() -> Arc<AppCtx> {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://test:test@127.0.0.1:54321/nonexistent")
            .expect("connect_lazy не повинен відкривати з'єднання одразу");

        Arc::new(AppCtx::new(pool, Uuid::nil()))
    }

    #[test]
    fn spawn_background_tasks_uses_explicit_runtime_handle() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime повинен створитися для тесту");
        let ctx = {
            let _guard = rt.enter();
            fake_ctx()
        };

        let handle = spawn_background_tasks(&ctx, rt.handle());

        handle.abort();
    }
}
