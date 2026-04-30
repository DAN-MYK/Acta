import { get, writable } from "svelte/store";
import { taskDelete, taskOpenEditor, taskSave, taskSetStatus, tasksList } from "../api";
import type { TaskDraftFormDto, TaskEditorDto, TasksScreenDto } from "../types";

export type TaskTab = "open" | "done" | "all";

interface TasksState {
  screen: TasksScreenDto | null;
  editor: TaskEditorDto | null;
  loading: boolean;
  error: string | null;
  message: string | null;
  query: string;
  tab: TaskTab;
}

const initialState: TasksState = {
  screen: null,
  editor: null,
  loading: false,
  error: null,
  message: null,
  query: "",
  tab: "open"
};

function createTasksStore() {
  const { subscribe, update } = writable<TasksState>(initialState);

  return {
    subscribe,
    async load(query = get({ subscribe }).query) {
      update((state) => ({ ...state, loading: true, error: null, query }));

      try {
        const screen = await tasksList(query);
        update((state) => ({ ...state, screen, loading: false }));
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
        update((state) => ({ ...state, editor, loading: false }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },
    closeEditor() {
      update((state) => ({ ...state, editor: null }));
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
        const screen = await tasksList(snapshot.query);
        update((state) => ({
          ...state,
          screen,
          editor: null,
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
        const screen = await tasksList(snapshot.query);
        const editor =
          snapshot.editor?.form.id === taskId ? await taskOpenEditor(taskId) : snapshot.editor;
        update((state) => ({
          ...state,
          screen,
          editor,
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
