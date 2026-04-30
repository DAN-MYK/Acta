# Counterparties → Tauri command spec — 2026-04-29

> **Pre-cutover source note:** документ був написаний під час перенесення зі Slint. Посилання на `src/ui/*` нижче є historical reference; live implementation зараз у `src/tauri_api/counterparties.rs`, `src-tauri/src/commands/counterparties.rs`, `frontend/src/lib/stores/counterparties.ts` і `frontend/src/lib/screens/CounterpartiesScreen.svelte`.

## Призначення

Цей документ фіксує Tauri API для модуля `counterparties`. Slint/Rust references нижче залишені тільки як джерело історичної логіки.

Основні джерела логіки:

- [src/ui/counterparties.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:56)
- [src/db/counterparties.rs](/C:/Users/MykhailoDan/apps/Acta/src/db/counterparties.rs:1)

## Ключові доменні правила

### Company scoping

Усі операції зі списком і деталями контрагентів мають бути scoped до активної компанії.

Джерело:

- [prepare_counterparties_data](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:56)
- [prepare_counterparty_detail](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:69)
- [db::counterparties::create](/C:/Users/MykhailoDan/apps/Acta/src/db/counterparties.rs:132)
- [db::counterparties::list_filtered](/C:/Users/MykhailoDan/apps/Acta/src/db/counterparties.rs:45)

### Валідація форми контрагента

При збереженні контрагента застосовуються такі правила:

- `name` є обов'язковим;
- `edrpou` — рівно 8 цифр (якщо заповнений);
- `ipn` — рівно 10 цифр (якщо заповнений);
- `iban` — починається з `UA` і містить рівно 29 символів (якщо заповнений).

Джерело:

- [validate_counterparty_form](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:175)
- [is_valid_edrpou / is_valid_ipn / is_valid_iban](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:14)

### М'яке видалення (архівування)

Контрагенти не видаляються фізично — вони позначаються `is_archived = TRUE`.
За замовчуванням список показує лише не архівованих контрагентів (`is_archived = FALSE`).

Джерело:

- [db::counterparties::archive](/C:/Users/MykhailoDan/apps/Acta/src/db/counterparties.rs:197)
- [db::counterparties::list_filtered](/C:/Users/MykhailoDan/apps/Acta/src/db/counterparties.rs:45)

### Деталі контрагента: паралельне завантаження

Акти, накладні та платежі завантажуються паралельно через `tokio::join!`.
Документи сортуються за датою (desc).

Джерело:

- [prepare_counterparty_detail](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:69)

### Перехід до документів

`cp_create_doc` переводить користувача на екран Documents із попередньо заповненим контрагентом.
Цей cross-screen side effect має бути явно описаний у Tauri command response.

Джерело:

- [on_cp_create_doc](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:385)

### Пошук

Пошук виконується по `name` та `edrpou` (ILIKE, без урахування регістру).
При активному пошуковому запиті результати обмежені 100 записами.

Джерело:

- [db::counterparties::list_filtered](/C:/Users/MykhailoDan/apps/Acta/src/db/counterparties.rs:84)

## Поточні callback-и

Поточний wiring знаходиться у:

- [wire_counterparty_callbacks](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:229)

Перелік callbacks:

- `cp-selected(id: string)` — вибір контрагента у списку, завантажує деталі
- `cp-search-changed(query: string)` — текстовий пошук по списку
- `cp-new()` — відкрити форму нового контрагента
- `cp-edit(id: string)` — відкрити форму редагування контрагента
- `cp-draft-saved(CounterpartyDraftForm)` — зберегти (create або update) контрагента
- `cp-create-doc(id: string)` — перейти до Documents із попередньо заповненим контрагентом
- `cp-tab-changed(tab)` — зміна вкладки у detail view (локальний UI state, без backend)

## Команди

### `counterparties_list(query?) -> CounterpartiesScreenDto`

Джерело логіки:

- [prepare_counterparties_data](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:56)
- [apply_counterparties_to_ui](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:111)

Призначення:

- завантажити список контрагентів компанії;
- опціональний параметр `query` — текстовий пошук по `name` і `edrpou`.

### `counterparty_get(id) -> CounterpartyDetailScreenDto`

Джерело логіки:

- [prepare_counterparty_detail](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:69)
- [apply_counterparty_detail_to_ui](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:117)

Призначення:

- завантажити деталі контрагента;
- паралельно завантажити пов'язані документи та платежі;
- документи відсортовані за датою desc.

### `counterparty_open_editor(id?) -> CounterpartyEditorDto`

Джерело логіки:

- [on_cp_new](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:274)
- [on_cp_edit](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:283)
- [new_counterparty_form](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:134)
- [edit_counterparty_form](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:149)

Призначення:

- якщо `id` відсутній — повернути порожню форму для нового контрагента;
- якщо `id` заданий — завантажити контрагента і повернути заповнену форму.

### `counterparty_save(request) -> CounterpartySaveResultDto`

Джерело логіки:

- [on_cp_draft_saved](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:316)
- [validate_counterparty_form](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:175)
- [update_payload_from_form](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:215)

Призначення:

- якщо `id` порожній — створити нового контрагента в межах активної компанії;
- якщо `id` заданий — оновити існуючого контрагента;
- повернути оновлений список та деталі збереженого контрагента.

Примітка:

Після збереження поточна Slint реалізація рефрешить і список, і detail panel одночасно:

- [on_cp_draft_saved](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:364)

У Tauri це треба явно зберегти через targeted invalidation двох store:
`counterparties.ts` та `counterparty-detail.ts`.

### `counterparty_archive(id) -> MutationResultDto`

Джерело логіки:

- [db::counterparties::archive](/C:/Users/MykhailoDan/apps/Acta/src/db/counterparties.rs:197)

Призначення:

- м'яке видалення: встановити `is_archived = TRUE`;
- після операції клієнт має рефрешити список контрагентів.

### `counterparty_create_document_context(id) -> CreateDocumentContextDto`

Джерело логіки:

- [on_cp_create_doc](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:385)

Призначення:

- завантажити дані контрагента для передзаповнення форми нового документа;
- повернути `counterparty_id` і `counterparty_name` для ініціалізації type picker у Documents.

Примітка:

Перехід між екранами (`current_screen → Documents`) є frontend side effect.
Command лише повертає необхідні дані — навігацію виконує Svelte router.

## DTO

### `CounterpartyItemDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:119)
- [counterparty_to_item](/C:/Users/MykhailoDan/apps/Acta/src/ui/helpers.rs:249)

Поля:

- `id: string`
- `name: string`
- `edrpou: string`
- `kind: string` — `"ЮО"` | `"ФОП"` | `""`
- `balanceStr: string` — pre-formatted
- `docCount: number`
- `overdueCount: number`

Примітка:

`kind`, `balanceStr`, `docCount`, `overdueCount` зараз повертаються порожніми/нулями — ці поля зарезервовані для майбутніх агрегатних запитів:

- [counterparty_to_item](/C:/Users/MykhailoDan/apps/Acta/src/ui/helpers.rs:252)

### `CounterpartyDetailsDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:305)
- [counterparty_to_details](/C:/Users/MykhailoDan/apps/Acta/src/ui/helpers.rs:263)

Поля:

- `id: string`
- `name: string`
- `kind: string`
- `edrpou: string`
- `ipn: string`
- `vat: string`
- `iban: string`
- `bank: string`
- `address: string`
- `director: string`
- `phone: string`
- `email: string`
- `clientSince: string`
- `balanceStr: string`
- `balanceIsNegative: bool`
- `docCount: number`
- `overdueCount: number`
- `overdueAmountStr: string`
- `lastContactDays: number`
- `lastContactDate: string`

### `CounterpartyDraftFormDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:106)

Поля:

- `id: string` — порожній для нового контрагента
- `title: string` — заголовок форми (`"Новий контрагент"` або `"Редагування контрагента"`)
- `name: string`
- `edrpou: string`
- `ipn: string`
- `iban: string`
- `address: string`
- `phone: string`
- `email: string`
- `notes: string`

### `CounterpartiesScreenDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:340)

Поля:

- `items: CounterpartyItemDto[]`

### `CounterpartyDetailScreenDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:344)

Поля:

- `info: CounterpartyDetailsDto`
- `documents: DocumentItemDto[]`
- `payments: PaymentItemDto[]`

### `CounterpartyEditorDto`

Поля:

- `form: CounterpartyDraftFormDto`
- `showEditor: bool`

### `CounterpartySaveResultDto`

Поля:

- `ok: bool`
- `savedId: string`
- `message: string`
- `updatedList: CounterpartyItemDto[]` — оновлений список після збереження
- `updatedDetail: CounterpartyDetailScreenDto | null` — деталі збереженого контрагента

### `CreateDocumentContextDto`

Поля:

- `counterpartyId: string`
- `counterpartyName: string`

### `MutationResultDto`

Рекомендовані поля:

- `ok: bool`
- `message: string`

## Що лишається у frontend state

Ці сценарії не треба переносити у Rust commands:

- відкритість форми редагування (`showCpEditor`);
- локальна чернетка форми контрагента;
- поточний вибір контрагента у списку (`cpSelectedId`);
- активна вкладка у detail view — `cp-tab-changed` у поточній реалізації є no-op на backend;
- transient search input state.

Поточне джерело:

- [on_cp_tab_changed](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:433)
- [set_counterparty_form_state](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:166)

У Tauri/Svelte це має бути локальний component/store state.

## Мінімальний набір для першого vertical slice

1. `counterparties_list`
2. `counterparty_get`
3. `counterparty_open_editor`
4. `counterparty_save`
5. `counterparty_archive`

Далі вже можна додавати:

6. `counterparty_create_document_context`

## Поточні ризики, які треба врахувати при переносі

1. Поля `kind`, `balanceStr`, `docCount`, `overdueCount` у `CounterpartyItemDto` зараз не заповнюються реальними даними — підставляються порожні значення. При переносі треба явно вирішити: обчислювати агрегати в БД чи залишати заглушки.
2. Аналогічно поля `CounterpartyDetailsDto`: `vat`, `bank`, `director`, `clientSince`, `lastContactDays`, `lastContactDate` зараз завжди порожні. У Tauri варто або прибрати їх з DTO, або залишити з явним коментарем `// reserved`.
3. `cp_create_doc` зараз викликає cross-screen перехід із мутацією двох властивостей UI (`current_screen`, `show_doc_type_picker`). У Tauri ця логіка перейде повністю у frontend router — command лише повертає `CreateDocumentContextDto`.
4. `on_cp_search_changed` зараз тригерить повний re-render через `spawn_refresh_screen`. У Tauri це треба обробляти debounced запитом на `counterparties_list(query)` з frontend.
5. При помилці збереження (валідація або БД) поточна реалізація повертає `show_cp_editor = true` без повідомлення. У `CounterpartySaveResultDto` треба явно передавати `ok: false` + `message` для відображення у frontend.

## Пов'язані документи

- [tauri-migration-contract-matrix-2026-04-29.md](./tauri-migration-contract-matrix-2026-04-29.md)
- [tauri-migration-roadmap-2026-04-29.md](./tauri-migration-roadmap-2026-04-29.md)
- [tauri-payments-command-spec-2026-04-29.md](./tauri-payments-command-spec-2026-04-29.md)
- [tauri-documents-command-spec-2026-04-29.md](./tauri-documents-command-spec-2026-04-29.md)
