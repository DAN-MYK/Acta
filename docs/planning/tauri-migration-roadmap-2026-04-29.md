# Roadmap міграції на Tauri - post-cutover стан на 2026-04-30

## Канонічний стан

Tauri runtime є канонічним desktop shell для Acta. Поточний live UI складається з:

- `src-tauri/` - Tauri entrypoint, invoke handler і command wrappers.
- `frontend/src/` - Svelte screens, stores, typed frontend API.
- `src/tauri_api/` - backend DTO/command contract для Tauri.
- `src/` - домен, DB, імпорт, PDF та shared backend logic.

Root Slint runtime більше не є live runtime: `ui/`, `src/ui/`, root `build.rs`, `tests/ui_events.rs` і Slint dependencies відсутні в поточному дереві. Будь-які Slint-файли можна використовувати лише як історичну довідку з worktree/archive, а не як джерело truth для продуктового контракту.

## Що завершено

- Tauri scaffold піднято в `src-tauri/`.
- Svelte/Vite frontend є єдиним desktop UI.
- Shared Rust backend працює через `AppCtx` і `src/tauri_api/*`.
- Shell, navigation, command palette, company switcher і settings-backed theme flow перенесені в Tauri/Svelte.
- Основні vertical slices мають frontend stores/screens і Tauri commands: dashboard, documents, counterparties, payments, reports, tasks, settings, BAS import.
- Dashboard реалізований як backend-backed Tauri screen, не як placeholder.
- Slint runtime, wiring і Slint test suite прибрані з live root.
- CI має frontend checks, backend checks, DB vertical slice, Linux Tauri smoke і Windows Tauri build gate.

## Що є deliberate redesign

Dashboard у Tauri зафіксований як redesign-first screen, а не strict Slint parity migration. Це означає:

- Tauri dashboard показує operational summary: KPI, cashflow, recent documents, upcoming payments, urgent tasks.
- Старі Slint-only affordances `journal`, `inbox`, accounts sidebar, sparkline/delta strip і chart-first layout не вважаються незавершеним переносом.
- Якщо ці сценарії потрібні знову, вони мають входити в backlog як нові Tauri product requirements із власним контрактом, а не як механічне відновлення Slint UI.

## Архівна Slint reference

Slint можна дивитися тільки як historical reference:

- `.worktrees/sprint-2026-04-24/ui/dashboard.slint`
- `.worktrees/sprint-2026-04-24/src/ui/dashboard.rs`
- `.worktrees/slint-audit-2026-04-24/`
- planning/audit docs, які явно позначені як pre-cutover або archived reference

Не використовувати як live source:

- `ui/app.slint`
- `ui/*.slint`
- `src/ui/*`
- `src/bootstrap/*` Slint wiring
- `tests/ui_events.rs`

Якщо новий документ посилається на Slint, він має прямо казати: archived reference, not live runtime.

## Live contract rule

Для будь-якої зміни frontend/backend contract синхронізувати весь ланцюг:

- `src-tauri/src/commands/*`
- `src-tauri/src/lib.rs`
- `src/tauri_api/*`
- `frontend/src/lib/api.ts`
- `frontend/src/lib/types.ts`
- відповідний store у `frontend/src/lib/stores/`
- відповідний screen у `frontend/src/lib/screens/`
- frontend/Rust тести
- planning docs, якщо змінюється public surface або продуктове рішення

Public Tauri invoke surface має відповідати поточному frontend product surface. Backend helper у `src/tauri_api/*` не стає public command автоматично.

## Поточний backlog

### P0

Немає відкритих P0 для cutover. Tauri є канонічним runtime.

### P1

- ✅ `2026-04-30`: app-level Tauri e2e smoke додано в `e2e-tests/`; CI job `tauri-e2e-smoke` запускає реальний WebView через `tauri-driver` + `xvfb`.
- ✅ `2026-04-30`: Windows CI розширено до packaging gate через `npm run tauri build` і artifact upload з `src-tauri/target/release/bundle/**`.
- ✅ `2026-04-30`: frontend component tests додано для ризикових screen-рівнів: dashboard, documents, payments.

### P2

- Винести dashboard `journal`, `inbox` або accounts у нові Tauri feature specs, якщо вони знову стануть продуктовою потребою.
- ✅ `2026-04-30`: старі migration/audit/architecture docs позначено або переписано як archived/pre-cutover там, де Slint більше не є live runtime.
- ✅ `2026-04-30`: довгострокову design-system опору перенесено в `docs/architecture/svelte-tauri-design-system.md`.

## CI contract

Мінімальний post-cutover CI має ловити такі класи регресій:

- frontend typecheck/build/store+component tests - `npm run check`, `npm run build`, `npm run test:frontend`;
- Rust backend compile/unit tests - `cargo check`, backend tests;
- DB-backed vertical slice - `cargo test --test tauri_vertical_slice`;
- Linux Tauri compile smoke - `cargo check --manifest-path src-tauri/Cargo.toml`;
- Linux real WebView e2e smoke - `xvfb-run -a npm run test:e2e`;
- Windows Tauri packaging gate - `npm run tauri build` на `windows-latest`.

Windows job потрібен окремо, бо Windows-specific Tauri compile/link/bundling regressions не гарантується зловити Linux `cargo check`.

## Definition of done для майбутніх cutover-змін

- Немає live-посилань на видалений Slint runtime.
- Кожен public command має frontend use-case або тест, який пояснює його наявність.
- Frontend theme/settings/shell state синхронізовані через backend-backed settings.
- Required checks green:
  - `cargo build --manifest-path src-tauri/Cargo.toml`
  - `cargo test --no-run`
  - `cargo test --test tauri_vertical_slice`
  - `npm run check`
  - `npm run test:frontend`, якщо змінювалась frontend logic

## Пов'язані документи

- [tauri-migration-audit-2026-04-29.md](./tauri-migration-audit-2026-04-29.md)
- [tauri-migration-contract-matrix-2026-04-29.md](./tauri-migration-contract-matrix-2026-04-29.md)
- [tauri-documents-command-spec-2026-04-29.md](./tauri-documents-command-spec-2026-04-29.md)
- [dashboard-migration-contract-2026-04-30.md](./dashboard-migration-contract-2026-04-30.md)
