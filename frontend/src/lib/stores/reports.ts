import { get, writable } from "svelte/store";
import { reportsExportCsv, reportsExportExcel, reportsExportExcelAndOpen, reportsLoad } from "../api";
import { sortPayablesRows, sortReceivablesRows } from "../config/ui";
import type { ReportsFilterDto, ReportsScreenDto } from "../types";

interface ReportsState {
  screen: ReportsScreenDto | null;
  initialLoading: boolean;
  loading: boolean;
  error: string | null;
  message: string | null;
}

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

function normalizeScreen(screen: ReportsScreenDto): ReportsScreenDto {
  return {
    ...screen,
    receivablesRows: sortReceivablesRows(screen.receivablesRows),
    payablesRows: sortPayablesRows(screen.payablesRows)
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
  const { subscribe, set, update } = writable<ReportsState>(initialState);
  let latestLoadRequestId = 0;

  return {
    subscribe,
    reset() {
      latestLoadRequestId += 1;
      set(initialState);
    },
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
