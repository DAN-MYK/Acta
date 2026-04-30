use acta::tauri_api::dashboard::DashboardScreenDto;
use tauri::State;

use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn dashboard_load(state: State<'_, TauriState>) -> CommandResult<DashboardScreenDto> {
    acta::tauri_api::dashboard::dashboard_load(&state.ctx)
        .await
        .map_err(|error| error.to_string())
}
