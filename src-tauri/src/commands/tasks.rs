use acta::tauri_api::tasks::{
    TaskEditorDto, TaskMutationResultDto, TaskSaveRequest, TaskSaveResultDto, TasksListRequest,
    TasksScreenDto,
};
use tauri::State;

use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn tasks_list(
    state: State<'_, TauriState>,
    request: TasksListRequest,
) -> CommandResult<TasksScreenDto> {
    acta::tauri_api::tasks::tasks_list(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn task_open_editor(
    state: State<'_, TauriState>,
    task_id: Option<String>,
) -> CommandResult<TaskEditorDto> {
    acta::tauri_api::tasks::task_open_editor(&state.ctx, task_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn task_save(
    state: State<'_, TauriState>,
    request: TaskSaveRequest,
) -> CommandResult<TaskSaveResultDto> {
    acta::tauri_api::tasks::task_save(&state.ctx, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn task_delete(
    state: State<'_, TauriState>,
    task_id: String,
) -> CommandResult<TaskMutationResultDto> {
    acta::tauri_api::tasks::task_delete(&state.ctx, task_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn task_set_status(
    state: State<'_, TauriState>,
    task_id: String,
    status: String,
) -> CommandResult<TaskMutationResultDto> {
    acta::tauri_api::tasks::task_set_status(&state.ctx, task_id, status)
        .await
        .map_err(|error| error.to_string())
}
