import { get, writable } from "svelte/store";
import {
  documentAdvanceStatus,
  documentChainCreateDraft,
  documentChainGet,
  documentCreateDraft,
  documentDelete,
  documentGeneratePdf,
  documentOpen,
  documentPdfApplyTextReplace,
  documentPdfAttachExisting,
  documentPdfOpenCurrent,
  documentSave,
  documentsBulkAdvanceStatus,
  documentsBulkDelete,
  documentsList
} from "../api";
import {
  cloneSnapshot,
  isEditorFormDirty,
  type CloseEditorResult
} from "../editorDirty";
import type { DocumentChainDto, DocumentEditorDto, DocumentKind, DocumentsListDto } from "../types";

type EditorPayload = Pick<DocumentEditorDto, "form" | "items">;

function snapshotEditor(editor: DocumentEditorDto): EditorPayload {
  return cloneSnapshot({ form: editor.form, items: editor.items });
}

interface DocumentsState {
  list: DocumentsListDto | null;
  editor: DocumentEditorDto | null;
  editorSnapshot: EditorPayload | null;
  chain: DocumentChainDto | null;
  draftContext: { counterpartyId: string; counterpartyName: string } | null;
  selectedIds: string[];
  initialLoading: boolean;
  loading: boolean;
  error: string | null;
  message: string | null;
  query: string;
  activeTab: "all" | "outgoing" | "incoming";
  kindFilter: DocumentKind | null;
}

const initialState: DocumentsState = {
  list: null,
  editor: null,
  editorSnapshot: null,
  chain: null,
  draftContext: null,
  selectedIds: [],
  initialLoading: true,
  loading: false,
  error: null,
  message: null,
  query: "",
  activeTab: "all",
  kindFilter: null
};

async function loadEditorAndChain(docId: string): Promise<{
  editor: DocumentEditorDto;
  chain: DocumentChainDto;
}> {
  const [editor, chain] = await Promise.all([documentOpen(docId), documentChainGet(docId)]);
  return { editor, chain };
}

function createDocumentsStore() {
  const { subscribe, update } = writable<DocumentsState>(initialState);

  function tabToDirection(tab: "all" | "outgoing" | "incoming"): "outgoing" | "incoming" | undefined {
    if (tab === "outgoing") return "outgoing";
    if (tab === "incoming") return "incoming";
    return undefined;
  }

  async function reloadList(state: DocumentsState): Promise<DocumentsListDto> {
    return documentsList(
      state.query,
      tabToDirection(state.activeTab),
      state.kindFilter ?? undefined
    );
  }

  return {
    subscribe,
    async load(query = "") {
      update((state) => ({
        ...state,
        loading: true,
        error: null,
        message: state.message,
        query
      }));

      try {
        const snap = get({ subscribe });
        const list = await reloadList({ ...snap, query });
        update((state) => ({
          ...state,
          list,
          selectedIds: state.selectedIds.filter((id) => list.items.some((item) => item.id === id)),
          initialLoading: false,
          loading: false
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async open(docId: string) {
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const { editor, chain } = await loadEditorAndChain(docId);
        update((state) => ({
          ...state,
          editor,
          editorSnapshot: snapshotEditor(editor),
          chain,
          loading: false
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async reloadCurrent() {
      const snapshot = get({ subscribe });
      const documentId = snapshot.editor?.form.id;
      if (!documentId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null }));

      try {
        const [{ editor, chain }, list] = await Promise.all([
          loadEditorAndChain(documentId),
          reloadList(snapshot)
        ]);
        update((state) => ({
          ...state,
          editor,
          editorSnapshot: snapshotEditor(editor),
          chain,
          list,
          loading: false
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async loadChain(docId?: string) {
      const snapshot = get({ subscribe });
      const targetId = docId ?? snapshot.editor?.form.id;
      if (!targetId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null }));

      try {
        const chain = await documentChainGet(targetId);
        update((state) => ({ ...state, chain, loading: false }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    closeEditor(force = false): CloseEditorResult {
      const snapshot = get({ subscribe });
      if (!snapshot.editor) {
        return { ok: true };
      }

      const dirty = isEditorFormDirty(
        snapshot.editorSnapshot,
        { form: snapshot.editor.form, items: snapshot.editor.items }
      );
      if (dirty && !force) {
        return { ok: false, reason: "dirty" };
      }

      update((state) => ({
        ...state,
        editor: null,
        editorSnapshot: null,
        chain: null,
        message: null
      }));
      return { ok: true };
    },
    isEditorDirty(): boolean {
      const snapshot = get({ subscribe });
      if (!snapshot.editor) return false;
      return isEditorFormDirty(
        snapshot.editorSnapshot,
        { form: snapshot.editor.form, items: snapshot.editor.items }
      );
    },
    setDraftContext(counterpartyId: string, counterpartyName: string) {
      update((state) => ({
        ...state,
        draftContext: {
          counterpartyId,
          counterpartyName
        }
      }));
    },
    clearDraftContext() {
      update((state) => ({ ...state, draftContext: null }));
    },
    clearMessage() {
      update((state) => ({ ...state, message: null }));
    },
    toggleSelected(docId: string) {
      update((state) => ({
        ...state,
        selectedIds: state.selectedIds.includes(docId)
          ? state.selectedIds.filter((id) => id !== docId)
          : [...state.selectedIds, docId]
      }));
    },
    selectAllVisible() {
      update((state) => {
        const visibleIds = state.list?.items.map((item) => item.id) ?? [];
        if (visibleIds.length === 0) {
          return state;
        }

        const allVisibleSelected = visibleIds.every((id) => state.selectedIds.includes(id));
        return {
          ...state,
          selectedIds: allVisibleSelected
            ? state.selectedIds.filter((id) => !visibleIds.includes(id))
            : Array.from(new Set([...state.selectedIds, ...visibleIds]))
        };
      });
    },
    clearSelection() {
      update((state) => ({ ...state, selectedIds: [] }));
    },
    setEditor(editor: DocumentEditorDto) {
      update((state) => ({
        ...state,
        editor,
        editorSnapshot: snapshotEditor(editor)
      }));
    },
    async create(counterpartyId: string, kind: string) {
      update((state) => ({ ...state, loading: true, error: null, message: null }));
      const snap = get({ subscribe });
      const direction: "outgoing" | "incoming" =
        snap.activeTab === "incoming" ? "incoming" : "outgoing";

      try {
        const [editor, list] = await Promise.all([
          documentCreateDraft(counterpartyId, kind, direction),
          reloadList(snap)
        ]);
        const chain = await documentChainGet(editor.form.id);
        update((state) => ({
          ...state,
          editor,
          editorSnapshot: snapshotEditor(editor),
          chain,
          list,
          loading: false,
          message: "Чернетку створено",
          draftContext: null
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    updateFormField(field: keyof DocumentEditorDto["form"], value: string) {
      update((state) => ({
        ...state,
        editor: state.editor
          ? {
              ...state.editor,
              form: {
                ...state.editor.form,
                [field]: value
              }
            }
          : null
      }));
    },
    updateItemField(index: number, field: keyof DocumentEditorDto["items"][number], value: string) {
      update((state) => ({
        ...state,
        editor: state.editor
          ? {
              ...state.editor,
              items: state.editor.items.map((item, itemIndex) =>
                itemIndex === index
                  ? {
                      ...item,
                      [field]: value
                    }
                  : item
              )
            }
          : null
      }));
    },
    addItem() {
      update((state) => ({
        ...state,
        editor: state.editor
          ? {
              ...state.editor,
              items: [
                ...state.editor.items,
                {
                  description: "",
                  unit: "",
                  quantity: "1",
                  price: "0"
                }
              ]
            }
          : null
      }));
    },
    removeItem(index: number) {
      update((state) => ({
        ...state,
        editor: state.editor
          ? {
              ...state.editor,
              items: state.editor.items.filter((_, itemIndex) => itemIndex !== index)
            }
          : null
      }));
    },
    async save() {
      const snapshot = get({ subscribe });
      if (!snapshot.editor) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const response = await documentSave(snapshot.editor.form, snapshot.editor.items);
        const [{ editor, chain }, list] = await Promise.all([
          loadEditorAndChain(response.documentId),
          reloadList(snapshot)
        ]);
        update((state) => ({
          ...state,
          editor,
          editorSnapshot: snapshotEditor(editor),
          chain,
          list,
          loading: false,
          message: response.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async advanceStatus() {
      const snapshot = get({ subscribe });
      const documentId = snapshot.editor?.form.id;
      if (!documentId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const response = await documentAdvanceStatus(documentId);
        const [{ editor, chain }, list] = await Promise.all([
          loadEditorAndChain(documentId),
          reloadList(snapshot)
        ]);
        update((state) => ({
          ...state,
          editor,
          editorSnapshot: snapshotEditor(editor),
          chain,
          list,
          loading: false,
          message: response.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async generatePdf() {
      const snapshot = get({ subscribe });
      const docId = snapshot.editor?.form.id;
      if (!docId) return;

      update((state) => ({ ...state, loading: true, error: null, message: null }));
      try {
        const response = await documentGeneratePdf(docId);
        update((state) => ({ ...state, loading: false, message: response.message }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async attachExistingPdf(sourcePath?: string) {
      const snapshot = get({ subscribe });
      const docId = snapshot.editor?.form.id;
      if (!docId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));
      try {
        const response = await documentPdfAttachExisting(docId, sourcePath);
        update((state) => ({
          ...state,
          editor: response.editor,
          editorSnapshot: snapshotEditor(response.editor),
          loading: false,
          message: response.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async applyPdfTextReplace(findText: string, replaceText: string) {
      const snapshot = get({ subscribe });
      const docId = snapshot.editor?.form.id;
      if (!docId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));
      try {
        const response = await documentPdfApplyTextReplace(docId, findText, replaceText);
        update((state) => ({
          ...state,
          editor: response.editor,
          editorSnapshot: snapshotEditor(response.editor),
          loading: false,
          message: response.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async openCurrentPdf() {
      const snapshot = get({ subscribe });
      const docId = snapshot.editor?.form.id;
      if (!docId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));
      try {
        const response = await documentPdfOpenCurrent(docId);
        update((state) => ({
          ...state,
          loading: false,
          message: response.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async deleteCurrent() {
      const snapshot = get({ subscribe });
      const documentId = snapshot.editor?.form.id;
      if (!documentId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const response = await documentDelete(documentId);
        const list = await reloadList(snapshot);
        update((state) => ({
          ...state,
          list,
          editor: null,
          editorSnapshot: null,
          chain: null,
          loading: false,
          message: response.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async createChainDraft(targetKind: string) {
      const snapshot = get({ subscribe });
      const sourceId = snapshot.editor?.form.id;
      if (!sourceId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const editor = await documentChainCreateDraft(sourceId, targetKind);
        const [chain, list] = await Promise.all([
          documentChainGet(editor.form.id),
          reloadList(snapshot)
        ]);
        update((state) => ({
          ...state,
          editor,
          editorSnapshot: snapshotEditor(editor),
          chain,
          list,
          loading: false,
          message: "Пов’язану чернетку створено"
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async bulkDelete() {
      const snapshot = get({ subscribe });
      if (snapshot.selectedIds.length === 0) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const response = await documentsBulkDelete(snapshot.selectedIds);
        const list = await reloadList(snapshot);
        const deletedCurrent = snapshot.editor
          ? snapshot.selectedIds.includes(snapshot.editor.form.id)
          : false;

        update((state) => ({
          ...state,
          list,
          editor: deletedCurrent ? null : state.editor,
          editorSnapshot: deletedCurrent ? null : state.editorSnapshot,
          chain: deletedCurrent ? null : state.chain,
          selectedIds: [],
          loading: false,
          message: response.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async bulkAdvanceStatus() {
      const snapshot = get({ subscribe });
      if (snapshot.selectedIds.length === 0) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const response = await documentsBulkAdvanceStatus(snapshot.selectedIds);
        const list = await reloadList(snapshot);
        const reopenedCurrent = snapshot.editor
          ? snapshot.selectedIds.includes(snapshot.editor.form.id)
          : false;

        let nextEditor = snapshot.editor;
        let nextChain = snapshot.chain;
        let nextEditorSnapshot = snapshot.editorSnapshot;

        if (reopenedCurrent && snapshot.editor) {
          const { editor, chain } = await loadEditorAndChain(snapshot.editor.form.id);
          nextEditor = editor;
          nextChain = chain;
          nextEditorSnapshot = snapshotEditor(editor);
        }

        update((state) => ({
          ...state,
          list,
          editor: nextEditor,
          editorSnapshot: nextEditorSnapshot,
          chain: nextChain,
          selectedIds: [],
          loading: false,
          message: response.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    setTab(tab: "all" | "outgoing" | "incoming") {
      update((state) => ({ ...state, activeTab: tab, loading: true, error: null }));
      const snap = get({ subscribe });
      reloadList(snap).then((list) => {
        update((state) => ({ ...state, list, loading: false }));
      }).catch((error) => {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      });
    },
    setKindFilter(kind: DocumentKind | null) {
      update((state) => ({ ...state, kindFilter: kind, loading: true, error: null }));
      const snap = get({ subscribe });
      reloadList(snap).then((list) => {
        update((state) => ({ ...state, list, loading: false }));
      }).catch((error) => {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      });
    }
  };
}

export const documentsStore = createDocumentsStore();
