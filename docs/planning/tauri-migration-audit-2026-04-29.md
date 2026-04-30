# Аудит міграції на Tauri — оновлено 2026-04-30

## Стан на 2026-04-30

Міграція вже пройшла cutover на Tauri desktop shell. Канонічний runnable UI живе в `src-tauri/` + `frontend/`. Slint runtime, `ui/`, `build.rs`, `src/main.rs` і `tests/ui_events.rs` уже відсутні в робочому дереві, тому старі Slint-плани треба читати як архівні нотатки, а не як опис поточного runtime.

| Етап | Назва | Стан |
|------|-------|------|
| 1 | Tauri scaffold | ✅ Завершено |
| 2 | Shared backend bootstrap | ✅ Завершено |
| 3 | Command contract | ✅ Завершено |
| 4 | Shell + feature screens (Svelte) | 🟡 В процесі |
| 5 | Refresh/wiring модель | ✅ Завершено в invoke/store flow |
| 6 | Дизайн-система | 🟡 В процесі |
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
