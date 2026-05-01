# Tauri Command Surface

Оновлено: `2026-05-01`

## Призначення

Цей документ фіксує канонічний public Tauri invoke surface для live Svelte frontend.

Канонічні джерела truth:

- [src-tauri/src/lib.rs](/C:/Users/MykhailoDan/apps/Acta/src-tauri/src/lib.rs);
- [src-tauri/src/commands](/C:/Users/MykhailoDan/apps/Acta/src-tauri/src/commands);
- [src/tauri_api](/C:/Users/MykhailoDan/apps/Acta/src/tauri_api);
- [frontend/src/lib/api.ts](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/api.ts);
- [frontend/src/lib/types.ts](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/types.ts);
- [frontend/src/lib/stores](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/stores).

## Live matrix

| Slice | Public commands | Frontend contract |
| --- | --- | --- |
| Shell/navigation | `shell_load`, `shell_set_active_company`, `shell_palette_search`, `shell_palette_activate` | `shell.ts`, `palette.ts`, `navigation.ts`, `App.svelte` |
| Theme/settings chrome | `settings_load`, `settings_save_preferences`, `shell_load` | `settings.ts`, `theme.ts`, `SettingsScreen.svelte`, `App.svelte` |
| Dashboard | `dashboard_load` | `dashboard.ts`, `DashboardScreen.svelte` |
| Documents | `documents_list`, `document_open`, `document_create_draft`, `document_save`, `document_advance_status`, `document_delete`, `document_chain_get`, `document_chain_create_draft`, `documents_bulk_advance_status`, `documents_bulk_delete` | `documents.ts`, `DocumentsScreen.svelte` |
| Counterparties | `counterparties_list`, `counterparty_get`, `counterparty_open_editor`, `counterparty_save`, `counterparty_archive`, `counterparty_create_document_context` | `counterparties.ts`, `CounterpartiesScreen.svelte` |
| Payments | `payments_list`, `payments_import_latest_csv`, `payments_sync_bank`, `payments_open_manual_template`, `payment_create_or_update`, `payment_reconcile`, `payment_unreconcile` | `payments.ts`, `PaymentsScreen.svelte` |
| Tasks | `tasks_list`, `task_open_editor`, `task_save`, `task_delete`, `task_set_status` | `tasks.ts`, `TasksScreen.svelte` |
| Reports | `reports_load`, `reports_export_csv` | `reports.ts`, `ReportsScreen.svelte` |
| Settings | `settings_load`, `settings_save_preferences`, `settings_save_company`, `settings_configure_integration`, `settings_team_invite`, `settings_backup_now`, `settings_backup_open_latest` | `settings.ts`, `SettingsScreen.svelte` |
| BAS import | `import_bas_plan`, `import_bas_execute` | `import.ts`, `SettingsScreen.svelte` |

## Contract rules

- Public command існує лише тоді, коли для нього є live frontend path або тест.
- DTO names і casing мають збігатися між Rust serde `camelCase` і TypeScript types.
- Store після mutation або оновлює власний snapshot, або робить targeted reload.
- Theme/settings/shell state синхронізуються через backend-backed settings flow.

## Frontend-only state

У frontend мають лишатися:

- відкритість/закритість command palette;
- локальний query і highlighted state у palette;
- shortcuts overlay;
- keyboard handling для `Ctrl+1..7`, `Ctrl+K`, `Esc`;
- screen-local editor/open/selection state, якщо він не є domain state.

## Documents contract notes

- документні refs зберігаються у форматі `act:<uuid>`, `inv:<uuid>`, `wbl:<uuid>`;
- money/input values приходять з UI рядками і валідовуються у Rust;
- `chain-parent` у notes не можна втратити при save;
- bulk document actions є частиною live public surface лише поки для них є реальний frontend path.

## Payments contract notes

- date input підтримує `dd.mm.yyyy` і `yyyy-mm-dd`;
- сума приходить рядком і валідовується у Rust як `Decimal`;
- усі mutations повинні бути scoped до active company;
- `payments_sync_bank` і `payments_import_latest_csv` можуть мати однаковий backend import path, але різний user-facing flow.

## Counterparty/Documents boundary

Створення документа з контексту контрагента проходить через `counterparty_create_document_context`. Це canonical product path для create flow; нові helper-команди не слід відкривати публічно без окремого frontend use-case.

## Пов’язані документи

- [tauri-runtime.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/tauri-runtime.md)
- [app-state.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/app-state.md)
- [ui-canonicalization.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/ui-canonicalization.md)
