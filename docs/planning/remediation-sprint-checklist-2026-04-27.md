# Remediation Sprint Checklist

> **Archived/pre-cutover:** цей checklist описує Slint-era remediation. Після `2026-04-30` не використовуй `tests/ui_events.rs` або `ui/*.slint` з цього документа як live quality gate.

Оновлено: `2026-04-27`
Статус: `open`
План: [remediation-sprint-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-sprint-2026-04-27.md)

## Goal

- [x] Прибрати головні user-facing `TODO`
- [x] Довести `tasks` до реального MVP flow
- [x] Прибрати двозначність по `BAS import` і `payments`

## Workstream 1. Documents

- [x] Реалізувати `doc_new`
- [x] Реалізувати `doc_open`
- [x] Реалізувати `doc_edit`
- [x] Визначити долю `doc_more_actions`
- [x] Визначити долю bulk-дій
- [x] Якщо треба, прибрати misleading UI/state для нереалізованих дій

## Workstream 2. Tasks

- [x] Реалізувати `task_more` як корисний flow
- [x] Наповнити `day_events`
- [x] Перевірити `task_save` end-to-end behavior
- [x] Оновити `ui_events` або суміжні тести

## Workstream 3. BAS Import

- [x] Прибрати фінальну `TODO`-заглушку з `migrate.rs`
- [x] Додати file discovery
- [x] Додати реальний `dry-run`
- [x] Додати базові тести

## Workstream 4. Payments

- [x] Прийняти рішення по `unreconcile`
- [x] Якщо `unreconcile` входить у scope, реалізувати callback + UI + test
- [x] Якщо `unreconcile` не входить у scope, оновити docs без двозначності

## Verification

- [x] Прогнати релевантні unit-тести
- [x] Прогнати `tests/ui_events.rs`
- [x] Оновити `sprint-report` після фактичного завершення
