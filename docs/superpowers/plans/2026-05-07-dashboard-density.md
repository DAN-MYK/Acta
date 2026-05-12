# Dashboard Information Density Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Покращити інформаційну щільність дашборду: зібрати layout в три зони (KPI → Cashflow → Documents → Payments+Tasks) і прибрати надлишкові відступи.

**Architecture:** 2 файли, 3 точкові правки. Логіки не торкаємось — тільки CSS і клас в markup. Тест перевіряє клас `wide` на картці Documents.

**Tech Stack:** Svelte, CSS (без препроцесорів), Vitest + jsdom

---

## Files

| Файл | Зміна |
|------|-------|
| `frontend/src/lib/screens/__tests__/DashboardScreen.test.ts` | Додати тест для класу `wide` |
| `frontend/src/lib/screens/DashboardScreen.svelte` | Клас `wide` на Documents article |
| `frontend/src/styles/dashboard.css` | `align-items: start` + зменшити margin/padding list-row |

---

### Task 1: Failing test — Documents card has class `wide`

**Files:**
- Modify: `frontend/src/lib/screens/__tests__/DashboardScreen.test.ts`

- [ ] **Step 1: Додати тест після існуючого "renders operational dashboard sections"**

Відкрий `frontend/src/lib/screens/__tests__/DashboardScreen.test.ts` і додай новий `it`-блок у `describe("DashboardScreen component", ...)` одразу після тесту `"renders operational dashboard sections from the screen store"` (після рядка ~179):

```ts
it("renders recent-documents card as full-width (wide class)", () => {
  const { component, target } = renderDashboard();

  const card = target.querySelector('[data-testid="dashboard-recent-documents"]');
  expect(card?.classList.contains("wide")).toBe(true);

  component.$destroy();
});
```

- [ ] **Step 2: Запустити тест — переконатись що ПАДАЄ**

```bash
cd frontend && npx vitest run src/lib/screens/__tests__/DashboardScreen.test.ts
```

Очікувано: `AssertionError: expected false to be true` (клас `wide` ще відсутній).

---

### Task 2: Додати клас `wide` на Documents card

**Files:**
- Modify: `frontend/src/lib/screens/DashboardScreen.svelte:87`

- [ ] **Step 1: Замінити клас на articles "Останні документи"**

У `DashboardScreen.svelte` рядок 87 зараз:
```html
<article class="dashboard-card" data-testid="dashboard-recent-documents">
```

Замінити на:
```html
<article class="dashboard-card wide" data-testid="dashboard-recent-documents">
```

- [ ] **Step 2: Запустити тест — переконатись що ПРОХОДИТЬ**

```bash
cd frontend && npx vitest run src/lib/screens/__tests__/DashboardScreen.test.ts
```

Очікувано: всі тести `PASS`, включаючи новий.

- [ ] **Step 3: Зробити коміт**

```bash
git add frontend/src/lib/screens/DashboardScreen.svelte frontend/src/lib/screens/__tests__/DashboardScreen.test.ts
git commit -m "feat: make recent-documents card full-width on dashboard"
```

---

### Task 3: CSS — align-items і щільніші list-row

**Files:**
- Modify: `frontend/src/styles/dashboard.css:93-98` (`.dashboard-grid`)
- Modify: `frontend/src/styles/dashboard.css:177-187` (`.dashboard-list-row`)

CSS-правки не покриваються unit-тестами — верифікація візуальна в браузері.

- [ ] **Step 1: Додати `align-items: start` до `.dashboard-grid`**

У `frontend/src/styles/dashboard.css` знайди `.dashboard-grid` (~рядок 93):

```css
.dashboard-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  margin-top: 24px;
}
```

Додай `align-items: start`:

```css
.dashboard-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  margin-top: 24px;
  align-items: start;
}
```

- [ ] **Step 2: Зменшити margin-top і padding у `.dashboard-list-row`**

У тому ж файлі знайди `.dashboard-list-row` (~рядок 177):

```css
.dashboard-list-row {
  width: 100%;
  margin-top: 10px;
  text-align: left;
  border: 0;
  border-bottom: 1px solid var(--acta-color-border);
  border-radius: 0;
  padding: 10px 0;
  background: transparent;
  color: inherit;
}
```

Замінити на:

```css
.dashboard-list-row {
  width: 100%;
  margin-top: 7px;
  text-align: left;
  border: 0;
  border-bottom: 1px solid var(--acta-color-border);
  border-radius: 0;
  padding: 7px 0;
  background: transparent;
  color: inherit;
}
```

- [ ] **Step 3: Запустити всі frontend тести — переконатись нічого не зламалось**

```bash
cd frontend && npx vitest run
```

Очікувано: всі тести `PASS`.

- [ ] **Step 4: Зробити коміт**

```bash
git add frontend/src/styles/dashboard.css
git commit -m "fix: tighten dashboard grid alignment and list-row spacing"
```

---

### Task 4: Візуальна верифікація

- [ ] **Step 1: Запустити dev-сервер**

```bash
cd src-tauri && cargo tauri dev
```

- [ ] **Step 2: Перевірити дашборд**

Перевір що:
- Картка "Останні документи" займає повну ширину (під Cashflow)
- "Найближчі платежі" і "Завдання у фокусі" стоять поруч в одному рядку
- Картки не розтягуються до висоти сусіда (кожна своєї висоти)
- Рядки в списках щільніші ніж раніше

- [ ] **Step 3: Перевірити mobile breakpoint (980px)**

Звузь браузер до <980px. Grid і так перейде в одну колонку — переконайся що нічого не з'їхало.
