import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReportsScreenDto } from "../../types";
import { invokeMock, loadStores, snapshot } from "./helpers";

function makeReportsScreen(overrides: Partial<ReportsScreenDto> = {}): ReportsScreenDto {
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
    topCounterparties: [],
    summary: {
      openingBalanceStr: "0,00 грн",
      incomeStr: "0,00 грн",
      expenseStr: "0,00 грн",
      closingBalanceStr: "0,00 грн",
      receivablesTotalStr: "0,00 грн",
      payablesTotalStr: "0,00 грн"
    },
    bankRows: [],
    pnlRows: [],
    receivablesRows: [],
    payablesRows: [],
    ...overrides
  };
}

describe("reportsStore filter lifecycle", () => {
  beforeEach(() => {
    vi.resetModules();
    invokeMock.mockReset();
  });

  it("includes selectedCounterpartyId: null in defaultFilter and passes it on first load", async () => {
    const { reportsStore } = await loadStores();

    invokeMock.mockResolvedValue(
      makeReportsScreen({ filter: { tab: "bank", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: null } })
    );

    await reportsStore.load();

    expect(invokeMock).toHaveBeenCalledWith(
      "reports_load",
      expect.objectContaining({
        request: expect.objectContaining({ selectedCounterpartyId: null })
      })
    );
  });

  it("passes selectedCounterpartyId when load is called with it directly", async () => {
    const { reportsStore } = await loadStores();

    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "bank", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      })
    );

    await reportsStore.load({ selectedCounterpartyId: "cp-1" });

    expect(invokeMock).toHaveBeenCalledWith(
      "reports_load",
      expect.objectContaining({
        request: expect.objectContaining({ selectedCounterpartyId: "cp-1" })
      })
    );
  });

  it("resets selectedCounterpartyId to null when tab changes", async () => {
    const { reportsStore } = await loadStores();

    // First load: set selectedCounterpartyId
    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "bank", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      })
    );
    await reportsStore.load({ selectedCounterpartyId: "cp-1" });

    // Second load: change tab → selectedCounterpartyId must be reset
    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "pnl", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: null }
      })
    );
    await reportsStore.load({ tab: "pnl" });

    expect(invokeMock).toHaveBeenLastCalledWith(
      "reports_load",
      expect.objectContaining({
        request: expect.objectContaining({ tab: "pnl", selectedCounterpartyId: null })
      })
    );
  });

  it("resets selectedCounterpartyId to null when scope changes", async () => {
    const { reportsStore } = await loadStores();

    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "bank", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      })
    );
    await reportsStore.load({ selectedCounterpartyId: "cp-1" });

    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "bank", scope: "all", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: null }
      })
    );
    await reportsStore.load({ scope: "all" });

    expect(invokeMock).toHaveBeenLastCalledWith(
      "reports_load",
      expect.objectContaining({
        request: expect.objectContaining({ scope: "all", selectedCounterpartyId: null })
      })
    );
  });

  it("resets selectedCounterpartyId to null when query changes", async () => {
    const { reportsStore } = await loadStores();

    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "bank", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      })
    );
    await reportsStore.load({ selectedCounterpartyId: "cp-1" });

    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "bank", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "Ромашка", selectedCounterpartyId: null }
      })
    );
    await reportsStore.load({ query: "Ромашка" });

    expect(invokeMock).toHaveBeenLastCalledWith(
      "reports_load",
      expect.objectContaining({
        request: expect.objectContaining({ query: "Ромашка", selectedCounterpartyId: null })
      })
    );
  });

  it("does not reset selectedCounterpartyId when load is called with selectedCounterpartyId explicitly", async () => {
    const { reportsStore } = await loadStores();

    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "bank", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      })
    );
    await reportsStore.load({ selectedCounterpartyId: "cp-1" });

    // Calling load with selectedCounterpartyId again (toggle off)
    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "bank", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: null }
      })
    );
    await reportsStore.load({ selectedCounterpartyId: null });

    expect(invokeMock).toHaveBeenLastCalledWith(
      "reports_load",
      expect.objectContaining({
        request: expect.objectContaining({ selectedCounterpartyId: null })
      })
    );
  });

  it("toggleCounterparty selects a counterparty when none is selected", async () => {
    const { reportsStore } = await loadStores();

    // Initial load with no counterparty selected
    invokeMock.mockResolvedValue(makeReportsScreen());
    await reportsStore.load();

    // Toggle on cp-1
    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "bank", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      })
    );
    await reportsStore.toggleCounterparty("cp-1");

    expect(invokeMock).toHaveBeenLastCalledWith(
      "reports_load",
      expect.objectContaining({
        request: expect.objectContaining({ selectedCounterpartyId: "cp-1" })
      })
    );
  });

  it("toggleCounterparty deselects a counterparty when that counterparty is already selected", async () => {
    const { reportsStore } = await loadStores();

    // Load with cp-1 selected
    invokeMock.mockResolvedValue(
      makeReportsScreen({
        filter: { tab: "bank", scope: "active", dateFrom: "2026-02-01", dateTo: "2026-05-01", query: "", selectedCounterpartyId: "cp-1" },
        selectedCounterparty: { id: "cp-1", name: "ТОВ Ромашка" }
      })
    );
    await reportsStore.load({ selectedCounterpartyId: "cp-1" });

    // Toggle the same cp-1 → should deselect (pass null)
    invokeMock.mockResolvedValue(makeReportsScreen());
    await reportsStore.toggleCounterparty("cp-1");

    expect(invokeMock).toHaveBeenLastCalledWith(
      "reports_load",
      expect.objectContaining({
        request: expect.objectContaining({ selectedCounterpartyId: null })
      })
    );
  });
});
