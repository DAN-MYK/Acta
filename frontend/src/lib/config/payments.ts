import type { PaymentCalendarEventKind, PaymentCalendarFilterKind, PaymentMatchDecisionKind } from "../types";

export const CALENDAR_FILTER_OPTIONS: Array<{ kind: PaymentCalendarFilterKind; label: string }> = [
  { kind: "all", label: "Усе" },
  { kind: "schedule", label: "Платежі" },
  { kind: "task", label: "Задачі" }
];

export const CALENDAR_EVENT_KIND_LABELS: Record<PaymentCalendarEventKind, string> = {
  schedule: "Платіж",
  task: "Задача"
};

export const PAYMENT_PREVIEW_COPY: Record<PaymentMatchDecisionKind, { title: string; description: string }> = {
  exact: {
    title: "Рекомендована звірка",
    description:
      "Система знайшла найкращий документ для автозіставлення. Перевірте рекомендацію перед підтвердженням."
  },
  ambiguous: {
    title: "Кілька кандидатів на звірку",
    description:
      "Оберіть найкращий варіант у списку, або відкрийте ручний пошук, якщо потрібен інший документ."
  },
  split: {
    title: "Рекомендований розподіл платежу",
    description:
      "Система підготувала рекомендований розподіл платежу між кількома документами. Перевірте кандидатів і, за потреби, скоригуйте суми в чернетці нижче."
  },
  none: {
    title: "Автоматична звірка не знайшла точного документа",
    description:
      "Для цього платежу поки немає точного збігу. Перевірте реквізити або відкрийте ручний пошук документа."
  }
};

export const PAYMENT_RECONCILE_MESSAGES: Record<PaymentMatchDecisionKind, string> = {
  exact: "Знайдено рекомендовану звірку. Перевірте та підтвердіть автозіставлення.",
  ambiguous:
    "Знайдено кілька кандидатів. Цей платіж потребує уваги, а ручне підтвердження буде наступним кроком.",
  split:
    "Знайдено рекомендований розподіл платежу між кількома документами. Перевірте алокації перед підтвердженням.",
  none: "Точний кандидат не знайдено. Перевірте платіж або підготуйте ручне звіряння."
};

export const PAYMENT_FLOW_COPY: Record<string, { title: string; description: string }> = {
  import: {
    title: "Імпорт триває",
    description: "Імпортуємо виписку та оновлюємо список платежів, щоб одразу показати незведені рухи."
  },
  "import-pick": {
    title: "Готуємо preview виписки",
    description: "Розбираємо файл виписки і будуємо список платежів, що чекають на імпорт."
  },
  "import-commit": {
    title: "Імпортуємо нові платежі",
    description: "Записуємо нові платежі у БД на основі підтвердженого preview."
  },
  sync: {
    title: "Оновлюємо рухи з банку",
    description: "Підтягуємо свіжі банківські рухи та готуємо їх до наступного кроку звірки."
  },
  reconcile: {
    title: "Готуємо preview звірки",
    description: "Шукаємо документи-кандидати й готуємо наступний крок для цього платежу."
  },
  "manual-search": {
    title: "Шукаємо документи для ручної звірки",
    description: "Формуємо повний список відкритих актів і накладних для ручного вибору."
  },
  unreconcile: {
    title: "Знімаємо зведення",
    description: "Знімаємо зв'язок із документом та повертаємо платіж у чергу на повторну звірку."
  },
  save: {
    title: "Зберігаємо платіж",
    description: "Фіксуємо зміни в картці платежу та оновлюємо список."
  },
  "confirm-auto-match": {
    title: "Підтверджуємо автозвірку",
    description: "Підтверджуємо рекомендоване автозіставлення і оновлюємо статус платежу."
  },
  "confirm-candidate": {
    title: "Підтверджуємо ручну звірку",
    description: "Прив'язуємо платіж до вибраного кандидата з preview."
  },
  "confirm-manual-picker": {
    title: "Фіксуємо ручний вибір документа",
    description: "Прив'язуємо платіж до документа, обраного через ручний пошук."
  },
  "confirm-split": {
    title: "Зберігаємо розподіл платежу",
    description: "Записуємо розподіл платежу між кількома документами і оновлюємо статуси."
  }
};

export const PAYMENT_MANUAL_PICKER_DISABLED_REASON =
  "Спершу знайдіть хоча б одного кандидата, щоб підтвердити документ.";


export const PAYMENT_SCREEN_COPY = {
  stateUnmatched: "Не зведено",
  stateMatched: (matchedDoc: string) => `Зв'язано з ${matchedDoc}`,
  prepareImportPreview: "Готуємо preview...",
  importStatement: "Імпортувати виписку",
  importFromStorage: "Імпорт з storage",
  importing: "Імпортуємо...",
  syncWithBank: "Оновити з банку",
  syncing: "Оновлюємо...",
  confirmAutoMatch: "Підтвердити автозіставлення",
  confirmPreviewCandidate: "Підтвердити вибраний варіант",
  chooseAnotherDocument: "Інший документ",
  closePreview: "Закрити preview",
  splitRecommendationBadge: "Рекомендація для розподілу",
  emptyNoMatchTitle: "Автоматична звірка не знайшла точного документа",
  emptyNoMatchDescription:
    "Перевірте референс платежу, контрагента або відкрийте ручний пошук документа.",
  openManualSearch: "Ручний пошук документа",
  manualPickerTitle: "Ручний вибір документа",
  manualPickerDescription:
    "Знайдіть акт або накладну за номером, назвою чи призначенням платежу.",
  refreshManualSearch: "Оновити пошук",
  confirmManualDocument: "Підтвердити вибраний документ",
  closeManualSearch: "Закрити пошук",
  emptyManualSearch: "За цим запитом кандидатів поки немає.",
  splitDraftTitle: "Чернетка розподілу",
  confirmSplit: "Підтвердити розподіл",
  emptyUnmatchedTitle: "Ще немає жодного платежу",
  emptyUnmatchedDescription:
    "Імпортуйте виписку або створіть ручний платіж, щоб почати звірку руху грошей.",
  emptyMatchedTitle: "Ще немає зведених платежів",
  emptyMatchedDescription:
    "Проведіть першу звірку в лівому блоці, щоб тут з'явився готовий результат.",
  reconcileAction: "Зводимо...",
  reconcileIdle: "Звести"
} as const;

export const PAYMENT_CALENDAR_COPY = {
  title: "Платіжний календар",
  loadingMonth: "Завантажуємо місяць",
  previousMonth: "Попередній",
  nextMonth: "Наступний",
  visibleEventsSummary: "Подій у поточному фільтрі",
  errorTitle: "Календар не завантажився",
  retryAction: "Спробувати ще раз",
  emptyTitle: "Календар поки порожній",
  emptyDescription: "Коли з’являться події графіка платежів або задачі з дедлайнами, вони відобразяться тут.",
  filterEmptyTitle: "У цьому місяці немає подій для поточного фільтра",
  filterEmptyDescription: "Перемкніть фільтр або перейдіть на інший місяць, щоб подивитися інші записи.",
  emptyDayLabel: "День не вибрано",
  emptyDayEvents: "На цей день подій не знайдено",
  emptyDayFiltered: "На цей день немає подій у поточному фільтрі"
} as const;


export const PAYMENT_MANUAL_MATCH_COPY = {
  missingAutoMatch: "Немає рекомендованої звірки для автоматичного підтвердження.",
  previewCandidateUnavailable: "Ручне підтвердження доступне лише для preview з кількома кандидатами.",
  previewCandidateMissing: "Виберіть кандидата для підтвердження звірки.",
  previewCandidateSelected: "Кандидата вибрано. Ручне підтвердження буде наступним кроком.",
  manualSearchClosed: "Спершу відкрийте ручний пошук для цього платежу.",
  manualCandidateSelected: "Вибрано документ для ручного звіряння.",
  splitPickerClosed: "Спершу відкрийте ручний picker для розподілу платежу.",
  splitCandidateMissing: "Виберіть документ, який треба додати до розподілу.",
  splitCandidateDuplicate: "Цей документ уже додано до розподілу.",
  splitFullyAllocated: "Увесь платіж уже розподілено. За потреби змініть суми в чернетці.",
  splitDraftUpdated: "Документ додано до чернетки розподілу.",
  splitCandidateAdded: "Документ додано до розподілу",
  splitAmountInvalid: "Сума розподілу має бути числом у форматі 0,00.",
  splitAmountTooSmall: "Сума розподілу має бути більшою за нуль.",
  splitAmountAboveDocument: "Сума розподілу не може перевищувати залишок документа.",
  splitAmountAbovePayment: "Сума розподілу не може перевищувати залишок платежу.",
  splitDraftRemoved: "Документ прибрано з чернетки розподілу.",
  manualPickerClosed: "Ручний picker ще не відкрито.",
  manualPickerCandidateMissing: "Виберіть документ для ручного звіряння.",
  splitDraftMissing: "Немає чернетки розподілу для підтвердження.",
  splitDraftEmpty: "Додайте хоча б один документ до розподілу.",
  splitDraftIncomplete: "Розподіл ще не завершено. Закрийте залишок платежу або зменште суму."
} as const;

export function formatCalendarEventsLabel(count: number): string {
  if (count === 0) {
    return "без подій";
  }
  if (count === 1) {
    return "1 подія";
  }
  return `${count} подій`;
}

export function formatCalendarDayAriaLabel(args: {
  date: string;
  eventCount: number;
  today: boolean;
  selected: boolean;
}): string {
  const todayLabel = args.today ? ", сьогодні" : "";
  const selectedLabel = args.selected ? ", вибрано" : "";
  return `${args.date}${todayLabel}${selectedLabel}, ${formatCalendarEventsLabel(args.eventCount)}`;
}

export function getCalendarEventDirectionLabel(direction: string): string {
  return direction === "income" ? "Надходження" : "Витрата";
}
