import { get, writable } from "svelte/store";
import { reportsExportCsv, reportsExportExcel, reportsExportExcelAndOpen, reportsLoad } from "../api";
import type { ReportsFilterDto, ReportsScreenDto } from "../types";

interface ReportsState {
  screen: ReportsScreenDto | null;
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
  query: ""
};

const initialState: ReportsState = {
  screen: null,
  loading: false,
  error: null,
  message: null
};

function createReportsStore() {
  const { subscribe, update } = writable<ReportsState>(initialState);

  return {
    subscribe,
    async load(partial?: Partial<ReportsFilterDto>) {
      const filter = {
        ...(get({ subscribe }).screen?.filter ?? defaultFilter),
        ...(partial ?? {})
      };

      update((state) => ({ ...state, loading: true, error: null }));

      try {
        const screen = await reportsLoad(filter);
        update((state) => ({ ...state, screen, loading: false }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
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
