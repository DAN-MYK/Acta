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

  let searchToken = 0;

  function buildClosedState() {
    return {
      open: false,
      query: "",
      items: [],
      loading: false,
      error: null
    };
  }

  function primeDefaultResults() {
    void search("");
  }

  async function search(query: string) {
    const currentToken = ++searchToken;
    update((state) => ({ ...state, query, loading: true, error: null }));

    try {
      const counterparties = getCounterpartiesSnapshot();
      const result = await shellPaletteSearch(query, counterparties.selectedId ?? undefined);

      update((state) => {
        if (currentToken !== searchToken) {
          return {
            ...state,
            loading: false
          };
        }

        return {
          ...state,
          items: result.items,
          loading: false
        };
      });
    } catch (error) {
      update((state) => {
        if (currentToken !== searchToken) {
          return {
            ...state,
            loading: false
          };
        }

        return {
          ...state,
          loading: false,
          error: String(error)
        };
      });
    }
  }

  return {
    subscribe,
    open() {
      searchToken += 1;
      set({
        ...buildClosedState(),
        open: true
      });
      primeDefaultResults();
    },
    toggle() {
      let shouldPrime = false;

      update((state) => {
        if (state.open) {
          searchToken += 1;
          return buildClosedState();
        }

        shouldPrime = true;
        return {
          ...buildClosedState(),
          open: true
        };
      });

      if (shouldPrime) {
        primeDefaultResults();
      }
    },
    close() {
      searchToken += 1;
      set(buildClosedState());
    },
    search,
    async activate(payload: string) {
      const counterparties = getCounterpartiesSnapshot();
      const result = await shellPaletteActivate(payload, counterparties.selectedId ?? undefined);
      applyActivation(result);
      return result;
    },
    reset() {
      searchToken += 1;
      set(buildClosedState());
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
