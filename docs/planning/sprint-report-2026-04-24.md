# Sprint Report

Оновлено: `2026-04-28`
План: [next-sprint-2026-04-24.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/next-sprint-2026-04-24.md)
Статус: `completed later than initially tracked / docs were stale`

## Що це за файл

Це фактичний звіт по sprint-ініціативі, запланованій `2026-04-24`, але звірений із реальним кодом уже після remediation-хвилі станом на `2026-04-28`.

## Підсумок

Початковий sprint не був завершений у тому вигляді, як це спершу описувалось у старих нотатках, але більшість ключових remediation-результатів згодом були доведені в коді.

Найважливіше:

- `documents` user-facing flow доведений
- `tasks` доведені, включно з `day_events` і details/status flow для `task_more`
- `payments` мають `pay_link` і `pay_unreconcile`
- `counterparties` мають create/edit/document bridge
- `migrate` більше не є пустою заглушкою

## Workstream Status

### 1. BAS Import MVP

Статус: `baseline delivered`

Підтверджено в коді:

- [src/bin/migrate.rs](/C:/Users/MykhailoDan/apps/Acta/src/bin/migrate.rs:1) має discovery, `--dry-run` і dispatcher
- у [src/import/mod.rs](/C:/Users/MykhailoDan/apps/Acta/src/import/mod.rs:1) є окремі importer-модулі
- реальні XML importer-и є для контрагентів, договорів і актів

Що ще не варто перебільшувати:

- pipeline ще можна розширювати
- dry-run / UX можна робити глибшими

### 2. Documents Flow Completion

Статус: `done`

Підтверджено в коді:

- базові `doc_new` / `doc_open` / `doc_edit` працюють
- є draft editor і item editing
- `doc_more_actions` і bulk-дії wired
- `doc_chain_load` / `doc_chain_create` більше не є TODO

### 3. Tasks Flow Completion

Статус: `done`

Підтверджено в коді:

- `task_save` працює
- `day_events` наповнюється
- `task_more` веде в реальний details flow
- з details flow можна змінювати статус задачі

### 4. Payments Import/Reconcile Completion

Статус: `done for current scope`

Підтверджено в коді:

- CSV import / sync працюють
- manual template flow є
- `pay_link` працює
- `pay_unreconcile` працює

Уточнення:

- поточний `unreconcile` — це rollback `is_reconciled`
- окремий unlink document relations не був частиною цього sprint scope

## Що реально можна вважати зробленим

- прибрано основні user-facing `TODO` у P1 remediation scope
- planning-гіпотези по `documents`, `tasks`, `counterparties`, `payments` підтверджені кодом
- import baseline для `migrate` став реальним, а не декоративним

## Що лишається після цього sprint

- синхронізація planning-документів із кодом
- cleanup `inbox_action` і `palette_item_activated`
- подальше розширення BAS import coverage
- backup/settings clarification

## Рекомендована інтерпретація

Цей sprint коректно трактувати як такий, що фактично завершився через remediation-хвилю, а не в момент першого написання старих checklist/report нотаток.
