use acta::tauri_api::settings::{
    SettingsActionResultDto, SettingsIntegrationActionRequest, SettingsPreferencesRequest,
    SettingsSaveCompanyRequest, SettingsScreenDto, SettingsScreenMutationResultDto,
};
use tauri::State;

use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn settings_load(state: State<'_, TauriState>) -> CommandResult<SettingsScreenDto> {
    acta::tauri_api::settings::settings_load(&state.ctx)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn settings_save_preferences(
    state: State<'_, TauriState>,
    request: SettingsPreferencesRequest,
) -> CommandResult<SettingsScreenMutationResultDto> {
    acta::tauri_api::settings::settings_save_preferences(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn settings_save_company(
    state: State<'_, TauriState>,
    request: SettingsSaveCompanyRequest,
) -> CommandResult<SettingsScreenMutationResultDto> {
    acta::tauri_api::settings::settings_save_company(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn settings_configure_integration(
    state: State<'_, TauriState>,
    request: SettingsIntegrationActionRequest,
) -> CommandResult<SettingsScreenMutationResultDto> {
    acta::tauri_api::settings::settings_configure_integration(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn settings_team_invite(
    state: State<'_, TauriState>,
) -> CommandResult<SettingsScreenMutationResultDto> {
    acta::tauri_api::settings::settings_team_invite(&state.ctx)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn settings_backup_now(
    state: State<'_, TauriState>,
) -> CommandResult<SettingsScreenMutationResultDto> {
    acta::tauri_api::settings::settings_backup_now(&state.ctx)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn settings_backup_open_latest(
    state: State<'_, TauriState>,
) -> CommandResult<SettingsActionResultDto> {
    acta::tauri_api::settings::settings_backup_open_latest(&state.ctx)
        .await
        .map_err(|error| error.to_string())
}
