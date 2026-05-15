# UI Polish 3 Issues Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Реалізувати три затверджені UI polish fixes: спільні underline-таби, Tasks KPI/token cleanup, Counterparties flex-height без magic number.

**Architecture:** Зміна лише CSS/Svelte markup. Спільні стилі табів живуть у `frontend/src/styles.css`; screen-specific Tasks стилі живуть у `frontend/src/styles/tasks.css` і мають namespace від `.tasks-panel`, щоб не ламати `BankTabContent.svelte`; Counterparties висота йде через flex-chain `.screen-outlet -> .panel-fill -> .counterparties-layout`.

**Tech Stack:** Svelte, TypeScript, CSS tokens `--acta-*`, Vitest/Svelte checks.

---

## Spec Review Notes

Spec `docs/superpowers/specs/2026-05-13-ui-polish-3-issues-design.md` придатний до реалізації, але план фіксує три уточнення з рев'ю коду:

- `frontend/src/styles/documents.css` має responsive selector `.documents-nav-tabs`; його теж треба перевести на `.nav-tabs`.
- `.task-kpis` використовується не лише в `TasksScreen.svelte`, а й у `frontend/src/lib/components/BankTabContent.svelte`; новий Tasks strip треба писати як `.tasks-panel .task-kpis`.
- `.linked-row` shared з `CounterpartiesScreen.svelte`; Tasks-specific вигляд today panel треба писати як `.task-today-panel .linked-row`, `.task-today-panel .linked-row-title`, `.task-today-panel .linked-row-time`, а не глобально.

---

### Task 1: Додати глобальні underline tabs

**Files:**
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Додати flex-chain основу для screen outlet і panel fill**

У `frontend/src/styles.css` замінити блок `.screen-outlet` на:

```css
.screen-outlet {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
}
```

Після `.panel { margin: ...; padding: ...; }` додати:

```css
.panel-fill {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  flex: 1;
  min-height: 0;
}
```

- [ ] **Step 2: Додати глобальні nav tab класи**

Після `.panel-fill` додати:

```css
.nav-tabs {
  display: flex;
  gap: 2px;
  border-bottom: 1px solid var(--acta-color-border);
  padding: 0 16px;
}

.nav-tab {
  padding: 8px 16px;
  border: none;
  background: none;
  cursor: pointer;
  color: var(--acta-color-text-muted);
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
  font-weight: 500;
  transition: color var(--acta-motion-fast), border-color var(--acta-motion-fast);
}

.nav-tab:hover {
  color: var(--acta-color-text);
}

.nav-tab-active,
.nav-tab[aria-selected="true"] {
  color: var(--acta-color-accent);
  border-bottom-color: var(--acta-color-accent);
}
```

- [ ] **Step 3: Зберегти mobile behavior**

У mobile media query не змінювати `.screen-outlet { overflow: visible; }`; flex-direction може залишитись із базового блоку. Це зберігає поточну поведінку мобільного scroll.

---

### Task 2: Перевести Payments і Documents на глобальні nav tabs

**Files:**
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte`
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`
- Modify: `frontend/src/styles/documents.css`

- [ ] **Step 1: Оновити Payments markup**

У `PaymentsScreen.svelte` замінити tab row:

```svelte
<div class="nav-tabs payments-nav-tabs" role="tablist">
  <button
    class="nav-tab"
    class:nav-tab-active={activeTab === "bank"}
    role="tab"
    aria-selected={activeTab === "bank"}
    on:click={() => (activeTab = "bank")}
  >Банк</button>
  <button
    class="nav-tab"
    class:nav-tab-active={activeTab === "calendar"}
    role="tab"
    aria-selected={activeTab === "calendar"}
    on:click={() => (activeTab = "calendar")}
  >Платіжний календар</button>
</div>
```

- [ ] **Step 2: Прибрати scoped Payments tab CSS**

У `PaymentsScreen.svelte` видалити лише ці scoped selectors:

```css
.payments-tabs { ... }
.payments-tab { ... }
.payments-tab:hover { ... }
.payments-tab.active { ... }
.payments-tab.active::after { ... }
```

Не видаляти `.editor-sheet`, `@keyframes payments-drawer-in`, `.editor-header`, `.payment-editor-grid` та інші стилі editor drawer.

- [ ] **Step 3: Оновити Documents markup**

У `DocumentsScreen.svelte` замінити:

```svelte
<div class="documents-nav-tabs" role="tablist" aria-label="Напрямок документів">
```

на:

```svelte
<div class="nav-tabs" role="tablist" aria-label="Напрямок документів">
```

- [ ] **Step 4: Прибрати scoped Documents tab CSS**

У `DocumentsScreen.svelte` видалити scoped selectors:

```css
.documents-nav-tabs { ... }
.nav-tab { ... }
.nav-tab-active { ... }
```

Не чіпати `.documents-kind-chips`, `.kind-chip`, document rows, drawer styles.

- [ ] **Step 5: Оновити responsive selector у documents.css**

У `frontend/src/styles/documents.css` у `@media (max-width: 980px)` замінити:

```css
.documents-nav-tabs {
  overflow-x: auto;
  scrollbar-width: none;
}

.documents-nav-tabs::-webkit-scrollbar {
  display: none;
}
```

на:

```css
.nav-tabs {
  overflow-x: auto;
  scrollbar-width: none;
}

.nav-tabs::-webkit-scrollbar {
  display: none;
}
```

---

### Task 3: Перенести Tasks scoped style у tasks.css без глобальних колізій

**Files:**
- Modify: `frontend/src/lib/screens/TasksScreen.svelte`
- Modify: `frontend/src/styles/tasks.css`

- [ ] **Step 1: Перейменувати Tasks drawer class**

У `TasksScreen.svelte` замінити:

```svelte
<section class="editor-sheet" role="dialog" aria-modal="true">
```

на:

```svelte
<section class="tasks-editor-sheet" role="dialog" aria-modal="true">
```

- [ ] **Step 2: Видалити scoped style з TasksScreen**

У `TasksScreen.svelte` видалити весь блок від `<style>` до `</style>`. Після цього файл має містити лише script/markup.

- [ ] **Step 3: Переписати tasks.css з acta tokens і namespace**

У `frontend/src/styles/tasks.css` замінити поточний вміст на стилі з такими правилами:

```css
.tasks-panel {
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 20px 22px 36px;
}

.tasks-panel .task-kpis {
  display: flex;
  align-items: stretch;
  background: var(--acta-color-bg-elevated);
  border: 1px solid var(--acta-color-border);
  border-radius: 10px;
  overflow: hidden;
  margin-top: 0;
}

.tasks-panel .kpi-cell {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 16px 20px;
}

.tasks-panel .kpi-divider {
  width: 1px;
  background: var(--acta-color-border);
  margin: 12px 0;
  flex-shrink: 0;
}

.tasks-panel .kpi-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--acta-color-text-faint);
  text-transform: uppercase;
  letter-spacing: 1.1px;
}

.tasks-panel .kpi-value {
  font-size: 24px;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  line-height: 1.05;
  color: var(--acta-color-text);
  font-family: var(--acta-font-sans);
}

.tasks-panel .kpi-value.kpi-danger {
  color: var(--acta-color-danger);
}

.tasks-layout {
  display: grid;
  grid-template-columns: 1fr 300px;
  gap: 14px;
  align-items: start;
}

.tasks-main,
.tasks-side-panel {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.tasks-card {
  background: var(--acta-color-bg-elevated);
  border: 1px solid var(--acta-color-border);
  border-radius: 10px;
  overflow: hidden;
}

.tasks-card-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--acta-color-border);
}

.tasks-card-header h3 {
  margin: 0;
  font-size: 13.5px;
  font-weight: 500;
  color: var(--acta-color-text);
  white-space: nowrap;
}

.task-tabs {
  display: flex;
  padding: 2px;
  background: var(--acta-color-bg-subtle);
  border: 1px solid var(--acta-color-border);
  border-radius: 6px;
  margin-left: 4px;
}

.task-tabs button {
  padding: 3px 9px;
  background: transparent;
  color: var(--acta-color-text-muted);
  border: none;
  cursor: pointer;
  border-radius: 4px;
  font-size: 11.5px;
  font-weight: 400;
  white-space: nowrap;
}

.task-tabs button.active {
  background: var(--acta-color-bg-elevated);
  color: var(--acta-color-text);
  font-weight: 500;
  box-shadow: 0 0 0 1px var(--acta-color-border);
}

.tasks-panel .tasks-list {
  min-height: 80px;
}

.tasks-panel .task-row {
  display: flex;
  align-items: stretch;
  border-bottom: 1px solid var(--acta-color-border);
  min-height: 52px;
}

.tasks-panel .task-row:hover {
  background: var(--acta-color-bg-hover);
}

.tasks-panel .task-row:last-child {
  border-bottom: none;
}

.tasks-panel .task-row-done {
  opacity: 0.55;
}

.tasks-panel .task-priority-bar {
  width: 3px;
  flex-shrink: 0;
}

.tasks-panel .task-priority-danger {
  background: var(--acta-color-danger);
}

.tasks-panel .task-priority-warning {
  background: var(--acta-color-warning);
}

.tasks-panel .task-priority-none {
  background: transparent;
}

.tasks-panel .task-row-main {
  flex: 1;
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 11px 14px;
  background: transparent;
  border: none;
  cursor: pointer;
  text-align: left;
  color: inherit;
  min-width: 0;
  flex-direction: column;
}

.tasks-panel .task-row-content {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.tasks-panel .task-row-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--acta-color-text);
  display: block;
}

.tasks-panel .task-row-done .task-row-title {
  text-decoration: line-through;
  color: var(--acta-color-text-faint);
}

.tasks-panel .task-row-link {
  font-size: 11px;
  color: var(--acta-color-accent-text);
  font-family: var(--acta-font-mono);
}

.tasks-panel .task-row-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 3px;
}

.tasks-panel .task-meta-date {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11px;
  color: var(--acta-color-text-faint);
  font-family: var(--acta-font-mono);
}

.task-pill {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.8px;
  padding: 1px 6px;
  border-radius: 3px;
  background: var(--acta-color-bg-subtle);
  color: var(--acta-color-text-muted);
}

.task-pill-high,
.task-pill-critical {
  background: var(--acta-color-danger-soft);
  color: var(--acta-color-danger);
}

.tasks-panel .task-status-label {
  font-size: 11px;
  color: var(--acta-color-text-faint);
}

.tasks-panel .task-row-status {
  align-self: center;
  margin: 0 12px 0 8px;
  flex-shrink: 0;
  font-size: 11.5px;
  padding: 4px 10px;
  height: 26px;
  white-space: nowrap;
}

.tasks-empty {
  padding: 48px 20px;
  text-align: center;
  color: var(--acta-color-text-faint);
  font-size: 13px;
}

.tasks-message {
  padding: 10px 16px;
  color: var(--acta-color-success);
  font-size: 12px;
  background: var(--acta-color-success-soft);
  border-radius: 8px;
}

.tasks-error {
  padding: 10px 16px;
  color: var(--acta-color-danger);
  font-size: 12px;
  background: var(--acta-color-danger-soft);
  border-radius: 8px;
}

.task-today-panel {
  display: flex;
  flex-direction: column;
}

.tasks-panel .today-header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--acta-color-border);
}

.tasks-panel .today-header h3 {
  margin: 0 0 2px;
  font-size: 13.5px;
  font-weight: 500;
  color: var(--acta-color-text);
}

.tasks-panel .today-day {
  font-size: 11px;
  color: var(--acta-color-text-muted);
  font-family: var(--acta-font-mono);
}

.task-today-panel .linked-list {
  display: flex;
  flex-direction: column;
}

.task-today-panel .linked-row {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 10px 16px;
  background: transparent;
  border: none;
  border-bottom: 1px solid var(--acta-color-border);
  cursor: pointer;
  text-align: left;
}

.task-today-panel .linked-row:last-child {
  border-bottom: none;
}

.task-today-panel .linked-row-title {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--acta-color-text);
}

.task-today-panel .linked-row-time {
  font-size: 11px;
  color: var(--acta-color-text-muted);
  font-family: var(--acta-font-mono);
}

.empty-state-card.compact {
  padding: 24px 16px;
  text-align: center;
}

.empty-state-card.compact strong {
  display: block;
  font-size: 13px;
  color: var(--acta-color-text-muted);
  font-weight: 500;
  margin-bottom: 6px;
}

.empty-state-card.compact p {
  font-size: 12px;
  color: var(--acta-color-text-faint);
  line-height: 1.5;
  margin: 0;
}

.tasks-new-btn {
  margin-left: auto;
}

.tasks-kpi-skeleton {
  height: 64px;
  border-radius: 10px;
  grid-column: 1 / -1;
}

.tasks-skeleton-wrapper {
  padding: 12px 16px;
}

.tasks-editor-sheet {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  z-index: 201;
  width: 480px;
  max-width: 100vw;
  background: var(--acta-color-bg-elevated);
  border-left: 1px solid var(--acta-color-border);
  display: flex;
  flex-direction: column;
  box-shadow: -6px 0 28px rgba(0, 0, 0, 0.1);
}

.tasks-editor-sheet .editor-header {
  align-items: flex-start;
  padding: 18px 20px 14px;
  border-bottom: 1px solid var(--acta-color-border);
  flex-shrink: 0;
}

.tasks-editor-sheet .editor-link {
  margin: 0;
  font-size: 11.5px;
  color: var(--acta-color-accent-text);
  font-family: var(--acta-font-mono);
}

.tasks-editor-sheet .editor-link-none {
  color: var(--acta-color-text-faint);
  font-family: var(--acta-font-sans);
}

.tasks-editor-sheet .editor-close {
  width: 28px;
  height: 28px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.tasks-editor-sheet .editor-grid {
  flex: 1;
  overflow-y: auto;
  grid-template-columns: 1fr 1fr;
  padding: 20px;
  align-content: start;
}

.tasks-editor-sheet .editor-grid label {
  display: flex;
  flex-direction: column;
  gap: 5px;
  font-size: 11.5px;
  color: var(--acta-color-text-muted);
  font-weight: 500;
}

.tasks-editor-sheet .editor-grid input,
.tasks-editor-sheet .editor-grid select,
.tasks-editor-sheet .editor-grid textarea {
  background: var(--acta-color-bg-page);
  color: var(--acta-color-text);
  font-family: var(--acta-font-sans);
}

.tasks-editor-sheet .editor-grid textarea {
  min-height: 68px;
}

.tasks-editor-sheet .required {
  color: var(--acta-color-danger);
  font-weight: 400;
}

@media (max-width: 1100px) {
  .tasks-layout {
    grid-template-columns: minmax(0, 1fr);
  }

  .tasks-card-header {
    flex-wrap: wrap;
    align-items: flex-start;
  }

  .task-tabs {
    order: 3;
    margin-left: 0;
    max-width: 100%;
    overflow-x: auto;
  }

  .tasks-new-btn {
    margin-left: 0;
  }
}

@media (max-width: 760px) {
  .tasks-panel {
    padding: 16px 16px 28px;
  }

  .tasks-panel .task-kpis {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .tasks-panel .kpi-divider {
    display: none;
  }

  .tasks-panel .kpi-cell {
    min-width: 0;
    border-bottom: 1px solid var(--acta-color-border);
  }

  .tasks-panel .kpi-cell:nth-child(7),
  .tasks-panel .kpi-cell:last-child {
    border-bottom: none;
  }

  .tasks-panel .task-row {
    flex-wrap: wrap;
  }

  .tasks-panel .task-row-main {
    min-width: 0;
    width: calc(100% - 3px);
  }

  .tasks-panel .task-row-status {
    margin: 0 14px 12px auto;
  }

  .tasks-editor-sheet {
    width: 100%;
  }

  .tasks-editor-sheet .editor-header,
  .tasks-editor-sheet .editor-actions {
    flex-wrap: wrap;
  }

  .tasks-editor-sheet .editor-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
```

- [ ] **Step 4: Перевірити відсутність старих token aliases**

Run:

```bash
rg -n -- '--bg-|--border|--text|--accent|--danger|--success|--font|--radius' frontend/src/styles/tasks.css
```

Expected: no matches except false positives inside `--acta-*` names are not acceptable; command should return exit code 1/no output.

- [ ] **Step 5: Перевірити відсутність конфліктного global KPI**

Run:

```bash
rg -n '^\.task-kpis|^\.task-kpi-card' frontend/src/styles/tasks.css
```

Expected: no output. `.tasks-panel .task-kpis` is allowed.

---

### Task 4: Замінити Counterparties magic height на flex-chain

**Files:**
- Modify: `frontend/src/lib/screens/CounterpartiesScreen.svelte`
- Modify: `frontend/src/styles/counterparties.css`

- [ ] **Step 1: Додати panel-fill на root section**

У `CounterpartiesScreen.svelte` замінити:

```svelte
class="panel"
```

на:

```svelte
class="panel panel-fill"
```

- [ ] **Step 2: Замінити calc height на flex**

У `frontend/src/styles/counterparties.css` замінити `.counterparties-layout` на:

```css
.counterparties-layout {
  display: grid;
  grid-template-columns: 380px minmax(0, 1fr);
  gap: 16px;
  margin-top: 18px;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
```

- [ ] **Step 3: Перевірити, що magic number зник**

Run:

```bash
rg -n 'calc\(100vh - 160px\)' frontend/src/styles/counterparties.css
```

Expected: no output.

---

### Task 5: Verification

**Files:**
- Verify only.

- [ ] **Step 1: Static search for removed selectors**

Run:

```bash
rg -n 'payments-tabs|payments-tab|documents-nav-tabs|class:active=\{|class="editor-sheet" role="dialog" aria-modal="true"' frontend/src/lib/screens/PaymentsScreen.svelte frontend/src/lib/screens/DocumentsScreen.svelte frontend/src/lib/screens/TasksScreen.svelte frontend/src/styles/documents.css
```

Expected: no output for `payments-tabs`, `payments-tab`, `documents-nav-tabs`, `class:active`, or Tasks drawer `editor-sheet`. Existing `editor-sheet` in Payments/Documents/Counterparties is allowed outside this exact Tasks check.

- [ ] **Step 2: Frontend type/check command**

Run:

```bash
npm run check
```

Expected: command exits 0.

- [ ] **Step 3: Focused frontend tests**

Run:

```bash
npm run test:frontend
```

Expected: command exits 0.

- [ ] **Step 4: Visual smoke**

Start the app using the existing project dev flow, then inspect:

```bash
cd src-tauri && cargo tauri dev
```

Expected visual checks:
- Payments tabs render underline style and preserve Bank/Calendar switching.
- Documents top tabs render the same underline style and scroll horizontally on narrow widths.
- Tasks KPI strip has visible background, border, labels, values, and no token fallback failures.
- Tasks editor drawer still opens as a fixed right drawer.
- Counterparties fills available height without `calc(100vh - 160px)` and inner list/detail panels scroll correctly.

---

## Self-Review

- Spec coverage: Issue #3 covered by Tasks 1-2; Issue #4 covered by Task 3; Issue #5 covered by Task 4; verification covered by Task 5.
- Placeholder scan: no `TBD`, `TODO`, or open-ended "handle later" steps.
- Type consistency: CSS selectors match current Svelte markup after planned replacements; Tasks-specific selectors are namespaced to avoid `BankTabContent.svelte` and `CounterpartiesScreen.svelte` collisions.
