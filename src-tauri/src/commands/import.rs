use acta::tauri_api::import::{ImportPlanDto, ImportResultDto};
use serde::Deserialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBasRequest {
    pub input_dir: Option<String>,
}

#[tauri::command]
pub async fn import_bas_plan(
    state: State<'_, TauriState>,
    request: Option<ImportBasRequest>,
) -> CommandResult<ImportPlanDto> {
    acta::tauri_api::import::import_bas_plan(
        &state.ctx,
        request.as_ref().and_then(|payload| payload.input_dir.as_deref()),
    )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_bas_execute(
    state: State<'_, TauriState>,
    request: Option<ImportBasRequest>,
) -> CommandResult<ImportResultDto> {
    acta::tauri_api::import::import_bas_execute(
        &state.ctx,
        request.as_ref().and_then(|payload| payload.input_dir.as_deref()),
    )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_bas_pick_directory(app: AppHandle) -> CommandResult<Option<String>> {
    let handle = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .map(|path| path.to_string())
    });

    handle.await.map_err(|e| e.to_string())
}
