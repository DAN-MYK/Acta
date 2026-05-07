import type { TaskPriority, TaskStatus } from "../types";

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
