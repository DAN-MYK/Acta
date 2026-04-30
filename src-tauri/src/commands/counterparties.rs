use acta::tauri_api::counterparties::{
    CounterpartiesListRequest, CounterpartiesScreenDto, CounterpartyDetailScreenDto,
    CounterpartyEditorDto, CounterpartySaveRequest, CounterpartySaveResultDto,
    CreateDocumentContextDto,
};
use acta::tauri_api::payments::MutationResultDto;
use tauri::State;

use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn counterparties_list(
    state: State<'_, TauriState>,
    request: CounterpartiesListRequest,
) -> CommandResult<CounterpartiesScreenDto> {
    acta::tauri_api::counterparties::counterparties_list(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn counterparty_get(
    state: State<'_, TauriState>,
    counterparty_id: String,
) -> CommandResult<CounterpartyDetailScreenDto> {
    acta::tauri_api::counterparties::counterparty_get(&state.ctx, counterparty_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn counterparty_open_editor(
    state: State<'_, TauriState>,
    counterparty_id: Option<String>,
) -> CommandResult<CounterpartyEditorDto> {
    acta::tauri_api::counterparties::counterparty_open_editor(&state.ctx, counterparty_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn counterparty_save(
    state: State<'_, TauriState>,
    request: CounterpartySaveRequest,
) -> CommandResult<CounterpartySaveResultDto> {
    acta::tauri_api::counterparties::counterparty_save(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn counterparty_archive(
    state: State<'_, TauriState>,
    counterparty_id: String,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::counterparties::counterparty_archive(&state.ctx, counterparty_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn counterparty_create_document_context(
    state: State<'_, TauriState>,
    counterparty_id: String,
) -> CommandResult<CreateDocumentContextDto> {
    acta::tauri_api::counterparties::counterparty_create_document_context(
        &state.ctx,
        counterparty_id,
    )
    .await
    .map_err(|error| error.to_string())
}
