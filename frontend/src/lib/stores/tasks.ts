import { get, writable } from "svelte/store";
import { taskDelete, taskOpenEditor, taskSave, taskSetStatus, tasksList } from "../api";
import {
  cloneSnapshot,
  isEditorFormDirty,
  type CloseEditorResult
} from "../editorDirty";
import type { TaskDraftFormDto, TaskEditorDto, TasksScreenDto } from "../types";

export type TaskTab = "open" | "done" | "all";

interface TasksState {
  screen: TasksScreenDto | null;
  editor: TaskEditorDto | null;
  editorSnapshot: TaskDraftFormDto | null;
  initialLoading: boolean;
  loading: boolean;
  error: string | null;
  message: string | null;
  tab: TaskTab;
}

const initialState: TasksState = {
  screen: null,
  editor: null,
  editorSnapshot: null,
  initialLoading: true,
  loading: false,
  error: null,
  message: null,
  tab: "open"
};

function createTasksStore() {
  const { subscribe, update } = writable<TasksState>(initialState);

  return {
    subscribe,
    async load() {
      update((state) => ({ ...state, loading: true, error: null }));

      try {
        const screen = await tasksList();
        update((state) => ({ ...state, screen, initialLoading: false, loading: false }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    setTab(tab: TaskTab) {
      update((state) => ({ ...state, tab }));
    },
    async openEditor(taskId?: string) {
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const editor = await taskOpenEditor(taskId);
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
    updateFormField(field: keyof TaskDraftFormDto, value: string) {
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
        const result = await taskSave(snapshot.editor.form);
        update((state) => ({
          ...state,
          screen: result.updatedList,
          editor: result.updatedEditor,
          editorSnapshot: result.updatedEditor ? cloneSnapshot(result.updatedEditor.form) : null,
          loading: false,
          message: result.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async deleteCurrent() {
      const snapshot = get({ subscribe });
      const taskId = snapshot.editor?.form.id;
      if (!taskId) {
        return;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await taskDelete(taskId);
        const screen = await tasksList();
        update((state) => ({
          ...state,
          screen,
          editor: null,
          editorSnapshot: null,
          loading: false,
          message: result.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    async setStatus(taskId: string, status: string) {
      const snapshot = get({ subscribe });
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await taskSetStatus(taskId, status);
        const screen = await tasksList();
        const editor =
          snapshot.editor?.form.id === taskId ? await taskOpenEditor(taskId) : snapshot.editor;
        const editorSnapshot =
          snapshot.editor?.form.id === taskId && editor
            ? cloneSnapshot(editor.form)
            : snapshot.editorSnapshot;
        update((state) => ({
          ...state,
          screen,
          editor,
          editorSnapshot,
          loading: false,
          message: result.message
        }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    }
  };
}

export const tasksStore = createTasksStore();
