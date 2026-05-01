import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DocumentChainDto, DocumentEditorDto, DocumentsListDto } from "../../types";
import { invokeMock, loadStores, snapshot } from "./helpers";

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

describe("documents store smoke", () => {
  beforeEach(() => {
    vi.resetModules();
    invokeMock.mockReset();
  });

  it("covers documents list, selection, bulk delete, open, create, save, chain, status and delete flow", async () => {
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
    expect(snapshot(documentsStore).initialLoading).toBe(false);
    expect(snapshot(documentsStore).loading).toBe(false);

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
