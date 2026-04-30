import { invoke } from "@tauri-apps/api/core";
import type {
  CounterpartyDetailScreenDto,
  CounterpartyEditorDto,
  CounterpartiesScreenDto,
  CounterpartySaveResultDto,
  CreateDocumentContextDto,
  DashboardScreenDto,
  DocumentsListDto,
  DocumentChainDto,
  DocumentEditorDto,
  PaletteActivationResultDto,
  PaletteSearchResultDto,
  ShellStateDto,
  PaymentsScreenDto,
  MutationResultDto,
  OpenTemplateResultDto,
  PaymentDraftFormDto,
  ReportsExportResultDto,
  ReportsFilterDto,
  ReportsScreenDto,
  SettingsActionResultDto,
  SettingsCompanyDto,
  SettingsScreenDto,
  SettingsScreenMutationResultDto,
  SaveDocumentResponse,
  TaskEditorDto,
  TaskMutationResultDto,
  TaskSaveResultDto,
  TasksScreenDto,
  ImportPlanDto,
  ImportResultDto
} from "./types";

export function shellLoad(): Promise<ShellStateDto> {
  return invoke("shell_load");
}

export function shellSetActiveCompany(companyId: string): Promise<ShellStateDto> {
  return invoke("shell_set_active_company", { companyId });
}

export function shellPaletteSearch(query: string, selectedCounterpartyId?: string): Promise<PaletteSearchResultDto> {
  return invoke("shell_palette_search", {
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
  return invoke("shell_palette_activate", {
    payload,
    selectedCounterpartyId
  });
}

export function dashboardLoad(): Promise<DashboardScreenDto> {
  return invoke("dashboard_load");
}

export function documentsList(query = "", tab?: string): Promise<DocumentsListDto> {
  return invoke("documents_list", {
    request: {
      query: query || null,
      tab: tab || null
    }
  });
}

export function documentOpen(docId: string): Promise<DocumentEditorDto> {
  return invoke("document_open", { docId });
}

export function documentCreateDraft(counterpartyId: string, kind: string): Promise<DocumentEditorDto> {
  return invoke("document_create_draft", {
    request: {
      counterpartyId,
      kind
    }
  });
}

export function documentSave(
  form: DocumentEditorDto["form"],
  items: DocumentEditorDto["items"]
): Promise<SaveDocumentResponse> {
  return invoke("document_save", {
    request: {
      form,
      items
    }
  });
}

export function documentAdvanceStatus(docId: string): Promise<MutationResultDto> {
  return invoke("document_advance_status", { docId });
}

export function documentDelete(docId: string): Promise<MutationResultDto> {
  return invoke("document_delete", { docId });
}

export function documentChainGet(docId: string): Promise<DocumentChainDto> {
  return invoke("document_chain_get", { docId });
}

export function documentChainCreateDraft(
  sourceId: string,
  targetKind: string
): Promise<DocumentEditorDto> {
  return invoke("document_chain_create_draft", {
    request: {
      sourceId,
      targetKind
    }
  });
}

export function counterpartiesList(query = ""): Promise<CounterpartiesScreenDto> {
  return invoke("counterparties_list", {
    request: {
      query: query || null
    }
  });
}

export function counterpartyGet(counterpartyId: string): Promise<CounterpartyDetailScreenDto> {
  return invoke("counterparty_get", { counterpartyId });
}

export function counterpartyOpenEditor(counterpartyId?: string): Promise<CounterpartyEditorDto> {
  return invoke("counterparty_open_editor", {
    counterpartyId: counterpartyId || null
  });
}

export function counterpartySave(form: CounterpartyEditorDto["form"]): Promise<CounterpartySaveResultDto> {
  return invoke("counterparty_save", {
    request: {
      form
    }
  });
}

export function counterpartyArchive(counterpartyId: string): Promise<MutationResultDto> {
  return invoke("counterparty_archive", { counterpartyId });
}

export function counterpartyCreateDocumentContext(
  counterpartyId: string
): Promise<CreateDocumentContextDto> {
  return invoke("counterparty_create_document_context", { counterpartyId });
}

export function tasksList(query = ""): Promise<TasksScreenDto> {
  return invoke("tasks_list", {
    request: {
      query: query || null
    }
  });
}

export function taskOpenEditor(taskId?: string): Promise<TaskEditorDto> {
  return invoke("task_open_editor", {
    taskId: taskId || null
  });
}

export function taskSave(form: TaskEditorDto["form"]): Promise<TaskSaveResultDto> {
  return invoke("task_save", {
    request: {
      form
    }
  });
}

export function taskDelete(taskId: string): Promise<TaskMutationResultDto> {
  return invoke("task_delete", { taskId });
}

export function taskSetStatus(taskId: string, status: string): Promise<TaskMutationResultDto> {
  return invoke("task_set_status", { taskId, status });
}

export function reportsLoad(filter: ReportsFilterDto): Promise<ReportsScreenDto> {
  return invoke("reports_load", {
    request: {
      tab: filter.tab,
      scope: filter.scope,
      dateFrom: filter.dateFrom,
      dateTo: filter.dateTo,
      query: filter.query
    }
  });
}

export function reportsExportCsv(filter: ReportsFilterDto): Promise<ReportsExportResultDto> {
  return invoke("reports_export_csv", {
    request: {
      tab: filter.tab,
      scope: filter.scope,
      dateFrom: filter.dateFrom,
      dateTo: filter.dateTo,
      query: filter.query
    }
  });
}

export function settingsLoad(): Promise<SettingsScreenDto> {
  return invoke("settings_load");
}

export function settingsSavePreferences(
  darkMode: boolean,
  density: number
): Promise<SettingsScreenMutationResultDto> {
  return invoke("settings_save_preferences", {
    request: {
      darkMode,
      density
    }
  });
}

export function settingsSaveCompany(
  company: SettingsCompanyDto
): Promise<SettingsScreenMutationResultDto> {
  return invoke("settings_save_company", {
    request: {
      company
    }
  });
}

export function settingsConfigureIntegration(tag: string): Promise<SettingsScreenMutationResultDto> {
  return invoke("settings_configure_integration", {
    request: { tag }
  });
}

export function settingsTeamInvite(): Promise<SettingsScreenMutationResultDto> {
  return invoke("settings_team_invite");
}

export function settingsBackupNow(): Promise<SettingsScreenMutationResultDto> {
  return invoke("settings_backup_now");
}

export function settingsBackupOpenLatest(): Promise<SettingsActionResultDto> {
  return invoke("settings_backup_open_latest");
}

export function paymentsList(): Promise<PaymentsScreenDto> {
  return invoke("payments_list");
}

export function paymentsImportLatestCsv(): Promise<MutationResultDto> {
  return invoke("payments_import_latest_csv");
}

export function paymentsSyncBank(): Promise<MutationResultDto> {
  return invoke("payments_sync_bank");
}

export function paymentsOpenManualTemplate(): Promise<OpenTemplateResultDto> {
  return invoke("payments_open_manual_template");
}

export function paymentCreateOrUpdate(form: PaymentDraftFormDto): Promise<MutationResultDto> {
  return invoke("payment_create_or_update", { request: form });
}

export function paymentReconcile(paymentId: string): Promise<MutationResultDto> {
  return invoke("payment_reconcile", { paymentId });
}

export function paymentUnreconcile(paymentId: string): Promise<MutationResultDto> {
  return invoke("payment_unreconcile", { paymentId });
}

export const importBasPlan = () => invoke<ImportPlanDto>("import_bas_plan");
export const importBasExecute = () => invoke<ImportResultDto>("import_bas_execute");
