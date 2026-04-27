# Remediation Sprint Checklist

Оновлено: `2026-04-27`
Статус: `open`
План: [remediation-sprint-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-sprint-2026-04-27.md)

## Goal

- [ ] Прибрати головні user-facing `TODO`
- [ ] Довести `tasks` до реального MVP flow
- [ ] Прибрати двозначність по `BAS import` і `payments`

## Workstream 1. Documents

- [ ] Реалізувати `doc_new`
- [ ] Реалізувати `doc_open`
- [ ] Реалізувати `doc_edit`
- [ ] Визначити долю `doc_more_actions`
- [ ] Визначити долю bulk-дій
- [ ] Якщо треба, прибрати misleading UI/state для нереалізованих дій

## Workstream 2. Tasks

- [ ] Реалізувати `task_more` як корисний flow
- [ ] Наповнити `day_events`
- [ ] Перевірити `task_save` end-to-end behavior
- [ ] Оновити `ui_events` або суміжні тести

## Workstream 3. BAS Import

- [ ] Прибрати фінальну `TODO`-заглушку з `migrate.rs`
- [ ] Додати file discovery
- [ ] Додати реальний `dry-run`
- [ ] Додати базові тести

## Workstream 4. Payments

- [ ] Прийняти рішення по `unreconcile`
- [ ] Якщо `unreconcile` входить у scope, реалізувати callback + UI + test
- [ ] Якщо `unreconcile` не входить у scope, оновити docs без двозначності

## Verification

- [ ] Прогнати релевантні unit-тести
- [ ] Прогнати `tests/ui_events.rs`
- [ ] Оновити `sprint-report` після фактичного завершення
