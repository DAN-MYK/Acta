# Sprint Report

Оновлено: `2026-04-29`
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

Статус: `baseline delivered and expanded`

Підтверджено в коді:

- [src/bin/migrate.rs](/C:/Users/MykhailoDan/apps/Acta/src/bin/migrate.rs:1) має discovery, `--dry-run`, DB-aware preview і dispatcher
- у [src/import/mod.rs](/C:/Users/MykhailoDan/apps/Acta/src/import/mod.rs:1) є окремі importer-модулі
- реальні importer-и є для контрагентів, договорів, актів, накладних і платежів

Що ще не варто перебільшувати:

- pipeline ще можна розширювати
- preview / UX можна робити глибшими

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

- подальше розширення BAS import coverage
- доведення `pay_new` до повнішого create-flow
- витягування reusable app actions зі Slint bootstrap для Tauri migration

## Рекомендована інтерпретація

Цей sprint коректно трактувати як такий, що фактично завершився через remediation-хвилю, а не в момент першого написання старих checklist/report нотаток.

Окреме уточнення на наступний етап: оскільки UI мігрує на Tauri, `src/bootstrap/*` не варто далі "чистити" як самостійну Slint-архітектуру. Правильний наступний крок — витягувати з цього шару reusable application logic, яку зможе повторно використати новий Tauri UI.
