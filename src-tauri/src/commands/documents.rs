use acta::tauri_api::documents::{
    BulkDocumentRequest, BulkMutationResultDto, ChangeCounterpartyResultDto,
    CreateChainDraftRequest, CreateDocumentDraftRequest, DocumentChainDto, DocumentEditorDto,
    DocumentPdfActionResultDto, DocumentsListDto, DocumentsListRequest, MutationResultDto,
    ReplaceDocumentPdfTextRequest, SaveDocumentRequest, SaveDocumentResponse,
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

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
pub async fn document_change_counterparty(
    state: State<'_, TauriState>,
    doc_id: String,
    counterparty_id: String,
) -> CommandResult<ChangeCounterpartyResultDto> {
    acta::tauri_api::documents::document_change_counterparty(&state.ctx, doc_id, counterparty_id)
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

#[tauri::command]
pub async fn document_generate_pdf(
    state: State<'_, TauriState>,
    doc_id: String,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::documents::generate_document_pdf(&state.ctx, doc_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn document_pdf_attach_existing(
    app: AppHandle,
    state: State<'_, TauriState>,
    doc_id: String,
    source_path: Option<String>,
) -> CommandResult<DocumentPdfActionResultDto> {
    let selected_path = match source_path {
        Some(path) => path,
        None => tauri::async_runtime::spawn_blocking(move || {
            app.dialog()
                .file()
                .add_filter("PDF", &["pdf"])
                .blocking_pick_file()
                .map(|path| path.to_string())
        })
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Вибір PDF скасовано".to_string())?,
    };

    acta::tauri_api::documents::document_pdf_attach_existing(&state.ctx, doc_id, selected_path)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn document_pdf_apply_text_replace(
    state: State<'_, TauriState>,
    request: ReplaceDocumentPdfTextRequest,
) -> CommandResult<DocumentPdfActionResultDto> {
    acta::tauri_api::documents::document_pdf_apply_text_replace(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn document_pdf_open_current(
    state: State<'_, TauriState>,
    doc_id: String,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::documents::document_pdf_open_current(&state.ctx, doc_id)
        .await
        .map_err(|error| error.to_string())
}
