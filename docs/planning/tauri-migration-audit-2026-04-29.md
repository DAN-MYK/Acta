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
- `src-tauri/tauri.conf.json` — packaging налаштовано на реальний Vite output `../dist`
- `package.json`, `vite.config.ts`, `svelte.config.js`, `tsconfig.json`, `index.html` — frontend build system
- `frontend/src/` — Svelte frontend directory

## Etap 2 — Shared backend bootstrap ✅

`acta::runtime` є спільним для Slint і Tauri:

- `src/runtime.rs` — `build_runtime`, `connect_pool`, `run_migrations`, `init_app_ctx`, `init_app_ctx_blocking`, `spawn_background_tasks`
- `src/config.rs` — `AppConfig::load()` зберігає `last_company_id` між сесіями
- Slint entrypoint (`src/main.rs`) і Tauri entrypoint (`src-tauri/src/lib.rs`) обидва використовують ці функції через `acta::runtime`
- `AppCtx` (pool + active company) однаковий для обох runtime

## Etap 3 — Command contract ✅

Всі backend-команди поточного Tauri UX реалізовані в `src/tauri_api/`:

| Модуль | Файл | Розмір |
|--------|------|--------|
| shell | `src/tauri_api/shell.rs` | 325 рядків |
| counterparties | `src/tauri_api/counterparties.rs` | 484 рядки |
| documents | `src/tauri_api/documents.rs` | 1322 рядки |
| payments | `src/tauri_api/payments.rs` | 634 рядки |
| tasks | `src/tauri_api/tasks.rs` | 435 рядків |
| reports | `src/tauri_api/reports.rs` | 748 рядків |
| settings | `src/tauri_api/settings.rs` | 623 рядки |
| **Разом** | | **4571 рядок** |

У `src-tauri/src/lib.rs` через `tauri::generate_handler!` зареєстровано лише команди, які реально використовує поточний Svelte/Tauri UX. `documents_bulk_advance_status`, `documents_bulk_delete` і невикористаний `document_prepare_new` прибрані з contract surface: frontend їх не викликає, а bulk stub-реалізації лише гарантовано повертали помилку "ще не перенесено", тоді як create-context уже покривається через `counterparty_create_document_context`.

## Etap 4 — Shell + feature screens (Svelte) 🟡

Frontend активно розробляється:

- `frontend/src/App.svelte` — 1285 рядків, основний компонент
- `frontend/src/lib/api.ts` — 277 рядків, Tauri invoke layer
- `frontend/src/lib/types.ts` — 460 рядків, TypeScript типи
- `frontend/src/lib/stores/` — 10 Svelte stores: navigation, shell, theme, palette, documents, counterparties, payments, tasks, reports, settings

**Dashboard parity update на 2026-04-30:** invoke → stores → UI для dashboard уже реальні, але parity зі Slint ще частковий. Поточний Tauri dashboard покриває KPI + cashflow + recent acts + upcoming payments + urgent tasks, тоді як канонічний Slint baseline з `.worktrees/sprint-2026-04-24/ui/dashboard.slint` додатково мав journal/inbox split, рахунки, richer KPI strip, YTD/delta/sparklines і dashboard-level quick actions.

**Залишається:** довести parity або явно переглянути contract migration → redesign по dashboard, а також завершити тестування й production-hardening.

### Dashboard parity diff

#### Перенесено повністю

- `dashboard_load` є окремою Tauri command і реально зареєстрований у `src-tauri/src/lib.rs`.
- Дані dashboard йдуть з Rust backend через `src/tauri_api/dashboard.rs`, а не з mock/frontend-only state.
- У Tauri є окремий production screen `frontend/src/lib/screens/DashboardScreen.svelte`, а не placeholder route.
- Працює refresh поточного dashboard slice.

#### Перенесено частково

- KPI block перенесений як backend-backed summary, але не повторює Slint KPI strip по складу метрик, delta й sparklines.
- Cashflow перенесений як backend-backed список місячних агрегатів, але не як Slint bar chart з легендою та YTD summary.
- Recent acts перенесені як окремий список з переходом у documents flow, але не як частина повного journal table.
- Urgent tasks перенесені як список з переходом у task editor, але без inline toggle/new-task/all-tasks UX зі старого dashboard.
- Upcoming payments перенесені як backend-backed список, але зараз це read-only блок без row action/drill-in.

#### Свідомо відкладено

- `Inbox` / `Вхідні` режим dashboard.
- Sidebar `Рахунки` з total balance.
- Journal-level filters/actions типу `Усі типи` і `Всі операції`.
- Візуальний 1:1 parity зі Slint layout: right sidebar composition, chart-first hierarchy, sparkline-driven KPI strip.

#### Відсутнє / ще треба реалізувати

- Чітке product-рішення: переносити `journal + inbox + accounts` чи офіційно фіксувати їх як deliberate cut.
- Dashboard-local task actions: toggle, quick add, footer flow `Усі задачі`.
- Повний data parity для YTD/delta/sparkline показників.
- Dashboard-specific inbox actions через Tauri surface, якщо inbox лишається частиною нового contract.

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
- [dashboard-migration-contract-2026-04-30.md](./dashboard-migration-contract-2026-04-30.md)

### Dashboard migration contract

- `dashboard implemented`
- `dashboard parity partial by design`
- `redesign-first, not strict Slint parity`

Поточний Tauri dashboard вважається реалізованим як робочий redesign-first screen, а не як strict parity-копія Slint dashboard.
