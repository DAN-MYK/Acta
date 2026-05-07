import type { TaskPriority, TaskStatus } from "../types";

export type TasksTab = "open" | "done" | "all";
export type TaskPriorityTone = "danger" | "warning" | "none";

export const TASK_PRIORITY_OPTIONS: Array<{ value: TaskPriority; label: string }> = [
  { value: "low", label: "Низький" },
  { value: "normal", label: "Звичайний" },
  { value: "high", label: "Високий" },
  { value: "critical", label: "Критичний" }
];

export const TASK_STATUS_OPTIONS: Array<{ value: TaskStatus; label: string }> = [
  { value: "open", label: "Відкрите" },
  { value: "in_progress", label: "В роботі" },
  { value: "done", label: "Виконано" },
  { value: "cancelled", label: "Скасовано" }
];

export const TASK_TAB_OPTIONS: Array<{ value: TasksTab; label: string }> = [
  { value: "open", label: "У фокусі" },
  { value: "done", label: "Завершені" },
  { value: "all", label: "Усі" }
];

export const TASK_TAB_VISIBLE_STATUSES: Record<TasksTab, TaskStatus[]> = {
  open: ["open", "in_progress"],
  done: ["done", "cancelled"],
  all: ["open", "in_progress", "done", "cancelled"]
};

export const TASK_PRIORITY_META: Record<TaskPriority, { tone: TaskPriorityTone; sortWeight: number }> = {
  critical: { tone: "danger", sortWeight: 0 },
  high: { tone: "danger", sortWeight: 1 },
  normal: { tone: "warning", sortWeight: 2 },
  low: { tone: "none", sortWeight: 3 }
};

export function getVisibleTaskStatuses(tab: TasksTab): TaskStatus[] {
  return TASK_TAB_VISIBLE_STATUSES[tab];
}

export function getTaskPriorityTone(priority: TaskPriority): TaskPriorityTone {
  return TASK_PRIORITY_META[priority].tone;
}

export function getTaskPrioritySortWeight(priority: TaskPriority): number {
  return TASK_PRIORITY_META[priority].sortWeight;
}
