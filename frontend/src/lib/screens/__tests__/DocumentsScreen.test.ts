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
    bulkAdvanceStatus: vi.fn(),
    bulkDelete: vi.fn(),
    closeEditor: vi.fn(),
    create: vi.fn(),
    createChainDraft: vi.fn(),
    deleteCurrent: vi.fn(),
    load: vi.fn(),
    open: vi.fn(),
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
    bulkAdvanceStatus: mocks.bulkAdvanceStatus,
    bulkDelete: mocks.bulkDelete,
    closeEditor: mocks.closeEditor,
    create: mocks.create,
    createChainDraft: mocks.createChainDraft,
    deleteCurrent: mocks.deleteCurrent,
    load: mocks.load,
    open: mocks.open,
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

function makeEditor(): DocumentEditorDto {
  return {
    form: {
      id: "doc-1",
      kind: "invoice",
      counterpartyId: "counterparty-1",
      counterpartyName: "ТОВ Ромашка",
      title: "Рахунок INV-7",
      number: "INV-7",
      date: "2026-04-30",
      notes: "Початковий коментар"
    },
    items: [
      {
        description: "Консультація",
        unit: "год",
        quantity: "2",
        price: "2500"
      }
    ],
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

function setDocumentsState(selectedIds: string[] = []) {
  mocks.documentsState.set({
    list: makeList(),
    editor: makeEditor(),
    chain: makeChain(),
    draftContext: {
      counterpartyId: "counterparty-1",
      counterpartyName: "ТОВ Ромашка"
    },
    selectedIds,
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
      mocks.bulkAdvanceStatus,
      mocks.bulkDelete,
      mocks.closeEditor,
      mocks.create,
      mocks.createChainDraft,
      mocks.deleteCurrent,
      mocks.load,
      mocks.open,
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

    mocks.toggleSelected.mockImplementation((docId: string) => {
      setDocumentsState([docId]);
    });
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders document rows, editor and scenario guidance", () => {
    const { component, target } = renderDocuments();

    expect(target.textContent).toContain("Документи");
    expect(target.textContent).toContain("INV-7");
    expect(target.textContent).toContain("ТОВ Ромашка");
    expect(target.textContent).toContain("Рахунок INV-7");
    expect(target.textContent).toContain("Новий документ");
    expect(target.textContent).toContain("Що далі");
    expect(target.textContent).toContain("Позиції документа");
    expect(target.textContent).toContain("Створити Акт");
    expect(target.textContent).toContain("Створити Накладна");

    component.$destroy();
  });

  it("uses canonical button hierarchy in create strip and editor header", () => {
    const { component, target } = renderDocuments();

    expect(buttonByText(target, "Створити чернетку").className).toContain("btn-primary");
    expect(buttonByText(target, "Додати позицію").className).toContain("btn-ghost");
    expect(buttonByText(target, "Зберегти").className).toContain("btn-primary");
    expect(buttonByText(target, "Наступний статус").className).toContain("btn-secondary");
    expect(buttonByText(target, "Видалити").className).toContain("btn-danger");
    expect(buttonByText(target, "Видалити позицію").className).toContain("btn-danger");
    expect(buttonByText(target, "Закрити").className).toContain("btn-ghost");

    component.$destroy();
  });

  it("shows create-strip as a guided flow and disables draft creation without counterparty", () => {
    setDocumentsStateWithoutDraftContext();
    const { component, target } = renderDocuments();

    const createButton = buttonByText(target, "Створити чернетку");

    expect(target.textContent).toContain("Новий документ");
    expect(target.textContent).toContain("1. Оберіть контрагента");
    expect(target.textContent).toContain("2. Вкажіть тип документа");
    expect(target.textContent).toContain("3. Створіть чернетку");
    expect(target.textContent).toContain("Спочатку оберіть контрагента");
    expect(createButton.disabled).toBe(true);

    component.$destroy();
  });

  it("routes create, search and editor actions into the documents store", async () => {
    const { component, target } = renderDocuments();

    const search = target.querySelector('input[placeholder="Пошук документів"]') as HTMLInputElement;
    search.value = "ромашка";
    search.dispatchEvent(new Event("input", { bubbles: true }));
    await tick();
    expect(mocks.load).toHaveBeenCalledWith("ромашка");

    buttonByText(target, "Створити чернетку").click();
    await tick();
    expect(mocks.create).toHaveBeenCalledWith("counterparty-1", "act");

    buttonByText(target, "Додати позицію").click();
    buttonByText(target, "Зберегти").click();
    buttonByText(target, "Наступний статус").click();
    buttonByText(target, "Створити Акт").click();
    await tick();

    expect(mocks.addItem).toHaveBeenCalled();
    expect(mocks.save).toHaveBeenCalled();
    expect(mocks.advanceStatus).toHaveBeenCalled();
    expect(mocks.createChainDraft).toHaveBeenCalledWith("act");

    component.$destroy();
  });

  it("uses canonical date input and primary action hierarchy in the editor", () => {
    const { component, target } = renderDocuments();

    const dateInput = Array.from(target.querySelectorAll("input")).find((input) =>
      (input as HTMLInputElement).value === "2026-04-30"
    ) as HTMLInputElement | undefined;
    const saveButton = buttonByText(target, "Зберегти");

    expect(dateInput).toBeTruthy();
    expect(dateInput?.type).toBe("date");
    expect(saveButton.className).toContain("btn-primary");

    component.$destroy();
  });

  it("shows chain panel as next-step guidance instead of only technical links", () => {
    const { component, target } = renderDocuments();

    expect(target.textContent).toContain("Що далі");
    expect(target.textContent).toContain("Поточний документ");
    expect(target.textContent).toContain("Наступний крок");
    expect(target.textContent).toContain("На основі рахунку можна одразу підготувати акт або накладну.");

    component.$destroy();
  });

  it("shows item editor as a guided section with line summaries", () => {
    const { component, target } = renderDocuments();

    expect(target.textContent).toContain("Позиції документа");
    expect(target.textContent).toContain("1 позиція");
    expect(target.textContent).toContain("Додайте товари або послуги");
    expect(target.textContent).toContain("Рядок 1");
    expect(target.textContent).toContain("Сума позиції");
    expect(target.textContent).toContain("5 000,00 грн");

    component.$destroy();
  });

  it("routes selection controls and bulk actions into the documents store", async () => {
    const { component, target } = renderDocuments();

    const selectionBoxes = target.querySelectorAll('input[type="checkbox"]');
    expect(selectionBoxes.length).toBeGreaterThanOrEqual(3);

    (selectionBoxes[0] as HTMLInputElement).click();
    await tick();
    expect(mocks.selectAllVisible).toHaveBeenCalled();

    (selectionBoxes[1] as HTMLInputElement).click();
    await tick();
    expect(mocks.toggleSelected).toHaveBeenCalledWith("doc-1");

    buttonByText(target, "Оновити статус вибраних").click();
    await tick();
    expect(mocks.bulkAdvanceStatus).toHaveBeenCalled();

    buttonByText(target, "Видалити вибрані").click();
    await tick();
    expect(mocks.bulkDelete).toHaveBeenCalled();

    component.$destroy();
  });
});
