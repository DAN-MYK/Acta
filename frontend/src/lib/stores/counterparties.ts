import { get, writable } from "svelte/store";
import {
  counterpartyArchive,
  counterpartyCreateDocumentContext,
  counterpartyGet,
  counterpartyOpenEditor,
  counterpartySave,
  counterpartiesList
} from "../api";
import { documentsStore } from "./documents";
import { navigationStore } from "./navigation";
import {
  cloneSnapshot,
  isEditorFormDirty,
  type CloseEditorResult
} from "../editorDirty";
import type {
  CounterpartyDetailScreenDto,
  CounterpartyDraftFormDto,
  CounterpartyEditorDto,
  CounterpartiesScreenDto
} from "../types";

interface CounterpartiesState {
  screen: CounterpartiesScreenDto | null;
  detail: CounterpartyDetailScreenDto | null;
  editor: CounterpartyEditorDto | null;
  editorSnapshot: CounterpartyDraftFormDto | null;
  selectedId: string | null;
  initialLoading: boolean;
  loading: boolean;
  error: string | null;
  message: string | null;
  query: string;
}

const initialState: CounterpartiesState = {
  screen: null,
  detail: null,
  editor: null,
  editorSnapshot: null,
  selectedId: null,
  initialLoading: true,
  loading: false,
  error: null,
  message: null,
  query: ""
};

function createCounterpartiesStore() {
  const { subscribe, update } = writable<CounterpartiesState>(initialState);

  return {
    subscribe,
    async load(query = "") {
      update((state) => ({ ...state, loading: true, error: null, query }));

      try {
        const screen = await counterpartiesList(query);
        const selectedId =
          get({ subscribe }).selectedId &&
          screen.items.some((item) => item.id === get({ subscribe }).selectedId)
            ? get({ subscribe }).selectedId
            : screen.items[0]?.id ?? null;
        const detail = selectedId ? await counterpartyGet(selectedId) : null;
        update((state) => ({
          ...state,
          screen,
          detail,
          selectedId,
          initialLoading: false,
          loading: false
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async open(counterpartyId: string) {
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const detail = await counterpartyGet(counterpartyId);
        update((state) => ({
          ...state,
          detail,
          selectedId: counterpartyId,
          loading: false
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async openEditor(counterpartyId?: string) {
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const editor = await counterpartyOpenEditor(counterpartyId);
        update((state) => ({
          ...state,
          editor,
          editorSnapshot: cloneSnapshot(editor.form),
          loading: false
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    closeEditor(force = false): CloseEditorResult {
      const snapshot = get({ subscribe });
      if (!snapshot.editor) {
        return { ok: true };
      }

      const dirty = isEditorFormDirty(snapshot.editorSnapshot, snapshot.editor.form);
      if (dirty && !force) {
        return { ok: false, reason: "dirty" };
      }

      update((state) => ({ ...state, editor: null, editorSnapshot: null }));
      return { ok: true };
    },
    updateFormField(field: keyof CounterpartyDraftFormDto, value: string) {
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
    async save() {
      const snapshot = get({ subscribe });
      if (!snapshot.editor) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await counterpartySave(snapshot.editor.form);
        update((state) => ({
          ...state,
          screen: {
            items: result.updatedList
          },
          detail: result.updatedDetail,
          selectedId: result.savedId,
          editor: null,
          editorSnapshot: null,
          loading: false,
          message: result.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async archiveCurrent() {
      const snapshot = get({ subscribe });
      if (!snapshot.selectedId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await counterpartyArchive(snapshot.selectedId);
        const screen = await counterpartiesList(snapshot.query);
        const selectedId = screen.items[0]?.id ?? null;
        const detail = selectedId ? await counterpartyGet(selectedId) : null;
        update((state) => ({
          ...state,
          screen,
          detail,
          selectedId,
          loading: false,
          message: result.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async createDocument() {
      const snapshot = get({ subscribe });
      if (!snapshot.selectedId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const context = await counterpartyCreateDocumentContext(snapshot.selectedId);
        documentsStore.setDraftContext(context.counterpartyId, context.counterpartyName);
        navigationStore.go("documents");
        update((state) => ({
          ...state,
          loading: false,
          message: "Контрагента передано у create flow документів"
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    setEditor(editor: CounterpartyEditorDto) {
      update((state) => ({
        ...state,
        editor,
        editorSnapshot: cloneSnapshot(editor.form)
      }));
    }
  };
}

export const counterpartiesStore = createCounterpartiesStore();
