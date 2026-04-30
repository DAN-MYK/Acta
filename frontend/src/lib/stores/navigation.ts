import { writable } from "svelte/store";
import type { ScreenId } from "../types";

function createNavigationStore() {
  const { subscribe, set } = writable<ScreenId>("dashboard");

  return {
    subscribe,
    go(screen: ScreenId) {
      set(screen);
    }
  };
}

export const navigationStore = createNavigationStore();
