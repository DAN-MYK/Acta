/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import ReportsScreen from "../ReportsScreen.svelte";
import type { ReportsScreenDto } from "../../types";

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

  const reportsState = createMockStore({
    screen: null as ReportsScreenDto | null,
    initialLoading: false,
    loading: false,
    error: null as string | null,
    message: null as string | null
  });

  return {
    reportsState,
    load: vi.fn(),
    toggleCounterparty: vi.fn(),
    resetFilters: vi.fn(),
    exportCsv: vi.fn(),
    exportExcel: vi.fn(),
    exportExcelAndOpen: vi.fn()
  };
});

vi.mock("../../stores/reports", () => ({
  reportsStore: {
    subscribe: mocks.reportsState.subscribe,
    get load() {
      return mocks.load;
    },
    get toggleCounterparty() {
      return mocks.toggleCounterparty;
    },
    get resetFilters() {
      return mocks.resetFilters;
    },
    get exportCsv() {
      return mocks.exportCsv;
    },
    get exportExcel() {
      return mocks.exportExcel;
    },
    get exportExcelAndOpen() {
      return mocks.exportExcelAndOpen;
    }
  }
}));

function makeReportsScreen(): ReportsScreenDto {
  return {
    filter: {
      tab: "bank",
      scope: "active",
      dateFrom: "2026-02-01",
      dateTo: "2026-05-01",
      query: "",
      selectedCounterpartyId: null
    },
    selectedCounterparty: null,
    topCounterparties: [
      {
        counterpartyId: "cp-1",
        counterpartyName: "ТОВ Ромашка",
        primaryAmountStr: "48 200,00 грн",
        secondaryLabel: "Чистий рух",
        secondaryValue: "29 200,00 грн",
        sharePercent: 100
      }
    ],
    summary: {
      openingBalanceStr: "125 000,00 грн",
      incomeStr: "48 200,00 грн",
      expenseStr: "19 000,00 грн",
      closingBalanceStr: "154 200,00 грн",
      receivablesTotalStr: "23 000,00 грн",
      payablesTotalStr: "14 500,00 грн",
      pnlIncomeStr: "62 000,00 грн",
      pnlExpenseStr: "21 400,00 грн",
      pnlNetResultStr: "40 600,00 грн"
    },
    bankRows: [
      {
        key: "ops",
        label: "Операційна діяльність",
        incomeStr: "48 200,00 грн",
        expenseStr: "19 000,00 грн",
        netStr: "29 200,00 грн"
      }
    ],
    pnlRows: [
      {
        key: "services",
        label: "Послуги",
        incomeStr: "62 000,00 грн",
        expenseStr: "0,00 грн",
        netStr: "62 000,00 грн"
      }
    ],
    receivablesRows: [],
    payablesRows: []
  };
}

function renderReports() {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new ReportsScreen({ target });
  return { component, target };
}

function makeReportsApiResponse(filter: ReportsScreenDto["filter"]): ReportsScreenDto {
  return {
    ...makeReportsScreen(),
    filter,
    selectedCounterparty: filter.selectedCounterpartyId
      ? {
          id: filter.selectedCounterpartyId,
          name: "ТОВ Ромашка"
        }
      : null
  };
}

describe("ReportsScreen", () => {
  beforeEach(() => {
    mocks.load.mockReset();
    mocks.toggleCounterparty.mockReset();
    mocks.resetFilters.mockReset();
    mocks.exportCsv.mockReset();
    mocks.exportExcel.mockReset();
    mocks.exportExcelAndOpen.mockReset();
    mocks.reportsState.set({
      screen: makeReportsScreen(),
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("uses Ukrainian scenario-first microcopy in the header, focus card and filters", () => {
    const { component, target } = renderReports();

    expect(target.textContent).toContain("Звіти");
    expect(target.textContent).toContain("Гроші на рахунках і в русі");
    expect(target.textContent).toContain("Що показати у звіті");
    expect(target.textContent).toContain("Період від");
    expect(target.textContent).not.toContain("Bank / receivables / payables");
    expect(target.textContent).not.toContain("Scope");

    component.$destroy();
  });

  it("renders canonical action classes and scenario tabs for report controls", () => {
    const { component, target } = renderReports();
    const openExcelButton = Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Відкрити Excel")
    );
    const exportExcelButton = Array.from(target.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Експортувати Excel"
    );
    const exportButton = Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Експортувати CSV")
    );

    expect(openExcelButton).toBeTruthy();
    expect(openExcelButton?.className).toContain("btn-secondary");
    expect(openExcelButton?.className).not.toContain("btn-primary");
    expect(exportExcelButton).toBeTruthy();
    expect(exportExcelButton?.className).toContain("btn-ghost");
    expect(exportButton).toBeTruthy();
    expect(exportButton?.className).toContain("btn-ghost");
    expect(target.textContent).toContain("Гроші на рахунках і в русі");
    expect(target.textContent).toContain("Дохід, витрати і результат");
    expect(target.textContent).toContain("Нам мають заплатити");
    expect(target.textContent).toContain("Ми маємо заплатити");

    component.$destroy();
  });

  it("exposes stable smoke markers for the shell and native e2e layer", () => {
    const { component, target } = renderReports();

    expect(target.querySelector('[data-testid="reports-screen"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="reports-focus-primary"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="reports-table-card"]')).toBeTruthy();

    component.$destroy();
  });

  it("adapts KPI context and overdue emphasis to the active report tab", () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: {
          ...makeReportsScreen().filter,
          tab: "receivables"
        },
        receivablesRows: [
          {
            docId: "doc-2",
            docType: "invoice",
            docNumber: "INV-2026-0100",
            docDate: "2026-05-02",
            companyName: "ТОВ Акт",
            counterparty: "ПП Дніпро",
            amountStr: "12 000,00 грн",
            expectedDate: "2026-05-10",
            overdueDays: 0,
            status: "Очікується"
          },
          {
            docId: "doc-1",
            docType: "invoice",
            docNumber: "INV-2026-0042",
            docDate: "2026-05-01",
            companyName: "ТОВ Акт",
            counterparty: "ТОВ Ромашка",
            amountStr: "48 200,00 грн",
            expectedDate: "2026-05-05",
            overdueDays: 4,
            status: "Очікується"
          }
        ]
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    const rows = Array.from(target.querySelectorAll(".reports-table-row"))
      .slice(1)
      .map((row) => row.textContent ?? "");
    const activeTab = target.querySelector('[role="tab"][aria-selected="true"]');

    expect(target.textContent).toContain("Нам мають заплатити");
    expect(target.textContent).toContain("Прострочені оплати");
    expect(target.textContent).toContain("Прострочено 4 дн.");
    expect(target.querySelector(".reports-table-row-overdue")).toBeTruthy();
    expect(activeTab?.textContent).toContain("заплатити");
    expect(rows[0]).toContain("INV-2026-0042");

    component.$destroy();
  });

  it("shows a strong empty state when the active report has no rows", () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: {
          ...makeReportsScreen().filter,
          tab: "payables"
        },
        payablesRows: []
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();

    expect(target.textContent).toContain("На цей період немає записів");
    expect(target.textContent).toContain("Змініть період");

    component.$destroy();
  });

  it("renders pnl tab summary and category rows for management result view", () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: {
          ...makeReportsScreen().filter,
          tab: "pnl"
        }
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();

    expect(target.textContent).toContain("Дохід, витрати і результат");
    expect(target.textContent).toContain("Фінансовий результат за період");
    expect(target.textContent).toContain("62 000,00 грн");
    expect(target.textContent).toContain("40 600,00 грн");
    expect(target.textContent).toContain("Послуги");

    component.$destroy();
  });

  it("wires filters, tabs and export actions to the reports store contract", async () => {
    const { component, target } = renderReports();
    const searchInput = target.querySelector('input[type="search"], input[placeholder*="контрагента"]') as HTMLInputElement | null;
    const scopeSelect = target.querySelector("select") as HTMLSelectElement | null;
    const dateInputs = Array.from(target.querySelectorAll('input[type="date"]')) as HTMLInputElement[];
    const receivablesTab = Array.from(target.querySelectorAll('[role="tab"]')).find((button) =>
      button.textContent?.includes("Нам мають заплатити")
    ) as HTMLButtonElement | undefined;
    const exportCsvButton = Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Експортувати CSV")
    ) as HTMLButtonElement | undefined;
    const exportExcelButton = Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent === "Експортувати Excel"
    ) as HTMLButtonElement | undefined;
    const openExcelButton = Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Відкрити Excel")
    ) as HTMLButtonElement | undefined;

    expect(searchInput).toBeTruthy();
    expect(scopeSelect).toBeTruthy();
    expect(dateInputs).toHaveLength(2);
    expect(receivablesTab).toBeTruthy();
    expect(exportCsvButton).toBeTruthy();
    expect(exportExcelButton).toBeTruthy();
    expect(openExcelButton).toBeTruthy();

    searchInput!.value = "Ромашка";
    searchInput!.dispatchEvent(new Event("input", { bubbles: true }));
    scopeSelect!.value = "all";
    scopeSelect!.dispatchEvent(new Event("change", { bubbles: true }));
    dateInputs[0]!.value = "2026-03-01";
    dateInputs[0]!.dispatchEvent(new Event("input", { bubbles: true }));
    dateInputs[1]!.value = "2026-05-15";
    dateInputs[1]!.dispatchEvent(new Event("input", { bubbles: true }));
    receivablesTab!.click();
    exportCsvButton!.click();
    exportExcelButton!.click();
    openExcelButton!.click();
    await tick();

    expect(mocks.load).toHaveBeenCalledWith({ query: "Ромашка" });
    expect(mocks.load).toHaveBeenCalledWith({ scope: "all" });
    expect(mocks.load).toHaveBeenCalledWith({ dateFrom: "2026-03-01" });
    expect(mocks.load).toHaveBeenCalledWith({ dateTo: "2026-05-15" });
    expect(mocks.load).toHaveBeenCalledWith({ tab: "receivables" });
    expect(mocks.exportCsv).toHaveBeenCalled();
    expect(mocks.exportExcel).toHaveBeenCalled();
    expect(mocks.exportExcelAndOpen).toHaveBeenCalled();

    component.$destroy();
  });

  it("supports arrow-key navigation between report tabs", async () => {
    const { component, target } = renderReports();
    await tick();

    const bankTab = Array.from(target.querySelectorAll('[role="tab"]')).find((button) =>
      button.textContent?.includes("Гроші")
    ) as HTMLButtonElement | undefined;

    expect(bankTab).toBeTruthy();
    bankTab!.focus();
    bankTab!.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    await tick();

    expect(mocks.load).toHaveBeenCalledWith({ tab: "pnl" });
    expect(document.activeElement?.textContent).toContain("Дохід");

    component.$destroy();
  });

  it("keeps chrome visible and skeletonizes only the reports table during initial loading", () => {
    mocks.reportsState.set({
      screen: null,
      initialLoading: true,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();

    expect(target.textContent).toContain("Звіти");
    expect(target.textContent).toContain("Відкрити Excel");
    expect(target.textContent).toContain("Експортувати Excel");
    expect(target.textContent).toContain("Експортувати CSV");
    expect(target.querySelector('[data-testid="reports-focus-primary"]')).toBeTruthy();
    expect(target.querySelector(".reports-kpis")).toBeTruthy();
    expect(target.querySelector(".reports-filters")).toBeTruthy();
    expect(target.querySelector('[data-testid="reports-table-card"]')).toBeTruthy();
    expect(
      target.querySelector('[data-testid="reports-table-card"]')?.querySelectorAll('[data-testid="skeleton-row-item"]')
    ).toHaveLength(6);
    expect(target.querySelector('[data-testid="reports-empty-state"]')).toBeNull();
    expect(target.textContent).not.toContain("Контрагентів немає у вибраному діапазоні.");
    expect(target.querySelector('[data-testid="reports-top-counterparties-skeleton"]')).toBeTruthy();

    component.$destroy();
  });

  it("does not return skeletons during operational export loading after initial load", () => {
    mocks.reportsState.set({
      screen: makeReportsScreen(),
      initialLoading: false,
      loading: true,
      error: null,
      message: null
    });

    const { component, target } = renderReports();

    expect(target.querySelector('[data-testid="reports-table-card"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="skeleton-row-item"]')).toBeNull();
    expect(target.textContent).toContain("Відкрити Excel");
    expect(target.textContent).toContain("Експортувати CSV");

    component.$destroy();
  });

  it("renders ranking numbers and a legend hint inside top counterparties", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        topCounterparties: [
          {
            counterpartyId: "cp-1",
            counterpartyName: "ТОВ Ромашка",
            primaryAmountStr: "48 200,00 грн",
            secondaryLabel: "Чистий рух",
            secondaryValue: "29 200,00 грн",
            sharePercent: 100
          },
          {
            counterpartyId: "cp-2",
            counterpartyName: "ПП Дніпро",
            primaryAmountStr: "32 000,00 грн",
            secondaryLabel: "Чистий рух",
            secondaryValue: "12 000,00 грн",
            sharePercent: 66
          }
        ]
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    const ranks = Array.from(target.querySelectorAll(".reports-top-cp-rank")).map(
      (node) => node.textContent?.trim()
    );
    expect(ranks).toEqual(["1", "2"]);
    expect(target.textContent).toContain("Сортовано за загальним рухом грошей");
    expect(target.textContent).toContain("% — частка від лідера");

    component.$destroy();
  });

  it("shows drill CTA only for an active counterparty with a real id and active scope", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: { ...makeReportsScreen().filter, selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" },
        topCounterparties: [
          {
            counterpartyId: "cp-1",
            counterpartyName: "ТОВ Ромашка",
            primaryAmountStr: "48 200,00 грн",
            secondaryLabel: "Чистий рух",
            secondaryValue: "29 200,00 грн",
            sharePercent: 100
          }
        ]
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    const drillCta = target.querySelector(
      '[data-testid="top-counterparty-open-cp-1"]'
    ) as HTMLButtonElement | null;
    expect(drillCta).toBeTruthy();
    expect(drillCta?.textContent).toContain("Відкрити картку контрагента");

    component.$destroy();
  });

  it("hides drill CTA for bank-name pseudo rows and tags them as без картки", async () => {
    const bankNameId = "bank-name:Privat";
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: { ...makeReportsScreen().filter, selectedCounterpartyId: bankNameId },
        selectedCounterparty: { id: bankNameId, name: "Privat" },
        topCounterparties: [
          {
            counterpartyId: bankNameId,
            counterpartyName: "Privat",
            primaryAmountStr: "12 000,00 грн",
            secondaryLabel: "Чистий рух",
            secondaryValue: "12 000,00 грн",
            sharePercent: 100
          }
        ]
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    expect(target.querySelector(`[data-testid="top-counterparty-open-${bankNameId}"]`)).toBeNull();
    expect(target.textContent).toContain("без картки");

    component.$destroy();
  });

  it("hides drill CTA when scope is all even for a real counterparty id", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: {
          ...makeReportsScreen().filter,
          scope: "all",
          selectedCounterpartyId: "cp-1"
        },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    expect(target.querySelector('[data-testid="top-counterparty-open-cp-1"]')).toBeNull();

    component.$destroy();
  });

  it("shows count of rows visible in the table when drill-down is active", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: {
          ...makeReportsScreen().filter,
          tab: "receivables",
          selectedCounterpartyId: "cp-1"
        },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" },
        receivablesRows: [
          {
            docId: "doc-1",
            docType: "invoice",
            docNumber: "INV-1",
            docDate: "2026-05-01",
            companyName: "ТОВ Акт",
            counterparty: "ТОВ Ромашка",
            amountStr: "10 000,00 грн",
            expectedDate: "2026-05-15",
            overdueDays: 0,
            status: "Очікується"
          },
          {
            docId: "doc-2",
            docType: "act",
            docNumber: "ACT-9",
            docDate: "2026-04-29",
            companyName: "ТОВ Акт",
            counterparty: "ТОВ Ромашка",
            amountStr: "8 000,00 грн",
            expectedDate: "2026-05-08",
            overdueDays: 0,
            status: "Очікується"
          }
        ]
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    expect(target.textContent).toContain("У таблиці нижче: 2");

    component.$destroy();
  });

  it("disables export buttons during operational loading and signals aria-busy", async () => {
    mocks.reportsState.set({
      screen: makeReportsScreen(),
      initialLoading: false,
      loading: true,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    const exportButtons = Array.from(target.querySelectorAll("button")).filter((button) =>
      button.textContent?.includes("Excel") || button.textContent?.includes("CSV")
    ) as HTMLButtonElement[];
    expect(exportButtons.length).toBeGreaterThan(0);
    for (const button of exportButtons) {
      expect(button.disabled).toBe(true);
    }

    const screen = target.querySelector('[data-testid="reports-screen"]');
    expect(screen?.getAttribute("aria-busy")).toBe("true");

    component.$destroy();
  });

  it("renders Скинути фільтри action in the empty state and triggers store reset", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: { ...makeReportsScreen().filter, tab: "payables" },
        payablesRows: []
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    const reset = target.querySelector(
      '[data-testid="reports-empty-reset-action"]'
    ) as HTMLButtonElement | null;
    expect(reset).toBeTruthy();
    reset!.click();
    await tick();

    expect(mocks.resetFilters).toHaveBeenCalled();

    component.$destroy();
  });

  it("calls toggleCounterparty when user clicks a top-counterparty row", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        topCounterparties: [
          {
            counterpartyId: "cp-1",
            counterpartyName: "ТОВ Ромашка",
            primaryAmountStr: "48 200,00 грн",
            secondaryLabel: "Чистий рух",
            secondaryValue: "29 200,00 грн",
            sharePercent: 100
          }
        ]
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    const row = target.querySelector('[data-testid="top-counterparty-cp-1"]') as HTMLButtonElement | null;
    expect(row).toBeTruthy();
    row!.click();
    await tick();

    expect(mocks.toggleCounterparty).toHaveBeenCalledWith("cp-1");

    component.$destroy();
  });

  it("renders top counterparties card with focus state and reset button", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: { ...makeReportsScreen().filter, selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });
    const { component, target } = renderReports();
    await tick();

    expect(target.querySelector('[data-testid="reports-top-counterparties"]')).toBeTruthy();
    expect(target.textContent).toContain("Топ контрагентів");
    expect(target.textContent).toContain("Фокус: ТОВ Ромашка");
    expect(target.textContent).toContain("Скинути");

    component.$destroy();
  });

  it("renders context text matching selected counterparty and tab", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: { ...makeReportsScreen().filter, tab: "receivables", selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });
    const { component, target } = renderReports();
    await tick();

    expect(target.textContent).toContain("Показано: дебіторка по контрагенту ТОВ Ромашка");

    component.$destroy();
  });

  it("shows a local empty state when top counterparties ranking is empty", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        topCounterparties: []
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    expect(target.textContent).toContain("Контрагентів немає у вибраному діапазоні.");

    component.$destroy();
  });

  it("clears focus through toggleCounterparty when user clicks reset", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: { ...makeReportsScreen().filter, selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    const resetButton = Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Скинути")
    ) as HTMLButtonElement | undefined;

    expect(resetButton).toBeTruthy();
    resetButton!.click();
    await tick();

    expect(mocks.toggleCounterparty).toHaveBeenCalledWith("cp-1");

    component.$destroy();
  });

  it("resets selectedCounterpartyId to null when tab changes", async () => {
    mocks.reportsState.set({
      screen: {
        ...makeReportsScreen(),
        filter: { ...makeReportsScreen().filter, selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      },
      initialLoading: false,
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();
    await tick();

    const pnlTab = Array.from(target.querySelectorAll('[role="tab"]')).find((b) =>
      b.textContent?.includes("Дохід, витрати і результат")
    ) as HTMLButtonElement | undefined;
    expect(pnlTab).toBeTruthy();
    pnlTab!.click();
    await tick();

    expect(mocks.load).toHaveBeenCalledWith({ tab: "pnl" });

    component.$destroy();
  });
});

describe("reportsStore", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it("loads drill-down filter when selecting a top counterparty", async () => {
    const reportsLoad = vi
      .fn<(filter: ReportsScreenDto["filter"]) => Promise<ReportsScreenDto>>()
      .mockImplementation(async (filter) => makeReportsApiResponse(filter));

    vi.doMock("../../api", () => ({
      reportsLoad,
      reportsExportCsv: vi.fn(),
      reportsExportExcel: vi.fn(),
      reportsExportExcelAndOpen: vi.fn()
    }));

    const { createReportsStore } = await vi.importActual<typeof import("../../stores/reports")>("../../stores/reports");
    const store = createReportsStore();
    let latestState: ReportsScreenDto | null = null;
    const unsubscribe = store.subscribe((state) => {
      latestState = state.screen;
    });

    await store.load();
    await store.toggleCounterparty("cp-1");

    expect(reportsLoad).toHaveBeenCalledTimes(2);
    expect(reportsLoad).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        selectedCounterpartyId: "cp-1"
      })
    );
    if (!latestState) {
      throw new Error("Очікували стан звіту після перемикання контрагента");
    }
    const stateAfterToggle = latestState as unknown as ReportsScreenDto;
    expect(stateAfterToggle.filter.selectedCounterpartyId).toBe("cp-1");
    unsubscribe();
  });

  it("resets selectedCounterpartyId to null when tab changes", async () => {
    const reportsLoad = vi
      .fn<(filter: ReportsScreenDto["filter"]) => Promise<ReportsScreenDto>>()
      .mockImplementation(async (filter) => makeReportsApiResponse(filter));

    vi.doMock("../../api", () => ({
      reportsLoad,
      reportsExportCsv: vi.fn(),
      reportsExportExcel: vi.fn(),
      reportsExportExcelAndOpen: vi.fn()
    }));

    const { createReportsStore } = await vi.importActual<typeof import("../../stores/reports")>("../../stores/reports");
    const store = createReportsStore();
    let latestState: ReportsScreenDto | null = null;
    const unsubscribe = store.subscribe((state) => {
      latestState = state.screen;
    });

    await store.load({ selectedCounterpartyId: "cp-1" });
    if (!latestState) {
      throw new Error("Очікували стан звіту перед зміною вкладки");
    }
    const stateBeforeTabReset = latestState as unknown as ReportsScreenDto;
    expect(stateBeforeTabReset.filter.selectedCounterpartyId).toBe("cp-1");
    await store.load({ tab: "pnl" });

    expect(reportsLoad).toHaveBeenCalledTimes(2);
    expect(reportsLoad).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        tab: "pnl",
        selectedCounterpartyId: null
      })
    );
    if (!latestState) {
      throw new Error("Очікували стан звіту після зміни вкладки");
    }
    const stateAfterTabReset = latestState as unknown as ReportsScreenDto;
    expect(stateAfterTabReset.filter.selectedCounterpartyId).toBeNull();
    unsubscribe();
  });
});
