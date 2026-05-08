# Skeleton Loaders Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Додати shimmer skeleton loaders для data-heavy Svelte screens, не ламаючи наявні busy-state під час save/open/reconcile/export.

**Architecture:** Рішення складається з двох нових UI-примітивів (`SkeletonRow`, `SkeletonCard`), спільного shimmer utility в design tokens та нового store-прапорця `initialLoading`, який живе окремо від операційного `loading`. Інтеграція екранів іде батчами: спершу прості спискові screens (`Documents`, `Counterparties`, `Tasks`), потім складніші з persistent chrome (`Payments`, `Dashboard`, `Reports`).

**Tech Stack:** Svelte 4, TypeScript, Vitest + jsdom, CSS tokens, Tauri frontend stores

---

## Файли, що змінюються

| Файл | Призначення |
|---|---|
| `frontend/src/lib/components/SkeletonRow.svelte` | Новий skeleton для рядків списків |
| `frontend/src/lib/components/SkeletonCard.svelte` | Новий skeleton для KPI/focus cards |
| `frontend/src/lib/components/__tests__/SkeletonRow.test.ts` | Юніт-тести `SkeletonRow` |
| `frontend/src/lib/components/__tests__/SkeletonCard.test.ts` | Юніт-тести `SkeletonCard` |
| `frontend/src/lib/styles/tokens.css` | Глобальні shimmer keyframes + `.sk` utility |
| `frontend/src/lib/stores/documents.ts` | `initialLoading` для documents store |
| `frontend/src/lib/stores/payments.ts` | `initialLoading` для payments store |
| `frontend/src/lib/stores/counterparties.ts` | `initialLoading` для counterparties store |
| `frontend/src/lib/stores/dashboard.ts` | `initialLoading` для dashboard store |
| `frontend/src/lib/stores/tasks.ts` | `initialLoading` для tasks store |
| `frontend/src/lib/stores/reports.ts` | `initialLoading` для reports store |
| `frontend/src/lib/stores/__tests__/documents-store.test.ts` | Регресія на documents store |
| `frontend/src/lib/stores/__tests__/dashboard.test.ts` | Регресія на dashboard store |
| `frontend/src/lib/stores/__tests__/counterparties-payments-settings.test.ts` | Регресія на counterparties + payments |
| `frontend/src/lib/screens/DocumentsScreen.svelte` | Skeleton для focus cards + list |
| `frontend/src/lib/screens/CounterpartiesScreen.svelte` | Skeleton для list pane |
| `frontend/src/lib/screens/TasksScreen.svelte` | Skeleton для compact task list |
| `frontend/src/lib/screens/PaymentsScreen.svelte` | Skeleton лише для payment lists |
| `frontend/src/lib/screens/DashboardScreen.svelte` | Per-section skeleton blocks |
| `frontend/src/lib/screens/ReportsScreen.svelte` | Skeleton лише для data table |
| `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts` | UI-регресія `initialLoading` |
| `frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts` | UI-регресія list skeleton |
| `frontend/src/lib/screens/__tests__/TasksScreen.test.ts` | UI-регресія compact skeleton |
| `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts` | UI-регресія persistent chrome |
| `frontend/src/lib/screens/__tests__/DashboardScreen.test.ts` | UI-регресія per-section skeleton |
| `frontend/src/lib/screens/__tests__/ReportsScreen.test.ts` | UI-регресія table-only skeleton |

## Базова перевірка після кожного таску

```bash
npm run check
npm run test:frontend -- <targeted-test-file>
```

Очікування:
- `npm run check` завершується без `Error`
- targeted `vitest` suite завершується `PASS`

---

### Task 1: Додати skeleton primitives і shimmer utility

**Files:**
- Create: `frontend/src/lib/components/SkeletonRow.svelte`
- Create: `frontend/src/lib/components/SkeletonCard.svelte`
- Create: `frontend/src/lib/components/__tests__/SkeletonRow.test.ts`
- Create: `frontend/src/lib/components/__tests__/SkeletonCard.test.ts`
- Modify: `frontend/src/lib/styles/tokens.css`

- [ ] **Step 1: Написати failing tests для нових компонентів**

Створити `frontend/src/lib/components/__tests__/SkeletonRow.test.ts`:

```ts
/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";
import SkeletonRow from "../SkeletonRow.svelte";

function renderRow(props: Record<string, unknown> = {}) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new SkeletonRow({ target, props });
  return { component, target };
}

describe("SkeletonRow", () => {
  it("renders the requested row count", () => {
    const { component, target } = renderRow({ count: 3 });
    expect(target.querySelectorAll('[data-testid="skeleton-row-item"]')).toHaveLength(3);
    component.$destroy();
  });

  it("renders icon block in default variant", () => {
    const { component, target } = renderRow();
    expect(target.querySelector('[data-testid="skeleton-row-icon"]')).toBeTruthy();
    component.$destroy();
  });

  it("omits icon block in compact variant", () => {
    const { component, target } = renderRow({ variant: "compact" });
    expect(target.querySelector('[data-testid="skeleton-row-icon"]')).toBeNull();
    component.$destroy();
  });
});
```

Створити `frontend/src/lib/components/__tests__/SkeletonCard.test.ts`:

```ts
/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";
import SkeletonCard from "../SkeletonCard.svelte";

it("renders the requested card count", () => {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new SkeletonCard({ target, props: { count: 4 } });

  expect(target.querySelectorAll('[data-testid="skeleton-card-item"]')).toHaveLength(4);

  component.$destroy();
});
```

- [ ] **Step 2: Запустити тести й зафіксувати падіння**

```bash
npm run test:frontend -- frontend/src/lib/components/__tests__/SkeletonRow.test.ts frontend/src/lib/components/__tests__/SkeletonCard.test.ts
```

Очікування: `FAIL` із повідомленням, що модулі `../SkeletonRow.svelte` та `../SkeletonCard.svelte` ще не існують.

- [ ] **Step 3: Додати shimmer utility у tokens**

У `frontend/src/lib/styles/tokens.css` додати в кінець файлу:

```css
@keyframes shimmer {
  0% {
    background-position: -600px 0;
  }

  100% {
    background-position: 600px 0;
  }
}

.sk {
  border-radius: var(--radius-md);
  background: linear-gradient(
    90deg,
    var(--bg-subtle) 25%,
    var(--bg) 50%,
    var(--bg-subtle) 75%
  );
  background-size: 1200px 100%;
  animation: shimmer 1.5s infinite linear;
}
```

- [ ] **Step 4: Реалізувати `SkeletonRow.svelte`**

Створити `frontend/src/lib/components/SkeletonRow.svelte`:

```svelte
<script lang="ts">
  export let count = 5;
  export let variant: "default" | "compact" = "default";

  const widths = ["40%", "55%", "65%", "48%", "60%", "52%"];
</script>

{#each Array.from({ length: count }) as _, index}
  <div class="skeleton-row" data-testid="skeleton-row-item">
    {#if variant === "default"}
      <div class="skeleton-icon sk" data-testid="skeleton-row-icon"></div>
    {/if}

    <div class="skeleton-copy">
      <div class="skeleton-line sk" style={`width:${widths[index % widths.length]}`}></div>
      <div class="skeleton-line skeleton-line-short sk" style={`width:${widths[(index + 2) % widths.length]}`}></div>
    </div>

    <div class="skeleton-meta">
      <div class="skeleton-amount sk"></div>
      <div class="skeleton-badge sk"></div>
    </div>
  </div>
{/each}
</script>
```

- [ ] **Step 5: Реалізувати `SkeletonCard.svelte`**

Створити `frontend/src/lib/components/SkeletonCard.svelte`:

```svelte
<script lang="ts">
  export let count = 4;
</script>

<div class="skeleton-card-grid" data-testid="skeleton-card-grid">
  {#each Array.from({ length: count }) as _, index}
    <article class="skeleton-card" data-testid="skeleton-card-item">
      <div class="skeleton-card-label sk" style={`width:${index % 2 === 0 ? "42%" : "55%"}`}></div>
      <div class="skeleton-card-value sk"></div>
      <div class="skeleton-card-subtitle sk" style={`width:${index % 2 === 0 ? "68%" : "58%"}`}></div>
    </article>
  {/each}
</div>
```

- [ ] **Step 6: Перезапустити перевірку**

```bash
npm run check
npm run test:frontend -- frontend/src/lib/components/__tests__/SkeletonRow.test.ts frontend/src/lib/components/__tests__/SkeletonCard.test.ts
```

Очікування: `PASS`.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/styles/tokens.css frontend/src/lib/components/SkeletonRow.svelte frontend/src/lib/components/SkeletonCard.svelte frontend/src/lib/components/__tests__/SkeletonRow.test.ts frontend/src/lib/components/__tests__/SkeletonCard.test.ts
git commit -m "feat(ui): add reusable skeleton loader primitives"
```

---

### Task 2: Ввести `initialLoading` у всі data-heavy stores

**Files:**
- Modify: `frontend/src/lib/stores/documents.ts`
- Modify: `frontend/src/lib/stores/payments.ts`
- Modify: `frontend/src/lib/stores/counterparties.ts`
- Modify: `frontend/src/lib/stores/dashboard.ts`
- Modify: `frontend/src/lib/stores/tasks.ts`
- Modify: `frontend/src/lib/stores/reports.ts`
- Modify: `frontend/src/lib/stores/__tests__/documents-store.test.ts`
- Modify: `frontend/src/lib/stores/__tests__/dashboard.test.ts`
- Modify: `frontend/src/lib/stores/__tests__/counterparties-payments-settings.test.ts`

- [ ] **Step 1: Написати failing assertions для `initialLoading`**

У `frontend/src/lib/stores/__tests__/documents-store.test.ts` після першого `await documentsStore.load()` додати:

```ts
expect(snapshot(documentsStore).initialLoading).toBe(false);
```

У `frontend/src/lib/stores/__tests__/dashboard.test.ts` у mock state і після render/load додати:

```ts
expect(snapshot(dashboardStore).initialLoading).toBe(false);
```

У `frontend/src/lib/stores/__tests__/counterparties-payments-settings.test.ts` додати по одному assert:

```ts
expect(snapshot(counterpartiesStore).initialLoading).toBe(false);
expect(snapshot(paymentsStore).initialLoading).toBe(false);
```

- [ ] **Step 2: Запустити store tests і побачити падіння**

```bash
npm run test:frontend -- frontend/src/lib/stores/__tests__/documents-store.test.ts frontend/src/lib/stores/__tests__/dashboard.test.ts frontend/src/lib/stores/__tests__/counterparties-payments-settings.test.ts
```

Очікування: `FAIL` через відсутнє поле `initialLoading`.

- [ ] **Step 3: Додати `initialLoading` у store state**

У кожному store-документі додати поле в state та initial state:

```ts
interface DashboardState {
  screen: DashboardScreenDto | null;
  initialLoading: boolean;
  loading: boolean;
  error: string | null;
}

const initialState: DashboardState = {
  screen: null,
  initialLoading: true,
  loading: false,
  error: null
};
```

Для `documents.ts`, `payments.ts`, `counterparties.ts`, `tasks.ts`, `reports.ts` зробити аналогічно: `initialLoading: true` в initial state.

- [ ] **Step 4: Скинути `initialLoading` лише після першого успішного fetch**

У кожному `load()`-методі оновити success branch:

```ts
update((state) => ({
  ...state,
  list,
  loading: false,
  initialLoading: false
}));
```

Для `dashboard.ts` це буде:

```ts
if (requestId === latestRequestId) {
  update((state) => ({
    ...state,
    screen,
    loading: false,
    initialLoading: false
  }));
}
```

Важливо:
- не скидати `initialLoading` у catch branch
- не встановлювати `initialLoading: true` повторно у `open/save/reconcile/export`

- [ ] **Step 5: Оновити test mocks**

У screen/store tests, де мокаються store snapshots, додати нове поле:

```ts
const documentsState = createMockStore({
  list: null,
  editor: null,
  chain: null,
  draftContext: null,
  selectedIds: [],
  initialLoading: false,
  loading: false,
  error: null,
  message: null,
  query: ""
});
```

Зробити те саме для `paymentsState`, `counterpartiesState`, `tasksState`, `dashboardState`, `reportsState`.

- [ ] **Step 6: Перезапустити перевірку**

```bash
npm run check
npm run test:frontend -- frontend/src/lib/stores/__tests__/documents-store.test.ts frontend/src/lib/stores/__tests__/dashboard.test.ts frontend/src/lib/stores/__tests__/counterparties-payments-settings.test.ts
```

Очікування: `PASS`.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/stores/documents.ts frontend/src/lib/stores/payments.ts frontend/src/lib/stores/counterparties.ts frontend/src/lib/stores/dashboard.ts frontend/src/lib/stores/tasks.ts frontend/src/lib/stores/reports.ts frontend/src/lib/stores/__tests__/documents-store.test.ts frontend/src/lib/stores/__tests__/dashboard.test.ts frontend/src/lib/stores/__tests__/counterparties-payments-settings.test.ts
git commit -m "feat(stores): add initialLoading for first-fetch skeleton states"
```

---

### Task 3: Підключити skeleton loaders до Documents, Counterparties і Tasks

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`
- Modify: `frontend/src/lib/screens/CounterpartiesScreen.svelte`
- Modify: `frontend/src/lib/screens/TasksScreen.svelte`
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`
- Modify: `frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts`
- Modify: `frontend/src/lib/screens/__tests__/TasksScreen.test.ts`

- [ ] **Step 1: Написати failing screen tests**

У `DocumentsScreen.test.ts` додати кейс:

```ts
it("shows skeletons only during initial loading", () => {
  mocks.documentsState.set({
    list: null,
    editor: null,
    chain: null,
    draftContext: null,
    selectedIds: [],
    initialLoading: true,
    loading: false,
    error: null,
    message: null,
    query: ""
  });

  const { component, target } = renderDocuments();
  expect(target.querySelector('[data-testid="skeleton-card-grid"]')).toBeTruthy();
  expect(target.querySelectorAll('[data-testid="skeleton-row-item"]')).toHaveLength(5);
  expect(target.querySelector('[data-testid="documents-list"]')).toBeNull();
  component.$destroy();
});
```

Для `CounterpartiesScreen.test.ts` та `TasksScreen.test.ts` додати аналогічні кейси з `initialLoading: true` і очікуваннями на `5/6` skeleton rows.

- [ ] **Step 2: Запустити screen tests і зафіксувати падіння**

```bash
npm run test:frontend -- frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts frontend/src/lib/screens/__tests__/TasksScreen.test.ts
```

Очікування: `FAIL`, бо screens ще не рендерять skeleton components.

- [ ] **Step 3: Інтегрувати skeletons у `DocumentsScreen.svelte`**

На початку файлу додати імпорти:

```svelte
import SkeletonCard from "../components/SkeletonCard.svelte";
import SkeletonRow from "../components/SkeletonRow.svelte";
```

Замінити data-driven частину:

```svelte
{#if $documents.initialLoading}
  <div class="documents-focus-grid">
    <SkeletonCard count={2} />
  </div>
  <SkeletonRow count={5} />
{:else if ($documents.list?.items.length ?? 0) === 0}
  <!-- existing empty state -->
{:else}
  <div class="documents-list" data-testid="documents-list">
    <!-- existing rows -->
  </div>
{/if}
```

- [ ] **Step 4: Інтегрувати skeletons у `CounterpartiesScreen.svelte` і `TasksScreen.svelte`**

У `CounterpartiesScreen.svelte`:

```svelte
{#if $counterparties.initialLoading}
  <SkeletonRow count={6} />
{:else}
  <!-- existing list/detail content -->
{/if}
```

У `TasksScreen.svelte`:

```svelte
{#if $tasks.initialLoading}
  <SkeletonRow count={5} variant="compact" />
{:else}
  <!-- existing task content -->
{/if}
```

Не ховати panel header, search або tabs.

- [ ] **Step 5: Додати regression test, що `loading` не показує skeleton**

У `DocumentsScreen.test.ts` додати окремий тест:

```ts
it("does not replace content with skeleton during save-like loading", () => {
  setDocumentsState();
  mocks.documentsState.set({
    list: makeList(),
    editor: makeEditor(),
    chain: makeChain(),
    draftContext: {
      counterpartyId: "counterparty-1",
      counterpartyName: "ТОВ Ромашка"
    },
    selectedIds: [],
    initialLoading: false,
    loading: true,
    error: null,
    message: null,
    query: ""
  });

  const { component, target } = renderDocuments();
  expect(target.querySelector('[data-testid="documents-list"]')).toBeTruthy();
  expect(target.querySelector('[data-testid="skeleton-card-grid"]')).toBeNull();
  component.$destroy();
});
```

- [ ] **Step 6: Перезапустити перевірку**

```bash
npm run check
npm run test:frontend -- frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts frontend/src/lib/screens/__tests__/TasksScreen.test.ts
```

Очікування: `PASS`.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte frontend/src/lib/screens/CounterpartiesScreen.svelte frontend/src/lib/screens/TasksScreen.svelte frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts frontend/src/lib/screens/__tests__/TasksScreen.test.ts
git commit -m "feat(ui): add skeleton loaders to document, counterparty and task screens"
```

---

### Task 4: Підключити skeleton loaders до Payments, Dashboard і Reports

**Files:**
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte`
- Modify: `frontend/src/lib/screens/DashboardScreen.svelte`
- Modify: `frontend/src/lib/screens/ReportsScreen.svelte`
- Modify: `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts`
- Modify: `frontend/src/lib/screens/__tests__/DashboardScreen.test.ts`
- Modify: `frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`

- [ ] **Step 1: Написати failing tests для persistent chrome**

У `PaymentsScreen.test.ts` додати кейс:

```ts
it("keeps import chrome visible and skeletonizes only lists during initial load", () => {
  setPaymentsState({
    list: null,
    initialLoading: true,
    loading: false
  });

  const { component, target } = renderPayments();
  expect(target.textContent).toContain("Імпорт");
  expect(target.textContent).toContain("Створити платіж");
  expect(target.querySelectorAll('[data-testid="skeleton-row-item"]')).toHaveLength(6);
  component.$destroy();
});
```

У `DashboardScreen.test.ts` додати кейс на `panel-header` + per-section skeletons.  
У `ReportsScreen.test.ts` додати кейс, що header, tabs і filters видимі, але table замінена на `SkeletonRow`.

- [ ] **Step 2: Запустити tests і побачити падіння**

```bash
npm run test:frontend -- frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts frontend/src/lib/screens/__tests__/DashboardScreen.test.ts frontend/src/lib/screens/__tests__/ReportsScreen.test.ts
```

Очікування: `FAIL`.

- [ ] **Step 3: Інтегрувати skeletons у `PaymentsScreen.svelte`**

Додати імпорт:

```svelte
import SkeletonRow from "../components/SkeletonRow.svelte";
```

Залишити `panel-header`, `create-strip-card` і KPI row поза skeleton branch.  
Замінити лише групи списків:

```svelte
{#if $payments.initialLoading}
  <div class="payments-groups">
    <section class="payments-group payments-group-unmatched">
      <SkeletonRow count={6} />
    </section>
  </div>
{:else}
  <!-- existing payments groups -->
{/if}
```

- [ ] **Step 4: Інтегрувати per-section skeletons у `DashboardScreen.svelte`**

Додати імпорти:

```svelte
import SkeletonCard from "../components/SkeletonCard.svelte";
import SkeletonRow from "../components/SkeletonRow.svelte";
```

Окремі блоки:

```svelte
<div class="dashboard-kpis">
  {#if $dashboard.initialLoading}
    <SkeletonCard count={4} />
  {:else}
    <!-- existing KPI cards -->
  {/if}
</div>
```

Для `cashflow`, `recentDocuments`, `upcomingPayments`, `urgentTasks` зробити окремі `{#if $dashboard.initialLoading}` навколо each/list content, але не ховати panel header і назви карток.

- [ ] **Step 5: Інтегрувати table-only skeleton у `ReportsScreen.svelte`**

Додати імпорт:

```svelte
import SkeletonRow from "../components/SkeletonRow.svelte";
```

Не ховати:
- `panel-header`
- action buttons
- tab bar
- filter controls
- reports focus cards

Замінити лише table branch:

```svelte
{#if $reports.initialLoading}
  <div class="reports-table reports-table-card" data-testid="reports-table-card">
    <SkeletonRow count={6} />
  </div>
{:else if !hasActiveRows($reports.screen?.filter.tab)}
  <!-- existing empty state -->
{:else}
  <!-- existing report tables -->
{/if}
```

- [ ] **Step 6: Додати regression tests на `loading !== initialLoading`**

Для `PaymentsScreen.test.ts` і `ReportsScreen.test.ts` додати кейси:

```ts
expect(target.querySelector('[data-testid="skeleton-row-item"]')).toBeNull();
expect(target.textContent).toContain("Імпорт триває");
```

Сенс: під час `activeAction: "import"` або export/reconcile UI лишається видимим, а skeleton не повертається.

- [ ] **Step 7: Перезапустити перевірку**

```bash
npm run check
npm run test:frontend -- frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts frontend/src/lib/screens/__tests__/DashboardScreen.test.ts frontend/src/lib/screens/__tests__/ReportsScreen.test.ts
```

Очікування: `PASS`.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/lib/screens/PaymentsScreen.svelte frontend/src/lib/screens/DashboardScreen.svelte frontend/src/lib/screens/ReportsScreen.svelte frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts frontend/src/lib/screens/__tests__/DashboardScreen.test.ts frontend/src/lib/screens/__tests__/ReportsScreen.test.ts
git commit -m "feat(ui): add skeleton loaders to payments dashboard and reports"
```

---

### Task 5: Фінальна інтеграційна перевірка і короткий docs cleanup

**Files:**
- Modify: `docs/superpowers/specs/2026-05-01-skeleton-loaders-design.md` (лише якщо під час імплементації виявиться drift)

- [ ] **Step 1: Запустити повний frontend check**

```bash
npm run check
npm run test:frontend
```

Очікування: весь frontend test suite `PASS`.

- [ ] **Step 2: Ручна smoke-перевірка сценаріїв initial load**

```bash
npm run dev
```

Перевірити вручну:
- skeleton видно лише на першому вході в `Documents`, `Counterparties`, `Tasks`, `Payments`, `Dashboard`, `Reports`
- при `save`, `reconcile`, `export` або `refresh` старий контент не зникає повністю
- `Payments` action buttons, `Dashboard` panel header і `Reports` filters лишаються видимими

- [ ] **Step 3: Зафіксувати spec drift тільки якщо він реально з’явився**

Якщо під час реалізації довелося змінити деталі плану/компонентів, оновити відповідний фрагмент spec. Якщо drift немає — цей крок пропустити без змін.

- [ ] **Step 4: Commit**

```bash
git add frontend docs/superpowers/specs/2026-05-01-skeleton-loaders-design.md
git commit -m "test(ui): verify skeleton loader rollout end to end"
```

---

## Покриття spec

- Shimmer animation + `.sk`: Task 1
- `SkeletonRow` / `SkeletonCard`: Task 1
- `initialLoading` vs `loading`: Task 2
- Documents / Counterparties / Tasks integration: Task 3
- Payments / Dashboard / Reports integration: Task 4
- Tests for components, stores, screens: Tasks 1-4
- Final verification that skeleton не миготить на мутаціях: Tasks 3-5

## Виконання

**Plan complete and saved to `docs/superpowers/plans/2026-05-01-skeleton-loaders.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
