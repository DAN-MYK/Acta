import { writable } from "svelte/store";
import { shellPaletteActivate, shellPaletteSearch } from "../api";
import { counterpartiesStore } from "./counterparties";
import { navigationStore } from "./navigation";
import { documentsStore } from "./documents";
import type { PaletteActivationResultDto, PaletteItemDto } from "../types";

function createPaletteStore() {
  const { subscribe, update, set } = writable<{
    open: boolean;
    query: string;
    items: PaletteItemDto[];
    loading: boolean;
    error: string | null;
  }>({
    open: false,
    query: "",
    items: [],
    loading: false,
    error: null
  });

  return {
    subscribe,
    toggle() {
      update((state) => ({ ...state, open: !state.open }));
    },
    close() {
      update((state) => ({ ...state, open: false }));
    },
    async search(query: string) {
      update((state) => ({ ...state, query, loading: true, error: null }));
      try {
        const counterparties = getCounterpartiesSnapshot();
        const result = await shellPaletteSearch(query, counterparties.selectedId ?? undefined);
        update((state) => ({
          ...state,
          items: result.items,
          loading: false
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async activate(payload: string) {
      const counterparties = getCounterpartiesSnapshot();
      const result = await shellPaletteActivate(payload, counterparties.selectedId ?? undefined);
      applyActivation(result);
      return result;
    },
    reset() {
      set({
        open: false,
        query: "",
        items: [],
        loading: false,
        error: null
      });
    }
  };
}

function getCounterpartiesSnapshot() {
  let snapshot = {
    selectedId: null as string | null
  };
  const unsubscribe = counterpartiesStore.subscribe((state) => {
    snapshot = {
      selectedId: state.selectedId
    };
  });
  unsubscribe();
  return snapshot;
}

function applyActivation(result: PaletteActivationResultDto) {
  if (result.screen) {
    navigationStore.go(result.screen);
  }

  if (result.documentEditor) {
    documentsStore.setEditor(result.documentEditor);
  } else if (result.documentId) {
    void documentsStore.open(result.documentId);
  }

  if (result.counterpartyId) {
    void counterpartiesStore.open(result.counterpartyId);
  }

  if (result.kind === "open_counterparty_create") {
    void counterpartiesStore.openEditor();
  }
}

export const paletteStore = createPaletteStore();
