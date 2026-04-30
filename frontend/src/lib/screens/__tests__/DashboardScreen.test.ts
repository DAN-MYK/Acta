/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import DashboardScreen from "../DashboardScreen.svelte";
import type { DashboardScreenDto } from "../../types";

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

  const dashboardState = createMockStore({
    screen: null as DashboardScreenDto | null,
    loading: false,
    error: null as string | null
  });

  return {
    dashboardState,
    dashboardLoad: vi.fn(),
    documentOpen: vi.fn(),
    navigationGo: vi.fn(),
    paymentOpenById: vi.fn(),
    taskOpenEditor: vi.fn()
  };
});

vi.mock("../../stores/dashboard", () => ({
  dashboardStore: {
    subscribe: mocks.dashboardState.subscribe,
    load: mocks.dashboardLoad
  }
}));

vi.mock("../../stores/documents", () => ({
  documentsStore: {
    open: mocks.documentOpen
  }
}));

vi.mock("../../stores/navigation", () => ({
  navigationStore: {
    go: mocks.navigationGo
  }
}));

vi.mock("../../stores/payments", () => ({
  paymentsStore: {
    openById: mocks.paymentOpenById
  }
}));

vi.mock("../../stores/tasks", () => ({
  tasksStore: {
    openEditor: mocks.taskOpenEditor
  }
}));

function makeDashboard(): DashboardScreenDto {
  return {
    kpis: [
      {
        label: "Документи",
        value: "12",
        detail: "3 очікують підпису",
        tone: "accent"
      }
    ],
    cashflowRows: [
      {
        key: "2026-04",
        label: "Квітень",
        incomeStr: "50 000,00 грн",
        expenseStr: "12 500,00 грн",
        netStr: "37 500,00 грн"
      }
    ],
    recentDocuments: [
      {
        id: "doc-1",
        kind: "act",
        number: "ACT-42",
        date: "2026-04-30",
        counterparty: "ТОВ Ромашка",
        amountStr: "9 600,00 грн",
        status: "issued",
        statusLabel: "Виставлено",
        linkedId: ""
      }
    ],
    upcomingPayments: [
      {
        id: "payment-1",
        dateLabel: "05 Тра",
        contractor: "ТОВ Ромашка",
        amountStr: "9 600,00 грн",
        isOverdue: true
      }
    ],
    urgentTasks: [
      {
        id: "task-1",
        title: "Підписати акт",
        description: "",
        status: "open",
        statusLabel: "Відкрите",
        priority: "high",
        priorityLabel: "Високий",
        dueDate: "2026-04-30",
        reminderAt: "",
        linkKind: "act",
        linkLabel: "ACT-42"
      }
    ]
  };
}

function renderDashboard() {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new DashboardScreen({ target });

  return { component, target };
}

function buttonByText(target: HTMLElement, text: string): HTMLButtonElement {
  const button = Array.from(target.querySelectorAll("button")).find((candidate) =>
    candidate.textContent?.includes(text)
  );

  expect(button).toBeTruthy();
  return button as HTMLButtonElement;
}

describe("DashboardScreen component", () => {
  beforeEach(() => {
    mocks.dashboardState.set({ screen: makeDashboard(), loading: false, error: null });
    mocks.dashboardLoad.mockReset();
    mocks.documentOpen.mockReset();
    mocks.navigationGo.mockReset();
    mocks.paymentOpenById.mockReset();
    mocks.taskOpenEditor.mockReset();
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders operational dashboard sections from the screen store", () => {
    const { component, target } = renderDashboard();

    expect(target.textContent).toContain("Дашборд");
    expect(target.textContent).toContain("Документи");
    expect(target.textContent).toContain("Грошовий потік");
    expect(target.textContent).toContain("ACT-42");
    expect(target.textContent).toContain("Найближчі платежі");
    expect(target.textContent).toContain("Підписати акт");

    component.$destroy();
  });

  it("opens dashboard rows through their owning stores", async () => {
    const { component, target } = renderDashboard();

    buttonByText(target, "ACT-42").click();
    await tick();
    expect(mocks.navigationGo).toHaveBeenCalledWith("documents");
    expect(mocks.documentOpen).toHaveBeenCalledWith("doc-1");

    buttonByText(target, "05 Тра").click();
    await tick();
    expect(mocks.navigationGo).toHaveBeenCalledWith("payments");
    expect(mocks.paymentOpenById).toHaveBeenCalledWith("payment-1");

    buttonByText(target, "Підписати акт").click();
    await tick();
    expect(mocks.navigationGo).toHaveBeenCalledWith("tasks");
    expect(mocks.taskOpenEditor).toHaveBeenCalledWith("task-1");

    component.$destroy();
  });

  it("keeps an explicit empty state for upcoming payments", () => {
    mocks.dashboardState.set({
      screen: { ...makeDashboard(), upcomingPayments: [] },
      loading: false,
      error: null
    });

    const { component, target } = renderDashboard();

    expect(target.textContent).toContain("Очікуваних платежів поки немає.");

    component.$destroy();
  });
});
