use acta::tauri_api::import::{ImportPlanDto, ImportResultDto};
use tauri::State;

use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn import_bas_plan(state: State<'_, TauriState>) -> CommandResult<ImportPlanDto> {
    acta::tauri_api::import::import_bas_plan(&state.ctx)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_bas_execute(state: State<'_, TauriState>) -> CommandResult<ImportResultDto> {
    acta::tauri_api::import::import_bas_execute(&state.ctx)
        .await
        .map_err(|e| e.to_string())
}
