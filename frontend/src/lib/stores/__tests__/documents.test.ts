/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", () => {
  return {
    documentsList: vi.fn(),
    documentOpen: vi.fn(),
    documentChainGet: vi.fn(),
    documentCreateDraft: vi.fn(),
    documentSave: vi.fn(),
    documentDelete: vi.fn(),
    documentAdvanceStatus: vi.fn(),
    documentChainCreateDraft: vi.fn(),
    documentGeneratePdf: vi.fn(),
    documentPdfApplyTextReplace: vi.fn(),
    documentPdfAttachExisting: vi.fn(),
    documentPdfOpenCurrent: vi.fn(),
    documentsBulkDelete: vi.fn(),
    documentsBulkAdvanceStatus: vi.fn(),
  };
});

import * as api from "../../api";
import { documentsStore } from "../documents";

const documentsListMock = api.documentsList as ReturnType<typeof vi.fn>;

const emptyList = {
  items: [],
  invoiceItems: [],
  actItems: [],
  waybillItems: [],
  totalCount: 0,
  pageCount: 0
};

beforeEach(() => {
  documentsListMock.mockReset();
  documentsListMock.mockResolvedValue(emptyList);
  documentsStore.clearAllFilters();
});

describe("documentsStore filter state", () => {
  it("starts without filters and without a search query field", () => {
    let snapshot: any;
    const unsub = documentsStore.subscribe((state) => { snapshot = state; });
    expect(snapshot.dateFrom).toBeNull();
    expect(snapshot.dateTo).toBeNull();
    expect(snapshot.statusFilter).toEqual([]);
    expect(snapshot.amountMin).toBeNull();
    expect(snapshot.amountMax).toBeNull();
    expect(snapshot.overdueOnly).toBe(false);
    expect(snapshot.activePresetId).toBeNull();
    expect("query" in snapshot).toBe(false);
    unsub();
  });
});
