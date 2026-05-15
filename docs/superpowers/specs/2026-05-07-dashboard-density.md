# Dashboard Information Density

**Date:** 2026-05-07
**Status:** Approved

## Problem

Dashboard виглядає розбалансованим при малій кількості даних:
- "Завдання у фокусі" самотня в рядку — права колонка порожня
- Картки в одному grid-рядку розтягуються до висоти сусіда (`align-items` відсутній)
- Відступи list-row (10px) виглядають щедро при 1–2 рядках

## Design

Три зміни, без нових компонентів і без торкання store/даних.

### 1. Grid alignment (`dashboard.css`)

```css
.dashboard-grid {
  align-items: start; /* додати */
}
```

Картки більше не розтягуються до висоти найвищого сусіда.

### 2. List row spacing (`dashboard.css`)

`.dashboard-list-row` має два значення що разом дають вертикальний ритм:
- `margin-top: 10px` — відстань між рядками
- `padding: 10px 0` — відступ всередині рядка

Обидва зменшуємо для щільнішого вигляду:

```css
.dashboard-list-row {
  margin-top: 7px;  /* було 10px */
  padding: 7px 0;   /* було 10px 0 */
}
```

### 3. Documents → full width (`DashboardScreen.svelte`)

"Останні документи" отримує клас `wide`. Порядок карток у template:

1. Cashflow (wide) — без змін
2. **Documents (wide)** — додати `wide`
3. Payments — без змін
4. Tasks — без змін

Payments і Tasks автоматично утворюють пару в останньому рядку грида.

## Результат

Три зони з чіткою ієрархією:
```
[ KPI strip                        ]
[ Cashflow            (full width) ]
[ Documents           (full width) ]
[ Payments  ] [ Tasks              ]
```

Баланс зберігається і при 1 записі, і при 5–6.

## Files

- `frontend/src/styles/dashboard.css` — 2 правки
- `frontend/src/lib/screens/DashboardScreen.svelte` — 1 правка (клас `wide`)
