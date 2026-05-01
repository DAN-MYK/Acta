import { writable } from "svelte/store";
import { dashboardLoad } from "../api";
import type { DashboardScreenDto } from "../types";

interface DashboardState {
  screen: DashboardScreenDto | null;
  initialLoading: boolean;
  loading: boolean;
  error: string | null;
}

const initialState: DashboardState = {
  screen: null,
  initialLoading: true,
  loading: false,
  error: null
};

function createDashboardStore() {
  const { subscribe, update } = writable<DashboardState>(initialState);
  let latestRequestId = 0;

  return {
    subscribe,
    async load() {
      const requestId = ++latestRequestId;
      update((state) => ({ ...state, loading: true, error: null }));

      try {
        const screen = await dashboardLoad();
        if (requestId === latestRequestId) {
          update((state) => ({ ...state, screen, initialLoading: false, loading: false }));
        }
        return screen;
      } catch (error) {
        if (requestId === latestRequestId) {
          update((state) => ({ ...state, loading: false, error: String(error) }));
        }
        return null;
      }
    }
  };
}

export const dashboardStore = createDashboardStore();
