/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import App from "../App.svelte";
import type { ScreenId, ShellStateDto } from "../lib/types";

const mocks = vi.hoisted(() => {
  function createMockStore<T>(initialValue: T) {
    let value = initialValue;
    const subscribers = new Set<(value: T) => void>();

    return {
      subscribe(run: (value: T) => void) {
        run(value);
        subscribers.add(run);
        return () => subscribers.delete(run);
      },
      set(nextValue: T) {
        value = nextValue;
        for (const run of subscribers) {
          run(value);
        }
      },
      get() {
        return value;
      }
    };
  }

  function makeShellState(activeCompanyId = "company-1"): ShellStateDto {
    return {
      chrome: {
        companyName: activeCompanyId === "company-2" ? "ФОП Тест" : "ТОВ Акт",
        userName: "Олена",
        userInitials: "ОО",
        userRole: "Адміністратор",
        documentsBadge: 4,
        tasksBadge: 2
      },
      companyItems: [
        {
          id: "company-1",
          name: "ТОВ Акт",
          subtitle: "Основна компанія",
          initials: "ТА",
          badge: "",
          active: activeCompanyId === "company-1"
        },
        {
          id: "company-2",
          name: "ФОП Тест",
          subtitle: "Друга компанія",
          initials: "ФТ",
          badge: "",
          active: activeCompanyId === "company-2"
        }
      ],
      activeCompanyId,
      isDark: activeCompanyId === "company-2"
    };
  }

  const shellState = createMockStore({
    state: makeShellState(),
    loading: false,
    error: null as string | null,
    phase: "idle" as "idle" | "initial" | "refresh" | "company-switch",
    pendingCompanyId: null as string | null,
    progressLabel: null as string | null
  });

  const appShellState = createMockStore({
    loading: false,
    phase: "idle",
    progressLabel: null as string | null
  });

  const paletteState = createMockStore({
    open: false,
    query: "",
    items: [] as Array<{ title: string; subtitle: string; shortcut: string; payload: string; kind: string }>,
    loading: false,
    error: null as string | null
  });

  const themeState = createMockStore("light");
  const navigationState = createMockStore("dashboard" as ScreenId);

  const appShellBootstrap = vi.fn().mockResolvedValue(undefined);
  const appShellSwitchActiveCompany = vi.fn();
  const appShellReloadShellChrome = vi.fn().mockResolvedValue(makeShellState());
  const appShellSyncThemeFromSettings = vi.fn();
  const paletteSearch = vi.fn(async (query: string) => {
    paletteState.set({
      ...paletteState.get(),
      query,
      items: query
        ? [
            {
              kind: "navigate",
              title: "Документи",
              subtitle: "Відкрити документи",
              shortcut: "Ctrl+2",
              payload: "screen:documents"
            }
          ]
        : [
            {
              kind: "navigate",
              title: "Дашборд",
              subtitle: "Повернутися на головний екран",
              shortcut: "Ctrl+1",
              payload: "screen:dashboard"
            }
          ],
      loading: false,
      error: null
    });
  });
  const paletteOpen = vi.fn(() => {
    paletteState.set({
      open: true,
      query: "",
      items: [],
      loading: false,
      error: null
    });
    void paletteSearch("");
  });
  const paletteClose = vi.fn(() => {
    paletteState.set({
      open: false,
      query: "",
      items: [],
      loading: false,
      error: null
    });
  });
  const paletteToggle = vi.fn(() => {
    if (paletteState.get().open) {
      paletteClose();
      return;
    }

    paletteOpen();
  });

  return {
    makeShellState,
    shellState,
    appShellState,
    paletteState,
    themeState,
    navigationState,
    appShellBootstrap,
    appShellSwitchActiveCompany,
    appShellReloadShellChrome,
    appShellSyncThemeFromSettings,
    paletteSearch,
    paletteOpen,
    paletteClose,
    paletteToggle,
    themeSetMode: vi.fn(),
    settingsUpdatePreference: vi.fn(),
    settingsLoad: vi.fn().mockResolvedValue({
      preferences: {
        darkMode: false
      }
    }),
    settingsSavePreferences: vi.fn().mockResolvedValue({
      screen: {
        preferences: {
          darkMode: false
        }
      }
    }),
    navigationGo: vi.fn((screen: ScreenId) => navigationState.set(screen))
  };
});

vi.mock("../lib/stores/app-shell", () => ({
  appShellStore: {
    subscribe: mocks.appShellState.subscribe,
    bootstrap: mocks.appShellBootstrap,
    switchActiveCompany: mocks.appShellSwitchActiveCompany,
    reloadShellChrome: mocks.appShellReloadShellChrome,
    syncThemeFromSettings: mocks.appShellSyncThemeFromSettings,
    syncThemeFromShell: vi.fn()
  }
}));

vi.mock("../lib/stores/shell", () => ({
  shellStore: {
    subscribe: mocks.shellState.subscribe,
    load: vi.fn(),
    setActiveCompany: vi.fn()
  }
}));

vi.mock("../lib/stores/palette", () => ({
  paletteStore: {
    subscribe: mocks.paletteState.subscribe,
    open: mocks.paletteOpen,
    close: mocks.paletteClose,
    toggle: mocks.paletteToggle,
    search: mocks.paletteSearch,
    activate: vi.fn().mockResolvedValue(undefined)
  }
}));

vi.mock("../lib/stores/theme", () => ({
  themeStore: {
    subscribe: mocks.themeState.subscribe,
    setMode: mocks.themeSetMode
  }
}));

vi.mock("../lib/stores/navigation", () => ({
  navigationStore: {
    subscribe: mocks.navigationState.subscribe,
    go: mocks.navigationGo
  }
}));

vi.mock("../lib/stores/settings", () => ({
  settingsStore: {
    load: mocks.settingsLoad,
    updatePreference: mocks.settingsUpdatePreference,
    savePreferences: mocks.settingsSavePreferences
  }
}));

vi.mock("../lib/screens/DashboardScreen.svelte", async () => ({
  default: (await import("./stubs/ComponentStub.svelte")).default
}));
vi.mock("../lib/screens/DocumentsScreen.svelte", async () => ({
  default: (await import("./stubs/ComponentStub.svelte")).default
}));
vi.mock("../lib/screens/CounterpartiesScreen.svelte", async () => ({
  default: (await import("./stubs/ComponentStub.svelte")).default
}));
vi.mock("../lib/screens/PaymentsScreen.svelte", async () => ({
  default: (await import("./stubs/ComponentStub.svelte")).default
}));
vi.mock("../lib/screens/ReportsScreen.svelte", async () => ({
  default: (await import("./stubs/ComponentStub.svelte")).default
}));
vi.mock("../lib/screens/SettingsScreen.svelte", async () => ({
  default: (await import("./stubs/ComponentStub.svelte")).default
}));
vi.mock("../lib/screens/TasksScreen.svelte", async () => ({
  default: (await import("./stubs/ComponentStub.svelte")).default
}));
vi.mock("../lib/components/AppIcon.svelte", async () => ({
  default: (await import("./stubs/ComponentStub.svelte")).default
}));

function renderApp() {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new App({ target });
  return { component, target };
}

describe("App shell orchestration", () => {
  beforeEach(() => {
    mocks.shellState.set({
      state: mocks.makeShellState("company-1"),
      loading: false,
      error: null,
      phase: "idle",
      pendingCompanyId: null,
      progressLabel: null
    });
    mocks.appShellState.set({
      loading: false,
      phase: "idle",
      progressLabel: null
    });
    mocks.paletteState.set({
      open: false,
      query: "",
      items: [],
      loading: false,
      error: null
    });
    mocks.themeState.set("light");
    mocks.navigationState.set("dashboard");
    document.body.innerHTML = "";
    Object.values(mocks).forEach((value) => {
      if (typeof value === "function" && "mockReset" in value) {
        value.mockReset();
      }
    });
    mocks.appShellBootstrap.mockResolvedValue(undefined);
    mocks.appShellReloadShellChrome.mockResolvedValue(mocks.makeShellState("company-1"));
    mocks.settingsLoad.mockResolvedValue({
      preferences: {
        darkMode: false
      }
    });
    mocks.settingsSavePreferences.mockResolvedValue({
      screen: {
        preferences: {
          darkMode: false
        }
      }
    });
    mocks.navigationGo.mockImplementation((screen: ScreenId) => mocks.navigationState.set(screen));
    mocks.paletteOpen.mockImplementation(() => {
      mocks.paletteState.set({
        open: true,
        query: "",
        items: [],
        loading: false,
        error: null
      });
      void mocks.paletteSearch("");
    });
    mocks.paletteClose.mockImplementation(() => {
      mocks.paletteState.set({
        open: false,
        query: "",
        items: [],
        loading: false,
        error: null
      });
    });
    mocks.paletteToggle.mockImplementation(() => {
      if (mocks.paletteState.get().open) {
        mocks.paletteClose();
        return;
      }

      mocks.paletteOpen();
    });
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("closes the palette on Escape and returns focus to the toggle button", async () => {
    const { component, target } = renderApp();
    await tick();

    const toggle = target.querySelector('[data-testid="palette-toggle"]') as HTMLButtonElement;
    expect(toggle).toBeTruthy();

    toggle.focus();
    toggle.click();
    await vi.waitFor(() => {
      const paletteInput = target.querySelector(".palette input") as HTMLInputElement | null;
      expect(paletteInput).toBeTruthy();
      expect(document.activeElement).toBe(paletteInput);
    });

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true }));
    await vi.waitFor(() => {
      expect(target.querySelector(".palette")).toBeNull();
    });
    expect(document.activeElement).toBe(toggle);
    expect(mocks.paletteClose).toHaveBeenCalledTimes(1);

    component.$destroy();
  });

  it("shows visible progress and disables critical actions during company reload", async () => {
    mocks.appShellSwitchActiveCompany.mockImplementation(async () => {
      mocks.appShellState.set({
        loading: true,
        phase: "company-switch",
        progressLabel: "Оновлюємо дані активної компанії..."
      });

      await Promise.resolve();

      mocks.shellState.set({
        state: mocks.makeShellState("company-2"),
        loading: false,
        error: null,
        phase: "idle",
        pendingCompanyId: null,
        progressLabel: null
      });
      mocks.appShellState.set({
        loading: false,
        phase: "idle",
        progressLabel: null
      });
      return mocks.makeShellState("company-2");
    });

    const { component, target } = renderApp();
    await tick();

    const companySelect = target.querySelector("select") as HTMLSelectElement;
    const navButton = target.querySelector('[data-testid="nav-documents"]') as HTMLButtonElement;
    const paletteToggle = target.querySelector('[data-testid="palette-toggle"]') as HTMLButtonElement;

    mocks.appShellState.set({
      loading: true,
      phase: "company-switch",
      progressLabel: "Оновлюємо дані активної компанії..."
    });
    await tick();

    expect(companySelect.disabled).toBe(true);
    expect(navButton.disabled).toBe(true);
    expect(paletteToggle.disabled).toBe(true);
    expect(target.textContent).toContain("Оновлюємо дані активної компанії...");
    expect(target.querySelector(".shell-progress")).toBeTruthy();

    mocks.appShellState.set({
      loading: false,
      phase: "idle",
      progressLabel: null
    });
    await tick();

    component.$destroy();
  });
});
