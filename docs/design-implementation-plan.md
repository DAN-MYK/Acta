# Acta — План впровадження дизайну

> Handoff-матеріали: `C:\Users\MykhailoDan\Downloads\Acta-handoff\acta\project\src\`
>
> Ключові файли handoff:
> - `dashboard.jsx` — Dashboard V1 (Ledger), V2 (Inbox), V3 (Journal)
> - `documents.jsx` — Documents V1 (Table), V2 (Timeline), V3 (Master-detail)
> - `screens.jsx` — Counterparties, Payments
> - `reports.jsx` — Reports з Charts
> - `ui.jsx` — Shared primitives (Icon, Badge, Button, Card, MetricStrip)
> - `tokens.jsx` — Design tokens (кольори, типографіка, радіуси)
> - `data.jsx` — Mock data для прев'ю

---

## Статус впровадження

| Елемент | Стан | Файли проекту |
|---|---|---|
| Дизайн-токени (CSS змінні, типографіка, радіуси) | ✅ Готово | `frontend/src/styles.css` |
| Іконки (37+ SVG, AppIcon компонент) | ✅ Готово | `frontend/src/lib/icons/` |
| Shell — сайдбар, topbar, company switcher | ✅ Готово | `frontend/src/App.svelte`, `styles.css` |
| DashboardScreen — плаский metric strip V1 | ✅ Готово | `frontend/src/lib/screens/DashboardScreen.svelte`, `styles/dashboard.css` |
| TasksScreen | ✅ Готово | `frontend/src/lib/screens/TasksScreen.svelte`, `styles/tasks.css` |
| CounterpartiesScreen — metric strip, list-detail | ✅ Готово | `frontend/src/lib/screens/CounterpartiesScreen.svelte`, `styles/counterparties.css` |
| DocumentsScreen — табличний layout V1 | ⬜ Не зроблено | `frontend/src/lib/screens/DocumentsScreen.svelte`, `styles/documents.css` |
| BarChart SVG компонент | ⬜ Не зроблено | `frontend/src/lib/components/BarChart.svelte` (новий) |
| ReportsScreen — MetricStrip flat | ⬜ Не зроблено | `styles/reports.css` |
| PaymentsScreen — KPI strip flat | ⬜ Не зроблено | `frontend/src/lib/screens/PaymentsScreen.svelte` |
| SettingsScreen — дрібні виправлення | ⬜ Не зроблено | `styles/settings.css` |

---

## Крок 1 — DocumentsScreen

**Пріоритет:** Високий — найвидиміший екран.

**Handoff-референс:** `documents.jsx` → `DocumentsV1`

**Поточний стан:** Кнопки-рядки з картками `.doc-row-open`, заголовок в `.panel`.

**Мета:** Один `Card` з tab strip зверху + `<table>` знизу.

### 1.1 HTML-зміни в `DocumentsScreen.svelte`

- Замінити `<section class="panel">` на `<section class="documents-v1">`
- Додати `.docs-card` обгортку навкруги tabs + таблиці
- Tab strip: `Усі | Рахунки | Акти | Видаткові` з лічильниками — перемістити пошук і кнопку "Створити" в правий край табів
- Замінити `.documents-list` + `.doc-row-open` на `<table class="docs-table">` з колонками:
  `Номер · Тип · Дата · Контрагент · Статус · Сума · Дії`
- Рядки таблиці залишаються клікабельними (відкривають drawer)

**Обмеження тестів:**
- `data-testid="documents-screen"` — зберегти
- Кнопка "Новий документ" — зберегти текст і обробник
- Клік на рядок документа → відкриває drawer — зберегти через `on:click` на `<tr>`

### 1.2 CSS-зміни в `styles/documents.css`

```css
.documents-v1 {
  display: flex;
  flex-direction: column;
  margin-top: var(--space-5);
}

.docs-card {
  background: var(--bg-glass);
  border: 1px solid var(--border-hairline);
  border-radius: var(--radius-3xl);
  overflow: hidden;
}

.docs-tab-bar {
  display: flex;
  align-items: center;
  padding: 0 8px;
  border-bottom: 1px solid var(--border-hairline);
  gap: 2px;
}

.docs-tab {
  padding: 13px 14px 11px;
  border: none;
  background: transparent;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-muted);
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.docs-tab.active {
  color: var(--text-primary);
  font-weight: 500;
  border-bottom-color: var(--accent);
}

.docs-tab-count {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 999px;
  background: var(--bg-subtle);
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
}

.docs-tab-spacer {
  flex: 1;
}

.docs-tab-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px;
}

.docs-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.docs-table thead tr {
  font-size: 10.5px;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  background: var(--bg-subtle);
}

.docs-table th {
  padding: 10px 12px;
  font-weight: 600;
  text-align: left;
}

.docs-table tbody tr {
  border-top: 1px solid var(--border-hairline);
  cursor: pointer;
  transition: background 120ms;
}

.docs-table tbody tr:hover {
  background: color-mix(in srgb, var(--accent-soft) 18%, transparent);
}

.docs-table td {
  padding: 12px;
}

.docs-cell-id {
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--accent-text);
  font-variant-numeric: tabular-nums;
}

.docs-cell-date {
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}

.docs-cell-amount {
  text-align: right;
  font-family: var(--font-mono);
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}

.docs-cell-actions {
  text-align: right;
  padding-right: 18px;
}
```

**Drawer залишається без змін** — вже реалізовано правильно.

---

## Крок 2 — BarChart SVG компонент

**Handoff-референс:** `dashboard.jsx` → `BarChart`, `reports.jsx` → `BarChart`

**Файл:** `frontend/src/lib/components/BarChart.svelte` (новий)

Обидва — Dashboard і Reports — використовують однаковий BarChart. Зараз Dashboard показує cashflow як таблицю рядків.

### 2.1 Компонент

```svelte
<script lang="ts">
  export let data: Array<{ label: string; income: number; expense: number }> = [];
  export let height = 180;

  $: max = Math.max(...data.map(d => Math.max(d.income, d.expense)), 1);
  $: barH = (v: number) => ((v / max) * (height - 36)).toFixed(1);
</script>

<div class="bar-chart" style="height: {height}px">
  {#each data as d}
    <div class="bar-group">
      <div class="bar-bars">
        <div class="bar-income" style="height: {barH(d.income)}px"></div>
        <div class="bar-expense" style="height: {barH(d.expense)}px"></div>
      </div>
      <span class="bar-label">{d.label}</span>
    </div>
  {/each}
</div>

<style>
  .bar-chart {
    display: flex;
    align-items: flex-end;
    gap: 14px;
    padding: 8px 0 28px;
    position: relative;
  }
  .bar-group {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    position: relative;
  }
  .bar-bars { display: flex; align-items: flex-end; gap: 2px; }
  .bar-income { width: 12px; background: var(--accent); border-radius: 2px 2px 0 0; min-height: 2px; }
  .bar-expense { width: 12px; border: 1px solid var(--border-strong, var(--border-hairline)); border-radius: 2px 2px 0 0; min-height: 2px; }
  .bar-label { font-size: 10.5px; color: var(--text-muted); font-family: var(--font-mono); position: absolute; bottom: -20px; }
</style>
```

### 2.2 Інтеграція в DashboardScreen

Всередині `data-testid="dashboard-cashflow"` — поряд зі списком рядків показувати chart.
Дані потрібно передати як `cashflowRows` зі store (поля `incomeStr`/`expenseStr` → `parseFloat`).

**Важливо:** `SkeletonRow` при `initialLoading` залишити — тест перевіряє `[data-testid="dashboard-cashflow"] [data-testid="skeleton-row-item"]`.

---

## Крок 3 — ReportsScreen MetricStrip

**Handoff-референс:** `reports.jsx` → `MetricStrip`

**Файл:** `styles/reports.css`

Мінімальна зміна CSS без торкання HTML — тести залежать від структури `.task-kpi-card`.

### 3.1 CSS-зміни

```css
/* Перевизначити .reports-kpis */
.reports-kpis {
  display: flex;
  align-items: flex-start;
  gap: 0;
  padding: 20px 0;
  border-top: 1px solid var(--border-hairline);
  border-bottom: 1px solid var(--border-hairline);
  margin-top: 18px;
}

.reports-kpis .task-kpi-card {
  flex: 1;
  min-width: 0;
  padding: 0 24px 0 0;
  background: transparent;
  border-radius: 0;
  display: grid;
  gap: 6px;
}

.reports-kpis .task-kpi-card + .task-kpi-card {
  padding-left: 24px;
  border-left: 1px solid var(--border-hairline);
}

.reports-kpis .task-kpi-card strong {
  font-size: 22px;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  line-height: 1.05;
  letter-spacing: -0.3px;
  order: 1;
}

.reports-kpis .task-kpi-card span {
  font-size: 10.5px;
  text-transform: uppercase;
  letter-spacing: 1.2px;
  font-weight: 600;
  color: var(--text-muted);
  order: 0;
}

.reports-kpis .task-kpi-card[data-tone="danger"] strong  { color: var(--danger); }
.reports-kpis .task-kpi-card[data-tone="warning"] strong { color: var(--warning, #c77a1c); }
.reports-kpis .task-kpi-card[data-tone="accent"] strong  { color: var(--accent-text); }
```

**Обмеження тестів:** `data-testid="reports-focus-primary"` — зберегти.

---

## Крок 4 — PaymentsScreen KPI Strip

**Файл:** `frontend/src/lib/screens/PaymentsScreen.svelte` (inline `<style>`)

Мінімальна зміна — лише CSS всередині `<style>` блоку компонента.

### 4.1 Замінити блок `.task-kpis` і `.task-kpi-card`

```css
.task-kpis {
  display: flex;
  align-items: flex-start;
  gap: 0;
  padding: 16px 0;
  border-top: 1px solid var(--border-hairline);
  border-bottom: 1px solid var(--border-hairline);
  margin-top: 14px;
}

.task-kpi-card {
  flex: 1;
  min-width: 0;
  padding-right: 20px;
  display: grid;
  gap: 6px;
  background: transparent;
  border-radius: 0;
}

.task-kpi-card + .task-kpi-card {
  padding-left: 20px;
  border-left: 1px solid var(--border-hairline);
}

.task-kpi-card strong {
  font-size: 22px;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  line-height: 1.05;
  letter-spacing: -0.3px;
}

.task-kpi-card span {
  font-size: 10.5px;
  text-transform: uppercase;
  letter-spacing: 1.2px;
  font-weight: 600;
  color: var(--text-muted);
}

/* alert → тональний колір тексту замість gradient background */
.task-kpi-card-alert strong {
  color: var(--accent-text);
}
```

**Обмеження тестів:** `data-testid="payments-kpis"` і `SkeletonCard` всередині — не чіпати.

---

## Крок 5 — SettingsScreen

**Файли:** `frontend/src/lib/screens/SettingsScreen.svelte`, `styles/settings.css`

### 5.1 Прибрати `.panel` wrapper

```svelte
<!-- Замінити -->
<section class="panel">
<!-- На -->
<section class="settings-screen">
```

```css
.settings-screen {
  margin-top: var(--space-5);
}
```

### 5.2 Nav buttons — стиль з handoff

```css
.settings-nav-button {
  font-size: 13px;
  font-weight: 400;
  color: var(--text-muted);
  padding: 9px 12px;
}

.settings-nav-button.active {
  color: var(--text-primary);
  font-weight: 500;
  background: color-mix(in srgb, var(--accent-soft) 55%, var(--bg-card));
  border-color: color-mix(in srgb, var(--accent) 22%, var(--border-hairline));
  box-shadow: none;
}
```

---

## Крок 6 — Адаптивна верстка

Після завершення кожного кроку — додати media queries.

| Breakpoint | Що змінюється |
|---|---|
| `≤ 1100px` | Documents: таблиця → card-список; Reports KPI 2 колонки |
| `≤ 980px` | Dashboard KPI wrap, grid 1 col; Payments KPI 2x2 |
| `≤ 720px` | Documents drawer 100vw; Settings nav горизонтально зверху |

---

## Крок 7 — Перевірка тестів

Після кожного кроку запускати:

```bash
cd frontend
npx vitest run src/lib/screens/__tests__/DocumentsScreen.test.ts
npx vitest run src/lib/screens/__tests__/DashboardScreen.test.ts
npx vitest run src/lib/screens/__tests__/ReportsScreen.test.ts
npx vitest run src/lib/screens/__tests__/PaymentsScreen.test.ts
# Повний прогін:
npx vitest run
```

**Правило:** крок вважається завершеним тільки після зелених тестів.

---

## Порядок виконання

```
1. DocumentsScreen (Крок 1)      — найвидиміший, таблиця + tabs
2. BarChart компонент (Крок 2)   — потрібен для Dashboard і Reports
3. Reports KPI Strip (Крок 3)    — чиста CSS зміна, безпечно
4. Payments KPI Strip (Крок 4)   — чиста CSS зміна, безпечно
5. Settings (Крок 5)             — дрібниці
6. Адаптив (Крок 6)              — фінальний прохід
```

---

## Що навмисно залишається без змін

| Елемент | Причина |
|---|---|
| Drawer (форми документів) | Вже реалізовано — handoff forms.jsx — це модальний варіант, drawer кращий для UX |
| Logo/Brand SVG | Проект має власний брендинг |
| Source Serif 4 | Виключено раніше |
| Reconciliation UI в Payments | Складна бізнес-логіка, переписування ризиковане |
| Reports charts (CashChart, AgingBar) | Складні SVG з реальними даними — окреме завдання |
