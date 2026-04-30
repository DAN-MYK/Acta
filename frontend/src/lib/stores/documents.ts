import { get, writable } from "svelte/store";
import {
  documentAdvanceStatus,
  documentChainCreateDraft,
  documentChainGet,
  documentCreateDraft,
  documentDelete,
  documentOpen,
  documentSave,
  documentsList
} from "../api";
import type { DocumentChainDto, DocumentEditorDto, DocumentsListDto } from "../types";

interface DocumentsState {
  list: DocumentsListDto | null;
  editor: DocumentEditorDto | null;
  chain: DocumentChainDto | null;
  draftContext: { counterpartyId: string; counterpartyName: string } | null;
  loading: boolean;
  error: string | null;
  message: string | null;
  query: string;
}

const initialState: DocumentsState = {
  list: null,
  editor: null,
  chain: null,
  draftContext: null,
  loading: false,
  error: null,
  message: null,
  query: ""
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
        const list = await documentsList(query);
        update((state) => ({ ...state, list, loading: false }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async open(docId: string) {
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const { editor, chain } = await loadEditorAndChain(docId);
        update((state) => ({ ...state, editor, chain, loading: false }));
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
          documentsList(snapshot.query)
        ]);
        update((state) => ({ ...state, editor, chain, list, loading: false }));
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
    closeEditor() {
      update((state) => ({ ...state, editor: null, chain: null, message: null }));
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
    setEditor(editor: DocumentEditorDto) {
      update((state) => ({ ...state, editor }));
    },
    async create(counterpartyId: string, kind: string) {
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const [editor, list] = await Promise.all([
          documentCreateDraft(counterpartyId, kind),
          documentsList(get({ subscribe }).query)
        ]);
        const chain = await documentChainGet(editor.form.id);
        update((state) => ({
          ...state,
          editor,
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
          documentsList(snapshot.query)
        ]);
        update((state) => ({
          ...state,
          editor,
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
          documentsList(snapshot.query)
        ]);
        update((state) => ({
          ...state,
          editor,
          chain,
          list,
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
        const list = await documentsList(snapshot.query);
        update((state) => ({
          ...state,
          list,
          editor: null,
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
          documentsList(snapshot.query)
        ]);
        update((state) => ({
          ...state,
          editor,
          chain,
          list,
          loading: false,
          message: "Пов’язану чернетку створено"
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    }
  };
}

export const documentsStore = createDocumentsStore();
