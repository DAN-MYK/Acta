# Tauri contract matrix - post-cutover стан на 2026-04-30

## Призначення

Цей документ більше не є матрицею перенесення Slint callback/property contract. Після cutover він фіксує live Tauri invoke surface, відповідний frontend contract і рішення щодо archived Slint references.

Канонічний продукт зараз визначають:

- `src-tauri/src/lib.rs` - public invoke handler.
- `src-tauri/src/commands/*` - Tauri command wrappers.
- `src/tauri_api/*` - backend DTO і mutation/load contract.
- `frontend/src/lib/api.ts` - frontend invoke calls.
- `frontend/src/lib/types.ts` - TypeScript DTO mirror.
- `frontend/src/lib/stores/*` і `frontend/src/lib/screens/*` - product UX surface.

## Live Tauri matrix

| Slice | Public commands | Frontend contract | Notes |
| --- | --- | --- | --- |
| Shell/navigation | `shell_load`, `shell_set_active_company`, `shell_palette_search`, `shell_palette_activate` | `shell.ts`, `palette.ts`, `navigation.ts`, `App.svelte` | Navigation stays frontend-local; shell state comes from backend. |
| Theme/settings chrome | `settings_load`, `settings_save_preferences`, `shell_load` | `settings.ts`, `theme.ts`, `SettingsScreen.svelte`, `App.svelte` | Sidebar quick toggle persists via settings save and re-syncs from shell `isDark`. |
| Dashboard | `dashboard_load` | `dashboard.ts`, `DashboardScreen.svelte` | Redesign-first operational summary, not strict Slint dashboard parity. |
| Documents | `documents_list`, `document_open`, `document_create_draft`, `document_save`, `document_advance_status`, `document_delete`, `document_chain_get`, `document_chain_create_draft` | `documents.ts`, `DocumentsScreen.svelte`, `api.ts`, `types.ts` | Public surface is single-item document flow plus chain flow. |
| Counterparties | `counterparties_list`, `counterparty_get`, `counterparty_open_editor`, `counterparty_save`, `counterparty_archive`, `counterparty_create_document_context` | `counterparties.ts`, `CounterpartiesScreen.svelte` | Create-document context lives here, not in an extra `document_prepare_new` command. |
| Payments | `payments_list`, `payments_import_latest_csv`, `payments_sync_bank`, `payments_open_manual_template`, `payment_create_or_update`, `payment_reconcile`, `payment_unreconcile` | `payments.ts`, `PaymentsScreen.svelte` | Current UI supports list, manual editor, import/sync/template and reconcile actions. |
| Tasks | `tasks_list`, `task_open_editor`, `task_save`, `task_delete`, `task_set_status` | `tasks.ts`, `TasksScreen.svelte`, dashboard drill-in | Dashboard reuses task editor/status contracts; no dashboard-only task command. |
| Reports | `reports_load`, `reports_export_csv` | `reports.ts`, `ReportsScreen.svelte` | Reports filter is frontend DTO mirrored to Rust request. |
| Settings | `settings_load`, `settings_save_preferences`, `settings_save_company`, `settings_configure_integration`, `settings_team_invite`, `settings_backup_now`, `settings_backup_open_latest` | `settings.ts`, `SettingsScreen.svelte` | Appearance, company, integrations, team and backup flows are backend-backed. |
| BAS import | `import_bas_plan`, `import_bas_execute` | `import.ts`, `SettingsScreen.svelte` | Import is exposed from settings/integrations UI. |

## Commands intentionally not public

These backend functions are not part of the current frontend product surface and must not be registered in `tauri::generate_handler!` unless a real frontend path and tests are added:

| Command/function | Decision | Reason |
| --- | --- | --- |
| `document_prepare_new` | Not public | Current product uses `counterparty_create_document_context` plus `document_create_draft`. |
| `documents_bulk_advance_status` | Not public | No bulk-selection UX exists in the Svelte documents screen. |
| `documents_bulk_delete` | Not public | No bulk-delete UX exists in the Svelte documents screen. |

If any of these return, update all layers in one change: `src-tauri/src/commands/*`, `src-tauri/src/lib.rs`, `src/tauri_api/*`, `frontend/src/lib/api.ts`, `frontend/src/lib/types.ts`, store, screen, tests and docs.

## Archived Slint reference policy

Slint callbacks/properties are no longer live contracts. Historical references are allowed only when explicitly labeled as archived:

| Archived reference | Current Tauri decision |
| --- | --- |
| `.worktrees/sprint-2026-04-24/ui/dashboard.slint` | Historical dashboard UI reference only. |
| `.worktrees/sprint-2026-04-24/src/ui/dashboard.rs` | Historical dashboard data-prep reference only. |
| Slint `journal`/`inbox` dashboard modes | Deliberate cut from current Tauri dashboard, not unfinished migration. |
| Slint accounts sidebar and chart-first layout | Deliberate redesign; reintroduce only through a new Tauri feature spec. |
| Slint `tests/ui_events.rs` callbacks | Replaced by frontend store tests, Rust integration tests and Tauri build checks. |

Do not cite `ui/app.slint`, `src/ui/*`, root `build.rs` or `tests/ui_events.rs` as live files in new planning docs.

## Backlog contract matrix

| Backlog item | Contract/status | Priority |
| --- | --- | --- |
| Real WebView e2e smoke | Implemented in `e2e-tests/`; CI runs Tauri shell through `tauri-driver` and validates navigation | P1 done 2026-04-30 |
| Windows packaging gate | Implemented in `.github/workflows/ci.yml` via `npm run tauri build` + bundle artifact upload | P1 done 2026-04-30 |
| Dashboard journal/inbox revival | New Tauri DTOs, commands and screens; not a Slint parity task | P2 |
| Bulk document actions | Bulk-selection UX, frontend API functions and Rust command tests | P2 |
| Svelte design-system docs | Implemented in `docs/architecture/svelte-tauri-design-system.md`; Slint token docs are archived/pre-cutover | P2 done 2026-04-30 |

## Review checklist for contract changes

- Public command exists only when `frontend/src/lib/api.ts` calls it or a documented product path is being added in the same change.
- DTO names and casing match between Rust serde `camelCase` and TypeScript types.
- Store mutation refreshes only the slices it owns or explicitly coordinates.
- Theme changes persist through `settings_save_preferences` and re-sync shell state.
- Tests cover both frontend contract and backend vertical slice when behavior changes.
