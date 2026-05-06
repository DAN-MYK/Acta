/**
 * @vitest-environment jsdom
 */
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
    query: ""
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
    load: vi.fn(),
    open: vi.fn(),
    openCurrentPdf: vi.fn(),
    reloadCurrent: vi.fn(),
    removeItem: vi.fn(),
    save: vi.fn(),
    selectAllVisible: vi.fn(),
    toggleSelected: vi.fn(),
    updateFormField: vi.fn(),
    updateItemField: vi.fn()
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
    load: mocks.load,
    open: mocks.open,
    openCurrentPdf: mocks.openCurrentPdf,
    reloadCurrent: mocks.reloadCurrent,
    removeItem: mocks.removeItem,
    save: mocks.save,
    selectAllVisible: mocks.selectAllVisible,
    toggleSelected: mocks.toggleSelected,
    updateFormField: mocks.updateFormField,
    updateItemField: mocks.updateItemField
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
    query: ""
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
    query: ""
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
      mocks.load,
      mocks.open,
      mocks.openCurrentPdf,
      mocks.reloadCurrent,
      mocks.removeItem,
      mocks.save,
      mocks.selectAllVisible,
      mocks.toggleSelected,
      mocks.updateFormField,
      mocks.updateItemField
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

    expect(target.textContent).toContain("Документи");
    expect(target.textContent).toContain("Позиції документа");
    expect(target.textContent).toContain("5 000,00 грн");
    expect(target.textContent).toContain("Існуючий PDF");
    expect(target.textContent).toContain("Створити акт");
    expect(target.textContent).toContain("Створити накладну");

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

  it("disables scenario creation without counterparty", () => {
    setDocumentsStateWithoutDraftContext();
    const { component, target } = renderDocuments();

    expect((target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).disabled).toBe(true);

    component.$destroy();
  });

  it("asks for confirmation before destructive document actions", async () => {
    setDocumentsState(["doc-1"]);
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);
    const { component, target } = renderDocuments();

    buttonByText(target, "Видалити вибрані").click();
    buttonByText(target, "Видалити").click();
    await tick();

    expect(confirmSpy).toHaveBeenCalledTimes(2);
    expect(mocks.bulkDelete).not.toHaveBeenCalled();
    expect(mocks.deleteCurrent).not.toHaveBeenCalled();

    confirmSpy.mockRestore();
    component.$destroy();
  });

  it("routes create, search and editor actions into the documents store", async () => {
    const { component, target } = renderDocuments();

    const search = target.querySelector('input[placeholder="Пошук документів"]') as HTMLInputElement;
    search.value = "ромашка";
    search.dispatchEvent(new Event("input", { bubbles: true }));
    await tick();

    (target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).click();
    buttonByText(target, "Додати позицію").click();
    buttonByText(target, "Зберегти").click();
    buttonByText(target, "Відкрити PDF").click();
    (target.querySelector('[data-testid="documents-chain-create-act"]') as HTMLButtonElement).click();
    await tick();

    expect(mocks.load).toHaveBeenCalledWith("ромашка");
    expect(mocks.create).toHaveBeenCalledWith("counterparty-1", "act");
    expect(mocks.addItem).toHaveBeenCalled();
    expect(mocks.save).toHaveBeenCalled();
    expect(mocks.openCurrentPdf).toHaveBeenCalled();
    expect(mocks.createChainDraft).toHaveBeenCalledWith("act");

    component.$destroy();
  });

  it("does not overwrite manual counterparty selection on unrelated store updates", async () => {
    const { component, target } = renderDocuments();

    const select = target.querySelector('[data-testid="documents-create-strip"] select') as HTMLSelectElement;
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
      query: ""
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
      query: ""
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
      query: ""
    });

    const { component, target } = renderDocuments();

    expect(target.textContent).toContain("Поки що документів немає");
    expect(target.textContent).toContain("Почніть зі створення першого рахунку, акта або накладної");

    component.$destroy();
  });
});
