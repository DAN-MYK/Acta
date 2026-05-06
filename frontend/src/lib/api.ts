import { invoke } from "@tauri-apps/api/core";
import { invokeInBrowser, isBrowserFallbackEnabled } from "./browser-api";
import type {
  BulkMutationResultDto,
  CounterpartyDetailScreenDto,
  CounterpartyEditorDto,
  CounterpartiesScreenDto,
  CounterpartySaveResultDto,
  CreateDocumentContextDto,
  DashboardScreenDto,
  DocumentChainDto,
  DocumentEditorDto,
  DocumentPdfActionResultDto,
  DocumentsListDto,
  ImportPlanDto,
  ImportResultDto,
  MutationResultDto,
  OpenTemplateResultDto,
  PaletteActivationResultDto,
  PaletteSearchResultDto,
  PaymentMatchApplyAutoRequest,
  PaymentMatchPreviewDto,
  PaymentMatchPreviewRequest,
  PaymentReconcileRequest,
  PaymentReconcileSplitRequest,
  PaymentReconcileSplitResultDto,
  PaymentDraftFormDto,
  PaymentCalendarMonthDto,
  PaymentCalendarMonthRequest,
  PaymentImportCommitRequest,
  PaymentImportPreviewDto,
  PaymentManualMatchCandidatesDto,
  PaymentsScreenDto,
  PaymentMatchManualCandidatesRequest,
  PaymentScheduleCompleteRequest,
  PaymentUnreconcileAllRequest,
  PaymentUnreconcileRequest,
  ReportsExportResultDto,
  ReportsFilterDto,
  ReportsScreenDto,
  SaveDocumentResponse,
  SettingsActionResultDto,
  SettingsCompanyDto,
  SettingsScreenDto,
  SettingsScreenMutationResultDto,
  ShellStateDto,
  TaskEditorDto,
  TaskMutationResultDto,
  TaskSaveResultDto,
  TasksScreenDto
} from "./types";

function appInvoke<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  if (isBrowserFallbackEnabled()) {
    return invokeInBrowser<T>(command, payload);
  }

  return invoke(command, payload);
}

export function shellLoad(): Promise<ShellStateDto> {
  return appInvoke("shell_load");
}

export function shellSetActiveCompany(companyId: string): Promise<ShellStateDto> {
  return appInvoke("shell_set_active_company", { companyId });
}

export function shellPaletteSearch(query: string, selectedCounterpartyId?: string): Promise<PaletteSearchResultDto> {
  return appInvoke("shell_palette_search", {
    request: {
      query,
      selectedCounterpartyId
    }
  });
}

export function shellPaletteActivate(
  payload: string,
  selectedCounterpartyId?: string
): Promise<PaletteActivationResultDto> {
  return appInvoke("shell_palette_activate", {
    payload,
    selectedCounterpartyId
  });
}

export function dashboardLoad(): Promise<DashboardScreenDto> {
  return appInvoke("dashboard_load");
}

export function documentsList(
  query = "",
  direction?: "outgoing" | "incoming",
  kind?: string
): Promise<DocumentsListDto> {
  return appInvoke("documents_list", {
    request: {
      query: query || null,
      direction: direction ?? null,
      kind: kind ?? null
    }
  });
}

export function documentOpen(docId: string): Promise<DocumentEditorDto> {
  return appInvoke("document_open", { docId });
}

export function documentCreateDraft(
  counterpartyId: string,
  kind: string,
  direction: "outgoing" | "incoming" = "outgoing"
): Promise<DocumentEditorDto> {
  return appInvoke("document_create_draft", {
    request: {
      counterpartyId,
      kind,
      direction
    }
  });
}

export function documentSave(
  form: DocumentEditorDto["form"],
  items: DocumentEditorDto["items"]
): Promise<SaveDocumentResponse> {
  return appInvoke("document_save", {
    request: {
      form,
      items
    }
  });
}

export function documentAdvanceStatus(docId: string): Promise<MutationResultDto> {
  return appInvoke("document_advance_status", { docId });
}

export function documentDelete(docId: string): Promise<MutationResultDto> {
  return appInvoke("document_delete", { docId });
}

export function documentChainGet(docId: string): Promise<DocumentChainDto> {
  return appInvoke("document_chain_get", { docId });
}

export function documentChainCreateDraft(sourceId: string, targetKind: string): Promise<DocumentEditorDto> {
  return appInvoke("document_chain_create_draft", {
    request: {
      sourceId,
      targetKind
    }
  });
}

export function documentsBulkDelete(docIds: string[]): Promise<BulkMutationResultDto> {
  return appInvoke("documents_bulk_delete", {
    request: {
      docIds
    }
  });
}

export function documentsBulkAdvanceStatus(docIds: string[]): Promise<BulkMutationResultDto> {
  return appInvoke("documents_bulk_advance_status", {
    request: {
      docIds
    }
  });
}

export function counterpartiesList(query = ""): Promise<CounterpartiesScreenDto> {
  return appInvoke("counterparties_list", {
    request: {
      query: query || null
    }
  });
}

export function counterpartyGet(counterpartyId: string): Promise<CounterpartyDetailScreenDto> {
  return appInvoke("counterparty_get", { counterpartyId });
}

export function counterpartyOpenEditor(counterpartyId?: string): Promise<CounterpartyEditorDto> {
  return appInvoke("counterparty_open_editor", {
    counterpartyId: counterpartyId || null
  });
}

export function counterpartySave(form: CounterpartyEditorDto["form"]): Promise<CounterpartySaveResultDto> {
  return appInvoke("counterparty_save", {
    request: {
      form
    }
  });
}

export function counterpartyArchive(counterpartyId: string): Promise<MutationResultDto> {
  return appInvoke("counterparty_archive", { counterpartyId });
}

export function counterpartyCreateDocumentContext(counterpartyId: string): Promise<CreateDocumentContextDto> {
  return appInvoke("counterparty_create_document_context", { counterpartyId });
}

export function tasksList(query = ""): Promise<TasksScreenDto> {
  return appInvoke("tasks_list", {
    request: {
      query: query || null
    }
  });
}

export function taskOpenEditor(taskId?: string): Promise<TaskEditorDto> {
  return appInvoke("task_open_editor", {
    taskId: taskId || null
  });
}

export function taskSave(form: TaskEditorDto["form"]): Promise<TaskSaveResultDto> {
  return appInvoke("task_save", {
    request: {
      form
    }
  });
}

export function taskDelete(taskId: string): Promise<TaskMutationResultDto> {
  return appInvoke("task_delete", { taskId });
}

export function taskSetStatus(taskId: string, status: string): Promise<TaskMutationResultDto> {
  return appInvoke("task_set_status", { taskId, status });
}

export function reportsLoad(filter: ReportsFilterDto): Promise<ReportsScreenDto> {
  return appInvoke("reports_load", {
    request: {
      tab: filter.tab,
      scope: filter.scope,
      dateFrom: filter.dateFrom,
      dateTo: filter.dateTo,
      query: filter.query,
      selectedCounterpartyId: filter.selectedCounterpartyId ?? null
    }
  });
}

export function reportsExportCsv(filter: ReportsFilterDto): Promise<ReportsExportResultDto> {
  return appInvoke("reports_export_csv", {
    request: {
      tab: filter.tab,
      scope: filter.scope,
      dateFrom: filter.dateFrom,
      dateTo: filter.dateTo,
      query: filter.query,
      selectedCounterpartyId: filter.selectedCounterpartyId ?? null
    }
  });
}

export function reportsExportExcel(filter: ReportsFilterDto): Promise<ReportsExportResultDto> {
  return appInvoke("reports_export_excel", {
    request: {
      tab: filter.tab,
      scope: filter.scope,
      dateFrom: filter.dateFrom,
      dateTo: filter.dateTo,
      query: filter.query,
      selectedCounterpartyId: filter.selectedCounterpartyId ?? null
    }
  });
}

export function reportsExportExcelAndOpen(
  filter: ReportsFilterDto
): Promise<ReportsExportResultDto> {
  return appInvoke("reports_export_excel_and_open", {
    request: {
      tab: filter.tab,
      scope: filter.scope,
      dateFrom: filter.dateFrom,
      dateTo: filter.dateTo,
      query: filter.query,
      selectedCounterpartyId: filter.selectedCounterpartyId ?? null
    }
  });
}

export function settingsLoad(): Promise<SettingsScreenDto> {
  return appInvoke("settings_load");
}

export function settingsSavePreferences(
  darkMode: boolean
): Promise<SettingsScreenMutationResultDto> {
  return appInvoke("settings_save_preferences", {
    request: {
      darkMode
    }
  });
}

export function settingsSaveCompany(company: SettingsCompanyDto): Promise<SettingsScreenMutationResultDto> {
  return appInvoke("settings_save_company", {
    request: {
      company
    }
  });
}

export function settingsConfigureIntegration(tag: string): Promise<SettingsScreenMutationResultDto> {
  return appInvoke("settings_configure_integration", {
    request: { tag }
  });
}

export function settingsTeamInvite(): Promise<SettingsScreenMutationResultDto> {
  return appInvoke("settings_team_invite");
}

export function settingsBackupNow(): Promise<SettingsScreenMutationResultDto> {
  return appInvoke("settings_backup_now");
}

export function settingsBackupOpenLatest(): Promise<SettingsActionResultDto> {
  return appInvoke("settings_backup_open_latest");
}

export function paymentsList(): Promise<PaymentsScreenDto> {
  return appInvoke("payments_list");
}

export function paymentsImportLatestCsv(): Promise<MutationResultDto> {
  return appInvoke("payments_import_latest_csv");
}

export function paymentsSyncBank(): Promise<MutationResultDto> {
  return appInvoke("payments_sync_bank");
}

export function paymentsOpenManualTemplate(): Promise<OpenTemplateResultDto> {
  return appInvoke("payments_open_manual_template");
}

export function paymentsCalendarLoad(
  request: PaymentCalendarMonthRequest
): Promise<PaymentCalendarMonthDto> {
  return appInvoke("payments_calendar_load", { request });
}

export function paymentScheduleComplete(
  request: PaymentScheduleCompleteRequest
): Promise<MutationResultDto> {
  return appInvoke("payment_schedule_complete", { request });
}

export function paymentCreateOrUpdate(form: PaymentDraftFormDto): Promise<MutationResultDto> {
  return appInvoke("payment_create_or_update", { request: form });
}

export function paymentReconcile(request: PaymentReconcileRequest): Promise<MutationResultDto> {
  return appInvoke("payment_reconcile", { request });
}

export function paymentReconcileSplit(
  request: PaymentReconcileSplitRequest
): Promise<PaymentReconcileSplitResultDto> {
  return appInvoke("payment_reconcile_split", { request });
}

export function paymentUnreconcile(request: PaymentUnreconcileRequest): Promise<MutationResultDto> {
  return appInvoke("payment_unreconcile", { request });
}

export function paymentUnreconcileAll(
  request: PaymentUnreconcileAllRequest
): Promise<MutationResultDto> {
  return appInvoke("payment_unreconcile_all", { request });
}

export function paymentMatchPreview(request: PaymentMatchPreviewRequest): Promise<PaymentMatchPreviewDto> {
  return appInvoke("payment_match_preview", { request });
}

export function paymentMatchApplyAuto(
  request: PaymentMatchApplyAutoRequest
): Promise<MutationResultDto> {
  return appInvoke("payment_match_apply_auto", { request });
}

export function paymentMatchManualCandidates(
  request: PaymentMatchManualCandidatesRequest
): Promise<PaymentManualMatchCandidatesDto> {
  return appInvoke("payment_match_manual_candidates", { request });
}

export function paymentsImportPickAndPreview(): Promise<PaymentImportPreviewDto | null> {
  return appInvoke("payments_import_pick_and_preview");
}

export function paymentsImportPreview(path: string): Promise<PaymentImportPreviewDto> {
  return appInvoke("payments_import_preview", { path });
}

export function paymentsImportCommit(
  request: PaymentImportCommitRequest
): Promise<MutationResultDto> {
  return appInvoke("payments_import_commit", { request });
}

export function documentGeneratePdf(docId: string): Promise<MutationResultDto> {
  return appInvoke("document_generate_pdf", { docId });
}

export function documentPdfAttachExisting(
  docId: string,
  sourcePath?: string
): Promise<DocumentPdfActionResultDto> {
  return appInvoke("document_pdf_attach_existing", {
    docId,
    sourcePath: sourcePath || null
  });
}

export function documentPdfApplyTextReplace(
  docId: string,
  findText: string,
  replaceText: string
): Promise<DocumentPdfActionResultDto> {
  return appInvoke("document_pdf_apply_text_replace", {
    request: {
      docId,
      findText,
      replaceText
    }
  });
}

export function documentPdfOpenCurrent(docId: string): Promise<MutationResultDto> {
  return appInvoke("document_pdf_open_current", { docId });
}

export const importBasPickDirectory = () => appInvoke<string | null>("import_bas_pick_directory");
export const importBasPlan = (inputDir?: string | null) =>
  appInvoke<ImportPlanDto>("import_bas_plan", {
    request: {
      inputDir: inputDir ?? null
    }
  });
export const importBasExecute = (inputDir?: string | null) =>
  appInvoke<ImportResultDto>("import_bas_execute", {
    request: {
      inputDir: inputDir ?? null
    }
  });
