# Documents audit fixes — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Реалізувати 9 виправлень зі сторінки Документи: responsive bug 1024, формат дати, розузгодженість статусів у drawer, split-button create, header grouping, conditional bulk-bar, direction toggle, видалити дубль типу в рядку, softer accent for active chips.

**Architecture:** 10 атомарних задач. CSS-only фікси (A1, C6, C8) — швидкі (no TDD red-phase). Структурні зміни в Svelte (B3, B4, B7, B9) — оновлюються паралельно з тестами. A5 — змішана зміна Rust DTO + TS типів + Svelte UI; розбита на 3 задачі за шарами.

**Tech Stack:** Svelte 4, TypeScript, Vite, Vitest (jsdom), Rust + sqlx, Tauri 2.

**Spec:** `docs/superpowers/specs/2026-05-13-documents-audit-fixes-design.md`

**Finding під час планування:** `index.html:2` вже містить `<html lang="uk">`. Це означає, що A2 — це manual verification (поведінка вже може бути коректною) + fallback план якщо ні.

---

## Task 1: A1 — Responsive breakpoint для doc-row

**Files:**
- Modify: `frontend/src/styles/documents.css:788-810`

- [ ] **Step 1: Прочитати поточне media query**

Подивитись `frontend/src/styles/documents.css:788-810`. Зараз `@media (max-width: 1080px)` робить `.doc-row, .doc-row-body { flex-direction: column }` — це причина checkbox-орфана на 1024.

- [ ] **Step 2: Замінити media query на дві окремі**

У `frontend/src/styles/documents.css` замінити блок 788–810 на:

```css
@media (max-width: 1080px) {
  .chain-summary,
  .editor-item-head,
  .editor-item,
  .existing-pdf-replace {
    grid-template-columns: 1fr;
  }

  .editor-items-summary,
  .editor-actions {
    justify-content: flex-start;
  }
}

@media (max-width: 720px) {
  .doc-row-body {
    flex-direction: column;
    align-items: flex-start;
  }

  .doc-row-meta {
    justify-content: flex-start;
  }
}
```

Ключове: `.doc-row` НЕ перемикається в column — лише `.doc-row-body` на 720 px.

- [ ] **Step 3: Запустити svelte-check**

Run: `cd frontend && npm run check`
Expected: clean (CSS зміни не повинні створити проблем для TS).

- [ ] **Step 4: Manual visual verify (1024 px і 720 px)**

У робочому Vite dev server (port 1420) відкрити сторінку Документи в 1024×800 та 720×600. Перевірити: на 1024 — checkbox inline зі своєю карткою; на 720 — meta-чіпи стекаються вертикально, але checkbox все ще inline.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/styles/documents.css
git commit -m "fix(documents): doc-row responsive breakpoint to 720px"
```

---

## Task 2: A2 — Перевірити формат дати на цільовій ОС

**Files:**
- Manual: `index.html` (вже містить `lang="uk"`)
- Modify (можливо): `frontend/src/lib/screens/DocumentsScreen.svelte` (fallback на input level)

- [ ] **Step 1: Перевірити що `lang="uk"` уже стоїть**

Прочитати `index.html:2`. Має бути `<html lang="uk">`. Підтверджено під час планування.

- [ ] **Step 2: Manual verification у Vite dev**

Запустити `cd src-tauri && cargo tauri dev`. Відкрити Документи → натиснути «Фільтр» → відкрити «Період → Від» date input. Подивитись формат у picker:
- Якщо `дд.мм.рррр` placeholder і обрана дата у форматі `13.05.2026` — Acceptance пройдено.
- Якщо `mm/dd/yyyy` — переходити до Кроку 3 (fallback).

- [ ] **Step 3 (fallback, тільки якщо потрібно): Додати `lang="uk"` на самі date inputs**

У `frontend/src/lib/screens/DocumentsScreen.svelte` знайти всі `<input type="date" …>` (2 місця — filter panel `panelDateFrom`/`panelDateTo`, та editor `editor-date-field`). Додати атрибут `lang="uk"`:

```svelte
<input type="date" lang="uk" bind:value={panelDateFrom} />
<input type="date" lang="uk" bind:value={panelDateTo} />
<input type="date" lang="uk" value={$documents.editor.form.date} on:input={onEditorDateChange} … />
```

Знову manual verify (Крок 2). Якщо ще не допомогло — створити follow-up задачу на display-layer (Variant B зі специфікації, поза цією задачею).

- [ ] **Step 4: Commit (тільки якщо був fallback)**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte
git commit -m "fix(documents): explicit lang=uk on date inputs"
```

Якщо fallback не знадобився — нічого комітити, просто переходити до Task 3.

---

## Task 3: C8 — Softer active state для kind-chip

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte:1177-1181` (scoped style)

- [ ] **Step 1: Замінити `.kind-chip-active` правило**

У `frontend/src/lib/screens/DocumentsScreen.svelte` всередині `<style>` блоку рядки ~1177–1181:

```css
/* БУЛО */
.kind-chip-active {
  background: var(--acta-color-accent);
  border-color: var(--acta-color-accent);
  color: #fff;
}

/* СТАЛО */
.kind-chip-active {
  background: color-mix(in srgb, var(--acta-color-accent-soft) 60%, var(--acta-color-bg-elevated));
  border-color: color-mix(in srgb, var(--acta-color-accent) 40%, var(--acta-color-border));
  color: var(--acta-color-accent-text);
  font-weight: 600;
}
```

- [ ] **Step 2: Запустити existing tests**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts`
Expected: PASS (тести не перевіряють кольори).

- [ ] **Step 3: Manual verify обох тем**

У Vite dev — світла тема. Перемкнути на темну через user menu. Активний chip має softer accent у обох темах.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte
git commit -m "fix(documents): softer accent for kind-chip-active"
```

---

## Task 4: C6 — Прибрати дубль kind badge у рядку

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte:780-784`
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`

- [ ] **Step 1: Написати failing test**

Додати у `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts` (після існуючих тестів `documents-row-`):

```ts
it("doc-row-meta does NOT contain doc-kind-badge (dupe removed)", async () => {
  mocks.documentsState.set({ ...baseState(), list: makeList() });
  const { container } = render(DocumentsScreen);
  await tick();

  const firstRow = container.querySelector('[data-testid="documents-row-doc-1"]');
  expect(firstRow).toBeTruthy();
  const meta = firstRow!.querySelector(".doc-row-meta");
  expect(meta).toBeTruthy();
  const kindBadge = meta!.querySelector(".doc-kind-badge");
  expect(kindBadge).toBeNull();
});
```

Якщо у тестовому файлі немає `render`, додати імпорт `import { render } from "@testing-library/svelte";` та `baseState()` helper якщо немає.

- [ ] **Step 2: Запустити тест — verify it fails**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts -t "doc-kind-badge"`
Expected: FAIL (зараз meta містить badge).

- [ ] **Step 3: Видалити дубль у HTML**

У `frontend/src/lib/screens/DocumentsScreen.svelte:780–784` видалити блок:

```svelte
<span class="doc-kind-badge">
  <AppIcon name={resolveDocumentKindMeta(item.kind).icon} size={14} />
  <span>{getDocumentKindLabel(item.kind)}</span>
</span>
```

Залишити решту `doc-row-meta`: дата, money, status-chip, direction-badge.

- [ ] **Step 4: Запустити тест — verify it passes**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts -t "doc-kind-badge"`
Expected: PASS.

- [ ] **Step 5: Запустити весь тест-файл — verify no regressions**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts`
Expected: ALL PASS.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
git commit -m "fix(documents): remove duplicate kind badge in row meta"
```

---

## Task 5: B4 — Conditional bulk-bar з list-header

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte:664-698` (HTML)
- Modify: `frontend/src/styles/documents.css` (CSS секція bulk-actions + new .documents-list-header)
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`

- [ ] **Step 1: Написати failing test для conditional rendering**

Додати у `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`:

```ts
it("bulk-actions container not rendered when selectedIds is empty", async () => {
  mocks.documentsState.set({ ...baseState(), list: makeList(), selectedIds: [] });
  const { container } = render(DocumentsScreen);
  await tick();

  const bulkActions = container.querySelector('[data-testid="documents-bulk-actions"]');
  expect(bulkActions).toBeNull();
});

it("bulk-actions container rendered when selectedIds has items", async () => {
  mocks.documentsState.set({ ...baseState(), list: makeList(), selectedIds: ["doc-1"] });
  const { container } = render(DocumentsScreen);
  await tick();

  const bulkActions = container.querySelector('[data-testid="documents-bulk-actions"]');
  expect(bulkActions).not.toBeNull();
  const count = container.querySelector('[data-testid="documents-bulk-count"]');
  expect(count?.textContent).toContain("1");
});
```

- [ ] **Step 2: Запустити — verify fail**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts -t "bulk-actions"`
Expected: FAIL (зараз завжди рендериться).

- [ ] **Step 3: Додати `clearSelection` у mock**

У `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts` додати `clearSelection: vi.fn()` у `mocks` об'єкт (рядок ~63) і у `vi.mock("../../stores/documents", …)` (рядок ~97). Інакше mock zero-init викине помилку при кліку «Скасувати вибір».

- [ ] **Step 4: Замінити HTML розмітку bulk-actions**

У `frontend/src/lib/screens/DocumentsScreen.svelte` замінити блок 664–698 (`{#if ($documents.list?.items.length ?? 0) > 0} … {/if}` з bulk-actions) на:

```svelte
{#if ($documents.list?.items.length ?? 0) > 0}
  <div class="documents-list-header" data-testid="documents-list-header">
    <label class="bulk-select-all">
      <input
        type="checkbox"
        checked={
          ($documents.list?.items.length ?? 0) > 0 &&
          ($documents.list?.items ?? []).every((item) => $documents.selectedIds.includes(item.id))
        }
        on:click|stopPropagation={onToggleSelectAll}
      />
      <span>Вибрати все ({$documents.list?.items.length ?? 0})</span>
    </label>
    {#if $documents.selectedIds.length > 0}
      <span class="bulk-count" data-testid="documents-bulk-count">
        Вибрано: {$documents.selectedIds.length}
      </span>
    {/if}
  </div>

  {#if $documents.selectedIds.length > 0}
    <div
      class="bulk-actions"
      data-testid="documents-bulk-actions"
      transition:slide={{ duration: 140 }}
    >
      <button class="btn-secondary" on:click={onBulkAdvanceStatus} disabled={$documents.loading}>
        Оновити статус
      </button>
      <button class="btn-danger" on:click={onBulkDelete} disabled={$documents.loading}>
        Видалити
      </button>
      <button class="btn-ghost" on:click={() => documents.clearSelection()}>
        Скасувати вибір
      </button>
    </div>
  {/if}
{/if}
```

- [ ] **Step 5: Додати імпорт slide transition**

У початку `<script lang="ts">` секції DocumentsScreen.svelte додати:

```ts
import { slide } from "svelte/transition";
```

- [ ] **Step 6: Оновити CSS**

У `frontend/src/styles/documents.css`:

1. Знайти існуючі `.bulk-actions-idle` правила (в `@media (max-width: 980px)` блоці) і видалити їх — більше не потрібні (bulk-bar conditional).
2. Додати нові правила:

```css
.documents-list-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
}

.bulk-count {
  font-size: 11px;
  font-weight: 600;
  color: var(--acta-color-accent-text);
  padding: 2px 8px;
  background: var(--acta-color-accent-soft);
  border-radius: 999px;
}

.bulk-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
  align-items: center;
}
```

3. Видалити існуючий `.bulk-actions { ... }` блок (якщо є з попередньої версії з `bulk-actions-idle` модифікатором).

- [ ] **Step 7: Запустити тести — verify pass**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts`
Expected: ALL PASS включно з новими bulk-actions тестами.

- [ ] **Step 8: Manual verify**

У Vite dev — Документи. Без виборів bulk-actions невидимий. Вибрати документ — bar з'являється з slide. Натиснути «Скасувати вибір» → bar зникає, виборі очищені.

- [ ] **Step 9: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte frontend/src/styles/documents.css frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
git commit -m "feat(documents): conditional bulk-actions bar with list header"
```

---

## Task 6: B3 — Split-button «Створити»

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte:635-662` (HTML) + `<script>` секція
- Modify: `frontend/src/styles/documents.css` (add split-button styles, remove .documents-create-kind-chips)
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`

- [ ] **Step 1: Failing tests для split-button**

Додати у `DocumentsScreen.test.ts`:

```ts
it("split-button primary click calls documents.create with current kind", async () => {
  mocks.documentsState.set(baseState());
  const { container } = render(DocumentsScreen);
  await tick();

  const primary = container.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement;
  expect(primary).toBeTruthy();
  primary.click();
  await tick();

  expect(mocks.create).toHaveBeenCalled();
});

it("split-button caret toggles menu visible", async () => {
  mocks.documentsState.set(baseState());
  const { container } = render(DocumentsScreen);
  await tick();

  const caret = container.querySelector('[data-testid="documents-create-menu-trigger"]') as HTMLButtonElement;
  expect(caret).toBeTruthy();
  caret.click();
  await tick();

  const menu = container.querySelector('[role="menu"]');
  expect(menu).toBeTruthy();
  expect(menu!.hasAttribute("hidden")).toBe(false);
});

it("split-button menu item triggers create with picked kind", async () => {
  mocks.documentsState.set(baseState());
  const { container } = render(DocumentsScreen);
  await tick();

  const caret = container.querySelector('[data-testid="documents-create-menu-trigger"]') as HTMLButtonElement;
  caret.click();
  await tick();

  const invoiceItem = container.querySelector('[data-testid="documents-create-menu-invoice"]') as HTMLButtonElement;
  expect(invoiceItem).toBeTruthy();
  invoiceItem.click();
  await tick();

  expect(mocks.create).toHaveBeenCalledWith(undefined, "invoice");
});
```

Якщо `baseState()` helper не визначений — додати:
```ts
function baseState() {
  return {
    list: null, editor: null, chain: null, draftContext: null,
    selectedIds: [], initialLoading: false, loading: false,
    error: null, message: null,
    activeTab: "all" as const, kindFilter: null, counterpartyFilterId: null,
    dateFrom: null, dateTo: null, statusFilter: [],
    amountMin: null, amountMax: null, overdueOnly: false, activePresetId: null
  };
}
```

- [ ] **Step 2: Verify tests fail**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts -t "split-button"`
Expected: FAIL (selectors не існують).

- [ ] **Step 3: Замінити HTML розмітку create-bar**

У `frontend/src/lib/screens/DocumentsScreen.svelte` замінити блок 635–662 (`<div class="documents-create-bar"…>`):

```svelte
<div class="documents-create-bar" data-testid="documents-create-strip">
  <div class="split-button" class:split-button-open={createMenuOpen} bind:this={createMenuRoot}>
    <button
      bind:this={createButton}
      class="split-button-primary btn-primary"
      data-testid="documents-create-button"
      type="button"
      disabled={$documents.loading}
      on:click={onCreateDraft}
      aria-busy={$documents.loading ? "true" : "false"}
    >
      <AppIcon name={documentKindMeta[createKind].icon} surface={true} />
      <span>{getDocumentCreateLabel(createKind, $documents.activeTab)}</span>
    </button>
    <button
      class="split-button-caret btn-primary"
      type="button"
      aria-haspopup="menu"
      aria-expanded={createMenuOpen}
      aria-label="Вибрати інший тип документа"
      data-testid="documents-create-menu-trigger"
      disabled={$documents.loading}
      on:click|stopPropagation={toggleCreateMenu}
    >▾</button>
    <div
      class="split-button-menu"
      role="menu"
      hidden={!createMenuOpen}
    >
      {#each DOCUMENT_KIND_OPTIONS as option}
        <button
          role="menuitem"
          type="button"
          class="split-button-menu-item"
          data-testid={`documents-create-menu-${option.value}`}
          disabled={$documents.loading}
          on:click={() => onPickCreateKind(option.value)}
        >
          <AppIcon name={documentKindMeta[option.value].icon} size={16} />
          <span>{option.label}</span>
        </button>
      {/each}
    </div>
  </div>
</div>
```

- [ ] **Step 4: Оновити `<script>` секцію DocumentsScreen.svelte**

Знайти `let createCounterpartyId = "";` (рядок ~33). Перед/після додати:

```ts
let createMenuOpen = false;
let createMenuRoot: HTMLElement | null = null;
```

Замінити існуючий `let createKind: DocumentKind = "act";` на:
```ts
let createKind: DocumentKind = (() => {
  try {
    const stored = localStorage.getItem("acta:documents:lastCreateKind");
    if (stored && DOCUMENT_KIND_OPTIONS.some((o) => o.value === stored)) {
      return stored as DocumentKind;
    }
  } catch {}
  return "act";
})();
```

Замінити існуючу `function onCreateDraft()`:
```ts
function onCreateDraft() {
  closeCreateMenu();
  void documents.create(createCounterpartyId || undefined, createKind);
}
```

Видалити існуючу `function onSelectCreateKind(kind: string)` і додати нові функції:

```ts
function toggleCreateMenu() { createMenuOpen = !createMenuOpen; }
function closeCreateMenu() { createMenuOpen = false; }

function onPickCreateKind(kind: DocumentKind) {
  createKind = kind;
  try { localStorage.setItem("acta:documents:lastCreateKind", kind); } catch {}
  closeCreateMenu();
  void documents.create(createCounterpartyId || undefined, kind);
}
```

- [ ] **Step 5: Об'єднати window click handler**

Знайти існуючу `function onWindowClickForChainMenu(event: MouseEvent)` і замінити на:

```ts
function onWindowClickGlobalMenus(event: MouseEvent) {
  const target = event.target as Node | null;

  if (chainMenuOpen) {
    if (target && chainMenuButton?.contains(target)) {
      // continue — caret click буде оброблений
    } else if (target && chainMenuPopover?.contains(target)) {
      // continue
    } else {
      closeChainMenu();
    }
  }

  if (createMenuOpen) {
    if (target && createMenuRoot?.contains(target)) {
      return;
    }
    closeCreateMenu();
  }
}
```

Замінити `<svelte:window on:keydown={onDrawerKeydown} on:click={onWindowClickForChainMenu} />` на `<svelte:window on:keydown={onDrawerKeydown} on:click={onWindowClickGlobalMenus} />`.

Reactive block для закриття chain menu (`$: { ... if (!editorDocId && chainMenuOpen) ...`) — без змін; залишається.

- [ ] **Step 6: Додати CSS split-button**

У `frontend/src/styles/documents.css` додати після `.documents-create-bar`:

```css
.split-button {
  position: relative;
  display: inline-flex;
  isolation: isolate;
}

.split-button-primary {
  border-top-right-radius: 0;
  border-bottom-right-radius: 0;
}

.split-button-caret {
  border-top-left-radius: 0;
  border-bottom-left-radius: 0;
  padding: 0 12px;
  border-left: 1px solid color-mix(in srgb, white 22%, transparent);
  font-size: 12px;
  min-width: 36px;
}

.split-button-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  min-width: 220px;
  display: flex;
  flex-direction: column;
  padding: 6px;
  border-radius: var(--acta-radius-2xl);
  border: 1px solid var(--acta-color-border);
  background: var(--acta-color-bg-elevated);
  box-shadow: 0 12px 32px -12px color-mix(in srgb, #0b1220 28%, transparent);
  z-index: 60;
}

.split-button-menu[hidden] { display: none; }

.split-button-menu-item {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border: 0;
  border-radius: var(--acta-radius-lg);
  background: transparent;
  color: var(--acta-color-text);
  text-align: left;
  cursor: pointer;
  font: inherit;
  font-weight: 500;
}

.split-button-menu-item:hover:not(:disabled) {
  background: color-mix(in srgb, var(--acta-color-accent-soft) 60%, var(--acta-color-bg-elevated));
}
```

І видалити `.documents-create-kind-chips` (рядки 9–14) — більше не використовується.

- [ ] **Step 7: Запустити тести**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts`
Expected: ALL PASS.

- [ ] **Step 8: Manual verify**

У Vite dev. Створити документ — primary click. Змінити тип — caret → invoice → побачити що створено invoice, primary тепер показує «Створити рахунок». Перезавантажити сторінку (`F5`) → primary повинен показувати останній обраний (`invoice`).

- [ ] **Step 9: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte frontend/src/styles/documents.css frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
git commit -m "feat(documents): split-button create with kind dropdown and localStorage persist"
```

---

## Task 7: A5 backend — Rust ChainStepDto додати `document_id` і `status_label`

**Files:**
- Modify: `src/tauri_api/documents/dto.rs:93-101`
- Modify: `src/tauri_api/documents/api.rs` (build_chain ~742, document snapshot constructors)
- Test: `tests/db_integration.rs` або `tests/tauri_vertical_slice.rs` якщо створюють ChainStepDto напряму

- [ ] **Step 1: Failing test для chain step (якщо є integration test)**

Перевірити чи існують Rust тести які працюють з `ChainStepDto`:

Run: `Grep -r "ChainStepDto" tests/ src/`

Якщо є — оновити mock у тесті щоб очікувати `document_id: Some("…")` і `status_label: "Виставлено"`. Якщо немає — пропустити крок (TDD у Rust не обов'язковий для DTO-зміни без логіки).

- [ ] **Step 2: Додати поля у `ChainStepDto`**

У `src/tauri_api/documents/dto.rs:93-101` замінити:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChainStepDto {
    pub document_id: Option<String>,
    pub doc_type: String,
    pub doc_number: String,
    pub amount_str: String,
    pub status: String,
    pub status_label: String,
    pub exists: bool,
}
```

- [ ] **Step 3: Додати `status_label` у `DocumentSnapshot`**

У `src/tauri_api/documents/api.rs:36-44` (struct DocumentSnapshot) додати:

```rust
struct DocumentSnapshot {
    ref_id: String,
    kind: String,
    number: String,
    counterparty_id: Uuid,
    counterparty_name: String,
    date: NaiveDate,
    total_amount: Decimal,
    status: String,
    status_label: String,  // ← новий
    notes: Option<String>,
    items: Vec<DocumentDraftItemDto>,
}
```

(Існуючі поля окрім `status_label` і порядку залишити як є — не перепаковувати.)

- [ ] **Step 4: Заповнити `status_label` у конструкторах DocumentSnapshot**

У `api.rs` всі місця де створюється DocumentSnapshot (рядки 337, 360, 383, 405, 415, та можливо інші — пошукати `DocumentSnapshot {`):

Для кожної гілки act/invoice/waybill додати після поля `status`:
- act: `status_label: act.status.label().to_string(),`
- invoice: `status_label: invoice.status.label().to_string(),`
- waybill: `status_label: waybill.status.label().to_string(),`

`status.label()` уже використовується в `documents_list` (рядки 881, 899, 917), доступний у моделях.

- [ ] **Step 5: Заповнити нові поля у `build_chain`**

У `src/tauri_api/documents/api.rs` функція `build_chain` (рядки 748–766). Знайти `.map(|(kind, document)| ChainStepDto { ... })` блок і змінити на:

```rust
.map(|(kind, document)| ChainStepDto {
    document_id: document
        .as_ref()
        .and_then(|item| extract_document_id_from_ref(&item.ref_id)),
    doc_type: kind.to_string(),
    doc_number: document
        .as_ref()
        .map(|item| item.number.clone())
        .unwrap_or_default(),
    amount_str: document
        .as_ref()
        .map(|item| format_money_ua(item.total_amount))
        .unwrap_or_default(),
    status: document
        .as_ref()
        .map(|item| item.status.clone())
        .unwrap_or_default(),
    status_label: document
        .as_ref()
        .map(|item| item.status_label.clone())
        .unwrap_or_default(),
    exists: document.is_some(),
})
```

`extract_document_id_from_ref` — нова приватна функція. Додати в кінець файлу:

```rust
fn extract_document_id_from_ref(ref_id: &str) -> Option<String> {
    // ref_id має формат "act-<uuid>" / "invoice-<uuid>" / "waybill-<uuid>"
    ref_id.split_once('-').map(|(_, id)| id.to_string())
}
```

(Перевірити форматування ref_id — у `document_ref_string` на рядку 338. Якщо там не `-`, а інший формат — адаптувати.)

- [ ] **Step 6: Запустити cargo build**

Run: `cargo build --lib`
Expected: success. Якщо є помилки про non-exhaustive struct constructor — додати `status_label` у пропущених DocumentSnapshot конструкторах.

- [ ] **Step 7: Запустити cargo build --tests**

Run: `cargo build --tests`
Expected: success.

- [ ] **Step 8: Запустити Rust тести (без DB)**

Run: `cargo test --lib`
Expected: ALL PASS.

- [ ] **Step 9: Commit**

```bash
git add src/tauri_api/documents/dto.rs src/tauri_api/documents/api.rs
git commit -m "feat(documents): chain step includes document_id and status_label"
```

---

## Task 8: A5 frontend types + fixtures

**Files:**
- Modify: `frontend/src/lib/types.ts:104-110`
- Modify: `frontend/src/lib/browser-fixtures.ts:209-236`
- Modify: `frontend/src/lib/stores/__tests__/documents-store.test.ts` (chain fixtures if any)
- Modify: `frontend/src/lib/stores/__tests__/shell-documents.test.ts` (chain fixtures if any)

- [ ] **Step 1: Оновити `ChainStepDto` у `types.ts`**

У `frontend/src/lib/types.ts:104-110` замінити:

```ts
export interface ChainStepDto {
  documentId: string | null;
  docType: string;
  docNumber: string;
  amountStr: string;
  status: string;
  statusLabel: string;
  exists: boolean;
}
```

- [ ] **Step 2: Запустити svelte-check — побачити всі call sites**

Run: `cd frontend && npm run check`
Expected: FAIL з конкретними помилками про відсутні поля у fixtures (browser-fixtures.ts, можливо тестах).

- [ ] **Step 3: Оновити browser-fixtures.ts**

У `frontend/src/lib/browser-fixtures.ts:212-234` оновити кожен step у `documentChain()`:

```ts
function documentChain(): DocumentChainDto {
  return {
    sourceId: "doc-1",
    steps: [
      {
        documentId: "doc-1",
        docType: "invoice",
        docNumber: "INV-2026-0042",
        amountStr: "48 200,00 грн",
        status: "issued",
        statusLabel: "Виставлено",
        exists: true
      },
      {
        documentId: "doc-2",
        docType: "act",
        docNumber: "ACT-2026-0018",
        amountStr: "19 400,00 грн",
        status: "draft",
        statusLabel: "Чернетка",
        exists: true
      },
      {
        documentId: null,
        docType: "waybill",
        docNumber: "Ще не створено",
        amountStr: "—",
        status: "",
        statusLabel: "",
        exists: false
      }
    ]
  };
}
```

- [ ] **Step 4: Шукати інші fixtures з ChainStep**

Run: `Grep -r "docType\|ChainStep" frontend/src/lib/stores/__tests__/`

Якщо знайдено — додати `documentId: "..."` і `statusLabel: "..."` у кожен step.

- [ ] **Step 5: Запустити svelte-check — verify clean**

Run: `cd frontend && npm run check`
Expected: clean.

- [ ] **Step 6: Запустити vitest повний**

Run: `cd frontend && npm run test:frontend`
Expected: ALL PASS.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/types.ts frontend/src/lib/browser-fixtures.ts frontend/src/lib/stores/__tests__/
git commit -m "feat(documents): mirror ChainStep document_id and statusLabel in TS"
```

---

## Task 9: A5 + B7 — Drawer header: two status chips + 3 action groups + button renames

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte` (script + header HTML + scoped style видалити direction-fieldset)
- Modify: `frontend/src/lib/config/documents.ts` (DOCUMENTS_COPY)
- Modify: `frontend/src/styles/documents.css` (action groups, next-status-chip, chain-stage-chip)
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`

- [ ] **Step 1: Додати ключі у `DOCUMENTS_COPY`**

У `frontend/src/lib/config/documents.ts:47-58`:

```ts
export const DOCUMENTS_COPY = {
  confirmDeleteCurrent: "Видалити поточний документ? Цю дію не можна скасувати.",
  confirmDeleteBulk: "Видалити вибрані документи? Цю дію не можна скасувати.",
  emptyTitle: "Поки що документів немає",
  emptyDescription:
    "Почніть зі створення першого рахунку, акта або накладної, щоб запустити повний сценарій документа.",
  emptyAction: "Створити перший документ",
  ...EDITOR_DIRTY_COPY,
  itemsEmptyTitle: "Поки що без позицій",
  itemsEmptyDescription:
    "Додайте першу позицію, щоб менеджер одразу бачив номенклатуру, кількість, ціну й підсумок документа.",
  chainMenuLabel: "Створити пов'язаний",
  generatePdfLabel: "Згенерувати PDF",
  nextStepLabel: "Наступний крок →"
} as const;
```

- [ ] **Step 2: Failing tests**

Додати у `DocumentsScreen.test.ts`:

```ts
it("drawer shows document-status chip", async () => {
  const editor = makeEditor({ id: "doc-1", kind: "invoice" });
  const chain = {
    sourceId: "doc-1",
    steps: [
      { documentId: "doc-1", docType: "invoice", docNumber: "INV-7", amountStr: "5 000,00 грн", status: "issued", statusLabel: "Виставлено", exists: true },
      { documentId: null, docType: "act", docNumber: "—", amountStr: "—", status: "", statusLabel: "", exists: false },
      { documentId: null, docType: "waybill", docNumber: "—", amountStr: "—", status: "", statusLabel: "", exists: false }
    ]
  };
  mocks.documentsState.set({ ...baseState(), editor, chain });
  const { container } = render(DocumentsScreen);
  await tick();

  const docStatus = container.querySelector('[data-testid="documents-drawer-document-status"]');
  expect(docStatus?.textContent?.trim()).toBe("Виставлено");

  // No second exists=true beyond self → chain chip should NOT show
  const chainChip = container.querySelector('[data-testid="documents-drawer-chain-status"]');
  expect(chainChip).toBeNull();
});

it("drawer shows chain-stage chip when related document exists", async () => {
  const editor = makeEditor({ id: "doc-1", kind: "invoice" });
  const chain = {
    sourceId: "doc-1",
    steps: [
      { documentId: "doc-1", docType: "invoice", docNumber: "INV-7", amountStr: "5 000,00 грн", status: "issued", statusLabel: "Виставлено", exists: true },
      { documentId: "doc-2", docType: "act", docNumber: "ACT-3", amountStr: "5 000,00 грн", status: "draft", statusLabel: "Чернетка", exists: true },
      { documentId: null, docType: "waybill", docNumber: "—", amountStr: "—", status: "", statusLabel: "", exists: false }
    ]
  };
  mocks.documentsState.set({ ...baseState(), editor, chain });
  const { container } = render(DocumentsScreen);
  await tick();

  const chainChip = container.querySelector('[data-testid="documents-drawer-chain-status"]');
  expect(chainChip).toBeTruthy();
  expect(chainChip?.textContent).toContain("Чернетка");
});
```

`makeEditor` helper якщо немає:
```ts
function makeEditor(opts: { id: string; kind: string }): DocumentEditorDto {
  return {
    form: {
      id: opts.id,
      kind: opts.kind,
      counterpartyId: "cp-1",
      counterpartyName: "ТОВ Ромашка",
      title: opts.kind === "invoice" ? "Рахунок INV-7" : "Документ",
      number: opts.id.toUpperCase(),
      date: "2026-04-30",
      notes: "",
      direction: "outgoing"
    },
    items: [],
    pdf: null,
    showTypePicker: false,
    showEditor: true
  };
}
```

- [ ] **Step 3: Verify tests fail**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts -t "drawer"`
Expected: FAIL.

- [ ] **Step 4: Додати helper-функції у `<script>` секцію DocumentsScreen.svelte**

Перед `function getCurrentChainStatus()` додати:

```ts
function getDocumentStatusLabel(): string {
  const id = $documents.editor?.form.id ?? "";
  const steps = $documents.chain?.steps ?? [];
  const own = steps.find((s) => s.documentId === id);
  return own?.statusLabel ?? "Чернетка";
}

function hasChainBeyondSelf(): boolean {
  const id = $documents.editor?.form.id ?? "";
  const steps = $documents.chain?.steps ?? [];
  return steps.some((s) => s.exists && s.documentId && s.documentId !== id);
}
```

Замінити існуючу `function getCurrentChainStatus()`:

```ts
function getCurrentChainStatus(): string {
  const steps = $documents.chain?.steps ?? [];
  const lastExists = [...steps].reverse().find((s) => s.exists);
  return lastExists?.statusLabel ?? "Чернетка";
}
```

(Раніше повертало raw status; тепер statusLabel, і фільтрує тільки exists.)

- [ ] **Step 5: Замінити drawer editor-header HTML**

У `frontend/src/lib/screens/DocumentsScreen.svelte:866-946` замінити блок `<div class="editor-header">` на:

```svelte
<div class="editor-header">
  <div>
    <div class="editor-header-meta">
      <span class="doc-kind-badge">
        <AppIcon name={getEditorKindIcon($documents.editor.form.kind)} size={14} />
        <span>{getDocumentKindLabel($documents.editor.form.kind)}</span>
      </span>
      <span class="doc-status-chip" data-testid="documents-drawer-document-status">
        {getDocumentStatusLabel()}
      </span>
      {#if hasChainBeyondSelf()}
        <span class="chain-stage-chip" data-testid="documents-drawer-chain-status">
          <AppIcon name="git-branch" size={12} />
          <span>Ланцюг: {getCurrentChainStatus()}</span>
        </span>
      {/if}
      <button
        class="next-status-chip"
        type="button"
        data-testid="documents-next-status"
        on:click={() => void documents.advanceStatus()}
        disabled={$documents.loading}
      >
        {DOCUMENTS_COPY.nextStepLabel}
      </button>
    </div>
    <h3 id="documents-drawer-title" tabindex="-1">{$documents.editor.form.title}</h3>
    <p>{$documents.editor.form.counterpartyName}</p>
  </div>

  <div class="editor-actions">
    <div class="editor-actions-group editor-actions-primary">
      <button
        class="btn-primary"
        on:click={() => documents.save()}
        disabled={$documents.loading}
        aria-busy={$documents.loading ? "true" : "false"}
      >
        Зберегти
      </button>
    </div>

    <div class="editor-actions-group editor-actions-secondary">
      <div class="chain-menu" class:chain-menu-open={chainMenuOpen}>
        <button
          bind:this={chainMenuButton}
          class="btn-secondary chain-menu-trigger"
          type="button"
          aria-haspopup="menu"
          aria-expanded={chainMenuOpen}
          on:click|stopPropagation={toggleChainMenu}
          disabled={$documents.loading}
        >
          <span>{DOCUMENTS_COPY.chainMenuLabel}</span>
          <span aria-hidden="true" class="chain-menu-caret">▾</span>
        </button>
        <div
          bind:this={chainMenuPopover}
          class="chain-menu-popover"
          role="menu"
          hidden={!chainMenuOpen}
        >
          {#each getDocumentChainTargets($documents.editor.form.kind) as targetKind}
            <button
              role="menuitem"
              type="button"
              class="chain-menu-item"
              data-testid={`documents-chain-create-${targetKind}`}
              on:click={() => onChainMenuCreateChain(targetKind)}
              disabled={$documents.loading}
            >
              <AppIcon name={documentKindMeta[targetKind].icon} size={16} />
              <span>Створити {documentKindMeta[targetKind].actionLabel}</span>
            </button>
          {/each}
        </div>
      </div>

      {#if supportsDocumentPdfGeneration($documents.editor.form.kind)}
        <button class="btn-ghost" on:click={() => documents.generatePdf()} disabled={$documents.loading}>
          <AppIcon name="file-text" size={14} />
          <span>{DOCUMENTS_COPY.generatePdfLabel}</span>
        </button>
      {/if}
    </div>

    <div class="editor-actions-group editor-actions-destructive">
      <button class="btn-danger" on:click={onDeleteCurrent} disabled={$documents.loading} data-testid="documents-delete-current-btn">
        Видалити
      </button>
      <button class="btn-ghost" on:click={requestCloseDrawer} disabled={$documents.loading}>
        Закрити
      </button>
    </div>
  </div>
</div>
```

- [ ] **Step 6: Видалити `onChainMenuAdvanceStatus`**

У скриптовій секції видалити функцію `onChainMenuAdvanceStatus` — більше не використовується. «Наступний статус» тепер chip, не пункт меню.

- [ ] **Step 7: Додати CSS правила**

У `frontend/src/styles/documents.css` додати/оновити:

```css
.editor-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}

.editor-actions-group {
  display: flex;
  gap: 8px;
  align-items: center;
}

.editor-actions-secondary {
  margin-left: auto;
}

.editor-actions-destructive {
  padding-left: 12px;
  margin-left: 4px;
  border-left: 1px solid var(--acta-color-border);
}

.next-status-chip {
  display: inline-flex;
  align-items: center;
  min-height: var(--acta-density-chip-h);
  padding: 0 10px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--acta-color-accent) 30%, var(--acta-color-border));
  background: var(--acta-color-bg-elevated);
  color: var(--acta-color-accent-text);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
}

.next-status-chip:hover:not(:disabled) {
  background: var(--acta-color-accent-soft);
}

.next-status-chip:disabled {
  cursor: default;
  opacity: 0.5;
}

.chain-stage-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: var(--acta-density-chip-h);
  padding: 0 10px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  background: var(--acta-color-bg-subtle);
  color: var(--acta-color-text-muted);
  border: 1px dashed var(--acta-color-border);
}
```

Видалити існуюче правило `.editor-actions { ... }` (поточне просто flex) — замінюється новою версією вище. Видалити `.editor-actions-close` (більше не використовується).

- [ ] **Step 8: Запустити тести**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts`
Expected: ALL PASS.

- [ ] **Step 9: Manual verify**

У Vite dev — відкрити документ. Drawer header: 3 групи кнопок (Зберегти зліва, secondary посередині з spacer, destructive справа з вертикальною межею). Header-meta показує doc-status + (опціонально) chain-stage + «Наступний крок →» як chip. Меню «Створити пов'язаний ▾» без пункту «Наступний статус».

- [ ] **Step 10: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte frontend/src/lib/config/documents.ts frontend/src/styles/documents.css frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
git commit -m "feat(documents): drawer header — two status chips, action groups, next-step chip"
```

---

## Task 10: B9 — Direction toggle inline

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte:971-995` (HTML) + scoped style cleanup
- Modify: `frontend/src/styles/documents.css` (нові .editor-direction-* правила)
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`

- [ ] **Step 1: Failing test**

Додати у `DocumentsScreen.test.ts`:

```ts
it("direction toggle reflects current value and switches", async () => {
  const editor = makeEditor({ id: "doc-1", kind: "invoice" });
  editor.form.direction = "outgoing";
  mocks.documentsState.set({ ...baseState(), editor });
  const { container } = render(DocumentsScreen);
  await tick();

  const outgoing = container.querySelector('[role="radio"][aria-checked="true"]');
  expect(outgoing?.textContent).toContain("Вихідний");

  const incomingBtn = Array.from(container.querySelectorAll('[role="radio"]'))
    .find((b) => b.textContent?.includes("Вхідний")) as HTMLButtonElement;
  incomingBtn.click();
  await tick();

  expect(mocks.updateFormField).toHaveBeenCalledWith("direction", "incoming");
});
```

- [ ] **Step 2: Verify fail**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts -t "direction toggle"`
Expected: FAIL (зараз radio inputs, не button[role=radio]).

- [ ] **Step 3: Замінити direction fieldset на inline toggle**

У `frontend/src/lib/screens/DocumentsScreen.svelte:971-995` замінити `<fieldset class="editor-direction-fieldset" …>` блок на:

```svelte
<div class="editor-direction-field">
  <span class="editor-direction-label">Напрямок</span>
  <div class="editor-direction-toggle" role="radiogroup" aria-label="Напрямок документа">
    {#each DOCUMENT_DIRECTION_OPTIONS as opt}
      <button
        role="radio"
        type="button"
        aria-checked={$documents.editor?.form.direction === opt.value}
        class="editor-direction-option"
        class:editor-direction-active={$documents.editor?.form.direction === opt.value}
        on:click={() => documents.updateFormField("direction", opt.value)}
        disabled={$documents.loading}
      >
        <span aria-hidden="true">{opt.value === "outgoing" ? "↑" : "↓"}</span>
        <span>{opt.label}</span>
      </button>
    {/each}
  </div>
</div>
```

- [ ] **Step 4: Видалити старі scoped стилі**

У `<style>` блоці DocumentsScreen.svelte видалити правила `.editor-direction-fieldset`, `.editor-direction-fieldset legend`, `.editor-direction-option` (старий inline-flex з radio).

- [ ] **Step 5: Додати CSS у documents.css**

У `frontend/src/styles/documents.css` додати:

```css
.editor-direction-field {
  display: grid;
  gap: 8px;
}

.editor-direction-label {
  font-size: 11px;
  color: var(--acta-color-text-muted);
}

.editor-direction-toggle {
  display: inline-flex;
  border: 1px solid var(--acta-color-border);
  border-radius: var(--acta-radius-lg);
  overflow: hidden;
  background: var(--acta-color-bg-elevated);
}

.editor-direction-option {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 0;
  background: transparent;
  color: var(--acta-color-text-muted);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.editor-direction-option + .editor-direction-option {
  border-left: 1px solid var(--acta-color-border);
}

.editor-direction-active {
  background: var(--acta-color-accent-soft);
  color: var(--acta-color-accent-text);
}

.editor-direction-option:disabled {
  cursor: default;
  opacity: 0.5;
}
```

- [ ] **Step 6: Запустити тести**

Run: `cd frontend && npx vitest run --config vitest.config.mjs DocumentsScreen.test.ts`
Expected: ALL PASS.

- [ ] **Step 7: Manual verify**

Відкрити документ у drawer. Direction toggle inline біля поля Дата (НЕ span на всю ширину). Клік на «Вхідний» перемикає aria-checked і змінює форму.

- [ ] **Step 8: Запустити повний test suite + check**

Run: `cd frontend && npm run check && npm run test:frontend`
Expected: clean.

Run: `cargo build --tests`
Expected: success.

- [ ] **Step 9: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte frontend/src/styles/documents.css frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
git commit -m "feat(documents): inline direction toggle replaces fieldset"
```

---

## Final verification

- [ ] **Step 1: Full check**

Run: `cd frontend && npm run check && npm run test:frontend`
Expected: clean.

Run: `cargo build --tests`
Expected: success.

- [ ] **Step 2: Manual end-to-end візуальна перевірка**

У `cd src-tauri && cargo tauri dev`:

1. **1440×900:** Документи список — без duplicate kind badge у рядках; active chips мають softer accent; bulk-bar з'являється тільки при виборі; drawer header — 3 групи кнопок.
2. **1024×800:** Чекбокс рядка inline (не орфан).
3. **720×600:** meta-чіпи стекаються вертикально; bulk-bar conditional працює.
4. **Date inputs:** показують `dd.mm.yyyy`.
5. **Split-button:** primary → створює default; caret → menu з 3 типами; вибір persist у localStorage.
6. **Drawer:** два status chips (документ + ланцюг) коли є related; один — коли немає. «Наступний крок →» як chip. «Створити пов'язаний ▾» без пункту «Наступний статус». Direction inline toggle.
7. **Dark mode:** усі нові chips і split-button-menu коректні.

- [ ] **Step 3: Acceptance criteria check**

Звірити з spec `docs/superpowers/specs/2026-05-13-documents-audit-fixes-design.md` Acceptance criteria — всі ✅?
