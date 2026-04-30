# App State

Оновлено: `2026-04-30`

## Канонічний state path

Acta після cutover працює через Tauri + Svelte:

- backend runtime state: [src/app_ctx.rs](/C:/Users/MykhailoDan/apps/Acta/src/app_ctx.rs)
- Tauri commands: [src-tauri/src/commands](/C:/Users/MykhailoDan/apps/Acta/src-tauri/src/commands)
- frontend invoke layer: [frontend/src/lib/api.ts](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/api.ts)
- frontend stores: [frontend/src/lib/stores](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/stores)
- Svelte screens: [frontend/src/lib/screens](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/screens)

`AppWindow`, `Weak<AppWindow>`, `ModelRc`, `VecModel`, `apply_*_to_ui()` і `wire_*_callbacks()` більше не є live state model.

## Backend state

`AppCtx` тримає тільки shared backend context:

- `PgPool`;
- `active_company_id`;
- доступ до доменних DB/API функцій через `src/tauri_api/*`.

Backend не зберігає Svelte screen-local стан. Пошук, активні tabs, відкриті редактори та transient повідомлення живуть у frontend stores, якщо вони не потрібні як domain state.

## Frontend state

Кожен slice має власний store:

- `shell.ts` - chrome, active company, command palette handoff;
- `navigation.ts` - поточний screen id;
- `dashboard.ts`, `documents.ts`, `counterparties.ts`, `payments.ts`, `tasks.ts`, `reports.ts`, `settings.ts` - screen data, loading/error/message, локальні editor states;
- `theme.ts` - поточне theme відображення, синхронізоване через settings/shell.

Screen components читають store через `$store` і викликають тільки методи свого store або явний cross-slice flow, зафіксований у contract docs.

## Refresh model

Canonical refresh після mutation:

1. store викликає Tauri command через `api.ts`;
2. backend повертає DTO або mutation result;
3. store оновлює власний snapshot або робить targeted reload;
4. інші slices перевантажуються тільки якщо це продуктово потрібно.

Глобального imperative refresh поверх усього UI більше немає.

## Test contract

State regressions ловляться трьома рівнями:

- store tests у [frontend/src/lib/stores/__tests__](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/stores/__tests__);
- screen component tests у [frontend/src/lib/screens/__tests__](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/screens/__tests__);
- backend/Tauri vertical slice tests у [tests/tauri_vertical_slice.rs](/C:/Users/MykhailoDan/apps/Acta/tests/tauri_vertical_slice.rs).
