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
    const subscribers = new Set<(nextValue: T) => void>();

    return {
      subscribe(run: (nextValue: T) => void) {
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

  const settingsState = createMockStore({
    section: "appearance" as SettingsSection,
    screen: null as SettingsScreenDto | null,
    loading: false,
    error: null as string | null,
    message: null as string | null
  });

  const importState = createMockStore({
    selectedDirectory: null as string | null,
    plan: null as ImportPlanDto | null,
    result: null as ImportResultDto | null,
    loading: false,
    error: null as string | null
  });

  return {
    settingsState,
    importState,
    setSection: vi.fn((section: SettingsSection) => {
      settingsState.set({ ...settingsState.get(), section });
    }),
    savePreferences: vi.fn().mockResolvedValue(undefined),
    saveCompany: vi.fn().mockResolvedValue(true),
    updatePreference: vi.fn(),
    updateCompanyField: vi.fn(),
    configureIntegration: vi.fn(),
    inviteTeam: vi.fn(),
    openLatestBackup: vi.fn(),
    backupNow: vi.fn(),
    shellReloadChrome: vi.fn().mockResolvedValue(undefined),
    themeSetMode: vi.fn(),
    importChooseDirectory: vi.fn(),
    importFetchPlan: vi.fn(),
    importExecute: vi.fn(),
    importReset: vi.fn()
  };
  it("renders theme choice as segmented control without density leak", async () => {
    const { component, target } = renderSettings();
    await tick();

    const segmented = target.querySelector('[data-testid="theme-segmented"]');

    expect(segmented).toBeTruthy();
    expect(segmented?.getAttribute("role")).toBe("radiogroup");
    expect(target.textContent).toContain("Світла");
    expect(target.textContent).toContain("Темна");
    expect(target.textContent).not.toContain("selector не впливав");

    component.$destroy();
  });

  it("marks the active theme option via radio semantics", async () => {
    const { component, target } = renderSettings();
    await tick();

    const radios = Array.from(target.querySelectorAll('[data-testid="theme-segmented"] button'));

    expect(radios).toHaveLength(2);
    expect(radios[0].getAttribute("role")).toBe("radio");
    expect(radios[0].getAttribute("aria-checked")).toBe("true");
    expect(radios[1].getAttribute("aria-checked")).toBe("false");

    component.$destroy();
  });
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

vi.mock("../../stores/app-shell", () => ({
  appShellStore: {
    subscribe: vi.fn((run: (value: unknown) => void) => {
      run({});
      return () => {};
    }),
    reloadShellChrome: mocks.shellReloadChrome
  }
}));

vi.mock("../../stores/theme", () => ({
  themeStore: {
    subscribe: vi.fn((run: (value: string) => void) => {
      run("light");
      return () => {};
    }),
    setMode: mocks.themeSetMode
  }
}));

vi.mock("../../stores/import", () => ({
  importStore: {
    subscribe: mocks.importState.subscribe,
    chooseDirectory: mocks.importChooseDirectory,
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
      darkMode: false
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
      {
        name: "Іваненко І.І.",
        email: "ivan@test.com",
        role: "Адміністратор",
        lastActive: "01.05.2026"
      }
    ],
    numbering: [
      {
        docType: "Акт",
        template: "АКТ-{РРРР}-{nnnn}",
        example: "АКТ-2026-0001",
        nextNumber: "0042"
      }
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
  const button = Array.from(target.querySelectorAll("button")).find((candidate) =>
    candidate.textContent?.includes(text)
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
    mocks.importState.set({
      selectedDirectory: null,
      plan: null,
      result: null,
      loading: false,
      error: null
    });

    mocks.setSection.mockClear();
    mocks.savePreferences.mockClear();
    mocks.saveCompany.mockClear();
    mocks.updatePreference.mockClear();
    mocks.updateCompanyField.mockClear();
    mocks.configureIntegration.mockClear();
    mocks.inviteTeam.mockClear();
    mocks.openLatestBackup.mockClear();
    mocks.backupNow.mockClear();
    mocks.shellReloadChrome.mockClear();
    mocks.themeSetMode.mockClear();
    mocks.importChooseDirectory.mockClear();
    mocks.importFetchPlan.mockClear();
    mocks.importExecute.mockClear();
    mocks.importReset.mockClear();
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("показує всі 6 секцій навігації", () => {
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

  it("використовує канонічну ієрархію кнопок і прибирає selector density", () => {
    const { component, target } = renderSettings();

    expect(buttonByText(target, "Світла тема").disabled).toBe(false);
    expect(buttonByText(target, "Темна тема").disabled).toBe(false);
    expect(target.textContent).not.toContain("Компактно");
    expect(target.textContent).not.toContain("Щільно");
    expect(target.textContent).toContain("Налаштування щільності поки прибрано");

    component.$destroy();
  });

  it("перемикає тему через themeStore і reload shell chrome", async () => {
    const { component, target } = renderSettings();

    buttonByText(target, "Темна тема").click();
    await tick();

    expect(mocks.themeSetMode).toHaveBeenCalledWith("dark");
    expect(mocks.updatePreference).toHaveBeenCalledWith("darkMode", true);
    expect(mocks.savePreferences).toHaveBeenCalled();
    expect(mocks.shellReloadChrome).toHaveBeenCalled();

    component.$destroy();
  });

  it("блокує дії та показує loading banner під час збереження", async () => {
    mocks.settingsState.set({
      section: "company",
      screen: makeSettingsScreen(),
      loading: true,
      error: null,
      message: null
    });

    const { component, target } = renderSettings();
    await tick();

    const saveButton = buttonByText(target, "Зберігаємо");
    const fullNameInput = Array.from(target.querySelectorAll("input")).find(
      (input) => (input as HTMLInputElement).value === "ТОВ Тест"
    ) as HTMLInputElement | undefined;

    expect(target.textContent).toContain("Оновлюємо налаштування");
    expect(saveButton.disabled).toBe(true);
    expect(saveButton.getAttribute("aria-busy")).toBe("true");
    expect(fullNameInput).toBeDefined();
    expect((fullNameInput as HTMLInputElement).disabled).toBe(true);

    component.$destroy();
  });

  it("показує success та error banner для системних станів", async () => {
    mocks.settingsState.set({
      section: "appearance",
      screen: makeSettingsScreen(),
      loading: false,
      error: "Помилка запису",
      message: "Налаштування збережено"
    });

    const { component, target } = renderSettings();
    await tick();

    expect(target.textContent).toContain("Зміни збережено");
    expect(target.textContent).toContain("Налаштування збережено");
    expect(target.textContent).toContain("Не вдалося виконати дію");
    expect(target.textContent).toContain("Помилка запису");

    component.$destroy();
  });

  it("показує компанію та зберігає її через primary CTA", async () => {
    mocks.settingsState.set({
      section: "company",
      screen: makeSettingsScreen(),
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderSettings();
    await tick();

    expect(buttonByText(target, "Зберегти").disabled).toBe(false);

    buttonByText(target, "Зберегти").click();
    await tick();

    expect(mocks.saveCompany).toHaveBeenCalled();
    expect(mocks.shellReloadChrome).toHaveBeenCalled();

    component.$destroy();
  });

  it("показує integration state як chip та правильні варіанти кнопок", async () => {
    mocks.settingsState.set({
      section: "integrations",
      screen: makeSettingsScreen(),
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderSettings();
    await tick();

    expect(target.textContent).toContain("Активно");
    expect(buttonByText(target, "Налаштувати").disabled).toBe(false);
    expect(buttonByText(target, "Імпортувати").disabled).toBe(false);

    component.$destroy();
  });

  it("відкриває BAS import panel і викликає вибір папки", async () => {
    mocks.settingsState.set({
      section: "integrations",
      screen: makeSettingsScreen(),
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderSettings();
    await tick();

    buttonByText(target, "Імпортувати").click();
    await tick();
    buttonByText(target, "Обрати папку").click();
    await tick();

    expect(target.textContent).toContain("Імпорт BAS");
    expect(mocks.importChooseDirectory).toHaveBeenCalled();

    component.$destroy();
  });

  it("показує план BAS імпорту і робить primary CTA для перевірки файлів", async () => {
    mocks.settingsState.set({
      section: "integrations",
      screen: makeSettingsScreen(),
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderSettings();
    await tick();

    buttonByText(target, "Імпортувати").click();
    mocks.importState.set({
      selectedDirectory: "C:\\tmp\\bas-export",
      plan: {
        entities: [
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

    const verifyButton = buttonByText(target, "Перевірити файли");

    expect(verifyButton.disabled).toBe(false);
    expect(target.textContent).toContain("25 нових / 5 дублікатів");
    expect(target.textContent).toContain("C:\\tmp\\bas-export");

    component.$destroy();
  });

  it("викликає fetchPlan та execute для BAS flow", async () => {
    mocks.settingsState.set({
      section: "integrations",
      screen: makeSettingsScreen(),
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderSettings();
    await tick();

    buttonByText(target, "Імпортувати").click();
    await tick();

    mocks.importState.set({
      selectedDirectory: "C:\\tmp\\bas-export",
      plan: null,
      result: null,
      loading: false,
      error: null
    });
    await tick();

    buttonByText(target, "Перевірити файли").click();
    await tick();
    expect(mocks.importFetchPlan).toHaveBeenCalled();

    mocks.importState.set({
      selectedDirectory: "C:\\tmp\\bas-export",
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

  it("показує результати BAS імпорту та дозволяє закрити панель", async () => {
    mocks.settingsState.set({
      section: "integrations",
      screen: makeSettingsScreen(),
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderSettings();
    await tick();

    buttonByText(target, "Імпортувати").click();
    await tick();

    mocks.importState.set({
      selectedDirectory: "C:\\tmp\\bas-export",
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

    expect(target.textContent).toContain("Імпорт завершено");
    buttonByText(target, "Закрити").click();
    await tick();

    expect(mocks.importReset).toHaveBeenCalled();

    component.$destroy();
  });
});
