# Tauri migration contract matrix — 2026-04-29

## Призначення

Цей документ фіксує відповідність між поточними Slint callback/property контрактами та майбутніми Tauri commands і frontend stores.

## Matrix

| Поточний контракт | Де зараз | Майбутній Tauri backend | Frontend | Пріоритет |
| --- | --- | --- | --- | --- |
| `nav_changed(screen)` | [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:129) | не command | `navigation.ts` | P0 |
| `Shell.navigate(screen)` | [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:21) | не command | `navigation.ts`, `Shell.svelte` | P0 |
| `company-selected(id)` | [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:34) | `set_active_company(...)` | `shell.ts` | P0 |
| `company-manage-requested` | [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:35) | залежить від UX сценарію | `shell.ts` | P1 |
| `toggle-theme` | [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:36) | опційно `save_theme(...)` | `theme.ts` | P1 |
| `open-cmd-palette` / `close-cmd-palette` | [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:37) | не command | `palette.ts` | P1 |
| `CommandPalette.query-changed(value)` | [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:65) | опційно `search_global(...)` | `palette.ts` | P1 |
| `CommandPalette.navigated(screen)` | [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:56) | не command | `navigation.ts` | P1 |
| `inbox_action(id, action)` | [src/bootstrap/inbox.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/inbox.rs:13) | `handle_inbox_action(...)` або явні commands | `inbox.ts` | P2 |
| `doc_search_changed(query)` | [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1047) | `documents_list(...)` | `documents.ts` | P0 |
| `doc_tab_changed(tab)` | [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1063) | `documents_list(...)` або frontend-only filter | `documents.ts` | P0 |
| `doc_toggled(id, selected)` | [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1274) | не command | `documents.ts` | P1 |
| `wire_document_callbacks(...)` | [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1046) | `documents_list`, `document_open`, `document_save`, `document_delete`, `document_advance_status`, `document_chain_*` | `documents.ts`, `Documents.svelte` | P0 |
| `apply_documents_to_ui(...)` | [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:93) | DTO response | `documents.ts` | P0 |
| `wire_counterparty_callbacks(...)` | [src/ui/counterparties.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:229) | `load_counterparties`, `save_counterparty`, `delete_counterparty` | `counterparties.ts` | P0 |
| `apply_counterparties_to_ui(...)` | [src/ui/counterparties.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:111) | DTO response | `counterparties.ts` | P0 |
| `wire_payment_callbacks(...)` | [src/ui/payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:364) | `load_payments`, `save_payment`, `delete_payment`, `link_payment`, `reconcile_payment` | `payments.ts` | P0 |
| `apply_payments_to_ui(...)` | [src/ui/payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:61) | DTO response | `payments.ts` | P0 |
| `wire_task_callbacks(...)` | [src/ui/tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:239) | `load_tasks`, `save_task`, `toggle_task`, `delete_task` | `tasks.ts` | P1 |
| `apply_tasks_to_ui(...)` | [src/ui/tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:65) | DTO response | `tasks.ts` | P1 |
| `wire_reports_callbacks(...)` | [src/ui/reports.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/reports.rs:370) | `load_reports`, `export_report` | `reports.ts` | P1 |
| `apply_reports_to_ui(...)` | [src/ui/reports.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/reports.rs:247) | DTO response | `reports.ts` | P1 |
| `wire_settings_callbacks(...)` | [src/ui/settings.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/settings.rs:512) | `load_settings`, `save_settings`, `test_integration` | `settings.ts` | P1 |
| `apply_settings_to_ui(...)` | [src/ui/settings.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/settings.rs:498) | DTO response | `settings.ts` | P1 |
| dashboard initial load | [src/ui/dashboard.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/dashboard.rs:145) | `load_dashboard` | `dashboard.ts` | P0 |
| shell state load/apply | [src/bootstrap/shell.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/shell.rs) | `load_shell_state` або кілька точкових commands | `shell.ts` | P0 |
| `refresh_all_ui` / `refresh_screen` | [src/bootstrap/refresh.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/refresh.rs:99) | targeted re-fetch | усі stores | P0 |
| document chain callbacks | [src/bootstrap/document_chain.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/document_chain.rs:9) | `document_chain_get`, `document_chain_create_draft` | `documents.ts` | P2 |

## Пріоритети

### P0

Без цього Tauri UI не стане робочою заміною Slint.

### P1

Потрібно для повноцінного cutover, але не обов'язково для першого vertical slice.

### P2

Можна переносити після стабілізації базових flows.

## Правило міграції

Усе, що зараз є:

- `apply_*_to_ui(...)`
- `wire_*_callbacks(...)`
- `Weak<AppWindow>`
- `ModelRc` / `VecModel`

має перейти в одну з двох форм:

- Rust `#[tauri::command]`, якщо це дані або mutation;
- frontend store/local state, якщо це чисто UI-стан.

## Рекомендований порядок переносу

1. navigation + shell
2. dashboard
3. documents
4. counterparties
5. payments
6. tasks
7. reports
8. settings
9. inbox
10. document chain

## Пов'язані документи

- [tauri-migration-audit-2026-04-29.md](./tauri-migration-audit-2026-04-29.md)
- [tauri-migration-roadmap-2026-04-29.md](./tauri-migration-roadmap-2026-04-29.md)
- [tauri-documents-command-spec-2026-04-29.md](./tauri-documents-command-spec-2026-04-29.md)
