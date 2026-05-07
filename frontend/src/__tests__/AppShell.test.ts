/**
 * @vitest-environment jsdom
 */
// @ts-ignore Node typings are not included in the frontend test tsconfig.
import { readFileSync } from "fs";
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

describe("design-system tokens", () => {
  const tokens = readFileSync("frontend/src/lib/styles/tokens.css", "utf8");
  const styles = readFileSync("frontend/src/styles.css", "utf8");

  it("--font-body is 14px", () => {
    expect(tokens).toMatch(/--font-body:\s*14px/);
  });

  it("--control-height is 38px", () => {
    expect(tokens).toMatch(/--control-height:\s*38px/);
  });

  it("--font-serif is removed", () => {
    expect(tokens).not.toMatch(/--font-serif:/);
  });

  it("declares the missing CSS variable aliases", () => {
    for (const name of ["--font-base", "--accent-strong", "--text-primary", "--success-text", "--line"]) {
      expect(tokens).toMatch(new RegExp(`${name}:\\s*[^;]+;`));
    }
  });

  it(".currency utility uses tabular numbers", () => {
    expect(styles).toMatch(/\.currency\s*,?[\s\S]*font-variant-numeric:\s*tabular-nums/);
  });

  it("collapses shell chrome into a compact horizontal navigation on narrow widths", () => {
    expect(styles).toMatch(/@media\s*\(max-width:\s*980px\)[\s\S]*\.nav\s*\{[\s\S]*flex-direction:\s*row/);
    expect(styles).toMatch(/@media\s*\(max-width:\s*980px\)[\s\S]*\.user-footer,\s*\.sidebar-spacer\s*\{[\s\S]*display:\s*none/);
    expect(styles).toMatch(/@media\s*\(max-width:\s*980px\)[\s\S]*\.topbar\s*\{[\s\S]*grid-template-columns:\s*minmax\(0,\s*1fr\)\s+minmax\(220px,\s*1fr\)/);
  });

  it("switches the shell topbar to a single-column mobile layout at 720px", () => {
    expect(styles).toMatch(/@media\s*\(max-width:\s*720px\)[\s\S]*\.topbar\s*\{[\s\S]*grid-template-columns:\s*minmax\(0,\s*1fr\)/);
    expect(styles).toMatch(/@media\s*\(max-width:\s*720px\)[\s\S]*\.nav-item\s*\{[\s\S]*font-size:\s*13px/);
    expect(styles).toMatch(/@media\s*\(max-width:\s*720px\)[\s\S]*\.topbar-search\s*\{[\s\S]*width:\s*100%/);
  });
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

  it("keeps keyboard focus trapped inside the palette and supports arrow navigation", async () => {
    const { component, target } = renderApp();
    await tick();

    const toggle = target.querySelector('[data-testid="palette-toggle"]') as HTMLButtonElement;
    toggle.focus();
    toggle.click();

    const paletteInput = await vi.waitFor(() => {
      const input = target.querySelector(".palette input") as HTMLInputElement | null;
      expect(input).toBeTruthy();
      return input as HTMLInputElement;
    });

    paletteInput.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    await tick();

    const firstItem = target.querySelector('[data-testid="palette-item-0"]') as HTMLButtonElement | null;
    expect(firstItem).toBeTruthy();
    expect(document.activeElement).toBe(firstItem);

    firstItem!.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(paletteInput);

    paletteInput.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", shiftKey: true, bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(firstItem);

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

  it("updates the topbar search placeholder for the active screen", async () => {
    const { component, target } = renderApp();
    await tick();

    const searchPlaceholder = target.querySelector(".topbar-search-placeholder");
    expect(searchPlaceholder?.textContent).toContain("Пошук в Acta");

    mocks.navigationState.set("documents");
    await tick();
    expect(searchPlaceholder?.textContent).toContain("Пошук у документах");

    mocks.navigationState.set("counterparties");
    await tick();
    expect(searchPlaceholder?.textContent).toContain("Пошук у контрагентах");

    mocks.navigationState.set("payments");
    await tick();
    expect(searchPlaceholder?.textContent).toContain("Пошук у платежах");

    component.$destroy();
  });
});
