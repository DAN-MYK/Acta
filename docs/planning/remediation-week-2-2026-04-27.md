# Remediation Week 2

> **Archived/pre-cutover:** цей план описує Slint-era editor flow. Після `2026-04-30` `ui/*.slint` references не є live UI contract; нові screen specs мають бути Tauri/Svelte.

Оновлено: `2026-04-27`  
Горизонт: `5 робочих днів`  
Статус: `planned`

## Мета тижня

На другому тижні ми вже не просто готуємо каркас, а закриваємо найбільш болючий user-facing борг:

- documents editor state;
- `doc_new`, `doc_open`, `doc_edit`;
- bulk actions;
- початок document chains;
- старт counterparties remediation.

## Definition of Done

- documents screen більше не має головних дій, що лише логують `TODO`;
- існує базовий create/open/edit flow для документів;
- bulk actions мають або реальну реалізацію, або чесно відключені/зняті з UI;
- counterparties flow має план входу в editor layer;
- код після змін усе ще організований через `actions/*`, а не назад у `bootstrap.rs`.

## День 1. Documents editor contract

### Ціль

Підготувати Slint і Rust до реального editor flow.

### Задачі

- Оновити [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint):
  - додати editor-related типи;
  - state для режимів `create/edit/view`.
- Оновити [ui/app.slint](/C:/Users/MykhailoDan/apps/Acta/ui/app.slint):
  - додати properties/callbacks для editor flow.
- За потреби створити:
  - `ui/document-editor.slint`
- У Rust додати draft/state structs:
  - `DocumentDraft`
  - `DocumentEditorMode`

### Перевірка

- UI контракт компілюється;
- editor state можна відкрити/закрити без збереження.

## День 2. `doc_open` і `doc_edit`

### Ціль

Дати користувачу реальний open/edit flow для існуючого документа.

### Задачі

- Додати loader одного документа.
- Замапити domain model у `DocumentDraft`.
- Реалізувати:
  - `open_document`
  - `edit_document`
- Після відкриття editor має показувати коректні значення.

### Перевірка

- click/open не веде в `TODO`;
- edit mode відкриває форму з даними.

## День 3. `doc_new` і save/update

### Ціль

Замкнути create/edit flow end-to-end.

### Задачі

- Реалізувати create flow по мінімальному набору полів.
- Реалізувати save/update handlers.
- Після save оновлювати:
  - `Documents`
  - `Dashboard`, якщо метрики залежать від документа
  - інші екрани тільки за реальною потребою
- Додати базову валідацію draft.

### Перевірка

- документ реально створюється;
- існуючий документ реально редагується;
- після збереження список оновлюється.

## День 4. Bulk actions

### Ціль

Розібрати selection state і перестати тримати misleading bulk UI.

### Задачі

- Вирішити canonical selected state:
  - або локальний Slint;
  - або синхронізований із `AppCtx`.
- Реалізувати мінімум:
  - `bulk_delete`
  - `bulk_send`
- `bulk_archive` або реалізувати, або тимчасово зняти з UX, якщо доменне правило не готове.

### Перевірка

- користувач не бачить кнопок, які нічого не роблять;
- multi-select flow працює передбачувано.

## День 5. Chains + counterparties bridge

### Ціль

Почати пов’язувати documents і counterparties в одну систему.

### Задачі

- Звірити [document-chains-design-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/document-chains-design-2026-04-27.md) з фактичним кодом.
- Реалізувати перший working slice:
  - або `doc_chain_load`,
  - або `create document from counterparty`.
- Підготувати `CounterpartyDraft` та editor backlog на Week 3.

### Перевірка

- один із двох cross-feature flows перестає бути заглушкою;
- є чіткий список того, що переходить у Week 3.

## Stretch goals

Якщо documents phase пішла швидше, можна почати:

- counterparties editor contract;
- typed command payloads для more-actions;
- перші integration tests на create/edit/save documents.

## Чого не брати в цей тиждень

Не змішувати з documents completion:

- великий BAS import refactor;
- backup redesign;
- масовий cleanup усіх fallback-ів у проєкті.

Інакше є високий шанс зірвати головний user-facing результат тижня.
