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
    loading: false,
    error: null as string | null,
    message: null as string | null,
    query: ""
  });

  return {
    documentsState,
    addItem: vi.fn(),
    advanceStatus: vi.fn(),
    closeEditor: vi.fn(),
    create: vi.fn(),
    createChainDraft: vi.fn(),
    deleteCurrent: vi.fn(),
    load: vi.fn(),
    open: vi.fn(),
    reloadCurrent: vi.fn(),
    removeItem: vi.fn(),
    save: vi.fn(),
    updateFormField: vi.fn(),
    updateItemField: vi.fn()
  };
});

vi.mock("../../stores/documents", () => ({
  documentsStore: {
    subscribe: mocks.documentsState.subscribe,
    addItem: mocks.addItem,
    advanceStatus: mocks.advanceStatus,
    closeEditor: mocks.closeEditor,
    create: mocks.create,
    createChainDraft: mocks.createChainDraft,
    deleteCurrent: mocks.deleteCurrent,
    load: mocks.load,
    open: mocks.open,
    reloadCurrent: mocks.reloadCurrent,
    removeItem: mocks.removeItem,
    save: mocks.save,
    updateFormField: mocks.updateFormField,
    updateItemField: mocks.updateItemField
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
      }
    ],
    invoiceItems: [],
    actItems: [],
    waybillItems: [],
    totalCount: 1,
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
    mocks.documentsState.set({
      list: makeList(),
      editor: makeEditor(),
      chain: {
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
      },
      draftContext: {
        counterpartyId: "counterparty-1",
        counterpartyName: "ТОВ Ромашка"
      },
      loading: false,
      error: null,
      message: "Готово",
      query: ""
    });

    for (const fn of [
      mocks.addItem,
      mocks.advanceStatus,
      mocks.closeEditor,
      mocks.create,
      mocks.createChainDraft,
      mocks.deleteCurrent,
      mocks.load,
      mocks.open,
      mocks.reloadCurrent,
      mocks.removeItem,
      mocks.save,
      mocks.updateFormField,
      mocks.updateItemField
    ]) {
      fn.mockReset();
    }
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders document rows, editor and chain controls", () => {
    const { component, target } = renderDocuments();

    expect(target.textContent).toContain("Документи");
    expect(target.textContent).toContain("INV-7");
    expect(target.textContent).toContain("ТОВ Ромашка");
    expect(target.textContent).toContain("Рахунок INV-7");
    expect(target.textContent).toContain("Ланцюжок документа");
    expect(target.textContent).toContain("+ Акт");
    expect(target.textContent).toContain("+ Накладна");

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
    buttonByText(target, "+ Акт").click();
    await tick();

    expect(mocks.addItem).toHaveBeenCalled();
    expect(mocks.save).toHaveBeenCalled();
    expect(mocks.advanceStatus).toHaveBeenCalled();
    expect(mocks.createChainDraft).toHaveBeenCalledWith("act");

    component.$destroy();
  });
});
