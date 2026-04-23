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
}
