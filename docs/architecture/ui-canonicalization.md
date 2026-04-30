# UI Canonicalization

Оновлено: `2026-04-30`

## Канонічний UI

Канонічний desktop UI Acta після cutover:

- [src-tauri](/C:/Users/MykhailoDan/apps/Acta/src-tauri) - Tauri entrypoint, config, commands;
- [frontend/src/App.svelte](/C:/Users/MykhailoDan/apps/Acta/frontend/src/App.svelte) - shell/root component;
- [frontend/src/lib/screens](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/screens) - feature screens;
- [frontend/src/lib/stores](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/stores) - frontend orchestration;
- [frontend/src/lib/styles/tokens.css](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/styles/tokens.css) - design tokens.

Slint runtime видалено 2026-04-30. `ui/`, root `build.rs`, `src/ui/*`, `src/bootstrap/*` Slint wiring і `tests/ui_events.rs` не є live runtime.

## Archived Slint references

Slint можна цитувати лише як historical/pre-cutover reference:

- `.worktrees/sprint-2026-04-24/ui/*.slint`;
- `.worktrees/sprint-2026-04-24/src/ui/*`;
- старі planning/audit docs, якщо вони явно позначені як archived або pre-cutover.

Нові planning docs не повинні подавати Slint callback/property contract як поточний UI contract.

## Правило для розробки

Усі нові UI-зміни йдуть у Svelte/Tauri шлях:

- screen markup: `frontend/src/lib/screens/*.svelte`;
- reusable UI: `frontend/src/lib/components/*.svelte`;
- icons: `frontend/src/lib/icons/*` + `AppIcon.svelte`;
- tokens and page styling: `frontend/src/lib/styles/tokens.css` і `frontend/src/styles.css`;
- data contract: `src/tauri_api/*`, `src-tauri/src/commands/*`, `frontend/src/lib/api.ts`, `frontend/src/lib/types.ts`.

Якщо потрібно повернути старий Slint-only UX (`journal`, `inbox`, accounts block), це нова Tauri feature spec, а не задача "догнати parity".
