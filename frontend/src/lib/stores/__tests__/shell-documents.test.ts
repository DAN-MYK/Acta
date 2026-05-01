import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  DocumentChainDto,
  DocumentEditorDto,
  DocumentsListDto,
  PaletteActivationResultDto,
  PaletteSearchResultDto,
  ShellStateDto
} from "../../types";
import { invokeMock, loadStores, snapshot } from "./helpers";

async function flushAsyncWork() {
  await Promise.resolve();
  await Promise.resolve();
}

function createDeferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return { promise, resolve, reject };
}

function makeShellState(activeCompanyId = "company-1"): ShellStateDto {
  return {
    chrome: {
      companyName: "ТОВ Акт",
      userName: "Олена",
      userInitials: "ОО",
      userRole: "Адміністратор",
      documentsBadge: 4,
      tasksBadge: 2
    },
    companyItems: [
      {
        id: "company-1",
        name: "ТОВ Акт",
        subtitle: "Основна компанія",
        initials: "ТА",
        badge: "",
        active: activeCompanyId === "company-1"
      },
      {
        id: "company-2",
        name: "ФОП Тест",
        subtitle: "Друга компанія",
        initials: "ФТ",
        badge: "",
        active: activeCompanyId === "company-2"
      }
    ],
    activeCompanyId,
    isDark: activeCompanyId === "company-2"
  };
}

function makeDocumentsList(ids: string[]): DocumentsListDto {
  const items = ids.map((id, index) => ({
    id,
    kind: "invoice" as const,
    number: `INV-${index + 1}`,
    date: "2026-04-30",
    counterparty: "ТОВ Ромашка",
    amountStr: "1 234,50 грн",
    status: "draft" as const,
    statusLabel: "Чернетка",
    linkedId: ""
  }));

  return {
    items,
    invoiceItems: items,
    actItems: [],
    waybillItems: [],
    totalCount: items.length,
    pageCount: 1
  };
}

function makeEditor(id: string, kind = "invoice", notes = ""): DocumentEditorDto {
  return {
    form: {
      id,
      kind,
      counterpartyId: "cp-1",
      counterpartyName: "ТОВ Ромашка",
      title: `${kind} editor`,
      number: `NUM-${id}`,
      date: "2026-04-30",
      notes
    },
    items: [
      {
        description: "Послуга",
        unit: "шт",
        quantity: "1",
        price: "1234.50"
      }
    ],
    showTypePicker: false,
    showEditor: true
  };
}

function makeChain(sourceId: string, stepType = "invoice"): DocumentChainDto {
  return {
    sourceId,
    steps: [
      {
        docType: stepType,
        docNumber: `NUM-${sourceId}`,
        amountStr: "1 234,50 грн",
        status: "draft",
        exists: true
      }
    ]
  };
}

describe("frontend Tauri store smoke: shell + documents", () => {
  beforeEach(() => {
    vi.resetModules();
    invokeMock.mockReset();
  });

  it("orchestrates shell load, company switch and palette activation without Slint callbacks", async () => {
    const stores = await loadStores();
    const { shellStore, paletteStore, navigationStore, documentsStore, counterpartiesStore } = stores;

    let selectedCounterpartyId = "cp-9";
    counterpartiesStore.subscribe((state) => {
      if (state.selectedId) {
        selectedCounterpartyId = state.selectedId;
      }
    })();

    const paletteSearch: PaletteSearchResultDto = {
      items: [
        {
          kind: "navigate",
          title: "Документи",
          subtitle: "Перейти до екрану",
          shortcut: "Ctrl+2",
          payload: "nav:documents"
        }
      ]
    };

    const paletteActivation: PaletteActivationResultDto = {
      kind: "open_document",
      screen: "documents",
      documentId: "doc-open-1",
      counterpartyId: null,
      documentEditor: null,
      message: null
    };

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "shell_load":
          return makeShellState("company-1");
        case "shell_set_active_company":
          expect(payload).toEqual({ companyId: "company-2" });
          return makeShellState("company-2");
        case "shell_palette_search":
          expect(payload).toEqual({
            request: {
              query: "док",
              selectedCounterpartyId: undefined
            }
          });
          return paletteSearch;
        case "shell_palette_activate":
          expect(payload).toEqual({
            payload: "nav:documents",
            selectedCounterpartyId: undefined
          });
          return paletteActivation;
        case "document_open":
          expect(payload).toEqual({ docId: "doc-open-1" });
          return makeEditor("doc-open-1");
        case "document_chain_get":
          expect(payload).toEqual({ docId: "doc-open-1" });
          return makeChain("doc-open-1");
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await shellStore.load();
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-1");

    await shellStore.setActiveCompany("company-2");
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-2");
    expect(snapshot(shellStore).state?.isDark).toBe(true);

    await paletteStore.search("док");
    expect(snapshot(paletteStore).items).toEqual(paletteSearch.items);

    await paletteStore.activate("nav:documents");
    await flushAsyncWork();
    expect(snapshot(navigationStore)).toBe("documents");
    expect(snapshot(documentsStore).editor?.form.id).toBe("doc-open-1");
    expect(selectedCounterpartyId).toBe("cp-9");
  });

  it("resets palette query and items after close so next open starts predictably", async () => {
    const { paletteStore } = await loadStores();

    const paletteSearch: PaletteSearchResultDto = {
      items: [
        {
          kind: "navigate",
          title: "Документи",
          subtitle: "Перейти до екрану",
          shortcut: "Ctrl+2",
          payload: "nav:documents"
        }
      ]
    };

    invokeMock.mockImplementation(async (command) => {
      if (command === "shell_palette_search") {
        return paletteSearch;
      }

      throw new Error(`unexpected command: ${command}`);
    });

    paletteStore.toggle();
    await paletteStore.search("док");

    expect(snapshot(paletteStore)).toMatchObject({
      open: true,
      query: "док",
      items: paletteSearch.items
    });

    paletteStore.close();

    expect(snapshot(paletteStore)).toMatchObject({
      open: false,
      query: "",
      items: [],
      loading: false,
      error: null
    });
  });

  it("enters visible loading state while switching active company", async () => {
    const { shellStore } = await loadStores();
    const deferred = createDeferred<ShellStateDto>();

    invokeMock.mockImplementation(async (command) => {
      switch (command) {
        case "shell_load":
          return makeShellState("company-1");
        case "shell_set_active_company":
          return deferred.promise;
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await shellStore.load();

    const switchPromise = shellStore.setActiveCompany("company-2");
    expect(snapshot(shellStore).loading).toBe(true);
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-1");

    deferred.resolve(makeShellState("company-2"));
    await switchPromise;

    expect(snapshot(shellStore).loading).toBe(false);
    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-2");
  });

  it("covers documents list, selection, bulk status, bulk delete, open, create, save, chain, status and delete flow", async () => {
    const { documentsStore } = await loadStores();

    let docs = makeDocumentsList(["doc-1", "doc-2"]);
    const editors = new Map<string, DocumentEditorDto>([["doc-1", makeEditor("doc-1")]]);
    const chains = new Map<string, DocumentChainDto>([["doc-1", makeChain("doc-1")]]);

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "documents_list":
          return docs;
        case "documents_bulk_delete":
          expect(payload).toEqual({
            request: {
              docIds: ["doc-2"]
            }
          });
          docs = makeDocumentsList(["doc-1"]);
          return {
            total: 1,
            succeeded: 1,
            failed: 0,
            errors: [],
            message: "Видалено 1 документ"
          };
        case "document_open":
          return editors.get((payload as { docId: string }).docId) ?? makeEditor("missing");
        case "document_chain_get":
          return chains.get((payload as { docId: string }).docId) ?? makeChain("missing");
        case "document_create_draft": {
          const editor = makeEditor("doc-2");
          docs = makeDocumentsList(["doc-1", "doc-2"]);
          editors.set("doc-2", editor);
          chains.set("doc-2", makeChain("doc-2"));
          return editor;
        }
        case "document_save": {
          const request = payload as { request: { form: DocumentEditorDto["form"] } };
          const id = request.request.form.id;
          const saved = makeEditor(id, request.request.form.kind, request.request.form.notes);
          editors.set(id, saved);
          return {
            documentId: id,
            kind: request.request.form.kind,
            message: "Документ збережено"
          };
        }
        case "document_chain_create_draft": {
          const request = payload as { request: { sourceId: string; targetKind: string } };
          const editor = makeEditor("doc-3", request.request.targetKind);
          editors.set("doc-3", editor);
          chains.set("doc-3", makeChain("doc-3", request.request.targetKind));
          docs = makeDocumentsList(["doc-1", "doc-2", "doc-3"]);
          return editor;
        }
        case "document_advance_status":
          return {
            ok: true,
            message: "Статус оновлено"
          };
        case "document_delete": {
          const id = (payload as { docId: string }).docId;
          docs = makeDocumentsList(["doc-1"]);
          editors.delete(id);
          chains.delete(id);
          return {
            ok: true,
            message: "Документ видалено"
          };
        }
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await documentsStore.load();
    expect(snapshot(documentsStore).list?.totalCount).toBe(2);

    documentsStore.toggleSelected("doc-1");
    expect(snapshot(documentsStore).selectedIds).toEqual(["doc-1"]);

    documentsStore.toggleSelected("doc-2");
    expect(snapshot(documentsStore).selectedIds).toEqual(["doc-1", "doc-2"]);

    documentsStore.toggleSelected("doc-1");
    expect(snapshot(documentsStore).selectedIds).toEqual(["doc-2"]);

    await documentsStore.bulkDelete();
    expect(snapshot(documentsStore).list?.totalCount).toBe(1);
    expect(snapshot(documentsStore).selectedIds).toEqual([]);
    expect(snapshot(documentsStore).message).toBe("Видалено 1 документ");

    await documentsStore.open("doc-1");
    expect(snapshot(documentsStore).editor?.form.id).toBe("doc-1");
    expect(snapshot(documentsStore).chain?.sourceId).toBe("doc-1");

    await documentsStore.create("cp-1", "invoice");
    expect(snapshot(documentsStore).editor?.form.id).toBe("doc-2");
    expect(snapshot(documentsStore).message).toBe("Чернетку створено");

    documentsStore.updateFormField("notes", "Smoke notes");
    await documentsStore.save();
    expect(snapshot(documentsStore).editor?.form.notes).toBe("Smoke notes");
    expect(snapshot(documentsStore).message).toBe("Документ збережено");

    await documentsStore.createChainDraft("act");
    expect(snapshot(documentsStore).editor?.form.kind).toBe("act");
    expect(snapshot(documentsStore).list?.totalCount).toBe(3);

    await documentsStore.advanceStatus();
    expect(snapshot(documentsStore).message).toBe("Статус оновлено");

    await documentsStore.deleteCurrent();
    expect(snapshot(documentsStore).editor).toBeNull();
    expect(snapshot(documentsStore).list?.totalCount).toBe(1);
    expect(snapshot(documentsStore).message).toBe("Документ видалено");
  });
});
