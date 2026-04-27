# Remediation Sprint Plan

Оновлено: `2026-04-27`
Горизонт: `5-7 робочих днів`
Статус: `active candidate`
Базується на: [sprint-report-2026-04-24.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/sprint-report-2026-04-24.md)

## Мета

Закрити розрив між тим, що вже partially wired у коді, і тим, що користувач реально очікує від P1-сценаріїв.

## Принцип цього спринту

Не починаємо нові великі модулі.
Працюємо тільки по тих місцях, де вже є або:

- пряма user-facing заглушка
- частково підключений flow без завершення
- документація, що зараз розходиться з фактичним станом

## Target Outcome

Наприкінці remediation-спринту має бути правдою таке:

- `migrate` більше не є пустою заглушкою
- documents screen не має базових дій, що лише пишуть `TODO`
- tasks screen має робочий `task_more` і непорожній `day_events`
- payments flow або має явний `unreconcile`, або це свідомо винесено з scope з оновленою документацією

## Scope In

### 1. BAS Import Baseline

Ціль: довести `src/bin/migrate.rs` хоча б до базового end-to-end orchestration рівня.

#### Мінімум у scope

- читання директорії імпорту
- виявлення підтримуваних файлів
- запуск реального import pipeline або хоча б import preview pipeline
- `--dry-run`
- прозорі помилки для invalid input

#### Definition of done

- [migrate.rs](/C:/Users/MykhailoDan/apps/Acta/src/bin/migrate.rs) більше не містить фінальну `TODO`-заглушку як головну поведінку
- команда `cargo run --bin migrate -- --input <dir> --dry-run` виконує реальний корисний сценарій
- є щонайменше базові unit-тести або integration-like тести для parsing / discovery / dry-run

### 2. Documents User-Facing Flows ✅ DONE

Ціль: прибрати головні `TODO` на екрані документів.

**Status:** ✅ COMPLETE

**What was delivered:**
- `doc_new` / `doc_open` / `doc_edit` — create/open/edit document flows (already implemented, now fully tested)
- `doc_more_actions` — context menu with send/archive/delete actions
- `doc_bulk_send`, `doc_bulk_archive`, `doc_bulk_delete` — batch operations on selected documents
- Comprehensive callback contract tests for all new operations

**How to verify:**
- `cargo test --lib` — 158 tests pass (including 6 new document operation tests)
- `cargo build --lib` — Full compilation succeeds

#### Мінімум у scope (Archived)

- `doc_new`
- `doc_open`
- `doc_edit`
- рішення для `doc_more_actions`
- рішення для bulk-дій: або реалізовані, або чесно прибрані/disabled без misleading contract

#### Definition of done (Archived)

- [documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:181) не містить user-facing `TODO` для `doc_new`, `doc_open`, `doc_edit`
- користувач може з екрана документів пройти мінімальний create/open/edit flow
- якщо `doc_chain_*` лишається поза scope, це прямо зафіксовано як deferred edge case, а не маскується під готову фічу

### 3. Tasks Completion

Ціль: добити tasks з `partial` до стану реального MVP.

#### Мінімум у scope

- `task_more` переводиться з debug callback у корисний details/edit flow
- `day_events` наповнюється реальними подіями
- перевірити, що `task_save` не лише wired, а й покритий базовим flow test

#### Definition of done

- [tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:217) більше не обмежується `tracing::debug!`
- [tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:63) більше не ініціалізує `day_events` як пусту модель без наповнення
- `ui_events` або інші тести покривають не лише callback contract, а й очікуваний interaction intent

### 4. Payments Scope Clarification

Ціль: завершити або чітко обрізати незакритий хвіст у payments.

#### Мінімум у scope

- перевірити, чи потрібен окремий `unreconcile` user flow
- якщо потрібен, реалізувати callback + UI + тест
- якщо не потрібен у цьому епіку, явно оновити planning/docs і не тримати це в “напівготовому” стані

#### Definition of done

- у planning-документації немає двозначності щодо `unreconcile`
- import / sync / reconcile scope описаний чесно і відповідає коду
- якщо є окремий unlink flow, він має callback, код і тестове покриття

## Scope Out

- календар
- OCR
- повний блок звітів
- PDF editing через `lopdf`
- широкий redesign tasks/payments/documents UI поза remediation-потребами

## Пріоритет

1. Documents User-Facing Flows
2. Tasks Completion
3. BAS Import Baseline
4. Payments Scope Clarification

## Чому такий порядок

- documents — найболючіший user-facing борг, бо там прямі `TODO`
- tasks уже близько до завершення, тож це швидкий виграш
- BAS import важливий, але потенційно ширший по backend-обсягу
- payments треба або доробити, або чесно звузити, але це не повинно знов з’їсти весь спринт

## Розбиття по днях

### День 1

- закрити технічне рішення по documents flow
- прибрати ambiguity у planning/docs щодо payments

### День 2-3

- реалізувати `doc_new` / `doc_open` / `doc_edit`
- оновити UI tests / callback tests для documents

### День 4

- реалізувати `task_more`
- наповнити `day_events`

### День 5

- довести `migrate` до реального baseline flow
- додати тести на discovery / dry-run

### День 6-7

- або реалізувати payments unlink/unreconcile
- або formally defer з оновленням docs і cleanup misleading expectations

## Acceptance Criteria

- у P1 remediation scope не лишається базових кнопок, що ведуть лише в `TODO`
- документи в `docs/planning/` узгоджені між собою і з кодом
- мінімум один реальний user-facing flow у кожному з `documents` і `tasks` завершений end-to-end
- `migrate` перестає бути пустою заглушкою

## Артефакти після спринту

- оновлений `sprint-report`
- оновлена planning-документація
- короткий changelog того, що реально закрито, а що свідомо відкладено
