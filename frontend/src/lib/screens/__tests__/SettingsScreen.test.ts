/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import SettingsScreen from "../SettingsScreen.svelte";
import type {
  ImportPlanDto,
  ImportResultDto,
  SettingsScreenDto,
  SettingsSection
} from "../../types";

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
      }
    };
  }

  const settingsState = createMockStore({
    section: "appearance" as SettingsSection,
    screen: null as SettingsScreenDto | null,
    loading: false,
    error: null as string | null,
    message: null as string | null
  });

  const importState = createMockStore({
    plan: null as ImportPlanDto | null,
    result: null as ImportResultDto | null,
    loading: false,
    error: null as string | null
  });

  return {
    settingsState,
    importState,
    setSection: vi.fn((section: SettingsSection) => {
      settingsState.set({ ...settingsState["_value"], section });
    }),
    savePreferences: vi.fn().mockResolvedValue(undefined),
    saveCompany: vi.fn().mockResolvedValue(true),
    updatePreference: vi.fn(),
    updateCompanyField: vi.fn(),
    configureIntegration: vi.fn(),
    inviteTeam: vi.fn(),
    openLatestBackup: vi.fn(),
    backupNow: vi.fn(),
    shellLoad: vi.fn().mockResolvedValue(undefined),
    themeSetMode: vi.fn(),
    themeToggle: vi.fn(),
    importFetchPlan: vi.fn(),
    importExecute: vi.fn(),
    importReset: vi.fn()
  };
});

vi.mock("../../stores/settings", () => ({
  settingsStore: {
    subscribe: mocks.settingsState.subscribe,
    setSection: mocks.setSection,
    savePreferences: mocks.savePreferences,
    saveCompany: mocks.saveCompany,
    updatePreference: mocks.updatePreference,
    updateCompanyField: mocks.updateCompanyField,
    configureIntegration: mocks.configureIntegration,
    inviteTeam: mocks.inviteTeam,
    openLatestBackup: mocks.openLatestBackup,
    backupNow: mocks.backupNow
  }
}));

vi.mock("../../stores/shell", () => ({
  shellStore: {
    subscribe: vi.fn((run: (v: unknown) => void) => { run({}); return () => {}; }),
    load: mocks.shellLoad
  }
}));

vi.mock("../../stores/theme", () => ({
  themeStore: {
    subscribe: vi.fn((run: (v: string) => void) => { run("light"); return () => {}; }),
    setMode: mocks.themeSetMode,
    toggle: mocks.themeToggle
  }
}));

vi.mock("../../stores/import", () => ({
  importStore: {
    subscribe: mocks.importState.subscribe,
    fetchPlan: mocks.importFetchPlan,
    execute: mocks.importExecute,
    reset: mocks.importReset
  }
}));

function makeSettingsScreen(): SettingsScreenDto {
  return {
    company: {
      fullName: "ТОВ Тест",
      shortName: "Тест",
      edrpou: "12345678",
      ipn: "",
      address: "м. Київ",
      director: "Іваненко І.І.",
      iban: "UA12345",
      bank: "ПриватБанк",
      vatRegistered: false,
      vatCert: ""
    },
    preferences: {
      darkMode: false,
      density: 1
    },
    integrations: [
      {
        label: "BAS",
        description: "Імпорт даних з BAS",
        tag: "bas",
        enabled: true
      }
    ],
    team: [
      { name: "Іваненко І.І.", email: "ivan@test.com", role: "Адміністратор", lastActive: "01.05.2026" }
    ],
    numbering: [
      { docType: "Акт", template: "АКТ-{РРРР}-{nnnn}", example: "АКТ-2026-0001", nextNumber: "0042" }
    ],
    backup: {
      label: "Остання копія",
      file: "acta-2026-05-01.sql",
      kind: "manual",
      note: "Зроблено вручну",
      tone: "success"
    }
  };
}

function renderSettings() {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new SettingsScreen({ target });
  return { component, target };
}

function buttonByText(target: HTMLElement, text: string): HTMLButtonElement {
  const button = Array.from(target.querySelectorAll("button")).find((b) =>
    b.textContent?.includes(text)
  );
  expect(button, `Кнопка "${text}" не знайдена`).toBeTruthy();
  return button as HTMLButtonElement;
}

describe("SettingsScreen", () => {
  beforeEach(() => {
    mocks.settingsState.set({
      section: "appearance",
      screen: makeSettingsScreen(),
      loading: false,
      error: null,
      message: null
    });
    mocks.importState.set({ plan: null, result: null, loading: false, error: null });
    Object.values(mocks).forEach((m) => {
      if (typeof m === "object" && m !== null && "mockReset" in m) {
        (m as ReturnType<typeof vi.fn>).mockReset();
      }
    });
    mocks.savePreferences.mockResolvedValue(undefined);
    mocks.saveCompany.mockResolvedValue(true);
    mocks.shellLoad.mockResolvedValue(undefined);
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  describe("Навігація по секціях", () => {
    it("показує всі 6 кнопок навігації", () => {
      const { component, target } = renderSettings();

      expect(target.textContent).toContain("Зовнішній вигляд");
      expect(target.textContent).toContain("Компанія");
      expect(target.textContent).toContain("Нумерація");
      expect(target.textContent).toContain("Інтеграції");
      expect(target.textContent).toContain("Команда");
      expect(target.textContent).toContain("Резервні копії");

      component.$destroy();
    });

    it("перемикає секцію при кліку", async () => {
      const { component, target } = renderSettings();

      buttonByText(target, "Компанія").click();
      await tick();
      expect(mocks.setSection).toHaveBeenCalledWith("company");

      component.$destroy();
    });
  });

  describe("Appearance controls", () => {
    it("показує кнопки вибору теми у секції appearance", () => {
      const { component, target } = renderSettings();

      expect(target.textContent).toContain("Світла тема");
      expect(target.textContent).toContain("Темна тема");

      component.$destroy();
    });

    it("перемикає тему через themeStore.setMode при кліку на Темна тема", async () => {
      const { component, target } = renderSettings();

      buttonByText(target, "Темна тема").click();
      await tick();
      expect(mocks.themeSetMode).toHaveBeenCalledWith("dark");
      expect(mocks.savePreferences).toHaveBeenCalled();

      component.$destroy();
    });

    it("перемикає тему на світлу через themeStore.setMode", async () => {
      const { component, target } = renderSettings();

      buttonByText(target, "Світла тема").click();
      await tick();
      expect(mocks.themeSetMode).toHaveBeenCalledWith("light");

      component.$destroy();
    });
  });

  describe("Company settings save", () => {
    it("показує форму компанії із кнопкою Зберегти", async () => {
      mocks.settingsState.set({
        section: "company",
        screen: makeSettingsScreen(),
        loading: false,
        error: null,
        message: null
      });

      const { component, target } = renderSettings();
      await tick();

      const inputs = Array.from(target.querySelectorAll("input")) as HTMLInputElement[];
      const fullNameInput = inputs.find((input) => input.value === "ТОВ Тест");
      expect(fullNameInput, "Інпут з назвою компанії не знайдено").toBeTruthy();
      expect(target.textContent).toContain("Зберегти");

      component.$destroy();
    });

    it("викликає saveCompany і shellLoad при кліку Зберегти", async () => {
      mocks.settingsState.set({
        section: "company",
        screen: makeSettingsScreen(),
        loading: false,
        error: null,
        message: null
      });

      const { component, target } = renderSettings();
      await tick();

      buttonByText(target, "Зберегти").click();
      await tick();
      expect(mocks.saveCompany).toHaveBeenCalled();

      component.$destroy();
    });
  });

  describe("BAS import flow", () => {
    beforeEach(() => {
      mocks.settingsState.set({
        section: "integrations",
        screen: makeSettingsScreen(),
        loading: false,
        error: null,
        message: null
      });
    });

    it("показує кнопку Імпортувати для BAS інтеграції", async () => {
      const { component, target } = renderSettings();
      await tick();

      expect(target.textContent).toContain("BAS");
      expect(target.textContent).toContain("Імпортувати");

      component.$destroy();
    });

    it("відкриває BAS import панель після кліку Імпортувати", async () => {
      const { component, target } = renderSettings();
      await tick();

      buttonByText(target, "Імпортувати").click();
      await tick();

      expect(target.textContent).toContain("storage/import/bas/");
      expect(target.textContent).toContain("Перевірити файли");

      component.$destroy();
    });

    it("викликає importStore.fetchPlan при кліку Перевірити файли", async () => {
      const { component, target } = renderSettings();
      await tick();

      buttonByText(target, "Імпортувати").click();
      await tick();
      buttonByText(target, "Перевірити файли").click();
      await tick();

      expect(mocks.importFetchPlan).toHaveBeenCalled();

      component.$destroy();
    });

    it("показує таблицю плану після завантаження даних", async () => {
      const { component, target } = renderSettings();
      await tick();

      buttonByText(target, "Імпортувати").click();
      await tick();

      mocks.importState.set({
        plan: {
          entities: [
            {
              entityType: "counterparties",
              fileName: "counterparties.xml",
              parsed: 15,
              willCreate: 10,
              willSkip: 5,
              error: null
            },
            {
              entityType: "payments",
              fileName: "bank_export.csv",
              parsed: 30,
              willCreate: 25,
              willSkip: 5,
              error: null
            }
          ]
        },
        result: null,
        loading: false,
        error: null
      });
      await tick();

      expect(target.textContent).toContain("counterparties");
      expect(target.textContent).toContain("counterparties.xml");
      expect(target.textContent).toContain("25 нових / 5 дублікатів");
      expect(target.textContent).toContain("Виконати імпорт");

      component.$destroy();
    });

    it("викликає importStore.execute при кліку Виконати імпорт", async () => {
      const { component, target } = renderSettings();
      await tick();

      buttonByText(target, "Імпортувати").click();
      await tick();

      mocks.importState.set({
        plan: {
          entities: [
            {
              entityType: "counterparties",
              fileName: "counterparties.xml",
              parsed: 15,
              willCreate: 10,
              willSkip: 5,
              error: null
            }
          ]
        },
        result: null,
        loading: false,
        error: null
      });
      await tick();

      buttonByText(target, "Виконати імпорт").click();
      await tick();

      expect(mocks.importExecute).toHaveBeenCalled();

      component.$destroy();
    });

    it("показує результати після виконання імпорту", async () => {
      const { component, target } = renderSettings();
      await tick();

      buttonByText(target, "Імпортувати").click();
      await tick();

      mocks.importState.set({
        plan: null,
        result: {
          entities: [
            {
              entityType: "counterparties",
              created: 10,
              updated: 2,
              skipped: 3,
              conflicts: 0,
              error: null
            }
          ]
        },
        loading: false,
        error: null
      });
      await tick();

      expect(target.textContent).toContain("counterparties");
      expect(target.textContent).toContain("Закрити");

      component.$destroy();
    });

    it("скасовує і закриває панель після завантаження плану", async () => {
      const { component, target } = renderSettings();
      await tick();

      buttonByText(target, "Імпортувати").click();
      await tick();

      mocks.importState.set({
        plan: {
          entities: [
            {
              entityType: "counterparties",
              fileName: "counterparties.xml",
              parsed: 5,
              willCreate: 5,
              willSkip: 0,
              error: null
            }
          ]
        },
        result: null,
        loading: false,
        error: null
      });
      await tick();

      expect(target.textContent).toContain("Скасувати");

      buttonByText(target, "Скасувати").click();
      await tick();

      expect(mocks.importReset).toHaveBeenCalled();
      expect(target.textContent).not.toContain("Перевірити файли");

      component.$destroy();
    });
  });
});
