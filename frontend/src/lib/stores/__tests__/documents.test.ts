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
    counterpartyOpenEditor: vi.fn(),
    counterpartySave: vi.fn(),
    documentChangeCounterparty: vi.fn(),
  };
});

import * as api from "../../api";
import type { CounterpartyEditorDto } from "../../types";
import { documentsStore } from "../documents";

const documentsListMock = api.documentsList as ReturnType<typeof vi.fn>;
const counterpartyOpenEditorMock = api.counterpartyOpenEditor as ReturnType<typeof vi.fn>;
const counterpartySaveMock = api.counterpartySave as ReturnType<typeof vi.fn>;
const documentChangeCounterpartyMock = api.documentChangeCounterparty as ReturnType<typeof vi.fn>;

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

const emptyEditorDto: CounterpartyEditorDto = {
  form: {
    id: "",
    title: "Новий контрагент",
    name: "",
    edrpou: "",
    ipn: "",
    iban: "",
    address: "",
    phone: "",
    email: "",
    notes: "",
  },
  showEditor: true,
};

const filledEditorDto: CounterpartyEditorDto = {
  form: {
    id: "cp-1",
    title: "ТОВ Ромашка",
    name: "ТОВ Ромашка",
    edrpou: "12345678",
    ipn: "",
    iban: "UA123",
    address: "",
    phone: "",
    email: "",
    notes: "",
  },
  showEditor: true,
};

function getState() {
  let s: any;
  documentsStore.subscribe((st) => { s = st; })();
  return s;
}

describe("documentsStore cpModal", () => {
  beforeEach(() => {
    counterpartyOpenEditorMock.mockReset();
    counterpartySaveMock.mockReset();
    documentChangeCounterpartyMock.mockReset();
  });

  it("openCpCreate sets cpModal with mode create and isOpen true", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    const state = getState();
    expect(state.cpModal).not.toBeNull();
    expect(state.cpModal.mode).toBe("create");
    expect(state.cpModal.isOpen).toBe(true);
    expect(state.cpModal.confirmClose).toBe(false);
    expect(state.cpModal.form).toEqual(emptyEditorDto.form);
    expect(state.cpModal.snapshot).toEqual(emptyEditorDto.form);
  });

  it("openCpEdit sets cpModal with mode edit and filled form", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(filledEditorDto);
    await documentsStore.openCpEdit("cp-1");
    const state = getState();
    expect(state.cpModal.mode).toBe("edit");
    expect(state.cpModal.form.name).toBe("ТОВ Ромашка");
    expect(counterpartyOpenEditorMock).toHaveBeenCalledWith("cp-1");
  });

  it("updateCpField updates form field without touching snapshot", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    documentsStore.updateCpField("name", "Нова назва");
    const state = getState();
    expect(state.cpModal.form.name).toBe("Нова назва");
    expect(state.cpModal.snapshot.name).toBe("");
  });

  it("closeCpModal when not dirty sets cpModal to null", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    documentsStore.closeCpModal();
    expect(getState().cpModal).toBeNull();
  });

  it("closeCpModal when dirty sets confirmClose to true without closing", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    documentsStore.updateCpField("name", "змінено");
    documentsStore.closeCpModal();
    const state = getState();
    expect(state.cpModal).not.toBeNull();
    expect(state.cpModal.confirmClose).toBe(true);
  });

  it("confirmCloseCpModal closes modal regardless of dirty state", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    documentsStore.updateCpField("name", "змінено");
    documentsStore.confirmCloseCpModal();
    expect(getState().cpModal).toBeNull();
  });

  it("cancelCloseCpModal sets confirmClose back to false", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    documentsStore.updateCpField("name", "змінено");
    documentsStore.closeCpModal();
    documentsStore.cancelCloseCpModal();
    const state = getState();
    expect(state.cpModal).not.toBeNull();
    expect(state.cpModal.confirmClose).toBe(false);
  });
});
