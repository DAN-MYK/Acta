import type { AppIconName } from "../icons";
import type { ScreenId } from "../types";

export const MAIN_NAV_ITEMS: Array<{
  screen: ScreenId;
  label: string;
  icon: AppIconName;
  badgeKey?: "documentsBadge" | "tasksBadge";
}> = [
  { screen: "dashboard", label: "Головна", icon: "dashboard" },
  { screen: "documents", label: "Документи", icon: "documents", badgeKey: "documentsBadge" },
  { screen: "counterparties", label: "Контрагенти", icon: "counterparties" },
  { screen: "payments", label: "Платежі", icon: "payments" },
  { screen: "reports", label: "Звіти", icon: "reports" },
  { screen: "tasks", label: "Завдання", icon: "tasks", badgeKey: "tasksBadge" }
];

export const SCREEN_TITLES: Record<ScreenId, string> = {
  dashboard: "Головна",
  documents: "Документи",
  counterparties: "Контрагенти",
  payments: "Платежі",
  reports: "Звіти",
  tasks: "Завдання",
  settings: "Налаштування"
};
