import { writable } from "svelte/store";
import { shellLoad, shellSetActiveCompany } from "../api";
import type { ShellStateDto } from "../types";

function createShellStore() {
  const { subscribe, set, update } = writable<{
    state: ShellStateDto | null;
    loading: boolean;
    error: string | null;
  }>({
    state: null,
    loading: false,
    error: null
  });

  return {
    subscribe,
    async load() {
      update((state) => ({ ...state, loading: true, error: null }));
      try {
        const state = await shellLoad();
        set({ state, loading: false, error: null });
        return state;
      } catch (error) {
        set({ state: null, loading: false, error: String(error) });
        return null;
      }
    },
    async setActiveCompany(companyId: string) {
      update((state) => ({ ...state, loading: true, error: null }));
      try {
        const state = await shellSetActiveCompany(companyId);
        set({ state, loading: false, error: null });
        return state;
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
        return null;
      }
    }
  };
}

export const shellStore = createShellStore();
