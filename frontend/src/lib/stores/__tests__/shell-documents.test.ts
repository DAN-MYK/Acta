import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  DocumentChainDto,
  DocumentEditorDto,
  DocumentsListDto,
  PaletteActivationResultDto,
  PaletteSearchResultDto,
  ShellStateDto
} from "../../types";
import { invokeMock, loadStores, snapshot } from "./helpers";

function createDeferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return { promise, resolve, reject };
}

function makeShellState(activeCompanyId = "company-1"): ShellStateDto {
  return {
    chrome: {
      companyName: "ТОВ Акт",
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

function makeEditor(id: string): DocumentEditorDto {
  return {
    form: {
      id,
      kind: "invoice",
      counterpartyId: "cp-1",
      counterpartyName: "ТОВ Ромашка",
      title: "invoice editor",
      number: `NUM-${id}`,
      date: "2026-04-30",
      notes: ""
    },
    items: [
      {
        description: "Послуга",
        unit: "шт",
        quantity: "1",
        price: "1234.50"
      }
    ],
    pdf: null,
    showTypePicker: false,
    showEditor: true
  };
}

function makeDocumentsList(ids: string[]): DocumentsListDto {
  const items = ids.map((id, index) => ({
    id,
    kind: "invoice" as const,
    number: `INV-${index + 1}`,
    date: "2026-04-30",
    counterparty: "ТОВ Ромашка",
    amountStr: "1 234,50 грн",
    status: "draft" as const,
    statusLabel: "Чернетка",
    linkedId: ""
  }));

  return {
    items,
    invoiceItems: items,
    actItems: [],
    waybillItems: [],
    totalCount: items.length,
    pageCount: 1
  };
}

function makeChain(sourceId: string): DocumentChainDto {
  return {
    sourceId,
    steps: [
      {
        docType: "invoice",
        docNumber: `NUM-${sourceId}`,
        amountStr: "1 234,50 грн",
        status: "draft",
        exists: true
      }
    ]
  };
}

describe("frontend Tauri store smoke: shell + documents", () => {
  beforeEach(() => {
    vi.resetModules();
    invokeMock.mockReset();
  });

  it("orchestrates shell load, company switch and palette activation without Slint callbacks", async () => {
    const stores = await loadStores();
    const { appShellStore, shellStore, paletteStore, navigationStore, documentsStore, counterpartiesStore } = stores;

    let selectedCounterpartyId = "cp-9";
    counterpartiesStore.subscribe((state) => {
      if (state.selectedId) {
        selectedCounterpartyId = state.selectedId;
      }
    })();

    const paletteSearch: PaletteSearchResultDto = {
      items: [
        {
          kind: "navigate",
          title: "Документи",
          subtitle: "Перейти до екрану",
          shortcut: "Ctrl+2",
          payload: "nav:documents"
        }
      ]
    };

    const paletteActivation: PaletteActivationResultDto = {
      kind: "open_document",
      screen: "documents",
      documentId: "doc-open-1",
      counterpartyId: null,
      documentEditor: null,
      message: null
    };

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "shell_load":
          return makeShellState("company-1");
        case "shell_set_active_company":
          expect(payload).toEqual({ companyId: "company-2" });
          return makeShellState("company-2");
        case "settings_load":
          return {
            preferences: {
              darkMode: true
            }
          };
        case "dashboard_load":
          return {
            hero: null,
            alerts: [],
            metrics: [],
            cashflow: [],
            receivables: [],
            payables: []
          };
        case "documents_list":
          return makeDocumentsList([]);
        case "counterparties_list":
          return { items: [] };
        case "tasks_list":
          return { sections: [], summary: { overdue: 0, today: 0, upcoming: 0, done: 0 } };
        case "reports_load":
          return {
            filter: {
              tab: "bank",
              scope: "active",
              dateFrom: "2026-02-01",
              dateTo: "2026-05-01",
              query: ""
            },
            tabs: [],
            summary: [],
            rows: []
          };
        case "payments_list":
          return { items: [], totals: [], summary: { incoming: "0", outgoing: "0", balance: "0" } };
        case "shell_palette_search":
          expect(payload).toEqual({
            request: {
              query: "док",
              selectedCounterpartyId: undefined
            }
          });
          return paletteSearch;
        case "shell_palette_activate":
          expect(payload).toEqual({
            payload: "nav:documents",
            selectedCounterpartyId: undefined
          });
          return paletteActivation;
        case "document_open":
          expect(payload).toEqual({ docId: "doc-open-1" });
          return makeEditor("doc-open-1");
        case "document_chain_get":
          expect(payload).toEqual({ docId: "doc-open-1" });
          return makeChain("doc-open-1");
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await appShellStore.bootstrap();
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-1");

    await appShellStore.switchActiveCompany("company-2");
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-2");
    expect(snapshot(shellStore).state?.isDark).toBe(true);

    await paletteStore.search("док");
    expect(snapshot(paletteStore).items).toEqual(paletteSearch.items);

    await paletteStore.activate("nav:documents");
    await vi.waitFor(() => {
      expect(snapshot(navigationStore)).toBe("documents");
      expect(snapshot(documentsStore).editor?.form.id).toBe("doc-open-1");
    });
    expect(selectedCounterpartyId).toBe("cp-9");
  });

  it("centralizes canonical reload fan-out for bootstrap and company switch", async () => {
    const stores = await loadStores();
    const {
      appShellStore,
      dashboardStore,
      documentsStore,
      counterpartiesStore,
      tasksStore,
      reportsStore,
      paymentsStore,
      settingsStore
    } = stores;

    const dashboardSpy = vi.spyOn(dashboardStore, "load");
    const documentsSpy = vi.spyOn(documentsStore, "load");
    const counterpartiesSpy = vi.spyOn(counterpartiesStore, "load");
    const tasksSpy = vi.spyOn(tasksStore, "load");
    const reportsSpy = vi.spyOn(reportsStore, "load");
    const paymentsSpy = vi.spyOn(paymentsStore, "load");
    const settingsSpy = vi.spyOn(settingsStore, "load");

    invokeMock.mockImplementation(async (command) => {
      switch (command) {
        case "shell_load":
          return makeShellState("company-1");
        case "shell_set_active_company":
          return makeShellState("company-2");
        case "settings_load":
          return {
            preferences: {
              darkMode: true
            }
          };
        case "dashboard_load":
          return {
            hero: null,
            alerts: [],
            metrics: [],
            cashflow: [],
            receivables: [],
            payables: []
          };
        case "documents_list":
          return makeDocumentsList([]);
        case "counterparties_list":
          return { items: [] };
        case "tasks_list":
          return { sections: [], summary: { overdue: 0, today: 0, upcoming: 0, done: 0 } };
        case "reports_load":
          return {
            filter: {
              tab: "bank",
              scope: "active",
              dateFrom: "2026-02-01",
              dateTo: "2026-05-01",
              query: ""
            },
            tabs: [],
            summary: [],
            rows: []
          };
        case "payments_list":
          return { items: [], totals: [], summary: { incoming: "0", outgoing: "0", balance: "0" } };
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await appShellStore.bootstrap();

    expect(settingsSpy).toHaveBeenCalledTimes(1);
    expect(dashboardSpy).toHaveBeenCalledTimes(1);
    expect(documentsSpy).toHaveBeenCalledTimes(1);
    expect(counterpartiesSpy).toHaveBeenCalledTimes(1);
    expect(tasksSpy).toHaveBeenCalledTimes(1);
    expect(reportsSpy).toHaveBeenCalledTimes(1);
    expect(paymentsSpy).toHaveBeenCalledTimes(1);

    dashboardSpy.mockClear();
    documentsSpy.mockClear();
    counterpartiesSpy.mockClear();
    tasksSpy.mockClear();
    reportsSpy.mockClear();
    paymentsSpy.mockClear();
    settingsSpy.mockClear();

    await appShellStore.switchActiveCompany("company-2");

    expect(settingsSpy).toHaveBeenCalledTimes(1);
    expect(dashboardSpy).toHaveBeenCalledTimes(1);
    expect(documentsSpy).toHaveBeenCalledTimes(1);
    expect(counterpartiesSpy).toHaveBeenCalledTimes(1);
    expect(tasksSpy).toHaveBeenCalledTimes(1);
    expect(reportsSpy).toHaveBeenCalledTimes(1);
    expect(paymentsSpy).toHaveBeenCalledTimes(1);
  });

  it("resets palette query and items after close so next open starts predictably", async () => {
    const { paletteStore } = await loadStores();

    invokeMock.mockResolvedValue({
      items: [
        {
          kind: "navigate",
          title: "Документи",
          subtitle: "Перейти до екрану",
          shortcut: "Ctrl+2",
          payload: "nav:documents"
        }
      ]
    });

    paletteStore.toggle();
    await paletteStore.search("док");

    expect(snapshot(paletteStore)).toMatchObject({
      open: true,
      query: "док"
    });

    paletteStore.close();

    expect(snapshot(paletteStore)).toMatchObject({
      open: false,
      query: "",
      items: [],
      loading: false,
      error: null
    });
  });

  it("enters visible loading state while switching active company", async () => {
    const { shellStore } = await loadStores();
    const deferred = createDeferred<ShellStateDto>();

    invokeMock.mockImplementation(async (command) => {
      switch (command) {
        case "shell_load":
          return makeShellState("company-1");
        case "shell_set_active_company":
          return deferred.promise;
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await shellStore.load();

    const switchPromise = shellStore.setActiveCompany("company-2");
    expect(snapshot(shellStore).loading).toBe(true);
    expect(snapshot(shellStore).phase).toBe("company-switch");
    expect(snapshot(shellStore).pendingCompanyId).toBe("company-2");
    expect(snapshot(shellStore).progressLabel).toContain("компан");
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-1");

    deferred.resolve(makeShellState("company-2"));
    await switchPromise;

    expect(snapshot(shellStore).loading).toBe(false);
    expect(snapshot(shellStore).phase).toBe("idle");
    expect(snapshot(shellStore).pendingCompanyId).toBeNull();
    expect(snapshot(shellStore).progressLabel).toBeNull();
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-2");
  });

  it("exposes the initial loading contract before the first shell response arrives", async () => {
    const { shellStore } = await loadStores();
    const deferred = createDeferred<ShellStateDto>();

    invokeMock.mockImplementation(async (command) => {
      if (command === "shell_load") {
        return deferred.promise;
      }

      throw new Error(`unexpected command: ${command}`);
    });

    const loadPromise = shellStore.load();

    expect(snapshot(shellStore).loading).toBe(true);
    expect(snapshot(shellStore).phase).toBe("initial");
    expect(snapshot(shellStore).pendingCompanyId).toBeNull();
    expect(snapshot(shellStore).progressLabel).toContain("робочий простір");
    expect(snapshot(shellStore).state).toBeNull();

    deferred.resolve(makeShellState("company-1"));
    await loadPromise;

    expect(snapshot(shellStore).loading).toBe(false);
    expect(snapshot(shellStore).phase).toBe("idle");
  });

  it("exposes the refresh loading contract when shell data already exists", async () => {
    const { shellStore } = await loadStores();
    const deferred = createDeferred<ShellStateDto>();
    let loadCount = 0;

    invokeMock.mockImplementation(async (command) => {
      if (command !== "shell_load") {
        throw new Error(`unexpected command: ${command}`);
      }

      loadCount += 1;
      if (loadCount === 1) {
        return makeShellState("company-1");
      }

      return deferred.promise;
    });

    await shellStore.load();

    const refreshPromise = shellStore.load();

    expect(snapshot(shellStore).loading).toBe(true);
    expect(snapshot(shellStore).phase).toBe("refresh");
    expect(snapshot(shellStore).pendingCompanyId).toBeNull();
    expect(snapshot(shellStore).progressLabel).toContain("shell");
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-1");

    deferred.resolve(makeShellState("company-1"));
    await refreshPromise;

    expect(snapshot(shellStore).loading).toBe(false);
    expect(snapshot(shellStore).phase).toBe("idle");
  });

  it("keeps the previous shell state when refresh fails", async () => {
    const { shellStore } = await loadStores();
    let loadCount = 0;

    invokeMock.mockImplementation(async (command) => {
      if (command !== "shell_load") {
        throw new Error(`unexpected command: ${command}`);
      }

      loadCount += 1;
      if (loadCount === 1) {
        return makeShellState("company-1");
      }

      throw new Error("refresh failed");
    });

    await shellStore.load();
    const result = await shellStore.load();

    expect(result).toBeNull();
    expect(snapshot(shellStore).loading).toBe(false);
    expect(snapshot(shellStore).phase).toBe("idle");
    expect(snapshot(shellStore).pendingCompanyId).toBeNull();
    expect(snapshot(shellStore).progressLabel).toBeNull();
    expect(snapshot(shellStore).error).toContain("refresh failed");
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-1");
  });

  it("suppresses repeated company switches while a risky reload is already in progress", async () => {
    const { shellStore } = await loadStores();
    const deferred = createDeferred<ShellStateDto>();

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "shell_load":
          return makeShellState("company-1");
        case "shell_set_active_company":
          expect(payload).toEqual({ companyId: "company-2" });
          return deferred.promise;
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await shellStore.load();

    const firstSwitchPromise = shellStore.setActiveCompany("company-2");
    const secondSwitchPromise = shellStore.setActiveCompany("company-2");

    expect(invokeMock.mock.calls.filter(([command]) => command === "shell_set_active_company")).toHaveLength(1);
    expect(snapshot(shellStore).loading).toBe(true);

    deferred.resolve(makeShellState("company-2"));

    const [firstResult, secondResult] = await Promise.all([firstSwitchPromise, secondSwitchPromise]);
    expect(firstResult?.activeCompanyId).toBe("company-2");
    expect(secondResult?.activeCompanyId).toBe("company-2");
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-2");
  });

  it("clears switch-specific loading metadata after company switch failure", async () => {
    const { shellStore } = await loadStores();

    invokeMock.mockImplementation(async (command) => {
      switch (command) {
        case "shell_load":
          return makeShellState("company-1");
        case "shell_set_active_company":
          throw new Error("switch failed");
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await shellStore.load();
    const result = await shellStore.setActiveCompany("company-2");

    expect(result).toBeNull();
    expect(snapshot(shellStore).loading).toBe(false);
    expect(snapshot(shellStore).phase).toBe("idle");
    expect(snapshot(shellStore).pendingCompanyId).toBeNull();
    expect(snapshot(shellStore).progressLabel).toBeNull();
    expect(snapshot(shellStore).error).toContain("switch failed");
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-1");
  });
});
