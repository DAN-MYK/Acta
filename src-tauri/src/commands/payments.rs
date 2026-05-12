use acta::tauri_api::payments::{
    MutationResultDto, OpenTemplateResultDto, PaymentCreateOrUpdateRequest,
    PaymentReconcileRequest, PaymentUnreconcileRequest, PaymentsScreenDto,
};
use tauri::State;

use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn payments_list(state: State<'_, TauriState>) -> CommandResult<PaymentsScreenDto> {
    acta::tauri_api::payments::payments_list(&state.ctx)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn payments_import_latest_csv(
    state: State<'_, TauriState>,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::payments::payments_import_latest_csv(&state.ctx)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn payments_sync_bank(
    state: State<'_, TauriState>,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::payments::payments_sync_bank(&state.ctx)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn payments_open_manual_template(
    state: State<'_, TauriState>,
) -> CommandResult<OpenTemplateResultDto> {
    acta::tauri_api::payments::payments_open_manual_template(&state.ctx)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn payment_create_or_update(
    state: State<'_, TauriState>,
    request: PaymentCreateOrUpdateRequest,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::payments::payment_create_or_update(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn payment_reconcile(
    state: State<'_, TauriState>,
    request: PaymentReconcileRequest,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::payments::payment_reconcile(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn payment_unreconcile(
    state: State<'_, TauriState>,
    request: PaymentUnreconcileRequest,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::payments::payment_unreconcile(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}
