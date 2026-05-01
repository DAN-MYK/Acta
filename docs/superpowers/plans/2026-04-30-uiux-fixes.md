# UI/UX Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Усунути 16 дефектів UI/UX виявлених аудитом: артефакти розробки, відсутні CSS класи, відсутні hover/focus стани, UX проблеми редактора документів та платежів.

**Architecture:** Всі зміни — у frontend. Більшість у `styles.css` (глобальний CSS) та `.svelte` компонентах. Жодних змін бекенду, жодних нових файлів — тільки правки існуючих. Зміни незалежні між тасками, кожен таск можна комітити окремо.

**Tech Stack:** Svelte 4, TypeScript, CSS Custom Properties (tokens.css), Tauri 2 (для запуску dev-сервера)

---

## Файли, що змінюються

| Файл | Таски |
|---|---|
| `frontend/src/App.svelte` | 1 |
| `frontend/src/styles.css` | 2, 3, 4, 5, 6, 8, 9, 10 |
| `frontend/src/lib/screens/DocumentsScreen.svelte` | 6, 7, 8 |
| `frontend/src/lib/screens/DashboardScreen.svelte` | 6, 9 |
| `frontend/src/lib/screens/PaymentsScreen.svelte` | 5, 6 |

## Перевірка після кожного таску

```bash
# TypeScript + Svelte check (запускати з frontend/)
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
# Очікувано: "0 errors, 0 warnings"

# Store unit tests (не залежать від UI-змін, але перевіряємо регресії)
npx vitest run 2>&1 | tail -10
```

---

### Task 1: Прибрати артефакт "Tauri migration scaffold" з бренду

**Files:**
- Modify: `frontend/src/App.svelte:149`

- [x] **Step 1: Прибрати placeholder з бренду**

У `frontend/src/App.svelte` знайти рядок 149 і замінити:

```svelte
<!-- Було: -->
<p>Tauri migration scaffold</p>

<!-- Стало: -->
<p>Управлінський облік</p>
```

- [x] **Step 2: Запустити svelte-check**

```bash
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
```
Очікувано: `0 errors, 0 warnings`

- [x] **Step 3: Commit**

```bash
git add frontend/src/App.svelte
git commit -m "fix(ui): remove 'Tauri migration scaffold' placeholder from brand"
```

---

### Task 2: Palette — додати z-index

**Files:**
- Modify: `frontend/src/styles.css:849-863`

**Проблема:** `position: fixed` без `z-index`. Sidebar і panel мають `backdrop-filter` що створює stacking context. Без явного z-index palette може потрапити під інші елементи.

- [x] **Step 1: Додати z-index до backdrop і palette**

У `frontend/src/styles.css` знайти `.palette-backdrop` (~рядок 849) і додати `z-index`:

```css
.palette-backdrop {
  position: fixed;
  inset: 0;
  background: var(--bg-overlay);
  border: 0;
  z-index: 40;
}

.palette {
  position: fixed;
  top: 88px;
  left: 50%;
  transform: translateX(-50%);
  width: min(760px, calc(100vw - 32px));
  padding: 16px;
  z-index: 50;
}
```

- [x] **Step 2: svelte-check**

```bash
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
```

- [x] **Step 3: Commit**

```bash
git add frontend/src/styles.css
git commit -m "fix(ui): add z-index to command palette to prevent stacking issues"
```

---

### Task 3: Визначити відсутні CSS класи

**Files:**
- Modify: `frontend/src/styles.css`

**Проблема:** 7 класів використовуються в шаблонах але відсутні у styles.css. Відповідний HTML рендериться без стилів.

- [x] **Step 1: Додати всі 7 відсутніх класів у кінець styles.css** (перед `@media`)

```css
/* --- Відсутні класи (аудит 2026-04-30) --- */

.doc-row-title {
  display: flex;
  align-items: center;
  gap: 6px;
}

.chain-doc-title {
  display: flex;
  align-items: center;
  gap: 6px;
}

.doc-kind-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: var(--radius-md);
  background: var(--accent-soft);
  color: var(--accent-text);
  font-size: var(--font-sm);
}

.create-doc-button {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}

.chain-action-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 0;
  background: var(--bg-card);
  border-radius: var(--radius-xl);
  padding: 8px 12px;
  cursor: pointer;
  color: inherit;
  font: inherit;
}

.chain-action-button:hover {
  background: var(--bg-hover);
}

.dashboard-list-empty {
  padding: var(--space-4) 0;
  color: var(--text-muted);
  font-size: var(--font-sm);
}

.overdue strong,
.overdue .dashboard-list-row {
  color: var(--danger);
}
```

- [x] **Step 2: svelte-check**

```bash
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
```

- [x] **Step 3: Commit**

```bash
git add frontend/src/styles.css
git commit -m "fix(ui): add 7 missing CSS classes referenced in templates"
```

---

### Task 4: Hover-стани та focus-visible для клавіатурної навігації

**Files:**
- Modify: `frontend/src/styles.css`

**Проблема:** Grep на `:hover` і `:focus-visible` — нуль результатів. Кнопки не дають жодного feedback при hover. Focus ring відсутній — accessibility порушена попри заявлені Ctrl+1-7 shortcuts.

- [x] **Step 1: Додати hover-стани до всіх інтерактивних елементів**

Додати у `frontend/src/styles.css` одразу після нових класів з Task 3:

```css
/* --- Hover states --- */

.nav button:hover:not(.active) {
  background: var(--bg-hover);
}

.theme-switcher button:hover {
  background: var(--bg-hover);
}

.topbar-actions button:hover {
  background: var(--bg-hover);
}

.doc-row:hover {
  background: var(--bg-elevated);
}

.counterparty-row:hover:not(.active) {
  background: var(--bg-hover);
}

.linked-row:hover:not(.static) {
  background: var(--bg-hover);
}

.dashboard-list-row:hover {
  background: var(--bg-hover);
}

.task-row-main:hover {
  background: var(--bg-hover);
}

.settings-nav button:hover:not(.active) {
  background: var(--bg-hover);
}

.task-tabs button:hover:not(.active) {
  background: var(--bg-hover);
}

.settings-actions-row button:hover:not(.active) {
  background: var(--bg-hover);
}

.settings-row-actions button:hover {
  background: var(--bg-hover);
}

/* --- Focus-visible (keyboard navigation) --- */

button:focus-visible,
input:focus-visible,
select:focus-visible,
textarea:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
```

- [x] **Step 2: svelte-check**

```bash
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
```

- [x] **Step 3: Commit**

```bash
git add frontend/src/styles.css
git commit -m "fix(ui): add hover states and focus-visible ring for keyboard navigation"
```

---

### Task 5: task-pill як справжній бейдж + "Net" → "Баланс"

**Files:**
- Modify: `frontend/src/styles.css:322-324`
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte:39`

- [x] **Step 1: Виправити `.task-pill` у styles.css**

Знайти рядок ~322:
```css
/* Було: */
.task-pill {
  color: var(--accent-text);
}

/* Стало: */
.task-pill {
  display: inline-block;
  color: var(--accent-text);
  background: var(--accent-soft);
  border-radius: var(--radius-md);
  padding: 3px 8px;
  font-size: var(--font-sm);
}
```

- [x] **Step 2: Замінити "Net" на "Баланс" у PaymentsScreen.svelte**

У `frontend/src/lib/screens/PaymentsScreen.svelte` рядок ~39:
```svelte
<!-- Було: -->
<span>Net</span>

<!-- Стало: -->
<span>Баланс</span>
```

- [x] **Step 3: svelte-check**

```bash
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
```

- [x] **Step 4: Commit**

```bash
git add frontend/src/styles.css frontend/src/lib/screens/PaymentsScreen.svelte
git commit -m "fix(ui): style task-pill as actual badge; translate 'Net' to Ukrainian"
```

---

### Task 6: Ієрархія кнопок — primary, ghost, danger

**Files:**
- Modify: `frontend/src/styles.css`
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte:197-203`
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte:89-92`
- Modify: `frontend/src/lib/screens/DashboardScreen.svelte:36`

**Проблема:** Зберегти / Видалити / Наступний статус / Закрити — всі однаково виглядають. Ієрархія важливості невидима.

- [x] **Step 1: Додати `.btn-primary`, `.btn-ghost`, `.btn-danger` у styles.css**

Додати після hover-states з Task 4:

```css
/* --- Button hierarchy --- */

.btn-primary {
  border: 0;
  background: var(--accent);
  color: var(--text-on-accent);
  border-radius: var(--radius-xl);
  padding: var(--space-3) 14px;
  cursor: pointer;
  font: inherit;
}

.btn-primary:hover {
  background: var(--accent-hover);
}

.btn-primary:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.btn-ghost {
  border: 0;
  background: var(--bg-card);
  color: inherit;
  border-radius: var(--radius-xl);
  padding: var(--space-3) 14px;
  cursor: pointer;
  font: inherit;
}

.btn-ghost:hover {
  background: var(--bg-hover);
}

.btn-danger {
  border: 0;
  background: transparent;
  color: var(--danger);
  border-radius: var(--radius-xl);
  padding: var(--space-3) 14px;
  cursor: pointer;
  font: inherit;
}

.btn-danger:hover {
  background: var(--danger-soft);
}
```

- [x] **Step 2: Застосувати класи у DocumentsScreen.svelte**

У `frontend/src/lib/screens/DocumentsScreen.svelte` знайти `editor-actions` (~рядок 197):

```svelte
<!-- Було: -->
<div class="editor-actions">
  <button on:click={() => documents.addItem()}>Додати позицію</button>
  <button on:click={() => documents.save()}>Зберегти</button>
  <button on:click={() => documents.advanceStatus()}>Наступний статус</button>
  <button class="ghost-danger" on:click={onDeleteCurrent}>Видалити</button>
  <button on:click={() => documents.closeEditor()}>Закрити</button>
</div>

<!-- Стало: -->
<div class="editor-actions">
  <button class="btn-ghost" on:click={() => documents.addItem()}>Додати позицію</button>
  <button class="btn-primary" on:click={() => documents.save()}>Зберегти</button>
  <button class="btn-ghost" on:click={() => documents.advanceStatus()}>Наступний статус</button>
  <button class="btn-danger" on:click={onDeleteCurrent}>Видалити</button>
  <button class="btn-ghost" on:click={() => documents.closeEditor()}>Закрити</button>
</div>
```

- [x] **Step 3: Застосувати класи у PaymentsScreen.svelte**

У `frontend/src/lib/screens/PaymentsScreen.svelte` знайти editor-actions (~рядок 89):

```svelte
<!-- Було: -->
<div class="editor-actions">
  <button on:click={() => payments.save()}>Зберегти</button>
  <button on:click={() => payments.closeEditor()}>Закрити</button>
</div>

<!-- Стало: -->
<div class="editor-actions">
  <button class="btn-primary" on:click={() => payments.save()}>Зберегти</button>
  <button class="btn-ghost" on:click={() => payments.closeEditor()}>Закрити</button>
</div>
```

- [x] **Step 4: Виправити кнопку "Оновити" на DashboardScreen.svelte**

У `frontend/src/lib/screens/DashboardScreen.svelte` рядок ~36:

```svelte
<!-- Було: -->
<button on:click={() => dashboard.load()} disabled={$dashboard.loading}>
  {$dashboard.loading ? "Оновлення..." : "Оновити"}
</button>

<!-- Стало: -->
<button class="btn-ghost" on:click={() => dashboard.load()} disabled={$dashboard.loading}>
  {$dashboard.loading ? "Оновлення..." : "Оновити"}
</button>
```

- [x] **Step 5: svelte-check**

```bash
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
```
Очікувано: `0 errors, 0 warnings`

- [x] **Step 6: Commit**

```bash
git add frontend/src/styles.css \
        frontend/src/lib/screens/DocumentsScreen.svelte \
        frontend/src/lib/screens/PaymentsScreen.svelte \
        frontend/src/lib/screens/DashboardScreen.svelte
git commit -m "feat(ui): add btn-primary/ghost/danger hierarchy; apply to editor actions"
```

---

### Task 7: Замінити UUID-інпут на select контрагента

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`

**Проблема:** Рядок 142 показує поле `placeholder="UUID контрагента для нового документа"` — кінцевий користувач не знає UUID. Counterparties вже завантажені в `$counterparties.screen?.items`.

- [x] **Step 1: Додати імпорт counterpartiesStore у DocumentsScreen.svelte**

У блоці `<script>` знайти існуючі імпорти і додати:

```svelte
import { counterpartiesStore } from "../stores/counterparties";

// у тілі скрипту (після const documents = documentsStore):
const counterparties = counterpartiesStore;
```

- [x] **Step 2: Замінити text input на select у create-strip**

Знайти рядок ~141-143:

```svelte
<!-- Було: -->
<div class="create-strip">
  <input bind:value={createCounterpartyId} placeholder="UUID контрагента для нового документа" />
  <select bind:value={createKind}>

<!-- Стало: -->
<div class="create-strip">
  <select bind:value={createCounterpartyId}>
    <option value="">— Оберіть контрагента —</option>
    {#each $counterparties.screen?.items ?? [] as cp}
      <option value={cp.id}>{cp.name}</option>
    {/each}
  </select>
  <select bind:value={createKind}>
```

- [x] **Step 3: Прибрати hint-рядок нижче create-strip**

Знайти рядок ~154:

```svelte
<!-- Видалити повністю: -->
{#if $documents.draftContext?.counterpartyName}
  <p class="hint">Поточний create context: {$documents.draftContext.counterpartyName}</p>
{/if}
```

- [x] **Step 4: svelte-check**

```bash
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
```
Очікувано: `0 errors, 0 warnings`

- [x] **Step 5: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte
git commit -m "fix(ui): replace UUID text input with counterparty select in document create strip"
```

---

### Task 8: Заголовки колонок у рядках позицій документа

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte:258-270`
- Modify: `frontend/src/styles.css`

**Проблема:** `editor-item` grid (Опис / Од. / Кількість / Ціна / [кнопка]) не має лейблів. При масовому введенні стовпці легко переплутати.

- [x] **Step 1: Додати `.editor-item-head` у styles.css**

```css
.editor-item-head {
  background: transparent;
  font-size: var(--font-sm);
  color: var(--text-muted);
  padding-top: 0;
  padding-bottom: 0;
  pointer-events: none;
}
```

- [x] **Step 2: Додати header-рядок перед списком позицій у DocumentsScreen.svelte**

Знайти `<div class="editor-items">` (~рядок 258) і замінити:

```svelte
<!-- Було: -->
<div class="editor-items">
  {#each $documents.editor.items as item, index}

<!-- Стало: -->
<div class="editor-items">
  {#if $documents.editor.items.length > 0}
    <div class="editor-item editor-item-head">
      <span>Опис</span>
      <span>Од.</span>
      <span>Кількість</span>
      <span>Ціна, грн</span>
      <span></span>
    </div>
  {/if}
  {#each $documents.editor.items as item, index}
```

- [x] **Step 3: svelte-check**

```bash
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
```

- [x] **Step 4: Commit**

```bash
git add frontend/src/styles.css frontend/src/lib/screens/DocumentsScreen.svelte
git commit -m "feat(ui): add column headers to document line items editor"
```

---

### Task 9: Cashflow — grid-розмітка замість змішаного flex

**Files:**
- Modify: `frontend/src/styles.css:779-800`
- Modify: `frontend/src/lib/screens/DashboardScreen.svelte:61-75`

**Проблема:** "Грошовий потік" задекларований як chart але виглядає як неструктурований список. Cashflow-bars показує лише кольоровий текст. Мінімальне виправлення — зробити 4-колонковий grid з лейблами.

- [x] **Step 1: Оновити `.cashflow-row` у styles.css**

Знайти `cashflow-row` (~рядок 779) і замінити весь блок:

```css
.cashflow-list {
  display: grid;
  gap: 0;
  margin-top: 16px;
}

.cashflow-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) repeat(3, 110px);
  gap: 0 16px;
  align-items: center;
  padding: 10px 0;
  border-top: 1px solid var(--border-hairline);
}

.cashflow-row.cashflow-head {
  border-top: 0;
  font-size: var(--font-sm);
  color: var(--text-muted);
  padding-bottom: 6px;
}

.cashflow-row span {
  text-align: right;
}

.cashflow-row strong {
  text-align: left;
}

.cashflow-income {
  color: var(--success);
  text-align: right;
}

.cashflow-expense {
  color: var(--warning);
  text-align: right;
}

.cashflow-net {
  font-weight: 600;
  text-align: right;
}
```

- [x] **Step 2: Оновити розмітку cashflow у DashboardScreen.svelte**

Знайти `<div class="cashflow-list">` (~рядок 61) і замінити весь блок:

```svelte
<div class="cashflow-list">
  <div class="cashflow-row cashflow-head">
    <span style="text-align:left">Місяць</span>
    <span>Нетто</span>
    <span>Надходження</span>
    <span>Витрати</span>
  </div>
  {#each $dashboard.screen?.cashflowRows ?? [] as row}
    <div class="cashflow-row">
      <strong>{row.label}</strong>
      <span class="cashflow-net">{row.netStr}</span>
      <span class="cashflow-income">{row.incomeStr}</span>
      <span class="cashflow-expense">{row.expenseStr}</span>
    </div>
  {/each}
</div>
```

- [x] **Step 3: svelte-check**

```bash
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
```

- [x] **Step 4: Commit**

```bash
git add frontend/src/styles.css frontend/src/lib/screens/DashboardScreen.svelte
git commit -m "fix(ui): restructure cashflow section as aligned 4-column grid with headers"
```

---

### Task 10: Прибрати дублі dark-mode overrides у styles.css

**Files:**
- Modify: `frontend/src/styles.css`

**Проблема:** `styles.css` містить ~20 блоків виду `body[data-theme="dark"] .X { background: var(--bg-card); color: inherit; }`. Всі вони — NO-OPs, бо `--bg-card` вже перевизначений у `body[data-theme="dark"]` в `tokens.css`, а `color: inherit` і так є дефолтною поведінкою. Вони лише збільшують CSS specificity і ускладнюють підтримку.

**ВАЖЛИВО:** Зберегти тільки два блоки dark-mode в `styles.css`:
1. `body[data-theme="dark"] { color: var(--text); background: var(--page-bg); }` — потрібний (body не успадковує від :root де встановлено color/background)
2. `body[data-theme="dark"] .reports-table-head { background: var(--accent-soft); }` — потрібний (light mode `.reports-table-head` має `background: var(--accent-soft)`, dark mode `--accent-soft` = `#2f4063`, відрізняється — OK)
3. `body[data-theme="dark"] .nav button.active { ... }` — НЕ існує, active стан вже через `--accent` який overridden.

- [x] **Step 1: Видалити NO-OP dark mode блоки**

Видалити всі блоки що відповідають паттерну `body[data-theme="dark"] .CLASS { background: var(--bg-*); color: inherit; }`:

```css
/* Видалити ці блоки (вони NO-OP через token overrides): */

body[data-theme="dark"] .nav button,
body[data-theme="dark"] .theme-switcher button,
body[data-theme="dark"] .topbar-actions button,
body[data-theme="dark"] .doc-row,
body[data-theme="dark"] .palette-item { ... }          /* рядки ~79-86 */

body[data-theme="dark"] .topbar,
body[data-theme="dark"] .panel,
body[data-theme="dark"] .editor-sheet,
body[data-theme="dark"] .palette { ... }               /* рядки ~111-116 */

body[data-theme="dark"] .topbar-actions select,
body[data-theme="dark"] .panel-header input,
body[data-theme="dark"] .editor-grid input,
body[data-theme="dark"] .palette input { ... }         /* рядки ~156-163 */

body[data-theme="dark"] .task-kpi-card { ... }
body[data-theme="dark"] .task-tabs button,
body[data-theme="dark"] .task-row button:last-child { ... }
body[data-theme="dark"] .task-row-main { ... }
body[data-theme="dark"] .reports-filter-grid select,
body[data-theme="dark"] .reports-filter-grid input { ... }
body[data-theme="dark"] .settings-nav button, ...{ ... }
body[data-theme="dark"] .settings-card { ... }
body[data-theme="dark"] .settings-actions-row select { ... }
body[data-theme="dark"] .settings-row { ... }
body[data-theme="dark"] .counterparty-row,
body[data-theme="dark"] .linked-row { ... }
body[data-theme="dark"] .chain-panel { ... }
body[data-theme="dark"] .chain-step { ... }
body[data-theme="dark"] .dashboard-kpi-card, ...
body[data-theme="dark"] .dashboard-list-row { ... }
body[data-theme="dark"] .editor-item { ... }
body[data-theme="dark"] .create-strip input, ...{ ... }
```

**Залишити тільки:**
```css
body[data-theme="dark"] {
  color: var(--text);
  background: var(--page-bg);
}

body[data-theme="dark"] .reports-table-head {
  background: var(--accent-soft);
}
```

- [x] **Step 2: svelte-check**

```bash
cd C:/Users/MykhailoDan/apps/acta && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
```
Очікувано: `0 errors, 0 warnings`

- [x] **Step 3: Перевірити dark mode вручну**

```bash
# Запустити dev-сервер, переключити на dark theme, пройтись по всіх 7 екранах
cd src-tauri && cargo tauri dev
```
Перевірити: чи всі елементи темні (фони, текст), чи немає білих/сірих flash елементів.

- [x] **Step 4: Commit**

```bash
git add frontend/src/styles.css
git commit -m "refactor(css): remove ~20 redundant dark-mode overrides, token cascade handles them"
```

---

## Виконання

**План збережено до `docs/superpowers/plans/2026-04-30-uiux-fixes.md`.**

**Два варіанти виконання:**

1. **Subagent-Driven (рекомендовано)** — свіжий підагент на кожен таск з review між тасками. Skill: `superpowers:subagent-driven-development`

2. **Inline Execution** — виконати таски в цій сесії покроково. Skill: `superpowers:executing-plans`

**Який підхід?**
