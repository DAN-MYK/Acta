import { get } from "svelte/store";
import { vi } from "vitest";

export const invokeMock = vi.fn<(command: string, payload?: unknown) => Promise<unknown>>();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock
}));

export function snapshot<T>(store: { subscribe: (run: (value: T) => void) => () => void }): T {
  return get(store as never) as T;
}

export async function loadStores() {
  const [shellModule, dashboardModule, documentsModule, counterpartiesModule, paymentsModule, settingsModule, paletteModule, navigationModule] =
    await Promise.all([
      import("../shell"),
      import("../dashboard"),
      import("../documents"),
      import("../counterparties"),
      import("../payments"),
      import("../settings"),
      import("../palette"),
      import("../navigation")
    ]);

  return {
    shellStore: shellModule.shellStore,
    dashboardStore: dashboardModule.dashboardStore,
    documentsStore: documentsModule.documentsStore,
    counterpartiesStore: counterpartiesModule.counterpartiesStore,
    paymentsStore: paymentsModule.paymentsStore,
    settingsStore: settingsModule.settingsStore,
    paletteStore: paletteModule.paletteStore,
    navigationStore: navigationModule.navigationStore
  };
}
