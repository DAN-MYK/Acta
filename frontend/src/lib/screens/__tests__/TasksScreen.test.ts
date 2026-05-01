/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import TasksScreen from "../TasksScreen.svelte";
import type { TaskEditorDto, TaskItemDto, TasksScreenDto } from "../../types";

type TasksTab = "open" | "done" | "all";

const RealDate = Date;

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
    loading: false,
    error: null as string | null,
    message: null as string | null,
    query: "",
    tab: "open" as TasksTab
  });

  return {
    closeEditor: vi.fn(),
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

function freezeLocalKyivMidnight() {
  class MockDate extends RealDate {
    constructor(...args: any[]) {
      if (args.length === 0) {
        super("2026-04-30T21:30:00.000Z");
        return;
      }

      super(args[0]);
    }

    getFullYear() {
      return 2026;
    }

    getMonth() {
      return 4;
    }

    getDate() {
      return 1;
    }

    static now() {
      return new RealDate("2026-04-30T21:30:00.000Z").getTime();
    }
  }

  vi.stubGlobal("Date", MockDate);
}

describe("TasksScreen component", () => {
  beforeEach(() => {
    freezeLocalKyivMidnight();

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
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.innerHTML = "";
  });

  it("renders a focus-first task workflow with canonical actions and date controls", () => {
    const { component, target } = renderTasks();

    expect(target.textContent).toContain("Завдання");
    expect(target.textContent).toContain("У фокусі");
    expect(target.textContent).toContain("На сьогодні");
    expect(target.textContent).toContain("Потребують уваги зараз");
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
    expect(target.textContent).toContain("Створіть нове завдання");
    expect(target.textContent).toContain("або закрийте хвости");

    component.$destroy();
  });

  it("uses local calendar day for overdue emphasis near kyiv midnight", () => {
    setTasksState([
      makeTask("task-overdue", {
        dueDate: "2026-04-30",
        reminderAt: "2026-04-30T09:00",
        priority: "critical",
        priorityLabel: "Критичний"
      })
    ]);

    const { component, target } = renderTasks();

    expect(target.textContent).toContain("Прострочено");

    const criticalPill = target.querySelector(".task-pill-critical");
    expect(criticalPill).toBeTruthy();
    expect(criticalPill?.textContent).toContain("Критичний");

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
});
