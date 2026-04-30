# Remediation Week 1

> **Archived/pre-cutover:** цей план описує Slint-era presenter cleanup. Після `2026-04-30` `src/ui/*` є історичною довідкою, а live Tauri contract живе в `src/tauri_api/*`, `src-tauri/src/commands/*` і `frontend/src/lib/*`.

Оновлено: `2026-04-27`  
Горизонт: `5 робочих днів`  
Статус: `planned`

## Мета тижня

Не закривати весь борг одразу, а підготувати новий каркас системи:

- зафіксувати callback matrix;
- створити `src/actions/*`;
- винести navigation та palette/search;
- нормалізувати documents id через typed API;
- винести базові documents commands в application layer.

Наприкінці цього тижня код має стати простішим для подальших змін, навіть якщо create/edit UI ще не завершений.

## Definition of Done

- існує `src/actions/*` skeleton;
- `bootstrap.rs` став коротшим;
- navigation і command palette винесені з `bootstrap.rs`;
- documents flows не спираються на розкидані `strip_prefix(...)`;
- базові `doc_search`, `doc_tab`, `doc_send`, `doc_delete` йдуть через `actions/documents.rs`;
- проєкт збирається після кожного дня роботи.

## День 1. Карта системи

### Ціль

Зрозуміти поточний стан без зміни поведінки.

### Задачі

- Оновити [remediation-callback-matrix-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-callback-matrix-2026-04-27.md).
- Зафіксувати P1/P2/P3 борг у [remediation-master-plan-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-master-plan-2026-04-27.md).
- Повторно звірити:
  - [src/bootstrap.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap.rs)
  - [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs)
  - [src/ui/counterparties.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs)
  - [src/bin/migrate.rs](/C:/Users/MykhailoDan/apps/Acta/src/bin/migrate.rs)

### Перевірка

- planning-файли узгоджені між собою;
- команда може сказати, які сценарії `works`, `partial`, `TODO`.

## День 2. `actions/` skeleton

### Ціль

Створити новий application layer без зміни зовнішньої поведінки.

### Задачі

- Створити:
  - `src/actions/mod.rs`
  - `src/actions/common.rs`
  - `src/actions/navigation.rs`
  - `src/actions/search.rs`
  - `src/actions/documents.rs`
  - `src/actions/counterparties.rs`
  - `src/actions/tasks.rs`
  - `src/actions/payments.rs`
  - `src/actions/settings.rs`
- Оновити [src/lib.rs](/C:/Users/MykhailoDan/apps/Acta/src/lib.rs).
- У `common.rs` винести:
  - stale-check helper;
  - refresh одного екрана;
  - refresh кількох екранів.

### Перевірка

- `cargo build` проходить;
- новий шар існує, але ще не змінює поведінку.

## День 3. Navigation + palette extraction

### Ціль

Зменшити вагу `bootstrap.rs` без зміни UX.

### Задачі

- Перенести в `src/actions/navigation.rs`:
  - mapping `NavScreen -> AppScreen`;
  - navigation callbacks;
  - company switch logic.
- Перенести в `src/actions/search.rs`:
  - palette query flow;
  - palette activate flow;
  - payload helpers.
- Оновити `wire_app()` так, щоб `bootstrap.rs` лише делегував у `actions::*`.

### Перевірка

- працює navigation;
- працює company switch;
- palette як мінімум відкриває screens як раніше.

## День 4. `DocumentRef`

### Ціль

Прибрати строковий parsing документів як неформальний internal API.

### Задачі

- Додати в `src/actions/documents.rs`:
  - enum виду документа;
  - `DocumentRef`;
  - parser з `act:/inv:/wbl:`.
- Замінити ручні `strip_prefix(...)` там, де це вже безпечно зробити.
- Написати unit tests:
  - valid act/invoice/waybill ids;
  - invalid prefix;
  - invalid UUID.

### Перевірка

- тести проходять;
- новий тип реально використовується хоча б у частині flows.

## День 5. Documents command extraction

### Ціль

Зробити `src/ui/documents.rs` thin presenter-adapter замість місця для сценаріїв.

### Задачі

- Винести в `src/actions/documents.rs`:
  - search changed;
  - tab changed;
  - advance status;
  - delete.
- У [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs) лишити:
  - `prepare_documents_data`;
  - `apply_documents_to_ui`;
  - thin `wire_document_callbacks`.
- Stub-дїї `new/open/edit/bulk/chain` вже теж мають делегуватись у `actions/documents.rs`, навіть якщо тимчасово там ще не вся логіка.

### Перевірка

- `doc_search`, `doc_tab`, `doc_send`, `doc_delete` працюють як раніше;
- documents module став архітектурно чистішим.

## Наприкінці тижня

### Має бути

- новий application layer на місці;
- `bootstrap.rs` без частини orchestration;
- documents flow готовий до editor phase наступного тижня.

### Не треба форсувати

- documents editor UI;
- bulk actions;
- counterparties create form;
- BAS import pipeline;
- backup refactor.

Це свідомо переноситься на Week 2+.
