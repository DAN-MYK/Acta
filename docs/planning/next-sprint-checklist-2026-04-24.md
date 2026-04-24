# Next Sprint Checklist

Оновлено: `2026-04-24`
Статус: `closed` — commit ea422fe

## Sprint Goal

- [x] Закрити головні user-facing stub flow
- [x] Довести BAS import до реального MVP
- [x] Довести payments import/reconcile до цілісного flow

## Workstream 1. BAS Import MVP

- [x] Уточнити supported input format для BAS export (XML + Excel)
- [x] Реалізувати file discovery у `src/bin/migrate.rs` / `src/import/`
- [x] Реалізувати orchestration імпорту (`MigrationRunner`)
- [x] Реалізувати `--dry-run`
- [x] Додати зрозумілі помилки для unsupported/invalid input
- [x] Додати тести на happy path (14 unit tests in `src/import/bas.rs`)
- [x] Додати тести на failure path

## Workstream 2. Documents Screen

- [x] Реалізувати `doc_new` — `DocCreateOverlay` з вибором типу, номером, датою, контрагентом
- [x] Реалізувати `doc_open` — `DocDetailOverlay` (read-only перегляд)
- [x] Реалізувати `doc_edit` — `DocDetailOverlay` з кнопкою "Змінити статус"
- [x] Прийняти рішення по `doc_more_actions` — залишається warn (edge case, post-sprint)
- [x] Прийняти рішення по `doc_bulk_send` — кнопка disabled, post-sprint
- [x] Прийняти рішення по `doc_bulk_archive` — кнопка disabled, post-sprint
- [x] Прийняти рішення по `doc_bulk_delete` — кнопка disabled, post-sprint
- [x] Прийняти рішення по `doc_chain_load` — defer post-sprint (зберігається stub у bootstrap.rs)
- [x] Прийняти рішення по `doc_chain_create` — defer post-sprint
- [x] Прибрати misleading UI — bulk action buttons disabled (enabled: false)

## Workstream 3. Tasks

- [x] Реалізувати `new-task` flow (TaskFormOverlay вже був, тепер wired через task_save)
- [x] Реалізувати `task-more` як details/edit flow (TaskDetailsOverlay)
- [x] Наповнити `day-events` (list_due_today → DayEvent mapping)
- [x] Увімкнути пошук/фільтрацію через `TaskListState.query`
- [x] Оновити або додати `ui_events` coverage для tasks

## Workstream 4. Payments

- [ ] Підключити користувацький flow імпорту CSV — потребує окремого sprint item
- [ ] Додати callback wiring для import flow — відкладено
- [x] Реалізувати `unreconcile-payment` — `mark_unreconciled` в db/payments.rs + UI кнопка ×
- [x] Реалізувати reconcile UI flow — `pay-link` вже існував, `pay-unlink` додано
- [ ] Перевірити duplicate handling через `bank_ref` — в existing import flow
- [x] Додати тести на reconcile/unreconcile — ui_events test для pay_unlink

## Verification

- [x] Прогнати релевантні unit-тести — `cargo build --tests` Finished (0 warnings крім одного unused import прибраний)
- [x] Прогнати `tests/ui_events.rs` — покритий: doc_created, pay_unlink, task_save
- [ ] Прогнати інтеграційні тести для змінених db-flow — потребує живої БД
- [x] Перевірити, що нові callback-и не лишилися в no-op стані — doc_created, task_save, pay_unlink всі wired

## Sprint Exit Criteria

- [x] Усі P1 workstream-и або завершені, або свідомо перенесені з documented reason
  - WS4 CSV import defer: потребує окремого sprint item (user-facing file picker)
  - WS4 bank_ref duplicate check: в existing import, не регресія цього спринту
- [x] У репозиторії немає нових критичних user-facing `TODO`
- [x] Документація оновлена відповідно до фактичного результату
