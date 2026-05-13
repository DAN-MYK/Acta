# Frontend Hardcode Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Прибрати найризиковіші hardcode-вузли у Svelte/TypeScript фронтенді Acta без великої i18n-системи та без абстракцій "про всяк випадок".

**Architecture:** Лишаємо існуючий підхід `frontend/src/lib/config/ui.ts` як barrel-export і додаємо/доповнюємо маленькі typed config/helper модулі поруч із доменом: `documents.ts`, `payments.ts`, `reports.ts`, `tasks.ts`, новий `settings.ts`. Presentation-логіку, яка повторюється або має enum-like keys, тримаємо у config/presentation helper-ах; разові labels у формах лишаємо інлайн.

**Tech Stack:** Svelte, TypeScript, Vitest, Tauri invoke DTO через існуючі `frontend/src/lib/types.ts`.

---

## Межі Роботи

### Робимо

- Усуваємо behavioral drift у document PDF capability.
- Централізуємо payment direction meta/options.
- Типізуємо `PaymentActiveAction` так, щоб `PAYMENT_FLOW_COPY` не міг відстати від store.
- Додаємо formatter-и для overdue days і calendar event pluralization.
- Виносимо task tabs та settings section/integration meta.
- Починаємо CSS-token cleanup із `TasksScreen.svelte`, не чіпаючи весь дизайн одразу.

### Не Робимо

- Не додаємо i18n framework.
- Не виносимо всі одноразові labels форм.
- Не переписуємо table schema у reports.
- Не переносимо backend DTO-copy у frontend, якщо дані вже приходять із backend.

---

## File Structure

- Modify: `frontend/src/lib/config/documents.ts`
  - Єдиний helper для document PDF capability.
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`
  - Прибрати локальний `["act", "invoice"]`.
- Modify: `frontend/src/lib/config/payments.ts`
  - `PAYMENT_DIRECTION_META`, `PAYMENT_DIRECTION_OPTIONS`, `PaymentActiveAction`, typed `PAYMENT_FLOW_COPY`, calendar copy helpers.
- Modify: `frontend/src/lib/paymentsPresentation.ts`
  - Використати payment direction helper із config.
- Modify: `frontend/src/lib/stores/payments.ts`
  - Імпортувати `PaymentActiveAction`; прибрати локальний union.
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte`
  - Замінити локальні payment direction options/copy і import preview magic number.
- Modify: `frontend/src/lib/components/PaymentCalendarPanel.svelte`
  - Використати calendar pluralization helpers/copy.
- Modify: `frontend/src/lib/config/reports.ts`
  - Додати `formatOverdueDaysLabel`.
- Modify: `frontend/src/lib/screens/ReportsScreen.svelte`
  - Замінити duplicated overdue labels.
- Modify: `frontend/src/lib/config/tasks.ts`
  - Додати `TASK_TAB_OPTIONS`, `TASK_TAB_VISIBLE_STATUSES`, `TASK_PRIORITY_META`.
- Modify: `frontend/src/lib/tasksPresentation.ts`
  - Використати task config замість локальних maps.
- Modify: `frontend/src/lib/screens/TasksScreen.svelte`
  - Замінити локальні tab buttons на config; частково привести CSS vars до `--acta-*`.
- Create: `frontend/src/lib/config/settings.ts`
  - `SETTINGS_SECTION_OPTIONS`, `getIntegrationStateMeta`.
- Modify: `frontend/src/lib/config/ui.ts`
  - Export `settings.ts`.
- Modify: `frontend/src/lib/screens/SettingsScreen.svelte`
  - Використати settings config.
- Test: `frontend/src/lib/stores/__tests__/presentation-config.test.ts`
  - Легкі unit-тести formatter/meta helper-ів.
- Existing tests to run:
  - `npm run test:frontend`
  - `npm run check`

---

### Task 1: Document PDF Capability

**Files:**
- Modify: `frontend/src/lib/config/documents.ts`
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`
- Test: `frontend/src/lib/stores/__tests__/presentation-config.test.ts`

- [ ] **Step 1: Write failing tests for document PDF capability**

Create or extend `frontend/src/lib/stores/__tests__/presentation-config.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { supportsDocumentPdfGeneration, supportsExistingPdfFlow } from "../../config/ui";

describe("document presentation config", () => {
  it("keeps generated PDF capability explicit", () => {
    expect(supportsDocumentPdfGeneration("act")).toBe(true);
    expect(supportsDocumentPdfGeneration("invoice")).toBe(true);
    expect(supportsDocumentPdfGeneration("waybill")).toBe(false);
  });

  it("keeps existing PDF attach capability explicit", () => {
    expect(supportsExistingPdfFlow("invoice")).toBe(true);
    expect(supportsExistingPdfFlow("waybill")).toBe(true);
    expect(supportsExistingPdfFlow("act")).toBe(false);
  });
});
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```bash
npm run test:frontend -- presentation-config.test.ts
```

Expected: fail because `supportsDocumentPdfGeneration` does not exist.

- [ ] **Step 3: Add the explicit helper**

In `frontend/src/lib/config/documents.ts`, add:

```ts
export function supportsDocumentPdfGeneration(kind: string): boolean {
  return kind === "act" || kind === "invoice";
}
```

Keep `supportsExistingPdfFlow(kind)` unchanged unless product behavior says generated PDF and existing PDF support must be identical.

- [ ] **Step 4: Use helper in DocumentsScreen**

In `frontend/src/lib/screens/DocumentsScreen.svelte`, import `supportsDocumentPdfGeneration` from `../config/ui` and replace:

```svelte
{#if ["act", "invoice"].includes($documents.editor.form.kind)}
```

with:

```svelte
{#if supportsDocumentPdfGeneration($documents.editor.form.kind)}
```

- [ ] **Step 5: Verify**

Run:

```bash
npm run test:frontend -- presentation-config.test.ts
npm run check
```

Expected: both pass.

---

### Task 2: Payment Direction Meta

**Files:**
- Modify: `frontend/src/lib/config/payments.ts`
- Modify: `frontend/src/lib/paymentsPresentation.ts`
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte`
- Test: `frontend/src/lib/stores/__tests__/presentation-config.test.ts`

- [ ] **Step 1: Add tests for direction labels and options**

Append:

```ts
import {
  getCalendarEventDirectionLabel,
  getPaymentDirectionLabel,
  PAYMENT_DIRECTION_OPTIONS
} from "../../config/ui";

describe("payment direction presentation config", () => {
  it("maps API and form direction values to one label source", () => {
    expect(getPaymentDirectionLabel("in")).toBe("Надходження");
    expect(getPaymentDirectionLabel("income")).toBe("Надходження");
    expect(getPaymentDirectionLabel("out")).toBe("Витрата");
    expect(getPaymentDirectionLabel("expense")).toBe("Витрата");
    expect(getCalendarEventDirectionLabel("income")).toBe("Надходження");
    expect(getCalendarEventDirectionLabel("expense")).toBe("Витрата");
  });

  it("exposes select options for the payment editor", () => {
    expect(PAYMENT_DIRECTION_OPTIONS).toEqual([
      { value: "income", label: "Надходження" },
      { value: "expense", label: "Витрата" }
    ]);
  });
});
```

- [ ] **Step 2: Run focused test**

Run:

```bash
npm run test:frontend -- presentation-config.test.ts
```

Expected: fail because direction exports do not exist in config.

- [ ] **Step 3: Implement payment direction config**

In `frontend/src/lib/config/payments.ts`, add:

```ts
export const PAYMENT_DIRECTION_META = {
  income: { label: "Надходження" },
  expense: { label: "Витрата" }
} as const;

export const PAYMENT_DIRECTION_OPTIONS = [
  { value: "income", label: PAYMENT_DIRECTION_META.income.label },
  { value: "expense", label: PAYMENT_DIRECTION_META.expense.label }
] as const;

export function getPaymentDirectionLabel(direction: string): string {
  return direction === "in" || direction === "income"
    ? PAYMENT_DIRECTION_META.income.label
    : PAYMENT_DIRECTION_META.expense.label;
}

export function getCalendarEventDirectionLabel(direction: string): string {
  return getPaymentDirectionLabel(direction);
}
```

- [ ] **Step 4: Remove duplicate helper body**

In `frontend/src/lib/paymentsPresentation.ts`, remove local `getPaymentDirectionLabel` logic and re-export/import from config:

```ts
export { getPaymentDirectionLabel } from "./config/ui";
```

Keep `getPaymentStateLabel`, `getPaymentDocumentKindLabel`, `getPaymentPreviewCopy`, `getPaymentCandidateHint`.

- [ ] **Step 5: Keep calendar direction labels on the same source**

`frontend/src/lib/components/PaymentCalendarPanel.svelte` already imports `getCalendarEventDirectionLabel` from `../config/ui`; after Step 3 it must continue to use that helper. Do not add a second inline `income/expense` map in the component or store.

- [ ] **Step 6: Use direction options in Payment editor**

In `frontend/src/lib/screens/PaymentsScreen.svelte`, import `PAYMENT_DIRECTION_OPTIONS` and replace:

```svelte
<option value="income">Надходження</option>
<option value="expense">Витрата</option>
```

with:

```svelte
{#each PAYMENT_DIRECTION_OPTIONS as option}
  <option value={option.value}>{option.label}</option>
{/each}
```

- [ ] **Step 7: Verify**

Run:

```bash
npm run test:frontend -- presentation-config.test.ts
npm run test:frontend
npm run check
```

Expected: all pass.

---

### Task 3: Typed Payment Active Actions

**Files:**
- Modify: `frontend/src/lib/config/payments.ts`
- Modify: `frontend/src/lib/stores/payments.ts`
- Test: TypeScript via `npm run check`

- [ ] **Step 1: Move the action type to config**

In `frontend/src/lib/config/payments.ts`, add before `PAYMENT_FLOW_COPY`:

```ts
export type PaymentActiveAction =
  | "import"
  | "import-pick"
  | "import-commit"
  | "sync"
  | "reconcile"
  | "manual-search"
  | "confirm-auto-match"
  | "confirm-candidate"
  | "confirm-manual-picker"
  | "confirm-split"
  | "unreconcile"
  | "calendar-complete"
  | "save";
```

- [ ] **Step 2: Make `PAYMENT_FLOW_COPY` exhaustive**

Change:

```ts
export const PAYMENT_FLOW_COPY: Record<string, { title: string; description: string }> = {
```

to:

```ts
export const PAYMENT_FLOW_COPY = {
```

and close it with:

```ts
} satisfies Record<PaymentActiveAction, { title: string; description: string }>;
```

Add a copy entry for `"calendar-complete"`:

```ts
"calendar-complete": {
  title: "Позначаємо подію виконаною",
  description: "Оновлюємо графік платежів і календар, щоб завершена подія більше не потребувала дії."
},
```

- [ ] **Step 3: Use the shared type in the store**

In `frontend/src/lib/stores/payments.ts`, import:

```ts
import type { PaymentActiveAction } from "../config/ui";
```

Delete local `type PaymentsActiveAction = ... | null;` and change state field:

```ts
activeAction: PaymentActiveAction | null;
```

- [ ] **Step 4: Type action helpers with non-null actions**

In `frontend/src/lib/stores/payments.ts`, update local helper signatures so actions accept `PaymentActiveAction`, not the nullable state type. `null` should remain only in `PaymentsStoreState.activeAction` and reset assignments.

Use this shape:

```ts
function beginAction(action: PaymentActiveAction, paymentId: string | null = null) {
  update((state) => ({
    ...state,
    loading: true,
    error: null,
    activeAction: action,
    activePaymentId: paymentId
  }));
}

async function runMutationAction<T extends MutationResultDto>(
  action: PaymentActiveAction,
  mutation: () => Promise<T>,
  paymentId: string | null = null
): Promise<T> {
  beginAction(action, paymentId);
  try {
    return await mutation();
  } finally {
    finishAction();
  }
}
```

Keep `finishAction()` resetting:

```ts
activeAction: null,
activePaymentId: null
```

- [ ] **Step 5: Verify type safety**

Run:

```bash
npm run check
```

Expected: pass. If it fails on `PAYMENT_FLOW_COPY[$payments.activeAction]`, keep the existing null guard in `getFlowCopy()` and cast is not needed because the guard narrows `activeAction`.

---

### Task 4: Reports Overdue Formatter

**Files:**
- Modify: `frontend/src/lib/config/reports.ts`
- Modify: `frontend/src/lib/screens/ReportsScreen.svelte`
- Test: `frontend/src/lib/stores/__tests__/presentation-config.test.ts`

- [ ] **Step 1: Add tests for overdue labels**

Append:

```ts
import { formatOverdueDaysLabel } from "../../config/ui";

describe("reports overdue formatter", () => {
  it("formats non-overdue rows", () => {
    expect(formatOverdueDaysLabel(0)).toBe("Без прострочки");
    expect(formatOverdueDaysLabel(-2)).toBe("Без прострочки");
  });

  it("formats Ukrainian day forms for overdue rows", () => {
    expect(formatOverdueDaysLabel(1)).toBe("Прострочено 1 день");
    expect(formatOverdueDaysLabel(2)).toBe("Прострочено 2 дні");
    expect(formatOverdueDaysLabel(5)).toBe("Прострочено 5 днів");
    expect(formatOverdueDaysLabel(21)).toBe("Прострочено 21 день");
  });
});
```

- [ ] **Step 2: Implement formatter**

In `frontend/src/lib/config/reports.ts`, add:

```ts
export function formatDaysLabel(count: number): string {
  const abs = Math.abs(count);
  const lastTwo = abs % 100;
  const last = abs % 10;

  if (lastTwo >= 11 && lastTwo <= 14) {
    return `${count} днів`;
  }
  if (last === 1) {
    return `${count} день`;
  }
  if (last >= 2 && last <= 4) {
    return `${count} дні`;
  }
  return `${count} днів`;
}

export function formatOverdueDaysLabel(days: number): string {
  return days > 0 ? `Прострочено ${formatDaysLabel(days)}` : "Без прострочки";
}
```

- [ ] **Step 3: Replace duplicates in ReportsScreen**

Import `formatOverdueDaysLabel` from `../config/ui` and replace both expressions:

```svelte
{row.overdueDays > 0 ? `Прострочено ${row.overdueDays} дн.` : "Без прострочки"}
```

with:

```svelte
{formatOverdueDaysLabel(row.overdueDays)}
```

- [ ] **Step 4: Verify**

Run:

```bash
npm run test:frontend -- presentation-config.test.ts
npm run check
```

Expected: pass.

---

### Task 5: Calendar Event Pluralization

**Files:**
- Modify: `frontend/src/lib/config/payments.ts`
- Modify: `frontend/src/lib/components/PaymentCalendarPanel.svelte`
- Test: `frontend/src/lib/stores/__tests__/presentation-config.test.ts`

- [ ] **Step 1: Add tests for calendar labels**

Append:

```ts
import { formatCalendarEventsLabel, formatCalendarMoreEventsLabel } from "../../config/ui";

describe("calendar event labels", () => {
  it("formats event count labels", () => {
    expect(formatCalendarEventsLabel(0)).toBe("без подій");
    expect(formatCalendarEventsLabel(1)).toBe("1 подія");
    expect(formatCalendarEventsLabel(2)).toBe("2 події");
    expect(formatCalendarEventsLabel(5)).toBe("5 подій");
    expect(formatCalendarEventsLabel(11)).toBe("11 подій");
    expect(formatCalendarEventsLabel(14)).toBe("14 подій");
    expect(formatCalendarEventsLabel(21)).toBe("21 подія");
    expect(formatCalendarEventsLabel(22)).toBe("22 події");
    expect(formatCalendarEventsLabel(25)).toBe("25 подій");
  });

  it("formats compact more labels", () => {
    expect(formatCalendarMoreEventsLabel(1)).toBe("+1 ще");
    expect(formatCalendarMoreEventsLabel(3)).toBe("+3 ще");
  });
});
```

- [ ] **Step 2: Improve existing formatter**

In `frontend/src/lib/config/payments.ts`, update `formatCalendarEventsLabel`:

```ts
function formatCalendarEventWord(count: number): "подія" | "події" | "подій" {
  const abs = Math.abs(count);
  const lastTwo = abs % 100;
  const last = abs % 10;

  if (lastTwo >= 11 && lastTwo <= 14) {
    return "подій";
  }
  if (last === 1) {
    return "подія";
  }
  if (last >= 2 && last <= 4) {
    return "події";
  }
  return "подій";
}

export function formatCalendarEventsLabel(count: number): string {
  if (count === 0) {
    return "без подій";
  }
  return `${count} ${formatCalendarEventWord(count)}`;
}

export function formatCalendarMoreEventsLabel(count: number): string {
  return `+${count} ще`;
}
```

- [ ] **Step 3: Use helpers in PaymentCalendarPanel**

Import `formatCalendarEventsLabel` and `formatCalendarMoreEventsLabel`.

Replace:

```svelte
{selectedEvents.length} подій у вибраному дні
```

with:

```svelte
{formatCalendarEventsLabel(selectedEvents.length)} у вибраному дні
```

Replace:

```svelte
<span class="calendar-pill is-more">+{filteredEvents(day).length - 2} ще</span>
```

with:

```svelte
<span class="calendar-pill is-more">{formatCalendarMoreEventsLabel(filteredEvents(day).length - 2)}</span>
```

- [ ] **Step 4: Verify**

Run:

```bash
npm run test:frontend -- presentation-config.test.ts
npm run check
```

Expected: pass.

---

### Task 6: Task Tabs And Priority Meta

**Files:**
- Modify: `frontend/src/lib/config/tasks.ts`
- Modify: `frontend/src/lib/tasksPresentation.ts`
- Modify: `frontend/src/lib/screens/TasksScreen.svelte`
- Test: `frontend/src/lib/stores/__tests__/presentation-config.test.ts`

- [ ] **Step 1: Add tests for task tabs and priority tone**

Append:

```ts
import {
  getTaskPriorityTone,
  getVisibleTaskStatuses,
  TASK_TAB_OPTIONS
} from "../../config/ui";

describe("task presentation config", () => {
  it("exposes task tabs in UI order", () => {
    expect(TASK_TAB_OPTIONS.map((tab) => tab.value)).toEqual(["open", "done", "all"]);
  });

  it("maps tab to visible statuses", () => {
    expect(getVisibleTaskStatuses("open")).toEqual(["open", "in_progress"]);
    expect(getVisibleTaskStatuses("done")).toEqual(["done", "cancelled"]);
  });

  it("maps task priority to visual tone", () => {
    expect(getTaskPriorityTone("critical")).toBe("danger");
    expect(getTaskPriorityTone("high")).toBe("danger");
    expect(getTaskPriorityTone("normal")).toBe("warning");
    expect(getTaskPriorityTone("low")).toBe("none");
  });
});
```

- [ ] **Step 2: Move tab/status/priority maps to config**

In `frontend/src/lib/config/tasks.ts`, add:

```ts
export type TasksTab = "open" | "done" | "all";
export type TaskPriorityTone = "danger" | "warning" | "none";

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
```

- [ ] **Step 3: Use config in tasksPresentation**

In `frontend/src/lib/tasksPresentation.ts`, remove local `TasksTab`, `TaskPriorityTone`, `TASK_TAB_VISIBLE_STATUSES`, `TASK_PRIORITY_SORT_WEIGHT`, `TASK_PRIORITY_TONE`, `getVisibleTaskStatuses`, `getTaskPrioritySortWeight`, `getTaskPriorityTone`.

Import/re-export:

```ts
import {
  getTaskPrioritySortWeight,
  getVisibleTaskStatuses,
  type TasksTab
} from "./config/ui";

export {
  getTaskPriorityTone,
  getVisibleTaskStatuses,
  type TaskPriorityTone,
  type TasksTab
} from "./config/ui";
```

- [ ] **Step 4: Use `TASK_TAB_OPTIONS` in TasksScreen**

Import `TASK_TAB_OPTIONS` and replace the three hardcoded tab buttons with:

```svelte
{#each TASK_TAB_OPTIONS as tab}
  <button class:active={$tasks.tab === tab.value} on:click={() => tasks.setTab(tab.value)}>
    {tab.label}
  </button>
{/each}
```

- [ ] **Step 5: Verify**

Run:

```bash
npm run test:frontend -- presentation-config.test.ts
npm run test:frontend -- TasksScreen.test.ts
npm run check
```

Expected: pass.

---

### Task 7: Settings Config

**Files:**
- Create: `frontend/src/lib/config/settings.ts`
- Modify: `frontend/src/lib/config/ui.ts`
- Modify: `frontend/src/lib/screens/SettingsScreen.svelte`
- Test: `frontend/src/lib/stores/__tests__/presentation-config.test.ts`

- [ ] **Step 1: Add settings config tests**

Append:

```ts
import { getIntegrationStateMeta, SETTINGS_SECTION_OPTIONS } from "../../config/ui";

describe("settings presentation config", () => {
  it("keeps settings sections in navigation order", () => {
    expect(SETTINGS_SECTION_OPTIONS.map((section) => section.value)).toEqual([
      "appearance",
      "company",
      "numbering",
      "integrations",
      "team",
      "backup"
    ]);
  });

  it("maps integration enabled state to label and tone", () => {
    expect(getIntegrationStateMeta(true)).toEqual({ label: "Активно", tone: "is-success" });
    expect(getIntegrationStateMeta(false)).toEqual({ label: "Вимкнено", tone: "is-error" });
  });
});
```

- [ ] **Step 2: Create settings config**

Create `frontend/src/lib/config/settings.ts`:

```ts
import type { SettingsSection } from "../types";

export const SETTINGS_SECTION_OPTIONS: Array<{ value: SettingsSection; label: string }> = [
  { value: "appearance", label: "Зовнішній вигляд" },
  { value: "company", label: "Компанія" },
  { value: "numbering", label: "Нумерація" },
  { value: "integrations", label: "Інтеграції" },
  { value: "team", label: "Команда" },
  { value: "backup", label: "Резервні копії" }
];

export function getIntegrationStateMeta(enabled: boolean): { label: string; tone: "is-success" | "is-error" } {
  return enabled
    ? { label: "Активно", tone: "is-success" }
    : { label: "Вимкнено", tone: "is-error" };
}
```

- [ ] **Step 3: Export settings config**

In `frontend/src/lib/config/ui.ts`, add:

```ts
export * from "./settings";
```

- [ ] **Step 4: Use config in SettingsScreen**

In `frontend/src/lib/screens/SettingsScreen.svelte`, import:

```ts
import { getIntegrationStateMeta, SETTINGS_SECTION_OPTIONS } from "../config/ui";
```

Delete local `settingsSections` and `integrationState`.

Replace:

```svelte
{#each settingsSections as [section, label]}
```

with:

```svelte
{#each SETTINGS_SECTION_OPTIONS as sectionOption}
```

Then use:

```svelte
class:active={$settings.section === sectionOption.value}
on:click={() => onSettingsSectionChange(sectionOption.value)}
{sectionOption.label}
```

Replace `integrationState(integration.enabled)` with `getIntegrationStateMeta(integration.enabled)`.

- [ ] **Step 5: Verify**

Run:

```bash
npm run test:frontend -- presentation-config.test.ts
npm run test:frontend -- SettingsScreen.test.ts
npm run check
```

Expected: pass.

---

### Task 8: Payment Import Preview Copy And Limit

**Files:**
- Modify: `frontend/src/lib/config/payments.ts`
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte`
- Test: `frontend/src/lib/stores/__tests__/presentation-config.test.ts`

- [ ] **Step 1: Add config exports**

In `frontend/src/lib/config/payments.ts`, add:

```ts
export const PAYMENT_IMPORT_PREVIEW_VISIBLE_ROWS = 25;

function formatPaymentWord(count: number): "платіж" | "платежі" | "платежів" {
  const abs = Math.abs(count);
  const lastTwo = abs % 100;
  const last = abs % 10;

  if (lastTwo >= 11 && lastTwo <= 14) {
    return "платежів";
  }
  if (last === 1) {
    return "платіж";
  }
  if (last >= 2 && last <= 4) {
    return "платежі";
  }
  return "платежів";
}

export function formatPaymentImportCountLabel(count: number): string {
  return `Імпортувати ${count} ${formatPaymentWord(count)}`;
}

export const PAYMENT_IMPORT_PREVIEW_COPY = {
  staleTitle: "Файл виписки змінився",
  staleDescription: "Перечитайте виписку, щоб оновити план імпорту перед підтвердженням.",
  refreshFile: "Перечитати файл",
  importing: "Імпортуємо...",
  noNewPayments: "Немає нових платежів",
  cancel: "Скасувати",
  recognizedRows: "Розпізнано рядків",
  willCreate: "Буде створено",
  willSkip: "Уже існує (skip)",
  conflicts: "Конфлікти",
  action: "Дія",
  bankRef: "Bank ref",
  description: "Призначення",
  note: "Нотатка",
  createAction: "Нове",
  skipAction: "Пропуск",
  emptyRows: "У файлі не знайдено жодного рядка виписки.",
  visibleRows: (visible: number, total: number) => `Показано перші ${visible} з ${total} рядків.`
} as const;
```

- [ ] **Step 2: Add a small test for the visible rows copy**

Append:

```ts
import {
  formatPaymentImportCountLabel,
  PAYMENT_IMPORT_PREVIEW_COPY,
  PAYMENT_IMPORT_PREVIEW_VISIBLE_ROWS
} from "../../config/ui";

describe("payment import preview config", () => {
  it("keeps visible row limit and copy together", () => {
    expect(PAYMENT_IMPORT_PREVIEW_VISIBLE_ROWS).toBe(25);
    expect(PAYMENT_IMPORT_PREVIEW_COPY.visibleRows(25, 40)).toBe("Показано перші 25 з 40 рядків.");
  });

  it("formats import payment count labels", () => {
    expect(formatPaymentImportCountLabel(1)).toBe("Імпортувати 1 платіж");
    expect(formatPaymentImportCountLabel(2)).toBe("Імпортувати 2 платежі");
    expect(formatPaymentImportCountLabel(5)).toBe("Імпортувати 5 платежів");
    expect(formatPaymentImportCountLabel(21)).toBe("Імпортувати 21 платіж");
  });
});
```

- [ ] **Step 3: Replace local import preview literals**

In `frontend/src/lib/screens/PaymentsScreen.svelte`, import `PAYMENT_IMPORT_PREVIEW_COPY` and `PAYMENT_IMPORT_PREVIEW_VISIBLE_ROWS`.

Replace these local literals in the import preview block:

- stale title/description
- refresh button label
- import button loading/no-new/count labels
- summary labels
- table headers
- row action labels
- `slice(0, 25)`
- `rows.length > 25`
- visible rows text
- empty rows text

Use:

```svelte
{formatPaymentImportCountLabel($payments.importPreview.willCreate)}
```

for the import count button label, and:

```svelte
{#each $payments.importPreview.rows.slice(0, PAYMENT_IMPORT_PREVIEW_VISIBLE_ROWS) as row, idx (idx)}
```

and:

```svelte
{#if $payments.importPreview.rows.length > PAYMENT_IMPORT_PREVIEW_VISIBLE_ROWS}
  <p class="payment-import-preview-more">
    {PAYMENT_IMPORT_PREVIEW_COPY.visibleRows(
      PAYMENT_IMPORT_PREVIEW_VISIBLE_ROWS,
      $payments.importPreview.rows.length
    )}
  </p>
{/if}
```

- [ ] **Step 4: Verify**

Run:

```bash
npm run test:frontend -- presentation-config.test.ts
npm run test:frontend -- PaymentsScreen.test.ts
npm run check
```

Expected: pass.

---

### Task 9: Minimal TasksScreen Token Cleanup

**Files:**
- Modify: `frontend/src/lib/screens/TasksScreen.svelte`
- Test: `npm run check`

- [ ] **Step 1: Replace legacy CSS aliases in TasksScreen**

In `<style>` of `frontend/src/lib/screens/TasksScreen.svelte`, replace recurring aliases:

```css
var(--bg-elevated) -> var(--acta-color-bg-elevated)
var(--bg) -> var(--acta-color-bg-page)
var(--bg-hover) -> var(--acta-color-bg-hover)
var(--bg-subtle) -> var(--acta-color-bg-subtle)
var(--border) -> var(--acta-color-border)
var(--text) -> var(--acta-color-text)
var(--text-muted) -> var(--acta-color-text-muted)
var(--text-faint) -> var(--acta-color-text-faint)
var(--accent) -> var(--acta-color-accent)
var(--accent-text) -> var(--acta-color-accent-text)
var(--danger) -> var(--acta-color-danger)
var(--danger-soft) -> var(--acta-color-danger-soft)
var(--warning) -> var(--acta-color-warning)
var(--success) -> var(--acta-color-success)
var(--success-soft) -> var(--acta-color-success-soft)
var(--font-sans) -> var(--acta-font-sans)
var(--font-mono) -> var(--acta-font-mono)
```

- [ ] **Step 2: Replace most repeated local radius/spacing values where obvious**

Use existing tokens only where the mapping is direct:

```css
border-radius: 10px; -> border-radius: var(--acta-radius-xl);
border-radius: 8px; -> border-radius: var(--acta-radius-lg);
border-radius: 6px; -> border-radius: var(--acta-radius-md);
border-radius: 4px; -> border-radius: var(--acta-radius-sm);
```

Leave one-off layout values like `grid-template-columns: 1fr 300px`, `width: 480px`, `min-height: 52px` inline.

- [ ] **Step 3: Verify**

Run:

```bash
npm run check
```

Expected: pass.

Manual visual QA recommended after implementation: open Tasks screen and confirm task rows, tabs, drawer, messages and today panel still match current density.

---

### Task 10: Final Verification And Guardrails

**Files:**
- Check only, no planned source changes unless a command exposes a missed import/type.

- [ ] **Step 1: Run frontend tests**

Run:

```bash
npm run test:frontend
```

Expected: all Vitest suites pass.

- [ ] **Step 2: Run TypeScript/Svelte check**

Run:

```bash
npm run check
```

Expected: no TypeScript/Svelte errors.

- [ ] **Step 3: Search for old high-value hardcodes**

Run:

```bash
Select-String -Path frontend/src/lib/screens/*.svelte,frontend/src/lib/components/*.svelte,frontend/src/lib/stores/*.ts -Pattern '\"act\", \"invoice\"|Прострочено .*дн|slice\(0, 25\)|settingsSections|integrationState\('
```

Expected: no hits in production files.

- [ ] **Step 4: Leave intentionally inline copy alone**

Confirm these remain inline unless future repetition appears:

- Reports table headers.
- Form field labels in Documents/Payments/Tasks/Settings.
- DOM ids and aria wiring ids.
- Keyboard key strings.
- One-off layout dimensions.

---

## Self-Review

- Spec coverage: covers all high-value audit findings: document capability drift, local dictionaries/options, loading/import copy, pluralization, enum-like action kinds, task/status/priority maps, CSS token drift.
- Placeholder scan: no implementation step relies on `TODO`, `TBD`, or unspecified behavior.
- Type consistency: new exported names are imported via `../config/ui`; tests import from `../../config/ui`; `PaymentActiveAction` excludes `null`, state uses `PaymentActiveAction | null`.
- Overengineering guardrail: no i18n system, no table schema refactor, no global CSS rewrite.
