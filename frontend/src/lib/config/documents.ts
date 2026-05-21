import type { AppIconName } from "../icons";
import type { DocumentDirection, DocumentKind, DocumentStatus } from "../types";
import { EDITOR_DIRTY_COPY } from "./shared";

export const DOCUMENT_KIND_META: Record<
  DocumentKind,
  { label: string; icon: AppIconName; actionLabel: string }
> = {
  invoice: { label: "Рахунок", icon: "invoice", actionLabel: "рахунок" },
  act: { label: "Акт", icon: "act", actionLabel: "акт" },
  waybill: { label: "Накладна", icon: "waybill", actionLabel: "накладну" },
  adjustment_act: { label: "Акт коригування", icon: "act", actionLabel: "акт коригування" }
};

export const DOCUMENT_KIND_CREATBLE: DocumentKind[] = ["invoice", "act", "waybill"];

export const DOCUMENT_KIND_OPTIONS = DOCUMENT_KIND_CREATBLE.map((value) => ({
  value,
  label: DOCUMENT_KIND_META[value].label
}));

export const DOCUMENT_KIND_FILTER_OPTIONS: Array<{ value: DocumentKind | null; label: string }> = [
  { value: null, label: "Всі" },
  { value: "act", label: "Акти" },
  { value: "invoice", label: "Рахунки" },
  { value: "waybill", label: "Накладні" },
  { value: "adjustment_act", label: "Коригування" }
];

export const DOCUMENT_DIRECTION_LABELS: Record<DocumentDirection, string> = {
  outgoing: "↑ Вихідний",
  incoming: "↓ Вхідний"
};

export const DOCUMENT_DIRECTION_OPTIONS: Array<{ value: DocumentDirection; label: string }> = Object.entries(
  DOCUMENT_DIRECTION_LABELS
).map(([value, label]) => ({
  value: value as DocumentDirection,
  label
}));

export const DOCUMENT_TAB_OPTIONS: Array<{
  value: "all" | DocumentDirection;
  label: string;
}> = [
  { value: "all", label: "Всі" },
  { value: "outgoing", label: "Вихідні" },
  { value: "incoming", label: "Вхідні" }
];

export const DOCUMENTS_COPY = {
  confirmDeleteCurrent: "Видалити поточний документ? Цю дію не можна скасувати.",
  confirmDeleteBulk: "Видалити вибрані документи? Цю дію не можна скасувати.",
  emptyTitle: "Поки що документів немає",
  emptyDescription:
    "Почніть зі створення першого рахунку, акта або накладної, щоб запустити повний сценарій документа.",
  emptyAction: "Створити перший документ",
  ...EDITOR_DIRTY_COPY,
  itemsEmptyTitle: "Поки що без позицій",
  itemsEmptyDescription:
    "Додайте першу позицію, щоб менеджер одразу бачив номенклатуру, кількість, ціну й підсумок документа."
} as const;

export function formatDocumentsLabel(count: number): string {
  if (count === 1) {
    return "1 документ";
  }
  if (count >= 2 && count <= 4) {
    return `${count} документи`;
  }
  return `${count} документів`;
}

export function formatDocumentItemsLabel(count: number): string {
  if (count === 1) {
    return "1 позиція";
  }
  if (count >= 2 && count <= 4) {
    return `${count} позиції`;
  }
  return `${count} позицій`;
}


export function resolveDocumentKindMeta(kind: string): { label: string; icon: AppIconName } {
  const normalized = kind.toLowerCase();

  if (normalized === "invoice" || normalized.includes("рах")) {
    return { label: "Рахунок", icon: "invoice" };
  }
  if (normalized === "act" || normalized.includes("акт")) {
    return { label: "Акт", icon: "act" };
  }
  if (normalized === "waybill" || normalized.includes("наклад")) {
    return { label: "Накладна", icon: "waybill" };
  }
  if (normalized === "adjustment_act" || normalized.includes("кориг")) {
    return { label: "Акт коригування", icon: "act" };
  }
  if (normalized.includes("догов")) {
    return { label: "Договір", icon: "contract" };
  }
  if (normalized.includes("pdf")) {
    return { label: "PDF", icon: "pdf" };
  }
  if (normalized.includes("excel") || normalized.includes("xls")) {
    return { label: "Excel", icon: "excel" };
  }

  return { label: kind, icon: "documents" };
}

export function getDocumentChainTargets(kind: string): DocumentKind[] {
  if (kind === "invoice") {
    return ["act", "waybill"];
  }
  if (kind === "act") {
    return ["waybill"];
  }
  return [];
}

export function getDocumentCreateLabel(
  kind: DocumentKind,
  activeTab: "all" | DocumentDirection
): string {
  const dirSuffix =
    activeTab === "incoming" ? " (вхідний)" : activeTab === "outgoing" ? " (вихідний)" : "";
  return `Створити ${DOCUMENT_KIND_META[kind].actionLabel}${dirSuffix}`;
}

export function supportsExistingPdfFlow(kind: string): boolean {
  return kind === "invoice" || kind === "waybill";
}

export function supportsDocumentPdfGeneration(kind: string): boolean {
  return kind === "act" || kind === "invoice" || kind === "adjustment_act";
}

export const DOCUMENT_STATUS_OPTIONS: Array<{ value: DocumentStatus; label: string }> = [
  { value: "draft", label: "Чернетка" },
  { value: "issued", label: "Виставлено" },
  { value: "signed", label: "Підписано" },
  { value: "paid", label: "Оплачено" },
  { value: "delivered", label: "Доставлено" },
  { value: "applied", label: "Застосовано" }
];

export interface DocumentFilterPresetSnapshot {
  dateFrom: string | null;
  dateTo: string | null;
  statusFilter: string[];
  amountMin: string | null;
  amountMax: string | null;
  overdueOnly: boolean;
}

export interface DocumentFilterPreset {
  id: string;
  label: string;
  build(today: Date): DocumentFilterPresetSnapshot;
}

function isoDate(d: Date): string {
  return d.toISOString().slice(0, 10);
}

const emptyPreset = (): DocumentFilterPresetSnapshot => ({
  dateFrom: null,
  dateTo: null,
  statusFilter: [],
  amountMin: null,
  amountMax: null,
  overdueOnly: false
});

export const DOCUMENT_FILTER_PRESETS: DocumentFilterPreset[] = [
  { id: "all", label: "Усі", build: () => emptyPreset() },
  { id: "drafts", label: "Чернетки", build: () => ({ ...emptyPreset(), statusFilter: ["draft"] }) },
  {
    id: "unpaid",
    label: "Неоплачені",
    build: () => ({ ...emptyPreset(), statusFilter: ["issued", "signed"] })
  },
  { id: "overdue", label: "Прострочені", build: () => ({ ...emptyPreset(), overdueOnly: true }) },
  {
    id: "this-month",
    label: "Цього місяця",
    build: (today) => ({
      ...emptyPreset(),
      dateFrom: isoDate(new Date(today.getFullYear(), today.getMonth(), 1)),
      dateTo: isoDate(today)
    })
  }
];

export const DOCUMENTS_FILTER_COPY = {
  filterButton: "Фільтр",
  filterButtonWithCount: (n: number) => `Фільтр · ${n}`,
  clearAll: "Очистити",
  apply: "Застосувати",
  reset: "Скинути",
  activeFiltersLabel: "Активні:",
  presetsLabel: "Швидкі:",
  periodLabel: "Період",
  periodFrom: "Від",
  periodTo: "До",
  periodSubpresets: { today: "Сьогодні", week: "Тиждень", month: "Місяць", quarter: "Квартал", year: "Рік" },
  statusLabel: "Статус",
  counterpartyLabel: "Контрагент",
  counterpartyAll: "Усі контрагенти",
  amountLabel: "Сума, грн",
  amountFrom: "Від",
  amountTo: "До",
  errors: {
    dateRangeInvalid: "Кінцева дата раніше за початкову",
    amountRangeInvalid: "Максимальна сума менша за мінімальну",
    amountInvalidFormat: "Некоректна сума"
  }
} as const;
