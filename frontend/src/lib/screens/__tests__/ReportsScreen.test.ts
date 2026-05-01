/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
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
    loading: false,
    error: null as string | null,
    message: null as string | null
  });

  return {
    reportsState,
    load: vi.fn(),
    exportCsv: vi.fn()
  };
});

vi.mock("../../stores/reports", () => ({
  reportsStore: {
    subscribe: mocks.reportsState.subscribe,
    load: mocks.load,
    exportCsv: mocks.exportCsv
  }
}));

function makeReportsScreen(): ReportsScreenDto {
  return {
    filter: {
      tab: "bank",
      scope: "active",
      dateFrom: "2026-02-01",
      dateTo: "2026-05-01",
      query: ""
    },
    summary: {
      openingBalanceStr: "125 000,00 грн",
      incomeStr: "48 200,00 грн",
      expenseStr: "19 000,00 грн",
      closingBalanceStr: "154 200,00 грн",
      receivablesTotalStr: "23 000,00 грн",
      payablesTotalStr: "14 500,00 грн"
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

describe("ReportsScreen", () => {
  beforeEach(() => {
    mocks.load.mockReset();
    mocks.exportCsv.mockReset();
    mocks.reportsState.set({
      screen: makeReportsScreen(),
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
    expect(target.textContent).toContain("Контроль грошей і боргів");
    expect(target.textContent).toContain("Що аналізуємо");
    expect(target.textContent).toContain("У фокусі зараз");
    expect(target.textContent).toContain("Показувати");
    expect(target.textContent).not.toContain("Bank / receivables / payables");
    expect(target.textContent).not.toContain("Scope");

    component.$destroy();
  });

  it("renders canonical action classes and scenario tabs for report controls", () => {
    const { component, target } = renderReports();

    const exportButton = Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Експортувати CSV")
    );

    expect(exportButton).toBeTruthy();
    expect(exportButton?.className).toContain("btn-secondary");
    expect(target.textContent).toContain("Рух грошей");
    expect(target.textContent).toContain("Нам мають");
    expect(target.textContent).toContain("Ми винні");

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
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();

    expect(target.textContent).toContain("Дебіторка під контролем");
    expect(target.textContent).toContain("Прострочено 4 дн.");
    expect(target.textContent).toContain("Потрібно сьогодні");
    expect(target.querySelector(".reports-table-row-overdue")).toBeTruthy();

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
      loading: false,
      error: null,
      message: null
    });

    const { component, target } = renderReports();

    expect(target.textContent).toContain("На цей період немає записів");
    expect(target.textContent).toContain("Змініть період");

    component.$destroy();
  });
});
