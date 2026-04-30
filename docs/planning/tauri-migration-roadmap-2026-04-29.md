# Roadmap міграції на Tauri — 2026-04-29

## Принцип

Міграція має йти паралельно до чинного Slint UI, а не через миттєвий cutover. Найменш ризиковий шлях:

1. Підняти Tauri поруч.
2. Винести backend contract.
3. Перенести shell і екрани по одному.
4. Переписати тести й CI.
5. Лише після цього прибрати Slint.

## Етап 1. Tauri scaffold

### Ціль

Підготувати новий runtime без впливу на Slint.

### Нові файли

- `src-tauri/Cargo.toml`
- `src-tauri/src/main.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/build.rs`
- `src-tauri/tauri.conf.json`
- `package.json`
- `vite.config.ts`
- `svelte.config.js`
- `tsconfig.json`
- `index.html`
- `src/` frontend

### Ризик

Низький.

## Етап 2. Shared backend bootstrap

### Ціль

Зробити так, щоб Tauri міг використовувати той самий Rust backend, що й нинішній Slint app.

### Поточні точки інтеграції

- [src/main.rs](/C:/Users/MykhailoDan/apps/Acta/src/main.rs:13)
- [src/bootstrap.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap.rs:34)
- [src/app_ctx.rs](/C:/Users/MykhailoDan/apps/Acta/src/app_ctx.rs:54)

### Що переносимо

- `AppCtx`
- `PgPool`
- міграції
- background tasks
- active company context

### Ризик

Низький-середній.

## Етап 3. Command contract

### Ціль

Замінити Slint callbacks на Tauri commands.

### Джерела контракту

- [src/ui/mod.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/mod.rs:1)
- [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:1)
- presenter-модулі у [src/ui](/C:/Users/MykhailoDan/apps/Acta/src/ui)

### Ключове правило

- гроші лишаються рядками;
- дати на межі API лишаються рядками;
- валідація і `Decimal` лишаються в Rust.

### Ризик

Середній.

## Етап 4. Shell і navigation

### Ціль

Перенести root shell, navigation, company switcher, palette, theme toggle, shortcuts.

### Поточні Slint джерела

- [ui/app.slint](/C:/Users/MykhailoDan/apps/Acta/ui/app.slint:30)
- [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint)
- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:210)
- [src/bootstrap/navigation.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/navigation.rs:35)
- [src/bootstrap/shell.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/shell.rs)
- [src/bootstrap/palette.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/palette.rs:154)

### Нові frontend модулі

- `src/App.svelte`
- `src/lib/stores/navigation.ts`
- `src/lib/stores/shell.ts`
- `src/lib/stores/theme.ts`
- `src/lib/stores/palette.ts`
- `src/lib/components/Shell.svelte`
- `src/lib/components/Sidebar.svelte`
- `src/lib/components/CommandPalette.svelte`

### Ризик

Середній.

## Етап 5. Перенос feature screens

### Статус на 2026-04-30

- `dashboard` більше не є placeholder у Tauri UI.
- Пілотний екран працює через `dashboard_load` + `frontend/src/lib/stores/dashboard.ts` і тягне дані з Rust backend для KPI, cashflow, останніх актів, найближчих платежів і задач у фокусі.
- Канонічний Slint baseline для parity-аудиту зараз лежить у `.worktrees/sprint-2026-04-24/ui/dashboard.slint` та `.worktrees/sprint-2026-04-24/src/ui/dashboard.rs`, а не в root `ui/`, як у старішій документації.
- Dashboard parity зі Slint ще не досягнуто: Tauri покриває лише backend-backed operational slice, але ще не переносить Slint `inbox` mode, journal table, блок рахунків, dashboard-local task actions, YTD/delta/sparkline метрики та chart-first layout.
- Наступні vertical slices залишаються: `documents`, `counterparties`, `payments`, `tasks`, `reports`, `settings`.

### Dashboard parity note — аудит 2026-04-30

Базові джерела для звірки:

- Slint UI: `.worktrees/sprint-2026-04-24/ui/dashboard.slint`
- Slint wiring/data prep: `.worktrees/sprint-2026-04-24/src/ui/dashboard.rs`
- Tauri screen: `frontend/src/lib/screens/DashboardScreen.svelte`
- Tauri store/API/backend: `frontend/src/lib/stores/dashboard.ts`, `frontend/src/lib/api.ts`, `src/tauri_api/dashboard.rs`, `src-tauri/src/commands/dashboard.rs`

#### Що вже перенесено

- Dashboard у Tauri завантажується окремою командою `dashboard_load`.
- Дані йдуть з реального Rust backend, а не з frontend mock state.
- На екрані вже є п'ять реальних секцій: KPI summary, cashflow, recent acts, upcoming payments і urgent tasks.
- Доступний refresh поточного slice, переходи до `documents`, `payments`, `tasks`, а також drill-in у document/task flows.
- Клік по recent act переводить на `documents` і викликає `documents.open(docId)`; клік по urgent task переводить на `tasks` і викликає `tasks.openEditor(taskId)`.

#### Що перенесено частково

- KPI перенесені як новий Tauri-набір, але це не 1:1 зі Slint KPI strip: у Slint були `дохід`, `витрати`, `чистий прибуток`, `заборгованість`, `прострочка`, delta-рядки та sparklines, а в Tauri зараз інший набір business counters без цих visual/data affordances.
- Cashflow перенесено по даних, але не по UI-flow: Slint мав bar chart з легендою, YTD summary і empty-state навколо normalized bars; Tauri зараз показує read-only tabular/list presentation тих самих місячних агрегатів.
- Tasks slice на dashboard перенесено як список urgent tasks, але не як interactive sidebar widget зі switch done/open і quick add прямо з dashboard.
- Recent documents перенесено лише для актів; Slint journal показував ширший operational log із колонками `дата / id / операція / контрагент / дебет / кредит / статус`.
- Upcoming payments присутні як backend-backed список, але тільки як read-only rows без row-level drill-in, reconcile або dashboard-specific quick action.

#### Що свідомо опущено або відкладено

- `Inbox`/`Вхідні` режим старого dashboard.
- Sidebar блок `Рахунки` з total balance.
- Dashboard-local task toggle і quick add.
- Journal-level filters/buttons типу `Усі типи` / `Всі операції`.
- Slint-specific visual parity: KPI strip, right sidebar, chart-first composition, sparkline usage.

#### Що ще бракує для parity

1. Або перенести `journal + inbox + accounts` як окремі Tauri sections, або формально зафіксувати їх як deliberate cut із новим product contract.
2. Визначити долю dashboard-specific actions зі Slint: task toggle, new task, all tasks, all operations, inbox actions.
3. Вирівняти data contract: або дотягнути Tauri до Slint метрик/YTD/delta, або явно зафіксувати dashboard як redesign з частковим reuse backend-агрегатів, а не як strict parity migration.

### Рекомендований порядок

1. `dashboard`
2. `documents`
3. `counterparties`
4. `payments`
5. `tasks`
6. `reports`
7. `settings`

### Поточні Rust presenter-модулі

- [src/ui/dashboard.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/dashboard.rs:145)
- [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1046)
- [src/ui/counterparties.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:229)
- [src/ui/payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:364)
- [src/ui/tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:239)
- [src/ui/reports.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/reports.rs:370)
- [src/ui/settings.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/settings.rs:512)

### Ризик

Середній-високий.

## Етап 6. Заміна refresh/wiring моделі

### Поточна модель

- `Weak<AppWindow>`
- `apply_*_to_ui`
- `wire_*_callbacks`
- `VecModel` / `ModelRc`

### Цільова модель

- `invoke()` / `#[tauri::command]`
- Svelte stores
- targeted re-fetch після mutation
- мінімум глобального imperative refresh

### Поточні файли

- [src/bootstrap/refresh.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/refresh.rs:99)
- [src/bootstrap/wiring.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/wiring.rs:7)
- [src/bootstrap/document_chain.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/document_chain.rs:9)

### Ризик

Високий.

## Етап 7. Дизайн-система

### Джерела

- [ui/design-tokens.slint](/C:/Users/MykhailoDan/apps/Acta/ui/design-tokens.slint)
- [ui/components.slint](/C:/Users/MykhailoDan/apps/Acta/ui/components.slint)
- [ui/icons.slint](/C:/Users/MykhailoDan/apps/Acta/ui/icons.slint)
- `ui/assets/...`

### Ціль

- перенести токени в CSS custom properties;
- перенести reusable components;
- перепідключити SVG assets.

### Ризик

Середній.

## Етап 8. Тести

### Поточний стан

- [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:6) є Slint-specific test suite.

### Цільовий стан

- Rust unit/integration tests на backend commands;
- frontend component tests;
- Tauri/e2e smoke tests.

### Мінімальний набір

- command tests
- navigation smoke test
- documents CRUD smoke test
- payments flow smoke test
- settings persistence smoke test

### Ризик

Середній.

## Етап 9. CI

### Поточний стан

- [ci.yml](/C:/Users/MykhailoDan/apps/Acta/.github/workflows/ci.yml:39) має Slint UI job.

### Цільовий стан

- frontend install
- typecheck
- frontend build
- Tauri build smoke test
- backend tests
- DB integration tests

### Ризик

Низький-середній.

## Етап 10. Фінальний cutover

### Прибираємо лише після green build

- `ui/*.slint`
- поточний [build.rs](/C:/Users/MykhailoDan/apps/Acta/build.rs:1)
- `slint`
- `slint-build`
- `i-slint-backend-testing`
- Slint bootstrap/wiring

### Ризик

Високий, якщо зробити передчасно.

## Найкращий практичний порядок виконання

1. Tauri scaffold
2. shared backend bootstrap
3. shell/navigation
4. `dashboard` як пілот
5. `documents`
6. `payments`
7. решта screens
8. тести
9. CI
10. видалення Slint

## Пов'язані документи

- [tauri-migration-audit-2026-04-29.md](./tauri-migration-audit-2026-04-29.md)
- [tauri-migration-contract-matrix-2026-04-29.md](./tauri-migration-contract-matrix-2026-04-29.md)
- [tauri-documents-command-spec-2026-04-29.md](./tauri-documents-command-spec-2026-04-29.md)
- [dashboard-migration-contract-2026-04-30.md](./dashboard-migration-contract-2026-04-30.md)

### Dashboard migration contract

- `dashboard implemented`
- `dashboard parity partial by design`
- `redesign-first, not strict Slint parity`

Dashboard: реалізовано у Tauri як redesign-first screen; strict parity зі Slint не є поточною ціллю.
