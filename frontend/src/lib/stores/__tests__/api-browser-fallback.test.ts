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
  });

  it("supports palette search without crashing outside Tauri runtime", async () => {
    const { shellPaletteSearch } = await import("../../api");

    const result = await shellPaletteSearch("звіт");

    expect(result.items.some((item) => item.title.includes("Звіти"))).toBe(true);
  });
});
