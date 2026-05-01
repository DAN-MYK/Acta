pub mod commands;

use std::path::PathBuf;
use std::sync::Arc;

use acta::app_ctx::AppCtx;
use acta::runtime;
use tauri::Manager;

pub struct TauriState {
    pub ctx: Arc<AppCtx>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::try_init().ok();
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let template_dir = app
                .path()
                .resource_dir()
                .map(|p| p.join("templates"))
                .unwrap_or_else(|_| PathBuf::from("templates"));

            let storage_dir = app
                .path()
                .app_local_data_dir()
                .map(|p| p.join("documents"))
                .unwrap_or_else(|_| PathBuf::from("storage/documents"));

            let ctx = tauri::async_runtime::block_on(runtime::init_app_ctx_with_paths(
                template_dir,
                storage_dir,
            ))?;
            let runtime_handle = tauri::async_runtime::handle();
            let _ = runtime::spawn_background_tasks(&ctx, runtime_handle.inner());
            app.manage(TauriState { ctx });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::shell::shell_load,
            commands::shell::shell_set_active_company,
            commands::shell::shell_palette_search,
            commands::shell::shell_palette_activate,
            commands::dashboard::dashboard_load,
            commands::counterparties::counterparties_list,
            commands::counterparties::counterparty_get,
            commands::counterparties::counterparty_open_editor,
            commands::counterparties::counterparty_save,
            commands::counterparties::counterparty_archive,
            commands::counterparties::counterparty_create_document_context,
            commands::tasks::tasks_list,
            commands::tasks::task_open_editor,
            commands::tasks::task_save,
            commands::tasks::task_delete,
            commands::tasks::task_set_status,
            commands::reports::reports_load,
            commands::reports::reports_export_csv,
            commands::settings::settings_load,
            commands::settings::settings_save_preferences,
            commands::settings::settings_save_company,
            commands::settings::settings_configure_integration,
            commands::settings::settings_team_invite,
            commands::settings::settings_backup_now,
            commands::settings::settings_backup_open_latest,
            commands::documents::documents_list,
            commands::documents::document_open,
            commands::documents::document_create_draft,
            commands::documents::document_save,
            commands::documents::document_advance_status,
            commands::documents::document_delete,
            commands::documents::documents_bulk_advance_status,
            commands::documents::documents_bulk_delete,
            commands::documents::document_chain_get,
            commands::documents::document_chain_create_draft,
            commands::documents::document_generate_pdf,
            commands::payments::payments_list,
            commands::payments::payments_import_latest_csv,
            commands::payments::payments_sync_bank,
            commands::payments::payments_open_manual_template,
            commands::payments::payment_create_or_update,
            commands::payments::payment_reconcile,
            commands::payments::payment_unreconcile,
            commands::import::import_bas_pick_directory,
            commands::import::import_bas_plan,
            commands::import::import_bas_execute
        ])
        .run(tauri::generate_context!())
        .expect("не вдалося запустити Tauri runtime");
}
