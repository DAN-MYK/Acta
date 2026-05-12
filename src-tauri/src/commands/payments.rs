use acta::tauri_api::payments::{
    MutationResultDto, OpenTemplateResultDto, PaymentCalendarMonthDto, PaymentCalendarMonthRequest,
    PaymentCreateOrUpdateRequest, PaymentImportCommitRequest, PaymentImportPreviewDto,
    PaymentManualMatchCandidatesDto, PaymentMatchApplyAutoRequest,
    PaymentMatchManualCandidatesRequest, PaymentMatchPreviewDto, PaymentMatchPreviewRequest,
    PaymentReconcileRequest, PaymentReconcileSplitRequest, PaymentReconcileSplitResultDto,
    PaymentScheduleCompleteRequest, PaymentUnreconcileAllRequest, PaymentUnreconcileRequest,
    PaymentsScreenDto,
};
use tauri::State;
use tauri_plugin_dialog::{DialogExt, FilePath};

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
pub async fn payments_sync_bank(state: State<'_, TauriState>) -> CommandResult<MutationResultDto> {
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
pub async fn payments_calendar_load(
    state: State<'_, TauriState>,
    request: PaymentCalendarMonthRequest,
) -> CommandResult<PaymentCalendarMonthDto> {
    acta::tauri_api::payments::payments_calendar_load(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn payment_schedule_complete(
    state: State<'_, TauriState>,
    request: PaymentScheduleCompleteRequest,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::payments::payment_schedule_complete(&state.ctx, request)
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
pub async fn payment_reconcile_split(
    state: State<'_, TauriState>,
    request: PaymentReconcileSplitRequest,
) -> CommandResult<PaymentReconcileSplitResultDto> {
    acta::tauri_api::payments::payment_reconcile_split(&state.ctx, request)
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

#[tauri::command]
pub async fn payment_unreconcile_all(
    state: State<'_, TauriState>,
    request: PaymentUnreconcileAllRequest,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::payments::payment_unreconcile_all(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn payment_match_preview(
    state: State<'_, TauriState>,
    request: PaymentMatchPreviewRequest,
) -> CommandResult<PaymentMatchPreviewDto> {
    acta::tauri_api::payments::payment_match_preview(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn payment_match_apply_auto(
    state: State<'_, TauriState>,
    request: PaymentMatchApplyAutoRequest,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::payments::payment_match_apply_auto(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn payment_match_manual_candidates(
    state: State<'_, TauriState>,
    request: PaymentMatchManualCandidatesRequest,
) -> CommandResult<PaymentManualMatchCandidatesDto> {
    acta::tauri_api::payments::payment_match_manual_candidates(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

/// Відкриває нативний file picker та повертає dry-run попередній перегляд
/// імпорту банківської виписки. Користувач підтверджує commit окремо.
#[tauri::command]
pub async fn payments_import_pick_and_preview(
    app: tauri::AppHandle,
    state: State<'_, TauriState>,
) -> CommandResult<Option<PaymentImportPreviewDto>> {
    let picked = app
        .dialog()
        .file()
        .add_filter(
            "Банківська виписка (CSV, XLSX, XLS)",
            &["csv", "xlsx", "xls"],
        )
        .add_filter("Усі файли", &["*"])
        .blocking_pick_file();

    let path = match picked {
        Some(file_path) => match file_path {
            FilePath::Path(p) => p.to_string_lossy().into_owned(),
            FilePath::Url(url) => url.to_string(),
        },
        None => return Ok(None),
    };

    let preview = acta::tauri_api::payments::payments_import_preview(&state.ctx, path)
        .await
        .map_err(|error| error.to_string())?;
    Ok(Some(preview))
}

/// Виконує preview з заданим шляхом без файлового діалогу. Корисно для
/// browser-fixtures / тестів та повторного перегляду без re-pick.
#[tauri::command]
pub async fn payments_import_preview(
    state: State<'_, TauriState>,
    path: String,
) -> CommandResult<PaymentImportPreviewDto> {
    acta::tauri_api::payments::payments_import_preview(&state.ctx, path)
        .await
        .map_err(|error| error.to_string())
}

/// Підтверджує імпорт виписки за заданим шляхом (після успішного preview).
#[tauri::command]
pub async fn payments_import_commit(
    state: State<'_, TauriState>,
    request: PaymentImportCommitRequest,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::payments::payments_import_commit(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}
