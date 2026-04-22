// Acta - програма управлінського обліку.
//
// main.rs містить лише bootstrap: ініціалізацію runtime, БД, AppCtx та запуск UI.
// Уся orchestration-логіка винесена в окремі wire_* модулі.

slint::include_modules!();

mod bootstrap;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let _ = dotenvy::dotenv();

    let rt = bootstrap::build_runtime()?;
    let _guard = rt.enter();
    let ctx = bootstrap::init_app_ctx(&rt)?;
    bootstrap::spawn_background_tasks(&ctx);

    let ui = bootstrap::build_ui(&rt, &ctx)?;
    bootstrap::wire_app(&ui, &ctx);

    ui.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[test]
    fn tokio_runtime_multi_thread_builds_and_runs_async_tasks() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime повинен будуватися без помилок");

        let result = rt.block_on(async { 6u32 + 7 });
        assert_eq!(result, 13);

        let spawned = rt.block_on(async {
            tokio::spawn(async { "spawn_ok" })
                .await
                .expect("spawn не повинен панікувати")
        });
        assert_eq!(spawned, "spawn_ok");
    }

    #[test]
    fn tokio_runtime_join_runs_two_futures_in_parallel() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime");

        let (a, b) = rt.block_on(async { tokio::join!(async { 1u32 }, async { 2u32 }) });
        assert_eq!(a + b, 3);
    }

    #[test]
    fn active_company_id_starts_as_nil_and_updates_after_selection() {
        let company_id: Arc<Mutex<Uuid>> = Arc::new(Mutex::new(Uuid::nil()));
        assert!(company_id.lock().unwrap().is_nil());

        let selected = Uuid::new_v4();
        *company_id.lock().unwrap() = selected;

        assert_eq!(*company_id.lock().unwrap(), selected);
        assert!(!company_id.lock().unwrap().is_nil());
    }

    #[test]
    fn active_company_id_clones_share_the_same_mutex() {
        let id: Arc<Mutex<Uuid>> = Arc::new(Mutex::new(Uuid::nil()));
        let id_in_callback = Arc::clone(&id);

        let new_id = Uuid::new_v4();
        *id_in_callback.lock().unwrap() = new_id;

        assert_eq!(*id.lock().unwrap(), new_id);
    }
}
