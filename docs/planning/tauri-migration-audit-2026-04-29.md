# Аудит міграції на Tauri — оновлено 2026-04-30

## Стан на 2026-04-30

Міграція вже пройшла cutover на Tauri desktop shell. Канонічний runnable UI живе в `src-tauri/` + `frontend/`. Slint runtime, `ui/`, `build.rs`, `src/main.rs` і `tests/ui_events.rs` уже відсутні в робочому дереві, тому старі Slint-плани треба читати як архівні нотатки, а не як опис поточного runtime.

| Етап | Назва | Стан |
|------|-------|------|
| 1 | Tauri scaffold | ✅ Завершено |
| 2 | Shared backend bootstrap | ✅ Завершено |
| 3 | Command contract | ✅ Завершено |
| 4 | Shell + feature screens (Svelte) | ✅ Завершено |
| 5 | Refresh/wiring модель | ✅ Завершено в invoke/store flow |
| 6 | Дизайн-система | ✅ Завершено як migration foundation |
| 7 | Тести | 🟢 Є Rust vertical slice, Vitest store+component tests і Tauri WebView e2e smoke |
| 8 | CI/checks | 🟢 Є frontend/backend/DB gates, Linux Tauri smoke, Linux WebView e2e smoke і Windows packaging gate |
| 9 | Фінальний cutover (видалення Slint) | ✅ Завершено |

## Актуальний runtime

- `src-tauri/src/main.rs` + `src-tauri/src/lib.rs` — єдиний desktop entrypoint.
- `src-tauri/src/commands/` — shell, dashboard, counterparties, documents, payments, tasks, reports, settings, import.
- `frontend/src/App.svelte` + `frontend/src/lib/screens/*` — активний UI шар.
- `frontend/src/lib/api.ts` — Tauri invoke layer.
- `frontend/src/lib/stores/*` — orchestration на фронтенді.
- `src/tauri_api/*` — backend DTO/command surface для Tauri.

## Shell + feature screens статус

Етап `Shell + feature screens (Svelte)` для цілей cutover слід вважати завершеним:

- root shell живе в `frontend/src/App.svelte`;
- live screens існують для `dashboard`, `documents`, `counterparties`, `payments`, `reports`, `tasks`, `settings`;
- для цих slices є відповідні frontend stores у `frontend/src/lib/stores/*`;
- public Tauri commands для shell і feature screens зареєстровані в `src-tauri/src/lib.rs`.

Отже, статус `в процесі` для цього етапу більше не є коректним як migration-статус. Незакриті UX-питання треба трактувати як post-cutover product/UI backlog, а не як незавершений перенос shell/screens у Tauri.

## Дизайн-система статус

Етап `Дизайн-система` для цілей міграції також слід вважати завершеним на рівні foundation:

- канонічні web tokens живуть у `frontend/src/lib/styles/tokens.css`;
- глобальні layout/style rules живуть у `frontend/src/styles.css`;
- канонічну design-system опору для live runtime зафіксовано в `docs/architecture/svelte-tauri-design-system.md`.

Водночас це не означає, що весь UI polish завершено. Подальша уніфікація кнопок, empty states, form controls, action bars і screen-level UX належить до окремого post-cutover roadmap, а не до відкритого migration blocker.

## CI та packaging gates

- `tauri-e2e-smoke` у [.github/workflows/ci.yml](/C:/Users/MykhailoDan/apps/Acta/.github/workflows/ci.yml) вже є live gate для реального WebView smoke на Linux.
- `tauri-windows-build` у тому ж workflow вже є live release-oriented gate для Windows packaging і збирає `src-tauri/target/release/bundle/**`.
- Отже, окремий новий Windows packaging gate для цього sprint не потрібен: вимога вже активна в CI і має підтримуватися як release requirement, а не як future TODO.
- Якщо змінюється `tauri.conf.json`, signing/bundle resources або installer wiring, зміни треба валідувати проти цього існуючого Windows gate, а не створювати паралельний другий gate.

## Shared backend

`acta::runtime` лишається спільним bootstrap/runtime шаром для Tauri та тестів:

- `src/runtime.rs` — `connect_pool`, `run_migrations`, `init_app_ctx`, `spawn_background_tasks`.
- `src/app_ctx.rs` — `AppCtx` для `PgPool` + active company.
- `tests/tauri_vertical_slice.rs` використовує той самий runtime surface для smoke/end-to-end перевірок.

## Dashboard стан

Поточний dashboard є redesign-first реалізацією, а не strict parity-копією Slint. Стан contract на 2026-04-30:

- KPI, cashflow, recent acts, urgent tasks — backend-backed.
- Upcoming payments тепер віддають `id` реального payment record і підтримують drill-in у payment editor.
- Recent acts відкривають конкретний documents context.
- Urgent tasks відкривають конкретний task editor.

## Що вже неактуально

Нижче наведені твердження більше не відповідають дереву і не мають використовуватись як поточний опис системи:

- що Slint є чинним runtime UI;
- що `src/main.rs`, `build.rs`, `ui/` або `tests/ui_events.rs` ще існують;
- що cutover на Tauri ще не відбувся;
- що dashboard upcoming payments є read-only блоком без drill-in.

## Пов’язані документи

- [dashboard-migration-contract-2026-04-30.md](./dashboard-migration-contract-2026-04-30.md)
- [tauri-documents-command-spec-2026-04-29.md](./tauri-documents-command-spec-2026-04-29.md)
- [tauri-counterparties-command-spec-2026-04-29.md](./tauri-counterparties-command-spec-2026-04-29.md)
- [slint-final-cascade-removal-2026-04-30.md](./slint-final-cascade-removal-2026-04-30.md)
- [slint-safe-removal-checklist-2026-04-30.md](./slint-safe-removal-checklist-2026-04-30.md)
