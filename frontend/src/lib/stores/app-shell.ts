import { writable } from "svelte/store";
import type { SettingsScreenDto, ShellStateDto } from "../types";
import { counterpartiesStore } from "./counterparties";
import { dashboardStore } from "./dashboard";
import { documentsStore } from "./documents";
import { paymentsStore } from "./payments";
import { reportsStore } from "./reports";
import { settingsStore } from "./settings";
import { shellStore } from "./shell";
import { tasksStore } from "./tasks";
import { themeStore } from "./theme";

type AppShellPhase = "idle" | "bootstrap" | "company-switch" | "shell-refresh";

type AppShellState = {
  loading: boolean;
  phase: AppShellPhase;
  progressLabel: string | null;
};

const initialState: AppShellState = {
  loading: false,
  phase: "idle",
  progressLabel: null
};

function applyThemeFromSettings(screen: SettingsScreenDto | null) {
  if (!screen) {
    return;
  }

  themeStore.setMode(screen.preferences.darkMode ? "dark" : "light");
}

function applyThemeFromShell(state: ShellStateDto | null) {
  if (!state) {
    return;
  }

  themeStore.setMode(state.isDark ? "dark" : "light");
}

async function reloadCanonicalScreens() {
  await Promise.all([
    dashboardStore.load(),
    documentsStore.load(),
    counterpartiesStore.load(),
    tasksStore.load(),
    reportsStore.load(),
    paymentsStore.load()
  ]);
}

function createAppShellStore() {
  const { subscribe, set } = writable<AppShellState>(initialState);
  let activeRequest: Promise<unknown> | null = null;

  async function run<T>(phase: AppShellPhase, progressLabel: string, task: () => Promise<T>) {
    if (activeRequest) {
      await activeRequest;
    }

    set({
      loading: true,
      phase,
      progressLabel
    });

    activeRequest = task();

    try {
      return await activeRequest;
    } finally {
      activeRequest = null;
      set(initialState);
    }
  }

  return {
    subscribe,
    async bootstrap() {
      return run("bootstrap", "Завантажуємо робочий простір...", async () => {
        await shellStore.load();
        const settingsScreen = await settingsStore.load();
        applyThemeFromSettings(settingsScreen);
        await reloadCanonicalScreens();
      });
    },
    async switchActiveCompany(companyId: string) {
      return run("company-switch", "Оновлюємо дані активної компанії...", async () => {
        const nextShellState = await shellStore.setActiveCompany(companyId);
        if (!nextShellState) {
          return null;
        }

        const settingsScreen = await settingsStore.load();
        applyThemeFromSettings(settingsScreen);
        await reloadCanonicalScreens();
        return nextShellState;
      });
    },
    async reloadShellChrome() {
      return run("shell-refresh", "Оновлюємо shell...", async () => {
        const shellState = await shellStore.load();
        applyThemeFromShell(shellState);
        return shellState;
      });
    },
    syncThemeFromSettings: applyThemeFromSettings,
    syncThemeFromShell: applyThemeFromShell
  };
}

export const appShellStore = createAppShellStore();
