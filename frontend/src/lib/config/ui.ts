import { daysUntil } from "../date";
import type { AppIconName } from "../icons";
import { compareMinor, parseMoneyToMinor } from "../money";
import type {
  DocumentDirection,
  DocumentKind,
  PayableRowDto,
  PaymentCalendarEventKind,
  PaymentCalendarFilterKind,
  PaymentMatchDecisionKind,
  ReceivableRowDto,
  ReportsScreenDto,
  ReportsScope,
  ReportsTab,
  ScreenId,
  TaskPriority,
  TaskStatus
} from "../types";

export const MAIN_NAV_ITEMS: Array<{
  screen: ScreenId;
  label: string;
  icon: AppIconName;
  badgeKey?: "documentsBadge" | "tasksBadge";
}> = [
  { screen: "dashboard", label: "Головна", icon: "dashboard" },
  { screen: "documents", label: "Документи", icon: "documents", badgeKey: "documentsBadge" },
  { screen: "counterparties", label: "Контрагенти", icon: "counterparties" },
  { screen: "payments", label: "Платежі", icon: "payments" },
  { screen: "reports", label: "Звіти", icon: "reports" },
  { screen: "tasks", label: "Завдання", icon: "tasks", badgeKey: "tasksBadge" }
];

export const SCREEN_TITLES: Record<ScreenId, string> = {
  dashboard: "Головна",
  documents: "Документи",
  counterparties: "Контрагенти",
  payments: "Платежі",
  reports: "Звіти",
  tasks: "Завдання",
  settings: "Налаштування"
};

export const DOCUMENT_KIND_META: Record<
  DocumentKind,
  { label: string; icon: AppIconName; actionLabel: string }
> = {
  invoice: { label: "Рахунок", icon: "invoice", actionLabel: "рахунок" },
  act: { label: "Акт", icon: "act", actionLabel: "акт" },
  waybill: { label: "Накладна", icon: "waybill", actionLabel: "накладну" }
};

export const DOCUMENT_KIND_OPTIONS = Object.entries(DOCUMENT_KIND_META).map(([value, meta]) => ({
  value: value as DocumentKind,
  label: meta.label
}));

export const DOCUMENT_KIND_FILTER_OPTIONS: Array<{ value: DocumentKind | null; label: string }> = [
  { value: null, label: "Всі" },
  { value: "act", label: "Акти" },
  { value: "invoice", label: "Рахунки" },
  { value: "waybill", label: "Накладні" }
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

export const TASK_PRIORITY_OPTIONS: Array<{ value: TaskPriority; label: string }> = [
  { value: "low", label: "Низький" },
  { value: "normal", label: "Звичайний" },
  { value: "high", label: "Високий" },
  { value: "critical", label: "Критичний" }
];

export const TASK_STATUS_OPTIONS: Array<{ value: TaskStatus; label: string }> = [
  { value: "open", label: "Відкрите" },
  { value: "in_progress", label: "В роботі" },
  { value: "done", label: "Виконано" },
  { value: "cancelled", label: "Скасовано" }
];

export const CALENDAR_FILTER_OPTIONS: Array<{ kind: PaymentCalendarFilterKind; label: string }> = [
  { kind: "all", label: "Усе" },
  { kind: "schedule", label: "Платежі" },
  { kind: "task", label: "Задачі" }
];

export const CALENDAR_EVENT_KIND_LABELS: Record<PaymentCalendarEventKind, string> = {
  schedule: "Платіж",
  task: "Задача"
};

export const PAYMENT_PREVIEW_COPY: Record<PaymentMatchDecisionKind, { title: string; description: string }> = {
  exact: {
    title: "Рекомендована звірка",
    description:
      "Система знайшла найкращий документ для автозіставлення. Перевірте рекомендацію перед підтвердженням."
  },
  ambiguous: {
    title: "Кілька кандидатів на звірку",
    description:
      "Оберіть найкращий варіант у списку, або відкрийте ручний пошук, якщо потрібен інший документ."
  },
  split: {
    title: "Рекомендований розподіл платежу",
    description:
      "Система підготувала рекомендований розподіл платежу між кількома документами. Перевірте кандидатів і, за потреби, скоригуйте суми в чернетці нижче."
  },
  none: {
    title: "Автоматична звірка не знайшла точного документа",
    description:
      "Для цього платежу поки немає точного збігу. Перевірте реквізити або відкрийте ручний пошук документа."
  }
};

export const PAYMENT_RECONCILE_MESSAGES: Record<PaymentMatchDecisionKind, string> = {
  exact: "Знайдено рекомендовану звірку. Перевірте та підтвердіть автозіставлення.",
  ambiguous:
    "Знайдено кілька кандидатів. Цей платіж потребує уваги, а ручне підтвердження буде наступним кроком.",
  split:
    "Знайдено рекомендований розподіл платежу між кількома документами. Перевірте алокації перед підтвердженням.",
  none: "Точний кандидат не знайдено. Перевірте платіж або підготуйте ручне звіряння."
};

export const PAYMENT_FLOW_COPY: Record<string, { title: string; description: string }> = {
  import: {
    title: "Імпорт триває",
    description: "Імпортуємо виписку та оновлюємо список платежів, щоб одразу показати незведені рухи."
  },
  "import-pick": {
    title: "Готуємо preview виписки",
    description: "Розбираємо файл виписки і будуємо список платежів, що чекають на імпорт."
  },
  "import-commit": {
    title: "Імпортуємо нові платежі",
    description: "Записуємо нові платежі у БД на основі підтвердженого preview."
  },
  sync: {
    title: "Оновлюємо рухи з банку",
    description: "Підтягуємо свіжі банківські рухи та готуємо їх до наступного кроку звірки."
  },
  reconcile: {
    title: "Готуємо preview звірки",
    description: "Шукаємо документи-кандидати й готуємо наступний крок для цього платежу."
  },
  "manual-search": {
    title: "Шукаємо документи для ручної звірки",
    description: "Формуємо повний список відкритих актів і накладних для ручного вибору."
  },
  unreconcile: {
    title: "Знімаємо зведення",
    description: "Знімаємо зв'язок із документом та повертаємо платіж у чергу на повторну звірку."
  },
  save: {
    title: "Зберігаємо платіж",
    description: "Фіксуємо зміни в картці платежу та оновлюємо список."
  },
  "confirm-auto-match": {
    title: "Підтверджуємо автозвірку",
    description: "Підтверджуємо рекомендоване автозіставлення і оновлюємо статус платежу."
  },
  "confirm-candidate": {
    title: "Підтверджуємо ручну звірку",
    description: "Прив'язуємо платіж до вибраного кандидата з preview."
  },
  "confirm-manual-picker": {
    title: "Фіксуємо ручний вибір документа",
    description: "Прив'язуємо платіж до документа, обраного через ручний пошук."
  },
  "confirm-split": {
    title: "Зберігаємо розподіл платежу",
    description: "Записуємо розподіл платежу між кількома документами і оновлюємо статуси."
  }
};

export const PAYMENT_MANUAL_PICKER_DISABLED_REASON =
  "Спершу знайдіть хоча б одного кандидата, щоб підтвердити документ.";

export const EDITOR_DIRTY_COPY = {
  dirtyTitle: "У вас є незбережені зміни",
  dirtyDescription: "Скасувати їх і закрити форму?",
  dirtyStay: "Залишитися",
  dirtyDiscard: "Так, закрити"
} as const;

export const PAYMENT_MANUAL_MATCH_COPY = {
  missingAutoMatch: "Немає рекомендованої звірки для автоматичного підтвердження.",
  previewCandidateUnavailable: "Ручне підтвердження доступне лише для preview з кількома кандидатами.",
  previewCandidateMissing: "Виберіть кандидата для підтвердження звірки.",
  previewCandidateSelected: "Кандидата вибрано. Ручне підтвердження буде наступним кроком.",
  manualSearchClosed: "Спершу відкрийте ручний пошук для цього платежу.",
  manualCandidateSelected: "Вибрано документ для ручного звіряння.",
  splitPickerClosed: "Спершу відкрийте ручний picker для розподілу платежу.",
  splitCandidateMissing: "Виберіть документ, який треба додати до розподілу.",
  splitCandidateDuplicate: "Цей документ уже додано до розподілу.",
  splitFullyAllocated: "Увесь платіж уже розподілено. За потреби змініть суми в чернетці.",
  splitDraftUpdated: "Документ додано до чернетки розподілу.",
  splitCandidateAdded: "Документ додано до розподілу",
  splitAmountInvalid: "Сума розподілу має бути числом у форматі 0,00.",
  splitAmountTooSmall: "Сума розподілу має бути більшою за нуль.",
  splitAmountAboveDocument: "Сума розподілу не може перевищувати залишок документа.",
  splitAmountAbovePayment: "Сума розподілу не може перевищувати залишок платежу.",
  splitDraftRemoved: "Документ прибрано з чернетки розподілу.",
  manualPickerClosed: "Ручний picker ще не відкрито.",
  manualPickerCandidateMissing: "Виберіть документ для ручного звіряння.",
  splitDraftMissing: "Немає чернетки розподілу для підтвердження.",
  splitDraftEmpty: "Додайте хоча б один документ до розподілу.",
  splitDraftIncomplete: "Розподіл ще не завершено. Закрийте залишок платежу або зменште суму."
} as const;

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

export const COUNTERPARTIES_COPY = {
  archiveConfirm: "Архівувати поточного контрагента? Повернення потребуватиме окремої дії.",
  loadingMessage: "Оновлюємо картку контрагента…",
  searchPlaceholder: "Пошук контрагента…",
  loadingTitle: "Завантажуємо картку контрагента",
  loadingDescription: "Список уже готується. Деталі з'являться тут, щойно підтягнемо перші дані.",
  emptyTitle: "Оберіть контрагента",
  emptyDescription:
    "Оберіть зліва вже відомого контрагента або створіть нового, щоб одразу побачити баланс, прострочки та сценарій роботи.",
  ...EDITOR_DIRTY_COPY
} as const;

export const REPORT_TABS: Array<{ id: ReportsTab; label: string }> = [
  { id: "bank", label: "Гроші на рахунках і в русі" },
  { id: "pnl", label: "Дохід, витрати і результат" },
  { id: "receivables", label: "Нам мають заплатити" },
  { id: "payables", label: "Ми маємо заплатити" }
];

export const REPORT_SCOPE_OPTIONS: Array<{ value: ReportsScope; label: string }> = [
  { value: "active", label: "Лише активну компанію" },
  { value: "all", label: "Усі компанії" }
];

export type ReportKpiCard = {
  label: string;
  value: string;
  tone?: "default" | "accent" | "warning" | "danger";
};

const REPORT_HEADLINES: Record<ReportsTab, string> = {
  bank: "Гроші на рахунках і в русі",
  pnl: "Дохід, витрати і результат",
  receivables: "Нам мають заплатити",
  payables: "Ми маємо заплатити"
};

const REPORT_TOP_COUNTERPARTIES_SUBTITLES: Record<ReportsTab, string> = {
  bank: "Сортовано за загальним рухом грошей. % — частка від лідера, поряд — чистий рух.",
  pnl: "Сортовано за внеском у фінрезультат. % — частка від лідера, поряд — чистий результат.",
  receivables:
    "Сортовано за сумою дебіторки. % — частка контрагента від лідера, поряд — сума до отримання за період.",
  payables:
    "Сортовано за сумою кредиторки. % — частка від лідера, поряд — сума до оплати за період."
};

const REPORT_EMPTY_CONTEXT: Record<ReportsTab, string> = {
  bank: "Показано: загальний рух грошей по всіх контрагентах",
  pnl: "Показано: загальний фінрезультат по всіх контрагентах",
  receivables: "Показано: загальна дебіторка по всіх контрагентах",
  payables: "Показано: загальна кредиторка по всіх контрагентах"
};

const REPORT_FOCUSED_CONTEXT: Record<ReportsTab, (name: string) => string> = {
  bank: (name) => `Показано: рух грошей по контрагенту ${name}`,
  pnl: (name) => `Показано: фінрезультат по контрагенту ${name}`,
  receivables: (name) => `Показано: дебіторка по контрагенту ${name}`,
  payables: (name) => `Показано: кредиторка по контрагенту ${name}`
};

const reportSortCollator = new Intl.Collator("uk", {
  numeric: true,
  sensitivity: "base"
});

function compareReportStrings(left: string, right: string): number {
  return reportSortCollator.compare(left || "", right || "");
}

function compareReportDates(left: string, right: string): number {
  return compareReportStrings(left || "9999-12-31", right || "9999-12-31");
}

function compareAmountStrDesc(leftStr: string, rightStr: string): number {
  return compareMinor(parseMoneyToMinor(rightStr) ?? 0n, parseMoneyToMinor(leftStr) ?? 0n);
}

function stableSortRows<T>(rows: T[], compare: (left: T, right: T) => number): T[] {
  return rows
    .map((row, index) => ({ row, index }))
    .sort((left, right) => {
      const result = compare(left.row, right.row);
      return result !== 0 ? result : left.index - right.index;
    })
    .map(({ row }) => row);
}

function overdueReceivables(rows: ReceivableRowDto[]): ReceivableRowDto[] {
  return rows.filter((row) => row.overdueDays > 0);
}

function overduePayables(rows: PayableRowDto[]): PayableRowDto[] {
  return rows.filter((row) => row.overdueDays > 0);
}

function dueSoonCount(rows: Array<{ expectedDate?: string; dueDate?: string }>, field: "expectedDate" | "dueDate") {
  return rows.filter((row) => {
    const days = daysUntil(row[field] ?? "");
    return days !== null && days >= 0 && days <= 7;
  }).length;
}

function uniqueCounterpartiesCount(rows: Array<{ counterparty: string }>): number {
  return new Set(rows.map((row) => row.counterparty || "—")).size;
}

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

export function formatLastContactLabel(days: number): string {
  if (days <= 0) {
    return "сьогодні";
  }
  if (days === 1) {
    return "1 день тому";
  }
  if (days >= 2 && days <= 4) {
    return `${days} дні тому`;
  }
  return `${days} днів тому`;
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

export function formatCalendarEventsLabel(count: number): string {
  if (count === 0) {
    return "без подій";
  }
  if (count === 1) {
    return "1 подія";
  }
  return `${count} подій`;
}

export function formatCalendarDayAriaLabel(args: {
  date: string;
  eventCount: number;
  today: boolean;
  selected: boolean;
}): string {
  const todayLabel = args.today ? ", сьогодні" : "";
  const selectedLabel = args.selected ? ", вибрано" : "";
  return `${args.date}${todayLabel}${selectedLabel}, ${formatCalendarEventsLabel(args.eventCount)}`;
}

export function getCalendarEventDirectionLabel(direction: string): string {
  return direction === "income" ? "Надходження" : "Витрата";
}

export function sortReceivablesRows(rows: ReceivableRowDto[]): ReceivableRowDto[] {
  return stableSortRows(rows, (left, right) => {
    if (left.overdueDays !== right.overdueDays) {
      return right.overdueDays - left.overdueDays;
    }

    const dueDateOrder = compareReportDates(left.expectedDate, right.expectedDate);
    if (dueDateOrder !== 0) {
      return dueDateOrder;
    }

    const amountOrder = compareAmountStrDesc(left.amountStr, right.amountStr);
    if (amountOrder !== 0) {
      return amountOrder;
    }

    const counterpartyOrder = compareReportStrings(left.counterparty, right.counterparty);
    if (counterpartyOrder !== 0) {
      return counterpartyOrder;
    }

    return compareReportStrings(left.docNumber, right.docNumber);
  });
}

export function sortPayablesRows(rows: PayableRowDto[]): PayableRowDto[] {
  return stableSortRows(rows, (left, right) => {
    if (left.overdueDays !== right.overdueDays) {
      return right.overdueDays - left.overdueDays;
    }

    const dueDateOrder = compareReportDates(left.dueDate, right.dueDate);
    if (dueDateOrder !== 0) {
      return dueDateOrder;
    }

    const amountOrder = compareAmountStrDesc(left.amountStr, right.amountStr);
    if (amountOrder !== 0) {
      return amountOrder;
    }

    const counterpartyOrder = compareReportStrings(left.counterparty, right.counterparty);
    if (counterpartyOrder !== 0) {
      return counterpartyOrder;
    }

    return compareReportStrings(left.title, right.title);
  });
}

export function getReportHeadline(tab: ReportsTab | undefined): string {
  return REPORT_HEADLINES[tab ?? "bank"];
}

export function getReportTopCounterpartiesSubtitle(tab: ReportsTab | undefined): string {
  return REPORT_TOP_COUNTERPARTIES_SUBTITLES[tab ?? "bank"];
}

export function getReportActiveRowsCount(screen: ReportsScreenDto | null): number {
  if (!screen) {
    return 0;
  }
  if (screen.filter.tab === "pnl") {
    return screen.pnlRows?.length ?? 0;
  }
  if (screen.filter.tab === "receivables") {
    return screen.receivablesRows.length;
  }
  if (screen.filter.tab === "payables") {
    return screen.payablesRows.length;
  }
  return screen.bankRows.length;
}

export function getReportContextText(screen: ReportsScreenDto | null): string {
  const tab = screen?.filter.tab ?? "bank";
  const selected = screen?.selectedCounterparty;
  return selected ? REPORT_FOCUSED_CONTEXT[tab](selected.name) : REPORT_EMPTY_CONTEXT[tab];
}

export function hasReportActiveRows(screen: ReportsScreenDto | null, tab: ReportsTab | undefined): boolean {
  if (tab === "pnl") {
    return (screen?.pnlRows?.length ?? 0) > 0;
  }
  if (tab === "receivables") {
    return (screen?.receivablesRows?.length ?? 0) > 0;
  }
  if (tab === "payables") {
    return (screen?.payablesRows?.length ?? 0) > 0;
  }
  return (screen?.bankRows?.length ?? 0) > 0;
}

export function getReportKpiCards(screen: ReportsScreenDto | null, tab: ReportsTab | undefined): ReportKpiCard[] {
  if (tab === "pnl") {
    return [
      {
        label: "Дохід за період",
        value: screen?.summary.pnlIncomeStr ?? "0,00 грн",
        tone: "accent"
      },
      {
        label: "Витрати за період",
        value: screen?.summary.pnlExpenseStr ?? "0,00 грн"
      },
      {
        label: "Фінансовий результат за період",
        value: screen?.summary.pnlNetResultStr ?? "0,00 грн",
        tone: "warning"
      },
      {
        label: "Категорій у звіті",
        value: `${screen?.pnlRows?.length ?? 0}`
      }
    ];
  }

  if (tab === "receivables") {
    const rows = sortReceivablesRows(screen?.receivablesRows ?? []);
    const overdueCount = overdueReceivables(rows).length;
    const dueSoon = dueSoonCount(rows, "expectedDate");
    return [
      {
        label: "Очікуємо отримати",
        value: screen?.summary.receivablesTotalStr ?? "0,00 грн",
        tone: "accent"
      },
      {
        label: "Прострочені оплати",
        value: `${overdueCount}`,
        tone: overdueCount > 0 ? "danger" : "default"
      },
      {
        label: "Оплати цього тижня",
        value: `${dueSoon}`,
        tone: dueSoon > 0 ? "warning" : "default"
      },
      {
        label: "Контрагентів у роботі",
        value: `${uniqueCounterpartiesCount(rows)}`
      }
    ];
  }

  if (tab === "payables") {
    const rows = sortPayablesRows(screen?.payablesRows ?? []);
    const overdueCount = overduePayables(rows).length;
    const dueSoon = dueSoonCount(rows, "dueDate");
    return [
      {
        label: "Заплановано до оплати",
        value: screen?.summary.payablesTotalStr ?? "0,00 грн",
        tone: "accent"
      },
      {
        label: "Прострочені виплати",
        value: `${overdueCount}`,
        tone: overdueCount > 0 ? "danger" : "default"
      },
      {
        label: "Виплати цього тижня",
        value: `${dueSoon}`,
        tone: dueSoon > 0 ? "warning" : "default"
      },
      {
        label: "Регулярних платежів",
        value: `${rows.filter((row) => row.recurrence && row.recurrence !== "—").length}`
      }
    ];
  }

  return [
    {
      label: "Залишок на початок",
      value: screen?.summary.openingBalanceStr ?? "0,00 грн"
    },
    {
      label: "Надходження за період",
      value: screen?.summary.incomeStr ?? "0,00 грн",
      tone: "accent"
    },
    {
      label: "Виплати за період",
      value: screen?.summary.expenseStr ?? "0,00 грн"
    },
    {
      label: "Залишок на кінець",
      value: screen?.summary.closingBalanceStr ?? "0,00 грн",
      tone: "warning"
    }
  ];
}
