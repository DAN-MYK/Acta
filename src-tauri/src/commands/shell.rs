use acta::tauri_api::shell::{
    PaletteActivationResultDto, PaletteSearchRequestDto, PaletteSearchResultDto, ShellStateDto,
};
use tauri::State;

use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn shell_load(state: State<'_, TauriState>) -> CommandResult<ShellStateDto> {
    acta::tauri_api::shell::shell_load(&state.ctx)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn shell_set_active_company(
    state: State<'_, TauriState>,
    company_id: String,
) -> CommandResult<ShellStateDto> {
    acta::tauri_api::shell::shell_set_active_company(&state.ctx, company_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn shell_palette_search(
    state: State<'_, TauriState>,
    request: PaletteSearchRequestDto,
) -> CommandResult<PaletteSearchResultDto> {
    acta::tauri_api::shell::shell_palette_search(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn shell_palette_activate(
    state: State<'_, TauriState>,
    payload: String,
    selected_counterparty_id: Option<String>,
) -> CommandResult<PaletteActivationResultDto> {
    acta::tauri_api::shell::shell_palette_activate(&state.ctx, payload, selected_counterparty_id)
        .await
        .map_err(|error| error.to_string())
}
