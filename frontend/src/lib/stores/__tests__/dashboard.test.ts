import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DashboardScreenDto } from "../../types";
import { invokeMock, loadStores, snapshot } from "./helpers";

function makeDashboard(label = "Документи", suffix = "1"): DashboardScreenDto {
  return {
    kpis: [
      {
        label,
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
        key: `2026-0${suffix}`,
        label: `Квітень 202${suffix}`,
        incomeStr: "10 000,00 грн",
        expenseStr: "4 000,00 грн",
        netStr: "6 000,00 грн"
      }
    ],
    recentDocuments: [
      {
        id: `doc-${suffix}`,
        kind: "invoice",
        number: `INV-${suffix}`,
        date: "2026-04-30",
        counterparty: "ТОВ Ромашка",
        amountStr: "1 234,50 грн",
        direction: "outgoing",
        status: "issued",
        statusLabel: "Виставлено",
        linkedId: ""
      }
    ],
    upcomingPayments: [
      {
        id: `payment-${suffix}`,
        dateLabel: "05 Тра",
        contractor: "ТОВ Ромашка",
        amountStr: "8 450,00 грн",
        isOverdue: suffix === "2"
      }
    ],
    urgentTasks: [
      {
        id: `task-${suffix}`,
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

  it("starts on dashboard as the primary Tauri home screen", async () => {
    const { navigationStore } = await loadStores();

    expect(snapshot(navigationStore)).toBe("dashboard");
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

    expect(snapshot(dashboardStore).initialLoading).toBe(false);
    expect(snapshot(dashboardStore).loading).toBe(false);
    expect(snapshot(dashboardStore).error).toBeNull();
    expect(snapshot(dashboardStore).screen?.kpis).toHaveLength(2);
    expect(snapshot(dashboardStore).screen?.recentDocuments[0]?.id).toBe("doc-1");
    expect(snapshot(dashboardStore).screen?.upcomingPayments[0]?.id).toBe("payment-1");
  });

  it("keeps the dashboard error isolated in its store", async () => {
    const { dashboardStore } = await loadStores();

    invokeMock.mockRejectedValueOnce(new Error("dashboard unavailable"));

    await dashboardStore.load();

    expect(snapshot(dashboardStore).loading).toBe(false);
    expect(snapshot(dashboardStore).screen).toBeNull();
    expect(snapshot(dashboardStore).error).toContain("dashboard unavailable");
  }, 10000);

  it("ignores stale dashboard responses after a newer reload", async () => {
    const { dashboardStore } = await loadStores();

    let resolveFirst: ((value: DashboardScreenDto) => void) | undefined;
    let resolveSecond: ((value: DashboardScreenDto) => void) | undefined;

    invokeMock
      .mockImplementationOnce(
        () =>
          new Promise<DashboardScreenDto>((resolve) => {
            resolveFirst = resolve;
          })
      )
      .mockImplementationOnce(
        () =>
          new Promise<DashboardScreenDto>((resolve) => {
            resolveSecond = resolve;
          })
      );

    const firstLoad = dashboardStore.load();
    const secondLoad = dashboardStore.load();

    resolveSecond?.(makeDashboard("Оновлений dashboard", "2"));
    await secondLoad;

    resolveFirst?.(makeDashboard("Застарілий dashboard", "1"));
    await firstLoad;

    expect(snapshot(dashboardStore).loading).toBe(false);
    expect(snapshot(dashboardStore).screen?.kpis[0]?.label).toBe("Оновлений dashboard");
    expect(snapshot(dashboardStore).screen?.recentDocuments[0]?.id).toBe("doc-2");
    expect(snapshot(dashboardStore).screen?.upcomingPayments[0]?.id).toBe("payment-2");
    expect(snapshot(dashboardStore).screen?.upcomingPayments[0]?.isOverdue).toBe(true);
  });
});
