import { writable } from "svelte/store";
import { importBasPlan, importBasExecute, importBasPickDirectory } from "../api";
import type { ImportPlanDto, ImportResultDto } from "../types";

interface ImportState {
  selectedDirectory: string | null;
  plan: ImportPlanDto | null;
  result: ImportResultDto | null;
  loading: boolean;
  error: string | null;
}

function createImportStore() {
  const { subscribe, update, set } = writable<ImportState>({
    selectedDirectory: null,
    plan: null,
    result: null,
    loading: false,
    error: null
  });

  async function chooseDirectory() {
    update((state) => ({ ...state, loading: true, error: null }));
    try {
      const selectedDirectory = await importBasPickDirectory();
      update((state) => ({
        ...state,
        selectedDirectory,
        plan: null,
        result: null,
        loading: false
      }));
    } catch (error) {
      update((state) => ({ ...state, loading: false, error: String(error) }));
    }
  }

  async function fetchPlan() {
    let selectedDirectory: string | null = null;
    update((state) => {
      selectedDirectory = state.selectedDirectory;
      if (!selectedDirectory) {
        return {
          ...state,
          error: "Спочатку оберіть папку з файлами BAS."
        };
      }

      return { ...state, loading: true, error: null, result: null };
    });

    if (!selectedDirectory) {
      return;
    }

    try {
      const data = await importBasPlan(selectedDirectory);
      update((state) => ({ ...state, plan: data, loading: false }));
    } catch (error) {
      update((state) => ({ ...state, loading: false, error: String(error) }));
    }
  }

  async function execute() {
    let selectedDirectory: string | null = null;
    update((state) => {
      selectedDirectory = state.selectedDirectory;
      if (!selectedDirectory) {
        return {
          ...state,
          loading: false,
          error: "Спочатку оберіть папку з файлами BAS."
        };
      }

      return { ...state, loading: true, error: null };
    });

    if (!selectedDirectory) {
      return;
    }

    try {
      const data = await importBasExecute(selectedDirectory);
      update((state) => ({ ...state, result: data, loading: false }));
    } catch (error) {
      update((state) => ({ ...state, loading: false, error: String(error) }));
    }
  }

  function reset() {
    set({ selectedDirectory: null, plan: null, result: null, loading: false, error: null });
  }

  return { subscribe, chooseDirectory, fetchPlan, execute, reset };
}

export const importStore = createImportStore();
