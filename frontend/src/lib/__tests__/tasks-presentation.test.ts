import { describe, expect, it } from "vitest";
import {
  formatTaskDayLabel,
  getFocusedTaskItems,
  getTaskPrioritySortWeight,
  getTaskPriorityTone,
  getTodayTaskItems,
  getVisibleTaskStatuses,
  TASK_TAB_OPTIONS
} from "../tasksPresentation";
import type { TaskItemDto } from "../types";

function makeTask(id: string, overrides: Partial<TaskItemDto> = {}): TaskItemDto {
  return {
    id,
    title: `Завдання ${id}`,
    description: "",
    status: "open",
    statusLabel: "Відкрите",
    priority: "normal",
    priorityLabel: "Звичайний",
    dueDate: "",
    reminderAt: "",
    linkKind: "document",
    linkLabel: "",
    ...overrides
  };
}

describe("tasksPresentation", () => {
  it("exposes task tabs in canonical UI order", () => {
    expect(TASK_TAB_OPTIONS.map((tab) => tab.value)).toEqual(["open", "done", "all"]);
  });

  it("returns canonical visible statuses for each tasks tab", () => {
    expect(getVisibleTaskStatuses("open")).toEqual(["open", "in_progress"]);
    expect(getVisibleTaskStatuses("done")).toEqual(["done", "cancelled"]);
    expect(getVisibleTaskStatuses("all")).toEqual(["open", "in_progress", "done", "cancelled"]);
  });

  it("provides stable priority sort weights and tones", () => {
    expect(getTaskPrioritySortWeight("critical")).toBeLessThan(getTaskPrioritySortWeight("high"));
    expect(getTaskPrioritySortWeight("high")).toBeLessThan(getTaskPrioritySortWeight("normal"));
    expect(getTaskPrioritySortWeight("normal")).toBeLessThan(getTaskPrioritySortWeight("low"));

    expect(getTaskPriorityTone("critical")).toBe("danger");
    expect(getTaskPriorityTone("high")).toBe("danger");
    expect(getTaskPriorityTone("normal")).toBe("warning");
    expect(getTaskPriorityTone("low")).toBe("none");
  });

  it("filters and sorts focused task items by tab, priority, and due date", () => {
    const items = [
      makeTask("done-late", { status: "done", dueDate: "2026-05-11", priority: "low" }),
      makeTask("normal-early", { status: "open", dueDate: "2026-05-02", priority: "normal" }),
      makeTask("critical", { status: "in_progress", dueDate: "2026-05-10", priority: "critical" }),
      makeTask("high-early", { status: "open", dueDate: "2026-05-01", priority: "high" }),
      makeTask("cancelled", { status: "cancelled", dueDate: "2026-05-03", priority: "high" })
    ];

    expect(getFocusedTaskItems(items, "open").map((item) => item.id)).toEqual([
      "critical",
      "high-early",
      "normal-early"
    ]);
    expect(getFocusedTaskItems(items, "done").map((item) => item.id)).toEqual(["cancelled", "done-late"]);
    expect(getFocusedTaskItems(items, "all").map((item) => item.id)).toEqual([
      "critical",
      "high-early",
      "cancelled",
      "normal-early",
      "done-late"
    ]);
  });

  it("picks today items from dueDate or reminderAt using a local calendar day", () => {
    const items = [
      makeTask("due-today", { dueDate: "2026-05-07" }),
      makeTask("reminder-today", { reminderAt: "2026-05-07T09:30" }),
      makeTask("later", { dueDate: "2026-05-08", reminderAt: "2026-05-08T08:00" })
    ];

    expect(getTodayTaskItems(items, new Date(2026, 4, 7, 9, 0, 0)).map((item) => item.id)).toEqual([
      "due-today",
      "reminder-today"
    ]);
  });

  it("formats the current task day label in the canonical compact Ukrainian style", () => {
    expect(formatTaskDayLabel(new Date(2026, 4, 7, 9, 0, 0))).toBe("чт · 7 тра");
  });
});
