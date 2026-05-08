use acta::tauri_api::reports::{
    ReportsExportRequest, ReportsExportResultDto, ReportsLoadRequest, ReportsScreenDto,
};
use tauri::State;

use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn reports_load(
    state: State<'_, TauriState>,
    request: ReportsLoadRequest,
) -> CommandResult<ReportsScreenDto> {
    acta::tauri_api::reports::reports_load(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn reports_export_csv(
    state: State<'_, TauriState>,
    request: ReportsExportRequest,
) -> CommandResult<ReportsExportResultDto> {
    acta::tauri_api::reports::reports_export_csv(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn reports_export_excel(
    state: State<'_, TauriState>,
    request: ReportsExportRequest,
) -> CommandResult<ReportsExportResultDto> {
    acta::tauri_api::reports::reports_export_excel(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn reports_export_excel_and_open(
    state: State<'_, TauriState>,
    request: ReportsExportRequest,
) -> CommandResult<ReportsExportResultDto> {
    acta::tauri_api::reports::reports_export_excel_and_open(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}
