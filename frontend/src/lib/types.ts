export type ScreenId =
  | "dashboard"
  | "documents"
  | "counterparties"
  | "payments"
  | "reports"
  | "tasks"
  | "settings";

export type DocumentKind = "invoice" | "act" | "waybill";
export type DocumentStatus = "draft" | "issued" | "signed" | "paid" | "delivered";

export interface ShellChromeDto {
  companyName: string;
  userName: string;
  userInitials: string;
  userRole: string;
  documentsBadge: number;
  tasksBadge: number;
}

export interface CompanySwitcherItemDto {
  id: string;
  name: string;
  subtitle: string;
  initials: string;
  badge: string;
  active: boolean;
}

export interface ShellStateDto {
  chrome: ShellChromeDto;
  companyItems: CompanySwitcherItemDto[];
  activeCompanyId: string;
  isDark: boolean;
}

export interface PaletteItemDto {
  kind: string;
  title: string;
  subtitle: string;
  shortcut: string;
  payload: string;
}

export interface PaletteSearchResultDto {
  items: PaletteItemDto[];
}

export type PaletteActivationKind =
  | "navigate"
  | "open_document"
  | "open_counterparty"
  | "create_document_draft"
  | "open_counterparty_create"
  | "navigate_for_counterparty_selection"
  | "unsupported";

export interface DocumentItemDto {
  id: string;
  kind: DocumentKind;
  number: string;
  date: string;
  counterparty: string;
  amountStr: string;
  status: DocumentStatus;
  statusLabel: string;
  linkedId: string;
}

export interface DocumentDraftFormDto {
  id: string;
  kind: string;
  counterpartyId: string;
  counterpartyName: string;
  title: string;
  number: string;
  date: string;
  notes: string;
}

export interface DocumentDraftItemDto {
  description: string;
  unit: string;
  quantity: string;
  price: string;
}

export interface ChainStepDto {
  docType: string;
  docNumber: string;
  amountStr: string;
  status: string;
  exists: boolean;
}

export interface DocumentChainDto {
  sourceId: string;
  steps: ChainStepDto[];
}

export interface DocumentEditorDto {
  form: DocumentDraftFormDto;
  items: DocumentDraftItemDto[];
  showTypePicker: boolean;
  showEditor: boolean;
}

export interface PaletteActivationResultDto {
  kind: PaletteActivationKind;
  screen: ScreenId | null;
  documentId: string | null;
  counterpartyId: string | null;
  documentEditor: DocumentEditorDto | null;
  message: string | null;
}

export interface DocumentsListDto {
  items: DocumentItemDto[];
  invoiceItems: DocumentItemDto[];
  actItems: DocumentItemDto[];
  waybillItems: DocumentItemDto[];
  totalCount: number;
  pageCount: number;
}

export interface MutationResultDto {
  ok: boolean;
  documentId?: string;
  message: string;
}

export interface SaveDocumentResponse {
  documentId: string;
  kind: string;
  message: string;
}

export interface DashboardKpiDto {
  label: string;
  value: string;
  detail: string;
  tone: "positive" | "warning" | "neutral" | "accent" | "danger" | string;
}

export interface DashboardScreenDto {
  kpis: DashboardKpiDto[];
  cashflowRows: BankReportRowDto[];
  recentDocuments: DocumentItemDto[];
  urgentTasks: TaskItemDto[];
}

export interface OpenTemplateResultDto {
  ok: boolean;
  path: string;
  message: string;
}

export interface CounterpartyItemDto {
  id: string;
  name: string;
  edrpou: string;
  kind: string;
  balanceStr: string;
  docCount: number;
  overdueCount: number;
}

export interface CounterpartyDetailsDto {
  id: string;
  name: string;
  kind: string;
  edrpou: string;
  ipn: string;
  vat: string;
  iban: string;
  bank: string;
  address: string;
  director: string;
  phone: string;
  email: string;
  clientSince: string;
  balanceStr: string;
  balanceIsNegative: boolean;
  docCount: number;
  overdueCount: number;
  overdueAmountStr: string;
  lastContactDays: number;
  lastContactDate: string;
}

export interface CounterpartyDraftFormDto {
  id: string;
  title: string;
  name: string;
  edrpou: string;
  ipn: string;
  iban: string;
  address: string;
  phone: string;
  email: string;
  notes: string;
}

export interface CounterpartiesScreenDto {
  items: CounterpartyItemDto[];
}

export interface CounterpartyDetailScreenDto {
  info: CounterpartyDetailsDto;
  documents: DocumentItemDto[];
  payments: PaymentItemDto[];
}

export interface CounterpartyEditorDto {
  form: CounterpartyDraftFormDto;
  showEditor: boolean;
}

export interface CounterpartySaveResultDto {
  ok: boolean;
  savedId: string;
  message: string;
  updatedList: CounterpartyItemDto[];
  updatedDetail: CounterpartyDetailScreenDto | null;
}

export interface CreateDocumentContextDto {
  counterpartyId: string;
  counterpartyName: string;
}

export interface PaymentItemDto {
  id: string;
  date: string;
  counterparty: string;
  amountStr: string;
  direction: "in" | "out";
  matchedDoc: string;
  account: string;
}

export interface PaymentsKpiDto {
  incomingStr: string;
  outgoingStr: string;
  netStr: string;
  unmatchedStr: string;
  incomingSub: string;
  outgoingSub: string;
  unmatchedCount: number;
}

export interface PaymentCounterpartyItemDto {
  id: string;
  name: string;
}

export interface PaymentsScreenDto {
  items: PaymentItemDto[];
  counterparties: PaymentCounterpartyItemDto[];
  kpi: PaymentsKpiDto;
}

export interface PaymentDraftFormDto {
  id: string;
  date: string;
  amount: string;
  direction: string;
  counterpartyId: string;
  counterpartyName: string;
  bankName: string;
  reference: string;
  description: string;
}

export type TaskStatus = "open" | "in_progress" | "done" | "cancelled";
export type TaskPriority = "low" | "normal" | "high" | "critical";

export interface TaskItemDto {
  id: string;
  title: string;
  description: string;
  status: TaskStatus;
  statusLabel: string;
  priority: TaskPriority;
  priorityLabel: string;
  dueDate: string;
  reminderAt: string;
  linkKind: string;
  linkLabel: string;
}

export interface TasksScreenDto {
  items: TaskItemDto[];
  openCount: number;
  doneCount: number;
  highCount: number;
  todayCount: number;
}

export interface TaskDraftFormDto {
  id: string;
  title: string;
  description: string;
  priority: TaskPriority;
  dueDate: string;
  reminderAt: string;
  status: TaskStatus;
  counterpartyId: string;
  actId: string;
  linkKind: string;
  linkLabel: string;
}

export interface TaskEditorDto {
  title: string;
  form: TaskDraftFormDto;
  showEditor: boolean;
}

export interface TaskSaveResultDto {
  ok: boolean;
  savedId: string;
  message: string;
  updatedList: TasksScreenDto;
  updatedEditor: TaskEditorDto | null;
}

export interface TaskMutationResultDto {
  ok: boolean;
  taskId: string;
  message: string;
}

export type ReportsTab = "bank" | "receivables" | "payables";
export type ReportsScope = "active" | "all";
export type SettingsSection =
  | "appearance"
  | "company"
  | "numbering"
  | "integrations"
  | "team"
  | "backup";

export interface ReportsFilterDto {
  tab: ReportsTab;
  scope: ReportsScope;
  dateFrom: string;
  dateTo: string;
  query: string;
}

export interface ReportsSummaryDto {
  openingBalanceStr: string;
  incomeStr: string;
  expenseStr: string;
  closingBalanceStr: string;
  receivablesTotalStr: string;
  payablesTotalStr: string;
}

export interface BankReportRowDto {
  key: string;
  label: string;
  incomeStr: string;
  expenseStr: string;
  netStr: string;
}

export interface ReceivableRowDto {
  docId: string;
  docType: string;
  docNumber: string;
  docDate: string;
  companyName: string;
  counterparty: string;
  amountStr: string;
  expectedDate: string;
  overdueDays: number;
  status: string;
}

export interface PayableRowDto {
  id: string;
  title: string;
  companyName: string;
  counterparty: string;
  amountStr: string;
  dueDate: string;
  overdueDays: number;
  recurrence: string;
}

export interface ReportsScreenDto {
  filter: ReportsFilterDto;
  summary: ReportsSummaryDto;
  bankRows: BankReportRowDto[];
  receivablesRows: ReceivableRowDto[];
  payablesRows: PayableRowDto[];
}

export interface ReportsExportResultDto {
  ok: boolean;
  path: string;
  message: string;
}

export interface SettingsCompanyDto {
  fullName: string;
  shortName: string;
  edrpou: string;
  ipn: string;
  address: string;
  director: string;
  iban: string;
  bank: string;
  vatRegistered: boolean;
  vatCert: string;
}

export interface SettingsIntegrationDto {
  label: string;
  description: string;
  tag: string;
  enabled: boolean;
}

export interface SettingsTeamMemberDto {
  name: string;
  email: string;
  role: string;
  lastActive: string;
}

export interface SettingsNumberingRowDto {
  docType: string;
  template: string;
  example: string;
  nextNumber: string;
}

export interface SettingsPreferencesDto {
  darkMode: boolean;
  density: number;
}

export interface SettingsBackupDto {
  label: string;
  file: string;
  kind: string;
  note: string;
  tone: string;
}

export interface SettingsScreenDto {
  company: SettingsCompanyDto;
  integrations: SettingsIntegrationDto[];
  team: SettingsTeamMemberDto[];
  numbering: SettingsNumberingRowDto[];
  preferences: SettingsPreferencesDto;
  backup: SettingsBackupDto;
}

export interface SettingsScreenMutationResultDto {
  ok: boolean;
  message: string;
  screen: SettingsScreenDto;
}

export interface SettingsActionResultDto {
  ok: boolean;
  message: string;
  path: string;
}
