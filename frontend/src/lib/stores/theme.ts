import { writable } from "svelte/store";

function createThemeStore() {
  const { subscribe, update } = writable<"light" | "dark">("light");

  return {
    subscribe,
    setMode(mode: "light" | "dark") {
      update(() => mode);
    },
    toggle() {
      update((mode) => (mode === "light" ? "dark" : "light"));
    }
  };
}

export const themeStore = createThemeStore();
