# Svelte Screens Extraction Design

## Goal

Розбити `frontend/src/App.svelte` (1386 рядків) на 8 окремих файлів: тонка оболонка `App.svelte` + 7 screen-компонентів у `frontend/src/screens/`.

## Context

Весь функціонал вже реалізовано і підключено до Tauri backend. Dashboard має свій `dashboardStore` і `DashboardScreenDto`. Робота — виключно структурний рефакторинг без змін логіки або дизайну.

## Architecture

### Файлова структура після рефакторингу

```
frontend/src/
├── App.svelte              — тонка оболонка (~120 рядків)
├── main.ts                 — без змін
├── styles.css              — без змін
├── screens/
│   ├── Dashboard.svelte
│   ├── Documents.svelte
│   ├── Counterparties.svelte
│   ├── Payments.svelte
│   ├── Reports.svelte
│   ├── Tasks.svelte
│   └── Settings.svelte
└── lib/
    ├── stores/             — без змін
    ├── types.ts            — без змін
    └── api.ts              — без змін
```

### App.svelte після рефакторингу

Тримає тільки:
- `onMount`: `shell.load()` → `settings.load()` (тема) → паралельне завантаження всіх 6 stores
- `onCompanyChange`: повне перезавантаження після зміни активної компанії
- `onQuickThemeToggle` (якщо є в topbar)
- Sidebar: brand + nav кнопки + theme switcher
- Topbar: назва компанії / роль + company switcher select + Ctrl+K кнопка
- Palette overlay: backdrop + palette секція з input і items
- `handleKeydown`: Ctrl+K toggle + Ctrl+1..7 навігація
- `<svelte:window on:keydown={handleKeydown} />`
- `{#if currentScreen === "dashboard"}<Dashboard />{/if}` × 7
- Імпортує: всі 11 stores (navigationStore, shellStore, paletteStore, themeStore, settingsStore, dashboardStore, documentsStore, counterpartiesStore, tasksStore, reportsStore, paymentsStore — для onMount + onCompanyChange + palette + shell), всі 7 screen-компонентів

### Screen компоненти — store-bound

Кожен screen сам імпортує потрібні stores і містить весь свій HTML, локальний стан і обробники.

| Screen | Імпортує stores | Локальний стан |
|--------|-----------------|----------------|
| `Dashboard.svelte` | `dashboardStore`, `navigationStore`, `documentsStore`, `tasksStore` | — |
| `Documents.svelte` | `documentsStore`, `navigationStore` | `createCounterpartyId: string`, `createKind: DocumentKind` |
| `Counterparties.svelte` | `counterpartiesStore`, `navigationStore`, `documentsStore` | — |
| `Payments.svelte` | `paymentsStore` | — |
| `Reports.svelte` | `reportsStore` | — |
| `Tasks.svelte` | `tasksStore` | — |
| `Settings.svelte` | `settingsStore`, `themeStore`, `shellStore` | — |

### Editors залишаються в своєму screen

`editor-sheet` блоки рендеряться всередині відповідного screen-компонента (не окремі файли). Умова `{#if $documents.editor}` залишається в `Documents.svelte`.

## Data Flow

1. `App.svelte` `onMount` завантажує всі stores паралельно.
2. Screen-компоненти читають стан реактивно через `$store`.
3. Дії (save, open, search) викликаються методами store безпосередньо зі screen.
4. Крос-screen навігація (наприклад, з Counterparties → Documents) через `navigationStore.go()` + відповідний store метод.
5. Palette activation відбувається в App.svelte (`palette.activate()`), навігація і відкриття документів — через stores.

## Error Handling

Кожен store вже має поле `error: string | null`. Screens відображають `{#if $store.error}<p class="error">` — без змін.

## CSS

`styles.css` — без змін. Всі необхідні класи вже є: `dashboard-*`, `panel`, `editor-sheet`, `counterparties-layout`, тощо.

## What Does NOT Change

- `frontend/src/lib/` — жодних змін
- `src-tauri/` — жодних змін
- Rust код — жодних змін
- `styles.css` — жодних змін
- Поведінка додатку — повністю ідентична до рефакторингу

## .gitignore

Додати `.superpowers/` до `frontend/.gitignore` або кореневого `.gitignore`.
