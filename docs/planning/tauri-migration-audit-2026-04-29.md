# Аудит міграції на Tauri — оновлено 2026-04-30

## Стан на 2026-04-30

Міграція активно виконується. Etap 1–3 завершені. Etap 4 в процесі. Slint залишається чинним runtime UI і живим wiring layer — прибирати його не можна до завершення Etap 5–10.

| Etap | Назва | Стан |
|------|-------|------|
| 1 | Tauri scaffold | ✅ Завершено |
| 2 | Shared backend bootstrap | ✅ Завершено |
| 3 | Command contract | ✅ Завершено |
| 4 | Shell + feature screens (Svelte) | 🟡 В процесі |
| 5 | Refresh/wiring модель | ⬜ Не розпочато |
| 6 | Дизайн-система | ⬜ Не розпочато |
| 7 | Тести | ⬜ Не розпочато |
| 8 | CI | ⬜ Не розпочато |
| 9 | Фінальний cutover (видалення Slint) | ⬜ Не розпочато |

---

## Etap 1 — Tauri scaffold ✅

Реальний Tauri runtime існує і збирається:

- `src-tauri/src/main.rs` + `src-tauri/src/lib.rs` — повноцінний Tauri entrypoint
- `src-tauri/src/commands/` — 7 модулів команд (shell, counterparties, documents, payments, tasks, reports, settings)
- `package.json`, `vite.config.ts`, `svelte.config.js`, `tsconfig.json`, `index.html` — frontend build system
- `frontend/src/` — Svelte frontend directory

## Etap 2 — Shared backend bootstrap ✅

`acta::runtime` є спільним для Slint і Tauri:

- `src/runtime.rs` — `build_runtime`, `connect_pool`, `run_migrations`, `init_app_ctx`, `init_app_ctx_blocking`, `spawn_background_tasks`
- `src/config.rs` — `AppConfig::load()` зберігає `last_company_id` між сесіями
- Slint entrypoint (`src/main.rs`) і Tauri entrypoint (`src-tauri/src/lib.rs`) обидва використовують ці функції через `acta::runtime`
- `AppCtx` (pool + active company) однаковий для обох runtime

## Etap 3 — Command contract ✅

Всі backend-команди реалізовані в `src/tauri_api/`:

| Модуль | Файл | Розмір |
|--------|------|--------|
| shell | `src/tauri_api/shell.rs` | 325 рядків |
| counterparties | `src/tauri_api/counterparties.rs` | 484 рядки |
| documents | `src/tauri_api/documents.rs` | 1379 рядків |
| payments | `src/tauri_api/payments.rs` | 634 рядки |
| tasks | `src/tauri_api/tasks.rs` | 435 рядків |
| reports | `src/tauri_api/reports.rs` | 748 рядків |
| settings | `src/tauri_api/settings.rs` | 622 рядки |
| **Разом** | | **4627 рядків** |

Всі команди зареєстровані у `src-tauri/src/lib.rs` через `tauri::generate_handler!`.

## Etap 4 — Shell + feature screens (Svelte) 🟡

Frontend активно розробляється:

- `frontend/src/App.svelte` — 1285 рядків, основний компонент
- `frontend/src/lib/api.ts` — 277 рядків, Tauri invoke layer
- `frontend/src/lib/types.ts` — 460 рядків, TypeScript типи
- `frontend/src/lib/stores/` — 10 Svelte stores: navigation, shell, theme, palette, documents, counterparties, payments, tasks, reports, settings

**Залишається:** реальне підключення invoke → stores → UI, тестування, доведення до production-ready.

## Чому Slint залишається зараз

- `src/main.rs` є чинним entrypoint і запускає Slint через `bootstrap::build_ui`
- Весь `src/ui/*.rs` і `src/bootstrap.rs` + `src/bootstrap/` submodules wiring layer активні і компілюються
- `tests/tauri_vertical_slice.rs` — integration tests на Tauri backend (green)
- Slint headless tests (`tests/ui_events.rs`) також live

Slint не можна прибирати до завершення Etap 5–9 і отримання green cutover без Slint-specific залежностей.

## Пов'язані документи

- [tauri-migration-roadmap-2026-04-29.md](./tauri-migration-roadmap-2026-04-29.md)
- [tauri-migration-contract-matrix-2026-04-29.md](./tauri-migration-contract-matrix-2026-04-29.md)
- [tauri-documents-command-spec-2026-04-29.md](./tauri-documents-command-spec-2026-04-29.md)
- [tauri-payments-command-spec-2026-04-29.md](./tauri-payments-command-spec-2026-04-29.md)
