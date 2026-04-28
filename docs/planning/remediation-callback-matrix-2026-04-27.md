# Remediation Callback Matrix

Оновлено: `2026-04-28`  
Призначення: швидка карта того, що вже wired, що реально працює, а що ще лишається на наступний cleanup-шар.

## Documents

| UI дія | Callback | Поточний handler | DB effect | Refresh | Статус |
|---|---|---|---|---|---|
| Пошук документів | `doc_search_changed` | `src/ui/documents.rs` | ні | `Documents` | works |
| Перемикання tab | `doc_tab_changed` | `src/ui/documents.rs` | ні | `Documents` | works |
| Відправити / просунути статус | `doc_send` | `src/ui/documents.rs` | так | `Documents` | works |
| Видалити | `doc_delete` | `src/ui/documents.rs` | так | `Documents` | works |
| Створити новий | `doc_new` | `src/ui/documents.rs` | так / editor draft | editor state | works |
| Відкрити | `doc_open` | `src/ui/documents.rs` | ні | editor state | works |
| Редагувати | `doc_edit` | `src/ui/documents.rs` | так | editor state | works |
| More actions | `doc_more_actions` | `src/ui/documents.rs` | ні | локальний Slint menu state | works |
| Bulk send | `doc_bulk_send` | `src/ui/documents.rs` | так | `Documents` | works |
| Bulk archive | `doc_bulk_archive` | `src/ui/documents.rs` | так | `Documents` | works |
| Bulk delete | `doc_bulk_delete` | `src/ui/documents.rs` | так | `Documents` | works |
| Load chain | `doc_chain_load` | `src/bootstrap.rs` | ні | chain state | works |
| Create from chain | `doc_chain_create` | `src/bootstrap.rs` | так / draft create | editor state | works |

## Counterparties

| UI дія | Callback | Поточний handler | DB effect | Refresh | Статус |
|---|---|---|---|---|---|
| Пошук | `cp_search_changed` | `src/ui/counterparties.rs` | ні | `Counterparties` | works |
| Вибір контрагента | `cp_selected` | `src/ui/counterparties.rs` | ні | detail apply | works |
| Створити нового | `cp_new` | `src/ui/counterparties.rs` | так | `Counterparties` + detail | works |
| Редагувати | `cp_edit` | `src/ui/counterparties.rs` | так | `Counterparties` + detail | works |
| Створити документ | `cp_create_doc` | `src/ui/counterparties.rs` | так / document draft | documents editor | works |
| Перемкнути tab detail | `cp_tab_changed` | `src/ui/counterparties.rs` | ні | локальний Slint state | works |

## Payments

| UI дія | Callback | Поточний handler | DB effect | Refresh | Статус |
|---|---|---|---|---|---|
| Імпорт CSV | `pay_import_csv` | `src/ui/payments.rs` | так | `Payments` | works |
| Sync bank | `pay_sync_bank` | `src/ui/payments.rs` | так | `Payments` | works |
| Створити manually | `pay_new` | `src/ui/payments.rs` | файл-шаблон | ні | partial |
| Link payment | `pay_link` | `src/ui/payments.rs` | так | `Payments` | works |
| Unreconcile payment | `pay_unreconcile` | `src/ui/payments.rs` | так | `Payments` | works |

## Tasks

| UI дія | Callback | Поточний handler | DB effect | Refresh | Статус |
|---|---|---|---|---|---|
| Toggle task | `task_toggled` | `src/ui/tasks.rs` | так | current screen | works |
| Filter changed | `task_filter_changed` | `src/ui/tasks.rs` | ні | локальний Slint state | works |
| New task | `task_new` | `src/ui/tasks.rs` | ні | локальний Slint state | works |
| Save task | `task_save` | `src/ui/tasks.rs` | так | `Tasks` + `Dashboard` | works |
| More / details | `task_more` | `src/ui/tasks.rs` | ні | detail overlay | works |
| Status change from details | `task_status_set` | `src/ui/tasks.rs` | так | `Tasks` + `Dashboard` | works |
| Day view data | `day_events` | `src/ui/tasks.rs` | ні | `Tasks` | works |

## Settings

| UI дія | Callback | Поточний handler | DB effect | Refresh | Статус |
|---|---|---|---|---|---|
| Save company | `settings_company_saved` | `src/ui/settings.rs` | так | `Settings` + shell data | works |
| Configure integration | `settings_integration_configure` | `src/ui/settings.rs` | файл | `Settings` | works |
| Team invite | `settings_team_invite` | `src/ui/settings.rs` | файл | `Settings` | works |
| Backup now | `settings_backup_now` | `src/ui/settings.rs` | файл | `Settings` | partial |
| Backup download/open | `settings_backup_download` | `src/ui/settings.rs` | ні | ні | partial |

## Navigation / Shell

| UI дія | Callback | Поточний handler | DB effect | Refresh | Статус |
|---|---|---|---|---|---|
| Перехід між screens | `nav_changed` | `src/bootstrap.rs` | ні | current screen | works |
| Company switch | `company_selected` | `src/bootstrap.rs` | ні | all screens | works |
| Inbox action | `inbox_action` | `src/bootstrap.rs` | інколи так | dashboard/tasks/payments | partial |
| Palette search | `palette_query_changed` | `src/bootstrap.rs` | ні | palette items | works |
| Palette activate | `palette_item_activated` | `src/bootstrap.rs` | інколи ні | різні | partial |

## BAS Import CLI

| Дія | Entry point | Поточний handler | DB effect | Статус |
|---|---|---|---|---|
| Parse args | `src/bin/migrate.rs` | local parse | ні | works |
| `--help` | `src/bin/migrate.rs` | local print | ні | works |
| Dry-run/import preview | `src/bin/migrate.rs` | discovery + importer dispatch | так / preview | works |
| Counterparties XML import | `src/import/bas_counterparties.rs` | real importer | так | works |
| Contracts XML import | `src/import/bas_contracts.rs` | real importer | так | works |
| Acts XML import | `src/import/bas_acts.rs` | real importer | так | works |

## Висновок

Головні remediation workstreams по user-facing P1 scope уже закриті.

Те, що реально лишається після цієї матриці:

1. Planning/docs sync
2. Navigation/search/inbox orchestration cleanup
3. Подальше розширення BAS import pipeline
4. Backup/settings clarification
