# Next Sprint Checklist

> **Archived/pre-cutover:** це Slint-era checklist. Після `2026-04-30` live UI, test safety net і design-system рішення ведуться через Tauri/Svelte docs; посилання на `ui/*.slint`, `src/ui/*` і `tests/ui_events.rs` тут не є поточним contract.

Оновлено: `2026-04-27`
Статус: `historical execution checklist`

> Це execution checklist для плану від `2026-04-24`.
> Галочки тут відображають планову чергу робіт, а не підтверджений факт виконання.
> Для фактичного статусу див. [sprint-report-2026-04-24.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/sprint-report-2026-04-24.md).

## Sprint Goal

- [x] Закрити головні user-facing stub flow
- [x] Довести BAS import до реального MVP
- [x] Довести payments import/reconcile до цілісного flow

## Workstream 1. BAS Import MVP

- [x] Уточнити supported input format для BAS export
- [x] Реалізувати file discovery у `src/bin/migrate.rs` / `src/import/`
- [x] Реалізувати orchestration імпорту
- [x] Реалізувати `--dry-run`
- [x] Додати зрозумілі помилки для unsupported/invalid input
- [x] Додати тести на happy path
- [x] Додати тести на failure path

## Workstream 2. Documents Screen

- [x] Реалізувати `doc_new`
- [x] Реалізувати `doc_open`
- [x] Реалізувати `doc_edit`
- [x] Прийняти рішення по `doc_more_actions`
- [x] Прийняти рішення по `doc_bulk_send`
- [x] Прийняти рішення по `doc_bulk_archive`
- [x] Прийняти рішення по `doc_bulk_delete`
- [x] Прийняти рішення по `doc_chain_load`
- [x] Прийняти рішення по `doc_chain_create`
- [x] Прибрати misleading UI для ще неготових дій

## Workstream 3. Tasks

- [x] Реалізувати `new-task` flow
- [x] Реалізувати `task-more` як details/edit flow
- [x] Наповнити `day-events`
- [x] Увімкнути пошук/фільтрацію через `TaskListState.query`
- [x] Оновити або додати `ui_events` coverage для tasks

## Workstream 4. Payments

- [x] Підключити користувацький flow імпорту CSV
- [x] Додати callback wiring для import flow
- [x] Реалізувати `unreconcile-payment`
- [x] Реалізувати reconcile UI flow
- [x] Перевірити duplicate handling через `bank_ref`
- [x] Додати тести на reconcile/unreconcile

## Verification

- [x] Прогнати релевантні unit-тести
- [x] Прогнати `tests/ui_events.rs`
- [x] Прогнати інтеграційні тести для змінених db-flow — потребує живої БД
- [x] Перевірити, що нові callback-и не лишилися в no-op стані

## Sprint Exit Criteria

- [x] Усі P1 workstream-и або завершені, або свідомо перенесені з documented reason
- [x] У репозиторії немає нових критичних user-facing `TODO`
- [x] Документація оновлена відповідно до фактичного результату
