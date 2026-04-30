# Remediation Sprint Plan

Оновлено: `2026-04-29`
Горизонт: `5-7 робочих днів`
Статус: `completed / follow-up backlog identified`
Базується на: [sprint-report-2026-04-24.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/sprint-report-2026-04-24.md)

## Мета

Закрити розрив між тим, що вже wired у коді, і тим, що користувач реально очікує від P1-сценаріїв.

## Target Outcome

На кінець remediation-спринту правдою має бути таке:

- `migrate` більше не є пустою заглушкою
- documents screen не має базових дій, що ведуть лише в `TODO`
- tasks screen має робочий `task_more` і непорожній `day_events`
- payments flow має явний `unreconcile`
- planning-документи узгоджені з кодом

## Scope In

### 1. BAS Import Baseline ✅ Delivered

Статус: `baseline delivered, pipeline still expandable`

Що є в коді:

- discovery вхідної директорії
- класифікація артефактів
- `--dry-run`
- реальні importer-модулі в `src/import/`
- робочий import baseline для XML-потоків, без фінальної "пустої" поведінки

Що ще лишається поза цим спринтом:

- подальше розширення coverage по форматах і сценаріях імпорту
- глибший import diff / richer preview UX поверх уже наявного DB-aware dry-run
- подальший cleanup import UX

### 2. Documents User-Facing Flows ✅ Delivered

Статус: `complete`

Що доставлено:

- `doc_new` / `doc_open` / `doc_edit`
- draft editor і редагування позицій документа
- `doc_more_actions`
- bulk-операції
- `doc_chain_load` / `doc_chain_create`
- bridge `counterparty -> create document`

### 3. Tasks Completion ✅ Delivered

Статус: `complete`

Що доставлено:

- `day_events` наповнюється реальними подіями
- `task_more` відкриває корисний details flow
- з details flow можна змінювати статус задачі (`in_progress` / `done` / `cancelled`)
- `task_save` і callback contract покриті тестами

### 4. Payments Scope Clarification ✅ Delivered

Статус: `complete`

Прийняте рішення:

- у поточній доменній моделі `unreconcile` трактується як rollback прапорця `is_reconciled`
- окремий unlink document relations у цей sprint scope не входив

Що доставлено:

- `pay_link`
- `pay_unreconcile`
- UI-кнопка для скасування звірки
- callback wiring і тести контракту

## Scope Out

- календар
- OCR
- повний блок звітів
- PDF editing через `lopdf`
- широкий redesign tasks/payments/documents UI поза remediation-потребами

## Підсумок по backlog

Після фактичного виконання спринту в open remediation scope більше не лишилось базових user-facing `TODO` для `documents`, `tasks`, `payments`, `settings backup` і shell-level callback wiring.

Що тепер логічно виносити в наступний шар:

1. Подальше розширення BAS import pipeline
2. Доведення `pay_new` від template-flow до повнішого create-flow
3. Витягування reusable app actions зі Slint bootstrap для Tauri migration

Що це означає practically:

- не інвестувати в Slint-only cleanup `src/bootstrap/*` заради самого рефакторингу
- виносити сценарну логіку в нейтральний шар `actions` / `services`, який переживе заміну UI
- готувати backend surface для Tauri commands замість поглиблення старого callback orchestration

## Acceptance Criteria

- у P1 remediation scope не лишається базових кнопок, що ведуть лише в `TODO`
- planning-документи узгоджені з кодом
- `migrate` перестає бути пустою заглушкою
- у `documents`, `tasks`, `payments` є завершені end-to-end user-facing flows

## Артефакти після спринту

- оновлений `sprint-report`
- оновлена planning-документація
- короткий backlog того, що свідомо перенесено в наступний етап
