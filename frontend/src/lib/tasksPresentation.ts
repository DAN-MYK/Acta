import {
  getTaskPrioritySortWeight,
  getVisibleTaskStatuses,
  TASK_TAB_OPTIONS,
  type TaskPriorityTone,
  type TasksTab
} from "./config/ui";
import type { TaskItemDto } from "./types";

const TASK_DAY_LABELS = ["нд", "пн", "вт", "ср", "чт", "пт", "сб"] as const;
const TASK_MONTH_LABELS = ["січ", "лют", "бер", "кві", "тра", "чер", "лип", "сер", "вер", "жов", "лис", "гру"] as const;

function localIsoDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function getFocusedTaskItems(items: TaskItemDto[], tab: TasksTab): TaskItemDto[] {
  const visibleStatuses = new Set(getVisibleTaskStatuses(tab));

  return items
    .filter((item) => visibleStatuses.has(item.status))
    .sort((left, right) => {
      const weightDiff = getTaskPrioritySortWeight(left.priority) - getTaskPrioritySortWeight(right.priority);
      if (weightDiff !== 0) {
        return weightDiff;
      }

      return (left.dueDate || "9999-99-99").localeCompare(right.dueDate || "9999-99-99");
    });
}

export function getTodayTaskItems(items: TaskItemDto[], date = new Date()): TaskItemDto[] {
  const today = localIsoDate(date);
  return items.filter((item) => item.dueDate === today || item.reminderAt.startsWith(today));
}

export function formatTaskDayLabel(date = new Date()): string {
  return `${TASK_DAY_LABELS[date.getDay()]} · ${date.getDate()} ${TASK_MONTH_LABELS[date.getMonth()]}`;
}

export {
  getTaskPrioritySortWeight,
  getTaskPriorityTone,
  getVisibleTaskStatuses,
  TASK_TAB_OPTIONS,
  type TaskPriorityTone,
  type TasksTab
} from "./config/ui";
