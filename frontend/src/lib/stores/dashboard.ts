import { writable } from "svelte/store";
import { dashboardLoad } from "../api";
import type { DashboardScreenDto } from "../types";

interface DashboardState {
  screen: DashboardScreenDto | null;
  loading: boolean;
  error: string | null;
}

const initialState: DashboardState = {
  screen: null,
  loading: false,
  error: null
};

function createDashboardStore() {
  const { subscribe, update } = writable<DashboardState>(initialState);

  return {
    subscribe,
    async load() {
      update((state) => ({ ...state, loading: true, error: null }));

      try {
        const screen = await dashboardLoad();
        update((state) => ({ ...state, screen, loading: false }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    }
  };
}

export const dashboardStore = createDashboardStore();
