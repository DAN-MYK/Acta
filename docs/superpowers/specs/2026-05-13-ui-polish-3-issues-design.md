# UI Polish — issues #3, #4, #5

**Date:** 2026-05-13  
**Scope:** 3 targeted CSS/UI fixes — tab consistency, KPI token migration, magic height constant  
**Підхід:** B для всіх трьох  
**Status:** Approved, ready for implementation

---

## Issue #3 — Уніфікація стилю табів (Підхід B: глобальний клас)

### Принцип (прийнятий, Variant B)
- **Underline tabs** = screen-level навігація (Payments, Documents)
- **Pill/segmented tabs** = card-internal switcher (Tasks всередині картки)

### Рішення: глобальний `.nav-tab`

Перенести CSS underline-таба у `frontend/src/styles.css` як загальний клас:

```css
/* styles.css — додати після .panel */
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

.nav-tab:hover { color: var(--acta-color-text); }

.nav-tab-active,
.nav-tab[aria-selected="true"] {
  color: var(--acta-color-accent);
  border-bottom-color: var(--acta-color-accent);
}
```

**`PaymentsScreen.svelte`** — HTML та scoped style:
- HTML: `class="payments-tabs"` → `class="nav-tabs"` (додати `style="margin-top: 18px"` або окремий модифікатор)
- HTML: `class="payments-tab"` → `class="nav-tab"`
- HTML: `class:active={...}` → `class:nav-tab-active={...}`
- CSS: видалити `.payments-tabs`, `.payments-tab`, `.payments-tab.active`, `.payments-tab.active::after`

**`DocumentsScreen.svelte`** — HTML та scoped style:
- HTML: `class="documents-nav-tabs"` → **`class="nav-tabs"`** (контейнер таб-рядку)  
  _(buttons вже мають `class="nav-tab"` та `class:nav-tab-active` — ці не змінюються)_
- CSS: видалити scoped `.documents-nav-tabs`, `.nav-tab`, `.nav-tab-active` — вони тепер глобальні

**`TasksScreen.svelte`** (`.task-tabs` всередині картки) — без змін. Pill-стиль залишається.

---

## Issue #4 — Tasks KPI strip: повна міграція в tasks.css (Підхід B)

### Поточна проблема
Весь scoped `<style>` блок у `TasksScreen.svelte` (~560 рядків) написаний на старих токенах (`--bg-elevated`, `--border`, `--text-faint`, `--font-mono` тощо) яких немає в legacy aliases → KPI strip, картки завдань і текст — без кольору, фону, рамок.

`tasks.css` містить `.task-kpis` (grid) і `.task-kpi-card` — конфліктують зі scoped стилем, не використовуються.

### Рішення: видалити scoped `<style>`, перенести в tasks.css

**Крок 1 — Tasks editor drawer (критично)**

Scoped `.editor-sheet` у TasksScreen — це fixed-drawer стиль (`position: fixed; display: flex; flex-direction: column; width: 480px; ...`), який відрізняється від глобального `.editor-sheet` у `styles.css` (той лише card background/border/radius). Не можна просто видалити scoped блок і покластися на global.

**Рішення:** перейменувати drawer на `tasks-editor-sheet`:
- HTML: `<section class="editor-sheet" role="dialog" ...>` → `<section class="tasks-editor-sheet" role="dialog" ...>`
- `tasks.css`: додати `.tasks-editor-sheet { position: fixed; top: 0; right: 0; bottom: 0; z-index: 201; width: 480px; max-width: 100vw; background: var(--acta-color-bg-elevated); border-left: 1px solid var(--acta-color-border); display: flex; flex-direction: column; box-shadow: ...; }`

**Крок 2 — Видалити** весь `<style>` блок з `TasksScreen.svelte`.

**Крок 3 — Перевірити** які класи з scoped блоку вже є у `styles.css` глобально:
- `.editor-backdrop`, `.editor-header`, `.editor-actions`, `.editor-dirty-banner`, `.editor-dirty-actions`, `.editor-grid`, `.editor-grid-span` — вже в styles.css, не дублювати
- `.editor-sheet` (глобальна) залишається для інших екранів

**Крок 4 — Перенести** в `tasks.css` screen-специфічні класи (з `--acta-*` токенами):
- `.tasks-panel` (layout/padding)
- `.task-kpis` — flex-strip з border/radius і правильними токенами (замість поточної grid-версії)
- `.kpi-cell`, `.kpi-divider`, `.kpi-label`, `.kpi-value`, `.kpi-value.kpi-danger`, `.tasks-kpi-skeleton`
- `.tasks-card`, `.tasks-card-header`, `.tasks-new-btn`
- `.task-row`, `.task-row:hover`, `.task-row-done`, `.task-row-main`, `.task-row-content`, `.task-row-title`, `.task-row-meta`, `.task-row-status`
- `.task-priority-bar` варіанти
- `.task-meta-date`, `.task-pill` варіанти, `.task-status-label`
- `.tasks-empty`, `.tasks-message`, `.tasks-error`, `.tasks-skeleton-wrapper`
- `.today-header`, `.today-day`
- `.task-tabs button` styles, media queries

**Крок 5 — Namespace** у tasks.css для класів з generic назвами (щоб styles.css після import не перекривав):
- Класи типу `.task-row-main`, `.task-row-content`, `.task-row-title`, `.tasks-card-header`, `.tasks-empty`, `.today-header`, `.today-day` — додати `.tasks-panel` prefix: `.tasks-panel .task-row-main { ... }`
- **Виняток — не namespaceити:** `.task-pill` і `.linked-row` — ці класи спільно використовуються в `CounterpartiesScreen.svelte`, тому залишаються глобальними без `.tasks-panel` prefix

**Крок 6 — Видалити** з tasks.css конфліктні `.task-kpis` (grid version) і невикористаний `.task-kpi-card`.

### Таблиця замін токенів

| Старий токен | Новий токен |
|---|---|
| `var(--bg-elevated)` | `var(--acta-color-bg-elevated)` |
| `var(--bg-card)` | `var(--acta-color-bg-elevated)` |
| `var(--bg-subtle)` | `var(--acta-color-bg-subtle)` |
| `var(--bg-hover)` | `var(--acta-color-bg-hover)` |
| `var(--bg)` | Контекст-залежно: `.task-tabs` → `--acta-color-bg-subtle`; inputs → `--acta-color-bg-page` |
| `var(--border)` | `var(--acta-color-border)` |
| `var(--text)` | `var(--acta-color-text)` |
| `var(--text-muted)` | `var(--acta-color-text-muted)` |
| `var(--text-faint)` | `var(--acta-color-text-faint)` |
| `var(--accent)` | `var(--acta-color-accent)` |
| `var(--accent-text)` | `var(--acta-color-accent-text)` |
| `var(--danger)` | `var(--acta-color-danger)` |
| `var(--danger-soft)` | `var(--acta-color-danger-soft)` |
| `var(--success)` | `var(--acta-color-success)` |
| `var(--success-soft)` | `var(--acta-color-success-soft)` |
| `var(--font-mono)` | `var(--acta-font-mono)` |
| `var(--font-sans)` | `var(--acta-font-sans)` |

---

## Issue #5 — Counterparties: flexbox замість calc (Підхід B)

### Поточна проблема
`frontend/src/styles/counterparties.css` рядок 6:
```css
height: calc(100vh - 160px);  /* магічне число */
```

### Батьківський ланцюг (поточний стан)
```
.main           { display: flex; flex-direction: column; height: 100vh; overflow: hidden; }
  .topbar       { flex-shrink: 0; height: 56px; }
  .screen-outlet { flex: 1; overflow-y: auto; overflow-x: hidden; }  ← НЕ flex-контейнер
    .panel      { margin: 20px; padding: 20px; }
      .counterparties-layout { height: calc(100vh - 160px); }
```

### Рішення: flex-chain через screen-outlet

**Крок 1 — `styles.css`**: зробити `.screen-outlet` flex-колонкою:
```css
.screen-outlet {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;           /* ← додати */
  flex-direction: column;  /* ← додати */
}
```
Інші екрани (Documents, Tasks) не зламаються: їхній `.panel` без `.panel-fill` буде `height: auto` → природна висота → screen-outlet scrolls як і раніше.

**Крок 2 — `styles.css`**: додати модифікатор `.panel-fill`:
```css
.panel-fill {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  flex: 1;
  min-height: 0;
}
```

**Крок 3 — `CounterpartiesScreen.svelte`**: додати клас на root `<section>`:
```html
<section class="panel panel-fill" data-testid="counterparties-screen" ...>
```

**Крок 4 — `counterparties.css`**: замінити `height: calc(100vh - 160px)` на flex:
```css
.counterparties-layout {
  display: grid;
  grid-template-columns: 380px minmax(0, 1fr);
  gap: 16px;
  margin-top: 18px;
  flex: 1;        /* ← замість height: calc(...) */
  min-height: 0;
  overflow: hidden;
}
```

---

## Файли, що змінюються

| Файл | Зміна |
|---|---|
| `frontend/src/styles.css` | Додати `.nav-tabs`, `.nav-tab`, `.nav-tab-active`; додати `.panel-fill`; розширити `.screen-outlet` flex column |
| `frontend/src/lib/screens/PaymentsScreen.svelte` | HTML: nav-tabs/nav-tab класи; style: видалити payments-tab CSS |
| `frontend/src/lib/screens/DocumentsScreen.svelte` | HTML: `documents-nav-tabs` → `nav-tabs`; style: видалити scoped `.documents-nav-tabs`, `.nav-tab`, `.nav-tab-active` |
| `frontend/src/lib/screens/TasksScreen.svelte` | HTML: `editor-sheet` → `tasks-editor-sheet`; видалити весь `<style>` блок |
| `frontend/src/lib/screens/CounterpartiesScreen.svelte` | HTML: додати `.panel-fill` на root `<section>` |
| `frontend/src/styles/tasks.css` | Повна перезапис: видалити конфліктний `.task-kpis`/`.task-kpi-card`; додати всі перенесені стилі з `--acta-*` токенами; namespace generic класи |
| `frontend/src/styles/counterparties.css` | Замінити `height: calc(100vh - 160px)` на `flex: 1; min-height: 0; overflow: hidden` |

## Що НЕ змінюється
- Tasks HTML структура (`.kpi-cell`, `.kpi-divider`, `.task-row`, тощо) — без змін, крім drawer class
- Логіка Svelte stores, компонентів, TypeScript
- `.task-tabs` (pill) — залишається у tasks.css без namespace (card-internal)
- `.task-pill`, `.linked-row` — залишаються без `.tasks-panel` prefix (shared з Counterparties)
- `App.svelte` — не змінюється

## Тести
- Жодних нових тестів — суто CSS fixes
- Візуальна перевірка: Tasks KPI strip (border + background), Payments таб (underline), Counterparties layout (заповнює висоту)
- Перевірити dark mode (всі `--acta-*` токени підтримують обидві теми)
- Перевірити responsive breakpoints у TasksScreen (media queries переносяться в tasks.css)
- Перевірити Documents scrolling (screen-outlet з flex-column + overflow-y:auto має скролити як раніше)
