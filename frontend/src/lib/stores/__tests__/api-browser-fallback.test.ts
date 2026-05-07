/**
 * @vitest-environment jsdom
 */
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock
}));

describe("browser fallback API", () => {
  beforeEach(() => {
    vi.resetModules();
    invokeMock.mockReset();
    invokeMock.mockRejectedValue(new Error("tauri bridge unavailable"));
    window.__ACTA_FORCE_BROWSER_FIXTURES__ = true;
  });

  it("returns stable shell data in browser-dev mode when Tauri invoke is unavailable", async () => {
    const { shellLoad } = await import("../../api");

    const shell = await shellLoad();

    expect(shell.chrome.companyName).toBe("ТОВ Акт");
    expect(shell.companyItems.length).toBeGreaterThan(1);
    expect(shell.activeCompanyId).toBeTruthy();
  });

  it("returns localized reports data and preserves the requested filter", async () => {
    const { reportsLoad } = await import("../../api");

    const screen = await reportsLoad({
      tab: "receivables",
      scope: "all",
      dateFrom: "2026-02-01",
      dateTo: "2026-05-01",
      query: "ромашка",
      selectedCounterpartyId: null
    });

    expect(screen.filter).toEqual({
      tab: "receivables",
      scope: "all",
      dateFrom: "2026-02-01",
      dateTo: "2026-05-01",
      query: "ромашка",
      selectedCounterpartyId: null
    });
    expect(screen.receivablesRows.length).toBeGreaterThan(0);
    expect(screen.selectedCounterparty).toBeNull();
    expect(screen.topCounterparties).toEqual([]);
  });

  it("supports palette search without crashing outside Tauri runtime", async () => {
    const { shellPaletteSearch } = await import("../../api");

    const result = await shellPaletteSearch("звіт");

    expect(result.items.some((item) => item.title.includes("Звіти"))).toBe(true);
  });

  it("supports batch split reconcile in browser-dev mode", async () => {
    const { paymentReconcileSplit } = await import("../../api");

    const result = await paymentReconcileSplit({
      paymentId: "pay-2",
      allocations: [
        {
          documentKind: "invoice",
          documentId: "inv-7",
          amount: "1 500,00"
        },
        {
          documentKind: "act",
          documentId: "act-9",
          amount: "1 500,00"
        }
      ]
    });

    expect(result.ok).toBe(true);
    expect(result.message).toBe("Розподіл платежу підтверджено");
    expect(result.paymentId).toBe("pay-2");
    expect(result.allocationCount).toBe(2);
    expect(result.allocations.map((allocation) => allocation.documentId)).toEqual(["inv-7", "act-9"]);
  });

  it("returns a browser calendar payload for payments instead of throwing fixture errors", async () => {
    const { paymentsCalendarLoad } = await import("../../api");

    const result = await paymentsCalendarLoad({
      month: "2026-05",
      selectedDate: "2026-05-01"
    });

    expect(result.month).toBe("2026-05");
    expect(result.selectedDate).toBe("2026-05-01");
    expect(result.days.length).toBeGreaterThan(0);
  });

  it("returns a localized success message for document PDF generation in browser-dev mode", async () => {
    const { documentGeneratePdf } = await import("../../api");

    const result = await documentGeneratePdf("doc-1");

    expect(result.ok).toBe(true);
    expect(result.message).toContain("PDF");
  });
});
