/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import TasksScreen from "../TasksScreen.svelte";
import type { TaskEditorDto, TaskItemDto, TasksScreenDto } from "../../types";

type TasksTab = "open" | "done" | "all";

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

  const tasksState = createMockStore({
    screen: null as TasksScreenDto | null,
    editor: null as TaskEditorDto | null,
    initialLoading: false,
    loading: false,
    error: null as string | null,
    message: null as string | null,
    query: "",
    tab: "open" as TasksTab
  });

  return {
    closeEditor: vi.fn(() => ({ ok: true })),
    deleteCurrent: vi.fn(),
    load: vi.fn(),
    openEditor: vi.fn(),
    save: vi.fn(),
    setStatus: vi.fn(),
    setTab: vi.fn(),
    tasksState,
    updateFormField: vi.fn()
  };
});

vi.mock("../../stores/tasks", () => ({
  tasksStore: {
    subscribe: mocks.tasksState.subscribe,
    closeEditor: mocks.closeEditor,
    deleteCurrent: mocks.deleteCurrent,
    load: mocks.load,
    openEditor: mocks.openEditor,
    save: mocks.save,
    setStatus: mocks.setStatus,
    setTab: mocks.setTab,
    updateFormField: mocks.updateFormField
  }
}));

function makeTask(id: string, overrides: Partial<TaskItemDto> = {}): TaskItemDto {
  return {
    id,
    title: `Завдання ${id}`,
    description: "Оновити статус документа",
    status: "open",
    statusLabel: "Відкрите",
    priority: "high",
    priorityLabel: "Високий",
    dueDate: "2026-05-01",
    reminderAt: "2026-05-01T10:00",
    linkKind: "document",
    linkLabel: "INV-2026-0042",
    ...overrides
  };
}

function makeScreen(items: TaskItemDto[]): TasksScreenDto {
  return {
    items,
    openCount: items.filter((item) => item.status === "open" || item.status === "in_progress").length,
    doneCount: items.filter((item) => item.status === "done" || item.status === "cancelled").length,
    highCount: items.filter((item) => item.priority === "high" || item.priority === "critical").length,
    todayCount: items.filter((item) => item.dueDate === "2026-05-01").length
  };
}

function makeEditor(): TaskEditorDto {
  return {
    title: "Редагування завдання",
    form: {
      id: "task-1",
      title: "Погодити оплату",
      description: "Контроль платежу",
      priority: "high",
      dueDate: "2026-05-01",
      reminderAt: "2026-05-01T15:30",
      status: "open",
      counterpartyId: "",
      actId: "",
      linkKind: "document",
      linkLabel: "INV-2026-0042"
    },
    showEditor: true
  };
}

function setTasksState(items: TaskItemDto[], overrides: Partial<{ tab: TasksTab }> = {}) {
  mocks.tasksState.set({
    screen: makeScreen(items),
    editor: makeEditor(),
    initialLoading: false,
    loading: false,
    error: null,
    message: null,
    query: "",
    tab: overrides.tab ?? "open"
  });
}

function renderTasks() {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new TasksScreen({ target });

  return { component, target };
}

function buttonByText(target: HTMLElement, text: string): HTMLButtonElement {
  const button = Array.from(target.querySelectorAll("button")).find((candidate) =>
    candidate.textContent?.includes(text)
  );

  expect(button).toBeTruthy();
  return button as HTMLButtonElement;
}

describe("TasksScreen component", () => {
  beforeEach(() => {
    setTasksState([
      makeTask("task-1"),
      makeTask("task-2", {
        dueDate: "2026-05-03",
        reminderAt: "2026-05-03T09:00",
        priority: "normal",
        priorityLabel: "Звичайний"
      })
    ]);

    for (const fn of [
      mocks.closeEditor,
      mocks.deleteCurrent,
      mocks.load,
      mocks.openEditor,
      mocks.save,
      mocks.setStatus,
      mocks.setTab,
      mocks.updateFormField
    ]) {
      fn.mockReset();
    }
    mocks.closeEditor.mockReturnValue({ ok: true });
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders a focus-first task workflow with canonical actions and date controls", () => {
    const { component, target } = renderTasks();

    expect(target.textContent).toContain("Завдання");
    expect(target.textContent).toContain("У фокусі");
    expect(target.textContent).toContain("На сьогодні");
    expect(target.textContent).toContain("Пов'язано з INV-2026-0042");

    expect(buttonByText(target, "Нове завдання").className).toContain("btn-primary");
    expect(buttonByText(target, "Готово").className).toContain("btn-secondary");
    expect(buttonByText(target, "Видалити").className).toContain("btn-danger");

    const reminderInput = target.querySelector('input[type="datetime-local"]');
    expect(reminderInput).toBeTruthy();

    component.$destroy();
  });

  it("toggles task status through the store action", async () => {
    const { component, target } = renderTasks();

    buttonByText(target, "Готово").click();
    await tick();

    expect(mocks.setStatus).toHaveBeenCalledWith("task-1", "done");

    component.$destroy();
  });

  it("shows a strong empty state for today panel when there are no immediate tasks", () => {
    setTasksState([
      makeTask("task-3", {
        dueDate: "2026-05-10",
        reminderAt: "2026-05-10T12:00"
      })
    ]);

    const { component, target } = renderTasks();

    expect(target.textContent).toContain("На сьогодні");
    expect(target.textContent).toContain("Сьогодні немає нагадувань");

    component.$destroy();
  });

  it("exposes stable smoke markers for focus workflow and today panel", () => {
    const { component, target } = renderTasks();

    expect(target.querySelector('[data-testid="tasks-screen"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="tasks-focus-primary"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="tasks-today-panel"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="tasks-list"]')).toBeTruthy();

    component.$destroy();
  });

  it("shows compact skeleton rows during initial loading while chrome stays visible", () => {
    mocks.tasksState.set({
      screen: null,
      editor: null,
      initialLoading: true,
      loading: false,
      error: null,
      message: null,
      query: "",
      tab: "open"
    });

    const { component, target } = renderTasks();

    expect(target.textContent).toContain("Завдання");
    expect(target.querySelector('[data-testid="tasks-focus-primary"]')).toBeTruthy();
    expect(target.querySelector('.task-kpis')).toBeTruthy();
    expect(target.querySelector('.task-tabs')).toBeTruthy();
    expect(target.querySelector('[data-testid="tasks-today-panel"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="tasks-list"]')).toBeTruthy();
    expect(
      target.querySelector('[data-testid="tasks-list"]')?.querySelectorAll('[data-testid="skeleton-row-item"]')
    ).toHaveLength(5);
    expect(target.textContent).not.toContain("Сьогодні немає нагадувань");
    expect(target.querySelector('[data-testid="tasks-today-skeleton"]')).toBeTruthy();

    component.$destroy();
  });

  it("shows the dirty banner on Escape before closing the editor", async () => {
    mocks.closeEditor.mockReturnValue({ ok: false, reason: "dirty" } as any);
    const { component, target } = renderTasks();

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true }));
    await tick();

    expect(target.querySelector('[data-testid="tasks-dirty-banner"]')).toBeTruthy();
    expect(mocks.closeEditor).toHaveBeenCalledWith(false);

    component.$destroy();
  });

  it("shows the dirty banner on backdrop click before closing the editor", async () => {
    mocks.closeEditor.mockReturnValue({ ok: false, reason: "dirty" } as any);
    const { component, target } = renderTasks();

    (target.querySelector(".editor-backdrop") as HTMLDivElement).click();
    await tick();

    expect(target.querySelector('[data-testid="tasks-dirty-banner"]')).toBeTruthy();
    expect(mocks.closeEditor).toHaveBeenCalledWith(false);

    component.$destroy();
  });

});
