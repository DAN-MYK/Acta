import { writable } from "svelte/store";
import { shellLoad, shellSetActiveCompany } from "../api";
import type { ShellStateDto } from "../types";

type ShellLoadingPhase = "idle" | "initial" | "refresh" | "company-switch";

type ShellStoreState = {
  state: ShellStateDto | null;
  loading: boolean;
  error: string | null;
  phase: ShellLoadingPhase;
  pendingCompanyId: string | null;
  progressLabel: string | null;
};

type ActiveRequestMeta =
  | {
      kind: "load";
    }
  | {
      kind: "company-switch";
      companyId: string;
    };

function createInitialState(): ShellStoreState {
  return {
    state: null,
    loading: false,
    error: null,
    phase: "idle",
    pendingCompanyId: null,
    progressLabel: null
  };
}

function toErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function createShellStore() {
  const { subscribe, set, update } = writable<ShellStoreState>(createInitialState());
  let snapshot = createInitialState();
  let activeRequest: Promise<ShellStateDto | null> | null = null;
  let activeRequestMeta: ActiveRequestMeta | null = null;
  let requestGeneration = 0;

  function commit(nextState: ShellStoreState) {
    snapshot = nextState;
    set(nextState);
  }

  function patch(updater: (state: ShellStoreState) => ShellStoreState) {
    update((state) => {
      snapshot = updater(state);
      return snapshot;
    });
  }

  return {
    subscribe,
    reset() {
      requestGeneration += 1;
      activeRequest = null;
      activeRequestMeta = null;
      commit(createInitialState());
    },
    async load(): Promise<ShellStateDto | null> {
      if (snapshot.loading && activeRequest) {
        if (activeRequestMeta?.kind === "load") {
          return activeRequest;
        }

        await activeRequest;
        return this.load();
      }

      const phase: ShellLoadingPhase = snapshot.state ? "refresh" : "initial";
      patch((state) => ({
        ...state,
        loading: true,
        error: null,
        phase,
        pendingCompanyId: null,
        progressLabel: phase === "initial" ? "Завантажуємо робочий простір..." : "Оновлюємо shell..."
      }));
      const generation = requestGeneration;

      activeRequest = (async () => {
        try {
          const state = await shellLoad();
          if (generation !== requestGeneration) {
            return null;
          }
          commit({
            state,
            loading: false,
            error: null,
            phase: "idle",
            pendingCompanyId: null,
            progressLabel: null
          });
          return state;
        } catch (error) {
          if (generation !== requestGeneration) {
            return null;
          }
          commit({
            ...snapshot,
            loading: false,
            error: toErrorMessage(error),
            phase: "idle",
            pendingCompanyId: null,
            progressLabel: null
          });
          return null;
        } finally {
          activeRequest = null;
          activeRequestMeta = null;
        }
      })();
      activeRequestMeta = { kind: "load" };

      try {
        return await activeRequest;
      } finally {
        activeRequest = null;
        activeRequestMeta = null;
      }
    },
    async setActiveCompany(companyId: string): Promise<ShellStateDto | null> {
      if (snapshot.loading && activeRequest) {
        if (activeRequestMeta?.kind === "company-switch" && activeRequestMeta.companyId === companyId) {
          return activeRequest;
        }

        await activeRequest;
        return this.setActiveCompany(companyId);
      }

      if (snapshot.state?.activeCompanyId === companyId) {
        return snapshot.state;
      }

      patch((state) => ({
        ...state,
        loading: true,
        error: null,
        phase: "company-switch",
        pendingCompanyId: companyId,
        progressLabel: "Оновлюємо дані активної компанії..."
      }));
      const generation = requestGeneration;

      activeRequest = (async () => {
        try {
          const state = await shellSetActiveCompany(companyId);
          if (generation !== requestGeneration) {
            return null;
          }
          commit({
            state,
            loading: false,
            error: null,
            phase: "idle",
            pendingCompanyId: null,
            progressLabel: null
          });
          return state;
        } catch (error) {
          if (generation !== requestGeneration) {
            return null;
          }
          patch((state) => ({
            ...state,
            loading: false,
            error: toErrorMessage(error),
            phase: "idle",
            pendingCompanyId: null,
            progressLabel: null
          }));
          return null;
        } finally {
          activeRequest = null;
          activeRequestMeta = null;
        }
      })();
      activeRequestMeta = {
        kind: "company-switch",
        companyId
      };

      try {
        return await activeRequest;
      } finally {
        activeRequest = null;
        activeRequestMeta = null;
      }
    }
  };
}

export const shellStore = createShellStore();
