use acta::tauri_api::documents::{
    BulkDocumentRequest, BulkMutationResultDto,
    CreateChainDraftRequest, CreateDocumentDraftRequest,
    DocumentChainDto, DocumentEditorDto, DocumentsListDto, DocumentsListRequest,
    MutationResultDto, SaveDocumentRequest, SaveDocumentResponse,
};
use tauri::State;

use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn documents_list(
    state: State<'_, TauriState>,
    request: DocumentsListRequest,
) -> CommandResult<DocumentsListDto> {
    acta::tauri_api::documents::documents_list(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn document_open(
    state: State<'_, TauriState>,
    doc_id: String,
) -> CommandResult<DocumentEditorDto> {
    acta::tauri_api::documents::document_open(&state.ctx, doc_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn document_create_draft(
    state: State<'_, TauriState>,
    request: CreateDocumentDraftRequest,
) -> CommandResult<DocumentEditorDto> {
    acta::tauri_api::documents::document_create_draft(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn document_save(
    state: State<'_, TauriState>,
    request: SaveDocumentRequest,
) -> CommandResult<SaveDocumentResponse> {
    acta::tauri_api::documents::document_save(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn document_advance_status(
    state: State<'_, TauriState>,
    doc_id: String,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::documents::document_advance_status(&state.ctx, doc_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn document_delete(
    state: State<'_, TauriState>,
    doc_id: String,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::documents::document_delete(&state.ctx, doc_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn documents_bulk_delete(
    state: State<'_, TauriState>,
    request: BulkDocumentRequest,
) -> CommandResult<BulkMutationResultDto> {
    acta::tauri_api::documents::documents_bulk_delete_live(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn documents_bulk_advance_status(
    state: State<'_, TauriState>,
    request: BulkDocumentRequest,
) -> CommandResult<BulkMutationResultDto> {
    acta::tauri_api::documents::documents_bulk_advance_status_live(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn document_chain_get(
    state: State<'_, TauriState>,
    doc_id: String,
) -> CommandResult<DocumentChainDto> {
    acta::tauri_api::documents::document_chain_get(&state.ctx, doc_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn document_chain_create_draft(
    state: State<'_, TauriState>,
    request: CreateChainDraftRequest,
) -> CommandResult<DocumentEditorDto> {
    acta::tauri_api::documents::document_chain_create_draft(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}
