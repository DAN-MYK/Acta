import { EDITOR_DIRTY_COPY } from "./shared";

export const COUNTERPARTIES_COPY = {
  archiveConfirm: "Архівувати поточного контрагента? Повернення потребуватиме окремої дії.",
  loadingMessage: "Оновлюємо картку контрагента…",
  searchPlaceholder: "Пошук контрагента…",
  loadingTitle: "Завантажуємо картку контрагента",
  loadingDescription: "Список уже готується. Деталі з'являться тут, щойно підтягнемо перші дані.",
  emptyTitle: "Оберіть контрагента",
  emptyDescription:
    "Оберіть зліва вже відомого контрагента або створіть нового, щоб одразу побачити баланс, прострочки та сценарій роботи.",
  ...EDITOR_DIRTY_COPY
} as const;

export function formatLastContactLabel(days: number): string {
  if (days <= 0) {
    return "сьогодні";
  }
  if (days === 1) {
    return "1 день тому";
  }
  if (days >= 2 && days <= 4) {
    return `${days} дні тому`;
  }
  return `${days} днів тому`;
}
