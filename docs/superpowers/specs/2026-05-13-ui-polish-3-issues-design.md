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

**`PaymentsScreen.svelte`** — змінити HTML та стилі:
- Замінити `class="payments-tabs"` → `class="nav-tabs"` + додати `margin-top: 18px` як модифікатор або inline
- Замінити `class="payments-tab"` → `class="nav-tab"`
- Замінити `class:active={}` → `class:nav-tab-active={}`
- Видалити весь CSS для `.payments-tabs`, `.payments-tab`, `.payments-tab.active`, `.payments-tab.active::after` зі scoped `<style>`

**`DocumentsScreen.svelte`** — HTML залишається (`nav-tab`, `nav-tab-active`), видалити scoped CSS для `.documents-nav-tabs`, `.nav-tab`, `.nav-tab-active` — вони тепер у global.

**`TasksScreen.svelte`** (`.task-tabs` всередині картки) — без змін. Pill-стиль залишається scoped.

---

## Issue #4 — Tasks KPI strip: повна міграція в tasks.css (Підхід B)

### Поточна проблема
Весь scoped `<style>` блок у `TasksScreen.svelte` (~560 рядків) написаний на старих токенах (`--bg-elevated`, `--border`, `--text-faint`, `--font-mono` тощо) яких немає в legacy aliases → KPI strip, картки завдань і текст — без кольору, фону, рамок.

`tasks.css` містить `.task-kpis` (grid) і `.task-kpi-card` — конфліктують зі scoped стилем, не використовуються.

### Рішення: видалити scoped `<style>`, перенести в tasks.css

**Кроки:**

1. **Видалити** весь `<style>` блок з `TasksScreen.svelte` — повністю.

2. **Перевірити** які класи з scoped блоку вже є у `styles.css` глобально:
   - `.editor-sheet`, `.editor-header`, `.editor-actions`, `.editor-dirty-banner`, `.editor-dirty-actions`, `.editor-grid`, `.editor-grid-span` — вже в styles.css, не дублювати
   - `.editor-backdrop` — вже в styles.css

3. **Перенести** в `tasks.css` тільки screen-специфічні класи (з `--acta-*` токенами):
   - `.tasks-panel` (layout/padding)
   - `.task-kpis` — уніфікувати: замість flex-strip переробити на той самий дизайн (flex + border + radius) але з правильними токенами
   - `.kpi-cell`, `.kpi-divider`, `.kpi-label`, `.kpi-value`, `.kpi-value.kpi-danger`, `.tasks-kpi-skeleton`
   - `.tasks-card` (background + border + radius)
   - `.tasks-card-header`
   - `.task-row`, `.task-row:hover`, `.task-row-done`, `.task-row-main`, `.task-row-content`, `.task-row-title`, `.task-row-meta`, `.task-row-link`, `.task-row-status`
   - `.task-priority-bar` варіанти
   - `.task-meta-date`, `.task-pill` варіанти, `.task-status-label`
   - `.tasks-empty`, `.tasks-message`, `.tasks-error`
   - `.today-header`, `.today-day`, `.linked-row`, `.linked-row-title`, `.linked-row-time`
   - `.tasks-new-btn`, `.tasks-skeleton-wrapper`
   - `.task-tabs button` (pill style) і media queries

4. **Видалити** з tasks.css конфліктне `.task-kpis` (grid version) і невикористаний `.task-kpi-card`.

### Таблиця замін токенів

| Старий токен | Новий токен |
|---|---|
| `var(--bg-elevated)` | `var(--acta-color-bg-elevated)` |
| `var(--bg-card)` | `var(--acta-color-bg-elevated)` |
| `var(--bg-subtle)` | `var(--acta-color-bg-subtle)` |
| `var(--bg-hover)` | `var(--acta-color-bg-hover)` |
| `var(--bg)` | Залежить від контексту: `.task-tabs` → `--acta-color-bg-subtle`; inputs → `--acta-color-bg-page` |
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

### Рішення: flex-chain

Замість viewport-relative height — зробити layout природньо розтягуватись через flex.

**Крок 1** — `CounterpartiesScreen.svelte`: додати клас `.panel-fill` до кореневого `<section>`:
```html
<section class="panel panel-fill" data-testid="counterparties-screen" ...>
```

**Крок 2** — `frontend/src/styles.css`: додати `.panel-fill` модифікатор:
```css
.panel-fill {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  /* 
   Щоб flex-chain працював, батьківський контейнер (main column)
   повинен мати визначену висоту. Якщо main column вже є grid-item 
   в app-shell{height:100vh}, це автоматично виконується.
   Перевірити при реалізації.
  */
}
```

**Крок 3** — `frontend/src/styles/counterparties.css`: видалити `height: calc(100vh - 160px)`, замінити на flex:
```css
.counterparties-layout {
  display: grid;
  grid-template-columns: 380px minmax(0, 1fr);
  gap: 16px;
  margin-top: 18px;
  flex: 1;           /* заповнює .panel-fill */
  min-height: 0;     /* дозволяє flex-item стискатись */
  overflow: hidden;  /* клипає content всередині layout */
}
```

**Примітка для реалізації**: якщо `flex: 1` не дає очікуваного результату (layout не заповнює панель), потрібно перевірити чи main column у `app-shell` є flex-контейнером. Якщо ні — або зробити main column flex-колонкою, або повернутись до Підходу A з token-based calc.

---

## Файли, що змінюються

| Файл | Зміна |
|---|---|
| `frontend/src/styles.css` | Додати `.nav-tabs`, `.nav-tab`, `.nav-tab-active`; додати `.panel-fill` |
| `frontend/src/lib/screens/PaymentsScreen.svelte` | HTML: nav-tabs/nav-tab класи; style: видалити payments-tab CSS |
| `frontend/src/lib/screens/DocumentsScreen.svelte` | Style: видалити scoped `.documents-nav-tabs`, `.nav-tab`, `.nav-tab-active` |
| `frontend/src/lib/screens/TasksScreen.svelte` | Видалити весь `<style>` блок; HTML: додати `.panel-fill` на root `<section>` |  
| `frontend/src/lib/screens/CounterpartiesScreen.svelte` | HTML: додати `.panel-fill` на root `<section>` |
| `frontend/src/styles/tasks.css` | Замінити `.task-kpis` (grid) → flex-strip з `--acta-*` токенами; видалити `.task-kpi-card`; додати всі screen-специфічні класи з TasksScreen |
| `frontend/src/styles/counterparties.css` | Замінити `height: calc(100vh - 160px)` на `flex: 1; min-height: 0; overflow: hidden` |

## Що НЕ змінюється
- Tasks HTML структура (`.kpi-cell`, `.kpi-divider`, `.task-row`, тощо) — без змін
- Логіка Svelte stores, компонентів, TypeScript
- DocumentsScreen HTML (`.nav-tab` вже відповідає глобальному класу)
- `.task-tabs` (pill) — залишається у tasks.css без змін

## Тести
- Жодних нових тестів не потрібно — це суто CSS fixes
- Візуальна перевірка: Tasks KPI strip (border + background), Payments taб (underline), Counterparties layout (заповнює висоту без скролу на пустому місці)
- Перевірити dark mode (всі `--acta-*` токени підтримують обидві теми)
- Перевірити responsive breakpoints у TasksScreen (media queries також переносяться в tasks.css)
