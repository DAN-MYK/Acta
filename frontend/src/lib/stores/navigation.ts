import { writable } from "svelte/store";
import type { ScreenId } from "../types";

function createNavigationStore() {
  const { subscribe, set } = writable<ScreenId>("documents");

  return {
    subscribe,
    go(screen: ScreenId) {
      set(screen);
    }
  };
}

export const navigationStore = createNavigationStore();
