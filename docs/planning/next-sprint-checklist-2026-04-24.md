# Next Sprint Checklist

Оновлено: `2026-04-24`
Статус: `open`

## Sprint Goal

- [ ] Закрити головні user-facing stub flow
- [ ] Довести BAS import до реального MVP
- [ ] Довести payments import/reconcile до цілісного flow

## Workstream 1. BAS Import MVP

- [ ] Уточнити supported input format для BAS export
- [ ] Реалізувати file discovery у `src/bin/migrate.rs` / `src/import/`
- [ ] Реалізувати orchestration імпорту
- [ ] Реалізувати `--dry-run`
- [ ] Додати зрозумілі помилки для unsupported/invalid input
- [ ] Додати тести на happy path
- [ ] Додати тести на failure path

## Workstream 2. Documents Screen

- [ ] Реалізувати `doc_new`
- [ ] Реалізувати `doc_open`
- [ ] Реалізувати `doc_edit`
- [ ] Прийняти рішення по `doc_more_actions`
- [ ] Прийняти рішення по `doc_bulk_send`
- [ ] Прийняти рішення по `doc_bulk_archive`
- [ ] Прийняти рішення по `doc_bulk_delete`
- [ ] Прийняти рішення по `doc_chain_load`
- [ ] Прийняти рішення по `doc_chain_create`
- [ ] Прибрати misleading UI для ще неготових дій

## Workstream 3. Tasks

- [ ] Реалізувати `new-task` flow
- [ ] Реалізувати `task-more` як details/edit flow
- [ ] Наповнити `day-events`
- [ ] Увімкнути пошук/фільтрацію через `TaskListState.query`
- [ ] Оновити або додати `ui_events` coverage для tasks

## Workstream 4. Payments

- [ ] Підключити користувацький flow імпорту CSV
- [ ] Додати callback wiring для import flow
- [ ] Реалізувати `unreconcile-payment`
- [ ] Реалізувати reconcile UI flow
- [ ] Перевірити duplicate handling через `bank_ref`
- [ ] Додати тести на reconcile/unreconcile

## Verification

- [ ] Прогнати релевантні unit-тести
- [ ] Прогнати `tests/ui_events.rs`
- [ ] Прогнати інтеграційні тести для змінених db-flow
- [ ] Перевірити, що нові callback-и не лишилися в no-op стані

## Sprint Exit Criteria

- [ ] Усі P1 workstream-и або завершені, або свідомо перенесені з documented reason
- [ ] У репозиторії немає нових критичних user-facing `TODO`
- [ ] Документація оновлена відповідно до фактичного результату
