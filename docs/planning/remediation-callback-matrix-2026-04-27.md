# Remediation Callback Matrix

Оновлено: `2026-04-27`  
Призначення: швидка карта того, що вже wired, що реально працює, а що ще потребує remediation.

## Documents

| UI дія | Callback | Поточний handler | DB effect | Refresh | Статус |
|---|---|---|---|---|---|
| Пошук документів | `doc_search_changed` | `src/ui/documents.rs` | ні | `Documents` | works |
| Перемикання tab | `doc_tab_changed` | `src/ui/documents.rs` | ні | `Documents` | works |
| Відправити / просунути статус | `doc_send` | `src/ui/documents.rs` | так | `Documents` | works |
| Видалити | `doc_delete` | `src/ui/documents.rs` | так | `Documents` | works |
| Створити новий | `doc_new` | `src/ui/documents.rs` | ні | ні | TODO |
| Відкрити | `doc_open` | `src/ui/documents.rs` | ні | ні | TODO |
| Редагувати | `doc_edit` | `src/ui/documents.rs` | ні | ні | TODO |
| More actions | `doc_more_actions` | `src/ui/documents.rs` | ні | ні | TODO |
| Bulk send | `doc_bulk_send` | `src/ui/documents.rs` | ні | ні | TODO |
| Bulk archive | `doc_bulk_archive` | `src/ui/documents.rs` | ні | ні | TODO |
| Bulk delete | `doc_bulk_delete` | `src/ui/documents.rs` | ні | ні | TODO |
| Load chain | `doc_chain_load` | `src/bootstrap.rs` | ні | локальний reset | TODO |
| Create from chain | `doc_chain_create` | `src/bootstrap.rs` | ні | ні | TODO |

## Counterparties

| UI дія | Callback | Поточний handler | DB effect | Refresh | Статус |
|---|---|---|---|---|---|
| Пошук | `cp_search_changed` | `src/ui/counterparties.rs` | ні | `Counterparties` | works |
| Вибір контрагента | `cp_selected` | `src/ui/counterparties.rs` | ні | detail apply | works |
| Створити нового | `cp_new` | `src/ui/counterparties.rs` | ні | ні | TODO |
| Створити документ | `cp_create_doc` | `src/ui/counterparties.rs` | ні | ні | TODO |
| Перемкнути tab detail | `cp_tab_changed` | `src/ui/counterparties.rs` | ні | локальний Slint state | partial |

## Payments

| UI дія | Callback | Поточний handler | DB effect | Refresh | Статус |
|---|---|---|---|---|---|
| Імпорт CSV | `pay_import_csv` | `src/ui/payments.rs` | так | `Payments` | works |
| Sync bank | `pay_sync_bank` | `src/ui/payments.rs` | так | `Payments` | works |
| Створити manually | `pay_new` | `src/ui/payments.rs` | файл-шаблон | ні | partial |
| Link payment | `pay_link` | `src/ui/payments.rs` | так | `Payments` | works |

## Tasks

| UI дія | Callback | Поточний handler | DB effect | Refresh | Статус |
|---|---|---|---|---|---|
| Toggle task | `task_toggled` | `src/ui/tasks.rs` | так | current screen | works |
| Filter changed | `task_filter_changed` | `src/ui/tasks.rs` | ні | локальний Slint state | works |
| New task | `task_new` | `src/ui/tasks.rs` | ні | локальний Slint state | works |
| Save task | `task_save` | `src/ui/tasks.rs` | так | `Tasks` + `Dashboard` | works |
| More | `task_more` | `src/ui/tasks.rs` | ні | ні | partial |

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
| Dry-run/import | `src/bin/migrate.rs` | заглушка | ні | TODO |

## Висновок

Головні remediation workstreams прямо випливають з matrix:

1. Documents completion.
2. Counterparties create/document bridge.
3. BAS import pipeline.
4. Navigation/search/inbox orchestration extraction.
