import { writable } from "svelte/store";
import { importBasPlan, importBasExecute } from "../api";
import type { ImportPlanDto, ImportResultDto } from "../types";

interface ImportState {
  plan: ImportPlanDto | null;
  result: ImportResultDto | null;
  loading: boolean;
  error: string | null;
}

function createImportStore() {
  const { subscribe, update, set } = writable<ImportState>({
    plan: null,
    result: null,
    loading: false,
    error: null
  });

  async function plan() {
    update((state) => ({ ...state, loading: true, error: null }));
    try {
      const data = await importBasPlan();
      update((state) => ({ ...state, plan: data, loading: false }));
    } catch (error) {
      update((state) => ({ ...state, loading: false, error: String(error) }));
    }
  }

  async function execute() {
    update((state) => ({ ...state, loading: true, error: null }));
    try {
      const data = await importBasExecute();
      update((state) => ({ ...state, result: data, loading: false }));
    } catch (error) {
      update((state) => ({ ...state, loading: false, error: String(error) }));
    }
  }

  function reset() {
    set({ plan: null, result: null, loading: false, error: null });
  }

  return { subscribe, plan, execute, reset };
}

export const importStore = createImportStore();
