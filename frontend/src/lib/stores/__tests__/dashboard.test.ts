import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DashboardScreenDto } from "../../types";
import { invokeMock, loadStores, snapshot } from "./helpers";

function makeDashboard(): DashboardScreenDto {
  return {
    kpis: [
      {
        label: "Документи",
        value: "12",
        detail: "1 сторінка у поточній вибірці",
        tone: "neutral"
      },
      {
        label: "Завдання",
        value: "3",
        detail: "1 сьогодні, 2 високий пріоритет",
        tone: "accent"
      }
    ],
    cashflowRows: [
      {
        key: "2026-04",
        label: "Квітень 2026",
        incomeStr: "10 000,00 грн",
        expenseStr: "4 000,00 грн",
        netStr: "6 000,00 грн"
      }
    ],
    recentDocuments: [
      {
        id: "doc-1",
        kind: "invoice",
        number: "INV-1",
        date: "2026-04-30",
        counterparty: "ТОВ Ромашка",
        amountStr: "1 234,50 грн",
        status: "draft",
        statusLabel: "Чернетка",
        linkedId: ""
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
        linkKind: "",
        linkLabel: ""
      }
    ]
  };
}

describe("frontend Tauri store smoke: dashboard", () => {
  beforeEach(() => {
    vi.resetModules();
    invokeMock.mockReset();
  });

  it("loads dashboard through the dedicated Tauri command", async () => {
    const { dashboardStore } = await loadStores();
    const dashboard = makeDashboard();

    invokeMock.mockImplementation(async (command, payload) => {
      expect(command).toBe("dashboard_load");
      expect(payload).toBeUndefined();
      return dashboard;
    });

    await dashboardStore.load();

    expect(snapshot(dashboardStore).loading).toBe(false);
    expect(snapshot(dashboardStore).error).toBeNull();
    expect(snapshot(dashboardStore).screen?.kpis).toHaveLength(2);
    expect(snapshot(dashboardStore).screen?.recentDocuments[0]?.id).toBe("doc-1");
  });

  it("keeps the dashboard error isolated in its store", async () => {
    const { dashboardStore } = await loadStores();

    invokeMock.mockRejectedValueOnce(new Error("dashboard unavailable"));

    await dashboardStore.load();

    expect(snapshot(dashboardStore).loading).toBe(false);
    expect(snapshot(dashboardStore).screen).toBeNull();
    expect(snapshot(dashboardStore).error).toContain("dashboard unavailable");
  });
});
