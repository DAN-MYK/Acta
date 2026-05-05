import { get, writable } from "svelte/store";
import { reportsExportCsv, reportsExportExcel, reportsExportExcelAndOpen, reportsLoad } from "../api";
import { compareMinor, parseMoneyToMinor } from "../money";
import type { PayableRowDto, ReceivableRowDto, ReportsFilterDto, ReportsScreenDto } from "../types";

interface ReportsState {
  screen: ReportsScreenDto | null;
  initialLoading: boolean;
  loading: boolean;
  error: string | null;
  message: string | null;
}

const sortCollator = new Intl.Collator("uk", {
  numeric: true,
  sensitivity: "base"
});

const today = new Date();
const defaultTo = today.toISOString().slice(0, 10);
const defaultFrom = new Date(today.getTime() - 89 * 24 * 60 * 60 * 1000)
  .toISOString()
  .slice(0, 10);

const defaultFilter: ReportsFilterDto = {
  tab: "bank",
  scope: "active",
  dateFrom: defaultFrom,
  dateTo: defaultTo,
  query: "",
  selectedCounterpartyId: null
};

const initialState: ReportsState = {
  screen: null,
  initialLoading: true,
  loading: false,
  error: null,
  message: null
};

function compareStrings(left: string, right: string): number {
  return sortCollator.compare(left || "", right || "");
}

function compareDates(left: string, right: string): number {
  return compareStrings(left || "9999-12-31", right || "9999-12-31");
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

function compareReceivables(left: ReceivableRowDto, right: ReceivableRowDto): number {
  if (left.overdueDays !== right.overdueDays) {
    return right.overdueDays - left.overdueDays;
  }

  const dueDateOrder = compareDates(left.expectedDate, right.expectedDate);
  if (dueDateOrder !== 0) {
    return dueDateOrder;
  }

  const amountOrder = compareAmountStrDesc(left.amountStr, right.amountStr);
  if (amountOrder !== 0) {
    return amountOrder;
  }

  const counterpartyOrder = compareStrings(left.counterparty, right.counterparty);
  if (counterpartyOrder !== 0) {
    return counterpartyOrder;
  }

  return compareStrings(left.docNumber, right.docNumber);
}

function comparePayables(left: PayableRowDto, right: PayableRowDto): number {
  if (left.overdueDays !== right.overdueDays) {
    return right.overdueDays - left.overdueDays;
  }

  const dueDateOrder = compareDates(left.dueDate, right.dueDate);
  if (dueDateOrder !== 0) {
    return dueDateOrder;
  }

  const amountOrder = compareAmountStrDesc(left.amountStr, right.amountStr);
  if (amountOrder !== 0) {
    return amountOrder;
  }

  const counterpartyOrder = compareStrings(left.counterparty, right.counterparty);
  if (counterpartyOrder !== 0) {
    return counterpartyOrder;
  }

  return compareStrings(left.title, right.title);
}

function normalizeScreen(screen: ReportsScreenDto): ReportsScreenDto {
  return {
    ...screen,
    receivablesRows: stableSortRows(screen.receivablesRows, compareReceivables),
    payablesRows: stableSortRows(screen.payablesRows, comparePayables)
  };
}

function shouldResetCounterparty(partial?: Partial<ReportsFilterDto>): boolean {
  return Boolean(
    partial &&
      ("tab" in partial ||
        "scope" in partial ||
        "dateFrom" in partial ||
        "dateTo" in partial ||
        "query" in partial)
  );
}

export function createReportsStore() {
  const { subscribe, update } = writable<ReportsState>(initialState);
  let latestLoadRequestId = 0;

  return {
    subscribe,
    async load(partial?: Partial<ReportsFilterDto>) {
      const requestId = ++latestLoadRequestId;
      const normalizedPartial =
        shouldResetCounterparty(partial) && !("selectedCounterpartyId" in (partial ?? {}))
          ? { ...partial, selectedCounterpartyId: null }
          : (partial ?? {});
      const filter = {
        ...(get({ subscribe }).screen?.filter ?? defaultFilter),
        ...normalizedPartial
      };

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const screen = normalizeScreen(await reportsLoad(filter));
        if (requestId === latestLoadRequestId) {
          update((state) => ({ ...state, screen, initialLoading: false, loading: false }));
        }
      } catch (error) {
        if (requestId === latestLoadRequestId) {
          update((state) => ({ ...state, loading: false, error: String(error) }));
        }
      }
    },
    async toggleCounterparty(counterpartyId: string) {
      const currentId = get({ subscribe }).screen?.filter.selectedCounterpartyId ?? null;
      await this.load({ selectedCounterpartyId: currentId === counterpartyId ? null : counterpartyId });
    },
    async resetFilters() {
      const tab = get({ subscribe }).screen?.filter.tab ?? defaultFilter.tab;
      await this.load({
        tab,
        scope: defaultFilter.scope,
        dateFrom: defaultFilter.dateFrom,
        dateTo: defaultFilter.dateTo,
        query: defaultFilter.query,
        selectedCounterpartyId: null
      });
    },
    async exportCsv() {
      const filter = get({ subscribe }).screen?.filter ?? defaultFilter;
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await reportsExportCsv(filter);
        update((state) => ({
          ...state,
          loading: false,
          message: `${result.message}: ${result.path}`
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async exportExcel() {
      const filter = get({ subscribe }).screen?.filter ?? defaultFilter;
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await reportsExportExcel(filter);
        update((state) => ({
          ...state,
          loading: false,
          message: `${result.message}: ${result.path}`
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async exportExcelAndOpen() {
      const filter = get({ subscribe }).screen?.filter ?? defaultFilter;
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await reportsExportExcelAndOpen(filter);
        update((state) => ({
          ...state,
          loading: false,
          message: `${result.message}: ${result.path}`
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    }
  };
}

export const reportsStore = createReportsStore();
