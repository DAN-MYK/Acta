/**
 * @vitest-environment jsdom
 */
// @ts-ignore Node typings are not included in the frontend test tsconfig.
import { readFileSync } from "fs";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import DocumentsScreen from "../DocumentsScreen.svelte";
import type { DocumentChainDto, DocumentEditorDto, DocumentsListDto } from "../../types";

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

  const documentsState = createMockStore({
    list: null as DocumentsListDto | null,
    editor: null as DocumentEditorDto | null,
    chain: null as DocumentChainDto | null,
    draftContext: null as { counterpartyId: string; counterpartyName: string } | null,
    selectedIds: [] as string[],
    initialLoading: false,
    loading: false,
    error: null as string | null,
    message: null as string | null,
    activeTab: "all" as const,
    kindFilter: null,
    counterpartyFilterId: null as string | null,
    dateFrom: null as string | null,
    dateTo: null as string | null,
    statusFilter: [] as string[],
    amountMin: null as string | null,
    amountMax: null as string | null,
    overdueOnly: false,
    activePresetId: null as string | null
  });

  const counterpartiesState = createMockStore({
    screen: {
      items: [
        { id: "counterparty-1", name: "ТОВ Ромашка" },
        { id: "counterparty-2", name: "ФОП Тест" }
      ]
    }
  });

  return {
    counterpartiesState,
    documentsState,
    addItem: vi.fn(),
    advanceStatus: vi.fn(),
    applyPdfTextReplace: vi.fn(),
    attachExistingPdf: vi.fn(),
    bulkAdvanceStatus: vi.fn(),
    bulkDelete: vi.fn(),
    closeEditor: vi.fn(() => ({ ok: true })),
    create: vi.fn(),
    createChainDraft: vi.fn(),
    deleteCurrent: vi.fn(),
    generatePdf: vi.fn(),
    open: vi.fn(),
    openCurrentPdf: vi.fn(),
    reloadCurrent: vi.fn(),
    removeItem: vi.fn(),
    save: vi.fn(),
    selectAllVisible: vi.fn(),
    setCounterpartyFilter: vi.fn(),
    setKindFilter: vi.fn(),
    setTab: vi.fn(),
    toggleSelected: vi.fn(),
    updateFormField: vi.fn(),
    updateItemField: vi.fn(),
    applyPreset: vi.fn(),
    applyFilters: vi.fn(),
    clearAllFilters: vi.fn(),
    setDateRange: vi.fn(),
    setStatusFilter: vi.fn(),
    setAmountRange: vi.fn()
  };
});

vi.mock("../../stores/documents", () => ({
  documentsStore: {
    subscribe: mocks.documentsState.subscribe,
    addItem: mocks.addItem,
    advanceStatus: mocks.advanceStatus,
    applyPdfTextReplace: mocks.applyPdfTextReplace,
    attachExistingPdf: mocks.attachExistingPdf,
    bulkAdvanceStatus: mocks.bulkAdvanceStatus,
    bulkDelete: mocks.bulkDelete,
    closeEditor: mocks.closeEditor,
    create: mocks.create,
    createChainDraft: mocks.createChainDraft,
    deleteCurrent: mocks.deleteCurrent,
    generatePdf: mocks.generatePdf,
    open: mocks.open,
    openCurrentPdf: mocks.openCurrentPdf,
    reloadCurrent: mocks.reloadCurrent,
    removeItem: mocks.removeItem,
    save: mocks.save,
    selectAllVisible: mocks.selectAllVisible,
    setCounterpartyFilter: mocks.setCounterpartyFilter,
    setKindFilter: mocks.setKindFilter,
    setTab: mocks.setTab,
    toggleSelected: mocks.toggleSelected,
    updateFormField: mocks.updateFormField,
    updateItemField: mocks.updateItemField,
    applyPreset: mocks.applyPreset,
    applyFilters: mocks.applyFilters,
    clearAllFilters: mocks.clearAllFilters,
    setDateRange: mocks.setDateRange,
    setStatusFilter: mocks.setStatusFilter,
    setAmountRange: mocks.setAmountRange
  }
}));

vi.mock("../../stores/counterparties", () => ({
  counterpartiesStore: {
    subscribe: mocks.counterpartiesState.subscribe
  }
}));

function makeList(): DocumentsListDto {
  return {
    items: [
      {
        id: "doc-1",
        kind: "invoice",
        number: "INV-7",
        date: "2026-04-30",
        counterparty: "ТОВ Ромашка",
        amountStr: "5 000,00 грн",
        direction: "outgoing",
        status: "draft",
        statusLabel: "Чернетка",
        linkedId: ""
      },
      {
        id: "doc-2",
        kind: "act",
        number: "ACT-9",
        date: "2026-05-01",
        counterparty: "ФОП Тест",
        amountStr: "2 500,00 грн",
        direction: "outgoing",
        status: "issued",
        statusLabel: "Виставлено",
        linkedId: ""
      }
    ],
    invoiceItems: [],
    actItems: [],
    waybillItems: [],
    totalCount: 2,
    pageCount: 1
  };
}

function makeEditor(items: boolean | DocumentEditorDto["items"] = true): DocumentEditorDto {
  return {
    form: {
      id: "doc-1",
      kind: "invoice",
      direction: "outgoing",
      counterpartyId: "counterparty-1",
      counterpartyName: "ТОВ Ромашка",
      title: "Рахунок INV-7",
      number: "INV-7",
      date: "2026-04-30",
      notes: "Початковий коментар"
    },
    items: Array.isArray(items)
      ? items
      : items
        ? [
            {
              description: "Консультація",
              unit: "год",
              quantity: "2",
              price: "2500"
            }
          ]
        : [],
    pdf: {
      filePath: "C:/tmp/working.pdf",
      pageCount: 1,
      extractedText: "DRAFT STATUS",
      hasTextOps: true,
      editable: true,
      warnings: []
    },
    showTypePicker: false,
    showEditor: true
  };
}

function makeChain(): DocumentChainDto {
  return {
    sourceId: "doc-1",
    steps: [
      {
        docType: "invoice",
        docNumber: "INV-7",
        amountStr: "5 000,00 грн",
        status: "Чернетка",
        exists: true
      }
    ]
  };
}

function setDocumentsState(selectedIds: string[] = [], items: boolean | DocumentEditorDto["items"] = true) {
  mocks.documentsState.set({
    list: makeList(),
    editor: makeEditor(items),
    chain: makeChain(),
    draftContext: {
      counterpartyId: "counterparty-1",
      counterpartyName: "ТОВ Ромашка"
    },
    selectedIds,
    initialLoading: false,
    loading: false,
    error: null,
    message: "Готово",
    activeTab: "all",
    kindFilter: null,
    counterpartyFilterId: null,
    dateFrom: null,
    dateTo: null,
    statusFilter: [],
    amountMin: null,
    amountMax: null,
    overdueOnly: false,
    activePresetId: null
  });
}

function setDocumentsStateWithoutDraftContext() {
  mocks.documentsState.set({
    list: makeList(),
    editor: makeEditor(),
    chain: makeChain(),
    draftContext: null,
    selectedIds: [],
    initialLoading: false,
    loading: false,
    error: null,
    message: "Готово",
    activeTab: "all",
    kindFilter: null,
    counterpartyFilterId: null,
    dateFrom: null,
    dateTo: null,
    statusFilter: [],
    amountMin: null,
    amountMax: null,
    overdueOnly: false,
    activePresetId: null
  });
}

function renderDocuments() {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new DocumentsScreen({ target });

  return { component, target };
}

function buttonByText(target: HTMLElement, text: string): HTMLButtonElement {
  const button = Array.from(target.querySelectorAll("button")).find((candidate) =>
    candidate.textContent?.includes(text)
  );

  expect(button).toBeTruthy();
  return button as HTMLButtonElement;
}

describe("DocumentsScreen component", () => {
  const source = readFileSync("frontend/src/lib/screens/DocumentsScreen.svelte", "utf8");
  const styles = readFileSync("frontend/src/styles/documents.css", "utf8");

  beforeEach(() => {
    setDocumentsState();

    for (const fn of [
      mocks.addItem,
      mocks.advanceStatus,
      mocks.applyPdfTextReplace,
      mocks.attachExistingPdf,
      mocks.bulkAdvanceStatus,
      mocks.bulkDelete,
      mocks.closeEditor,
      mocks.create,
      mocks.createChainDraft,
      mocks.deleteCurrent,
      mocks.generatePdf,
      mocks.open,
      mocks.openCurrentPdf,
      mocks.reloadCurrent,
      mocks.removeItem,
      mocks.save,
      mocks.selectAllVisible,
      mocks.setCounterpartyFilter,
      mocks.setKindFilter,
      mocks.setTab,
      mocks.toggleSelected,
      mocks.updateFormField,
      mocks.updateItemField,
      mocks.applyPreset,
      mocks.applyFilters,
      mocks.clearAllFilters,
      mocks.setDateRange,
      mocks.setStatusFilter,
      mocks.setAmountRange
    ]) {
      fn.mockReset();
    }

    mocks.selectAllVisible.mockImplementation(() => {
      setDocumentsState(["doc-1", "doc-2"]);
    });
    mocks.closeEditor.mockReturnValue({ ok: true });
    mocks.toggleSelected.mockImplementation((docId: string) => {
      setDocumentsState([docId]);
    });
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders the main shell, item summary and existing PDF flow", () => {
    const { component, target } = renderDocuments();

    expect(target.textContent).toContain("Позиції документа");
    expect(target.textContent).toContain("5 000,00 грн");
    expect(target.textContent).toContain("Існуючий PDF");
    expect(target.textContent).toContain("Створити акт");
    expect(target.textContent).toContain("Створити накладну");

    expect(target.querySelector('[data-testid="documents-bulk-actions"]')?.classList.contains("bulk-actions-idle")).toBe(true);

    component.$destroy();
  });

  it("uses canonical button hierarchy in create strip and editor header", () => {
    const { component, target } = renderDocuments();

    expect((target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).className).toContain("btn-primary");
    expect(buttonByText(target, "Зберегти").className).toContain("btn-primary");
    expect(buttonByText(target, "Дії далі").className).toContain("btn-secondary");
    expect(buttonByText(target, "Видалити").className).toContain("btn-danger");
    expect(buttonByText(target, "Закрити").className).toContain("btn-ghost");
    expect(buttonByText(target, "PDF").className).toContain("btn-ghost");
    expect(buttonByText(target, "Додати позицію").className).toContain("btn-secondary");

    component.$destroy();
  });

  it("marks the main panel inert while the editor drawer is open", () => {
    const { component, target } = renderDocuments();

    const panel = target.querySelector('[data-testid="documents-screen"]');
    expect(panel?.hasAttribute("inert")).toBe(true);
    expect(panel?.getAttribute("aria-hidden")).toBe("true");

    component.$destroy();
  });

  it("shows inline dirty banner before closing a dirty editor", async () => {
    const confirmSpy = vi.spyOn(window, "confirm");
    mocks.closeEditor.mockReturnValue({ ok: false, reason: "dirty" } as any);
    const { component, target } = renderDocuments();

    buttonByText(target, "Закрити").click();
    await tick();

    expect(target.querySelector('[data-testid="documents-dirty-banner"]')).toBeTruthy();
    expect(mocks.closeEditor).toHaveBeenCalledWith(false);
    expect(confirmSpy).not.toHaveBeenCalled();

    confirmSpy.mockRestore();
    component.$destroy();
  });

  it("shows the dirty banner on Escape without falling back to window.confirm", async () => {
    const confirmSpy = vi.spyOn(window, "confirm");
    mocks.closeEditor.mockReturnValue({ ok: false, reason: "dirty" } as any);
    const { component, target } = renderDocuments();

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true }));
    await tick();

    expect(target.querySelector('[data-testid="documents-dirty-banner"]')).toBeTruthy();
    expect(mocks.closeEditor).toHaveBeenCalledWith(false);
    expect(confirmSpy).not.toHaveBeenCalled();

    confirmSpy.mockRestore();
    component.$destroy();
  });

  it("forces close after confirming dirty editor dismissal from the backdrop", async () => {
    mocks.closeEditor.mockReturnValueOnce({ ok: false, reason: "dirty" } as any).mockReturnValueOnce({ ok: true });
    const { component, target } = renderDocuments();

    (target.querySelector('[data-testid="documents-drawer-backdrop"]') as HTMLButtonElement).click();
    await tick();
    buttonByText(target, "Так, закрити").click();
    await tick();

    expect(mocks.closeEditor).toHaveBeenCalledWith(true);

    component.$destroy();
  });

  it("creates a draft without a preliminary counterparty selection", () => {
    setDocumentsStateWithoutDraftContext();
    const { component, target } = renderDocuments();

    const createButton = target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement;
    expect(target.querySelector('[data-testid="documents-create-strip"] select')).toBeNull();

    createButton.click();

    expect(createButton.disabled).toBe(false);
    expect(mocks.create).toHaveBeenCalledWith(undefined, "act");

    component.$destroy();
  });

  it("does not reuse a stale draft counterparty after context is cleared", async () => {
    const { component, target } = renderDocuments();

    setDocumentsStateWithoutDraftContext();
    await tick();

    (target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).click();

    expect(mocks.create).toHaveBeenCalledWith(undefined, "act");

    component.$destroy();
  });

  it("keeps counterparty selection inside the document filters", async () => {
    const { component, target } = renderDocuments();

    (target.querySelector('[data-testid="documents-filter-button"]') as HTMLButtonElement).click();
    await tick();

    const select = target.querySelector('[data-testid="documents-counterparty-filter"]') as HTMLSelectElement;
    select.value = "counterparty-2";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    await tick();

    (target.querySelector('[data-testid="documents-filter-panel"] .btn-primary') as HTMLButtonElement).click();
    await tick();

    expect(mocks.applyFilters).toHaveBeenCalledWith(
      expect.objectContaining({ counterpartyFilterId: "counterparty-2" })
    );

    component.$destroy();
  });

  it("asks for confirmation before destructive document actions", async () => {
    setDocumentsState(["doc-1"]);
    const { component, target } = renderDocuments();

    // Bulk delete: shows in-app banner instead of calling action directly
    buttonByText(target, "Видалити вибрані").click();
    await tick();
    expect(target.querySelector('[data-testid="documents-confirm-bulk-banner"]')).toBeTruthy();
    expect(mocks.bulkDelete).not.toHaveBeenCalled();

    // Cancel hides the banner without acting
    buttonByText(target, "Скасувати").click();
    await tick();
    expect(target.querySelector('[data-testid="documents-confirm-bulk-banner"]')).toBeNull();

    // Single delete: shows in-app banner instead of calling action directly
    (target.querySelector('[data-testid="documents-delete-current-btn"]') as HTMLButtonElement).click();
    await tick();
    expect(target.querySelector('[data-testid="documents-confirm-delete-banner"]')).toBeTruthy();
    expect(mocks.deleteCurrent).not.toHaveBeenCalled();

    component.$destroy();
  });

  it("uses a compact mode that de-emphasizes idle bulk actions", () => {
    expect(source).toContain('data-testid="documents-bulk-actions"');
    expect(styles).toMatch(/@media\s*\(max-width:\s*980px\)[\s\S]*\.bulk-actions-idle\s+button\s*\{[\s\S]*display:\s*none/);
    expect(styles).toMatch(/@media\s*\(max-width:\s*980px\)[\s\S]*\.documents-create-bar\s*\{[\s\S]*grid-template-columns:\s*minmax\(0,\s*1fr\)\s+auto/);
    expect(styles).toMatch(/@media\s*\(max-width:\s*980px\)[\s\S]*\.documents-create-bar\s+\.btn-primary:disabled\s*\{[\s\S]*display:\s*none/);
    expect(source).toContain('class="documents-create-kind-chips"');
  });

  it("routes create and editor actions into the documents store", async () => {
    const { component, target } = renderDocuments();

    (target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).click();
    buttonByText(target, "Додати позицію").click();
    buttonByText(target, "Зберегти").click();
    buttonByText(target, "Відкрити PDF").click();
    (target.querySelector('[data-testid="documents-chain-create-act"]') as HTMLButtonElement).click();
    await tick();

    expect(mocks.create).toHaveBeenCalledWith("counterparty-1", "act");
    expect(mocks.addItem).toHaveBeenCalled();
    expect(mocks.save).toHaveBeenCalled();
    expect(mocks.openCurrentPdf).toHaveBeenCalled();
    expect(mocks.createChainDraft).toHaveBeenCalledWith("act");

    component.$destroy();
  });

  it("does not overwrite manual counterparty selection on unrelated store updates", async () => {
    const { component, target } = renderDocuments();

    (target.querySelector('[data-testid="documents-filter-button"]') as HTMLButtonElement).click();
    await tick();

    const select = target.querySelector('[data-testid="documents-counterparty-filter"]') as HTMLSelectElement;
    select.value = "counterparty-2";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    await tick();

    setDocumentsState();
    await tick();

    expect(select.value).toBe("counterparty-2");

    component.$destroy();
  });

  it("keeps checkbox outside of the row open button", () => {
    const { component, target } = renderDocuments();

    expect(target.querySelector('[data-testid="documents-row-doc-1"] button input[type="checkbox"]')).toBeNull();
    expect(target.querySelector('[data-testid="documents-row-doc-1"] .doc-row-open')).toBeTruthy();
    expect(target.querySelector('[data-testid="documents-row-doc-1"] .doc-row-checkbox input[type="checkbox"]')).toBeTruthy();

    component.$destroy();
  });

  it("uses canonical date input", () => {
    const { component, target } = renderDocuments();

    expect((target.querySelector('input[type="date"]') as HTMLInputElement | null)?.value).toBe("2026-04-30");

    component.$destroy();
  });

  it("calculates fractional item totals with decimal-safe rounding", () => {
    setDocumentsState([], [
      { description: "Рядок 1", unit: "шт", quantity: "3", price: "0,335" },
      { description: "Рядок 2", unit: "шт", quantity: "2", price: "0,335" }
    ]);

    const { component, target } = renderDocuments();

    expect(target.textContent).toContain("1,01 грн");
    expect(target.textContent).toContain("0,67 грн");
    expect(target.textContent).toContain("1,68 грн");

    component.$destroy();
  });

  it("shows skeletons only during initial loading", () => {
    mocks.documentsState.set({
      list: null,
      editor: null,
      chain: null,
      draftContext: null,
      selectedIds: [],
      initialLoading: true,
      loading: false,
      error: null,
      message: null,
      activeTab: "all",
      kindFilter: null,
      counterpartyFilterId: null,
      dateFrom: null,
      dateTo: null,
      statusFilter: [],
      amountMin: null,
      amountMax: null,
      overdueOnly: false,
      activePresetId: null,
    });

    const { component, target } = renderDocuments();

    expect(target.querySelectorAll('[data-testid="skeleton-row-item"]')).toHaveLength(5);
    expect(target.querySelector('[data-testid="documents-list"]')).toBeNull();

    component.$destroy();
  });

  it("renders existing pdf flow with preview and replace action", async () => {
    const { component, target } = renderDocuments();

    expect(target.textContent).toContain("Існуючий PDF");
    expect(target.textContent).toContain("Текстовий шар");
    expect((target.querySelector('[data-testid="documents-existing-pdf"] textarea[readonly]') as HTMLTextAreaElement).value).toContain("DRAFT STATUS");

    const inputs = target.querySelectorAll('[data-testid="documents-existing-pdf"] input');
    (inputs[0] as HTMLInputElement).value = "DRAFT";
    inputs[0].dispatchEvent(new Event("input", { bubbles: true }));
    (inputs[1] as HTMLInputElement).value = "PAID";
    inputs[1].dispatchEvent(new Event("input", { bubbles: true }));
    await tick();

    buttonByText(target, "Застосувати exact replace").click();
    expect(mocks.applyPdfTextReplace).toHaveBeenCalledWith("DRAFT", "PAID");

    component.$destroy();
  });

  it("keeps exact replace disabled for unreadable existing pdf", () => {
    mocks.documentsState.set({
      list: makeList(),
      editor: {
        ...makeEditor(),
        pdf: {
          filePath: "C:/tmp/unreadable.pdf",
          pageCount: 1,
          extractedText: "",
          hasTextOps: true,
          editable: false,
          warnings: ["Лише перегляд."]
        }
      },
      chain: makeChain(),
      draftContext: {
        counterpartyId: "counterparty-1",
        counterpartyName: "ТОВ Ромашка"
      },
      selectedIds: [],
      initialLoading: false,
      loading: false,
      error: null,
      message: null,
      activeTab: "all",
      kindFilter: null,
      counterpartyFilterId: null,
      dateFrom: null,
      dateTo: null,
      statusFilter: [],
      amountMin: null,
      amountMax: null,
      overdueOnly: false,
      activePresetId: null,
    });

    const { component, target } = renderDocuments();
    const replaceButton = buttonByText(target, "Застосувати exact replace");

    expect(target.textContent).toContain("Лише перегляд");
    expect(replaceButton.disabled).toBe(true);

    component.$destroy();
  });

  it("shows a useful empty state when there are no documents yet", () => {
    mocks.documentsState.set({
      list: {
        items: [],
        invoiceItems: [],
        actItems: [],
        waybillItems: [],
        totalCount: 0,
        pageCount: 0
      },
      editor: null,
      chain: null,
      draftContext: null,
      selectedIds: [],
      initialLoading: false,
      loading: false,
      error: null,
      message: null,
      activeTab: "all",
      kindFilter: null,
      counterpartyFilterId: null,
      dateFrom: null,
      dateTo: null,
      statusFilter: [],
      amountMin: null,
      amountMax: null,
      overdueOnly: false,
      activePresetId: null,
    });

    const { component, target } = renderDocuments();

    expect(target.textContent).toContain("Поки що документів немає");
    expect(target.textContent).toContain("Почніть зі створення першого рахунку, акта або накладної");

    component.$destroy();
  });

  it("preset chip calls applyPreset with correct id", () => {
    const { component, target } = renderDocuments();

    (target.querySelector('[data-testid="documents-preset-drafts"]') as HTMLButtonElement).click();

    expect(mocks.applyPreset).toHaveBeenCalledWith("drafts");

    component.$destroy();
  });

  it("filter button shows active filter count badge", () => {
    mocks.documentsState.set({
      ...{
        list: makeList(), editor: makeEditor(), chain: makeChain(),
        draftContext: null, selectedIds: [], initialLoading: false,
        loading: false, error: null, message: null,
        activeTab: "all" as const, kindFilter: null,
        dateFrom: null, dateTo: null,
        amountMin: null, amountMax: null, activePresetId: null,
        counterpartyFilterId: null
      },
      statusFilter: ["draft"],
      overdueOnly: true
    });
    const { component, target } = renderDocuments();

    const filterBtn = target.querySelector('[data-testid="documents-filter-button"]') as HTMLButtonElement;
    expect(filterBtn.textContent?.trim()).toBe("Фільтр · 2");

    component.$destroy();
  });

  it("active-chip × click removes the corresponding filter", async () => {
    mocks.documentsState.set({
      ...{
        list: makeList(), editor: makeEditor(), chain: makeChain(),
        draftContext: null, selectedIds: [], initialLoading: false,
        loading: false, error: null, message: null,
        activeTab: "all" as const, kindFilter: null,
        dateFrom: null, dateTo: null,
        amountMin: null, amountMax: null, overdueOnly: false, activePresetId: null,
        counterpartyFilterId: null
      },
      statusFilter: ["draft"]
    });
    const { component, target } = renderDocuments();

    const activeFilters = target.querySelector('[data-testid="documents-active-filters"]');
    expect(activeFilters).toBeTruthy();

    (target.querySelector('[aria-label="Прибрати фільтр статус"]') as HTMLButtonElement).click();
    await tick();

    expect(mocks.setStatusFilter).toHaveBeenCalledWith([]);

    component.$destroy();
  });

  it("panel Apply button calls applyFilters with panel draft values", async () => {
    const { component, target } = renderDocuments();

    (target.querySelector('[data-testid="documents-filter-button"]') as HTMLButtonElement).click();
    await tick();

    (target.querySelector('[data-testid="documents-filter-panel"] .btn-primary') as HTMLButtonElement).click();
    await tick();

    expect(mocks.applyFilters).toHaveBeenCalledWith({
      dateFrom: null,
      dateTo: null,
      statusFilter: [],
      amountMin: null,
      amountMax: null,
      counterpartyFilterId: null
    });

    component.$destroy();
  });

  it("clear-all button calls clearAllFilters", async () => {
    mocks.documentsState.set({
      ...{
        list: makeList(), editor: makeEditor(), chain: makeChain(),
        draftContext: null, selectedIds: [], initialLoading: false,
        loading: false, error: null, message: null,
        activeTab: "all" as const, kindFilter: null,
        dateFrom: null, dateTo: null,
        amountMin: null, amountMax: null, activePresetId: null,
        counterpartyFilterId: null, statusFilter: []
      },
      overdueOnly: true
    });
    const { component, target } = renderDocuments();

    (target.querySelector('[data-testid="documents-clear-filters"]') as HTMLButtonElement).click();
    await tick();

    expect(mocks.clearAllFilters).toHaveBeenCalled();

    component.$destroy();
  });
});
