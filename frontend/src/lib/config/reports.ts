import { daysUntil } from "../date";
import { compareMinor, parseMoneyToMinor } from "../money";
import type { PayableRowDto, ReceivableRowDto, ReportsScreenDto, ReportsScope, ReportsTab } from "../types";

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

export function formatDaysLabel(count: number): string {
  const absoluteCount = Math.abs(count);
  const lastTwoDigits = absoluteCount % 100;
  const lastDigit = absoluteCount % 10;

  if (lastTwoDigits >= 11 && lastTwoDigits <= 14) {
    return `${count} днів`;
  }

  if (lastDigit === 1) {
    return `${count} день`;
  }

  if (lastDigit >= 2 && lastDigit <= 4) {
    return `${count} дні`;
  }

  return `${count} днів`;
}

export function formatOverdueDaysLabel(days: number): string {
  if (days <= 0) {
    return "Без прострочки";
  }

  return `Прострочено ${formatDaysLabel(days)}`;
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
