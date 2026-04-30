# Payments → Tauri command spec — 2026-04-29

## Призначення

Цей документ фіксує цільовий Tauri API для модуля `payments` на базі поточного Slint/Rust контракту.

Основне джерело логіки:

- [src/ui/payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:24)

## Ключові доменні правила

### Формат дати

Платіж підтримує два формати вводу дати:

- `dd.mm.yyyy`
- `yyyy-mm-dd`

Джерело:

- [parse_payment_date](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:101)

### Money contract

Сума платежу приходить з UI рядком і валідовується в Rust як `Decimal`.

Джерело:

- [parse_payment_amount](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:108)

Правила:

- сума має бути числом;
- сума має бути більшою за нуль.

### Напрям платежу

У формі використовується строковий напрям:

- `income`
- `expense`

Джерело:

- [parse_payment_direction](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:119)

### Company scoping

Усі mutation-операції для платежів мають бути scoped до активної компанії.

Джерело:

- [load_payment_for_company](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:159)
- [save_payment_update_for_company](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:169)
- [delete_payment_for_company](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:181)
- [set_payment_reconciled_for_company](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:195)

## Поточні callback-и

Джерело root callbacks:

- [ui/app.slint](/C:/Users/MykhailoDan/apps/Acta/ui/app.slint:90)

Поточні payment callbacks:

- `pay-import-csv`
- `pay-sync-bank`
- `pay-new`
- `pay-open-payment-template`
- `pay-save-payment(PaymentDraftForm)`
- `pay-link(string)`
- `pay-unreconcile(string)`

Їх wiring зараз знаходиться у:

- [wire_payment_callbacks](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:364)

## Команди

### `payments_list() -> PaymentsScreenDto`

Джерело логіки:

- [prepare_payments_data](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:30)
- [apply_payments_to_ui](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:61)

Призначення:

- завантажити список платежів;
- завантажити список контрагентів для форми;
- завантажити KPI по платежах.

### `payments_import_latest_csv() -> MutationResultDto`

Джерело логіки:

- [import_latest_bank_csv](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:331)
- [on_pay_import_csv](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:365)

Призначення:

- знайти найновіший CSV у `storage/import/bank`;
- розпізнати формат;
- імпортувати нові рядки.

### `payments_sync_bank() -> MutationResultDto`

Джерело логіки:

- [on_pay_sync_bank](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:389)

Примітка:

Зараз `sync bank` доменно робить ту саму операцію, що і `import csv`, але з іншим юзерським повідомленням.

### `payments_open_manual_template() -> OpenTemplateResultDto`

Джерело логіки:

- [ensure_manual_import_template](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:341)
- [open_manual_import_template](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:357)
- [on_pay_open_payment_template](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:420)

Призначення:

- створити шаблон CSV, якщо його ще нема;
- відкрити його системним переглядачем/редактором.

### `payment_create_or_update(request) -> MutationResultDto`

Джерело логіки:

- [on_pay_save_payment](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:437)

Призначення:

- якщо `id` порожній — створити новий платіж;
- якщо `id` заданий — оновити існуючий платіж у межах активної компанії.

### `payment_reconcile(payment_id) -> MutationResultDto`

Джерело логіки:

- [on_pay_link](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:509)
- [set_payment_reconciled_for_company](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:195)

Примітка:

Назва callback `pay-link` зараз фактично означає mark reconciled.

### `payment_unreconcile(payment_id) -> MutationResultDto`

Джерело логіки:

- [on_pay_unreconcile](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:543)
- [set_payment_reconciled_for_company](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:195)

## DTO

### `PaymentItemDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:128)
- [payment_row_to_item](/C:/Users/MykhailoDan/apps/Acta/src/ui/helpers.rs:289)

Поля:

- `id: string`
- `date: string`
- `counterparty: string`
- `amountStr: string`
- `direction: "in" | "out"`
- `matchedDoc: string`
- `account: string`

Примітка:

`matchedDoc` зараз фактично показує `"Звірено"` або порожній рядок, а не реальний document id:

- [helpers.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/helpers.rs:299)

### `PaymentDraftFormDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:138)

Поля:

- `id`
- `date`
- `amount`
- `direction`
- `counterpartyId`
- `counterpartyName`
- `bankName`
- `reference`
- `description`

### `PaymentsKpiDto`

Базується на:

- [PaymentKpi](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:15)
- [apply_payments_to_ui](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:61)

Поля:

- `incomingStr`
- `outgoingStr`
- `netStr`
- `unmatchedStr`
- `incomingSub`
- `outgoingSub`
- `unmatchedCount`

### `PaymentsScreenDto`

Базується на:

- [PaymentsViewData](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:376)

Поля:

- `items: PaymentItemDto[]`
- `counterparties: CounterpartyItemDto[]`
- `kpi: PaymentsKpiDto`

### `OpenTemplateResultDto`

Рекомендовані поля:

- `ok: bool`
- `path: string`
- `message: string`

### `MutationResultDto`

Рекомендовані поля:

- `ok: bool`
- `message: string`

## Що лишається у frontend state

У `payments` майже немає складного локального state, який варто окремо зберігати в Rust. У frontend мають жити:

- відкритість форми платежу;
- локальна чернетка форми;
- вибір контрагента у dropdown;
- transient import/sync loading state.

Поточний Slint-specific приклад:

- [close_payment_form](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:94)

У Tauri/Svelte це має бути локальний component/store state.

## Рекомендований мінімальний набір для першого vertical slice

1. `payments_list`
2. `payment_create_or_update`
3. `payment_reconcile`
4. `payment_unreconcile`
5. `payments_import_latest_csv`

Далі вже можна додавати:

6. `payments_open_manual_template`
7. `payments_sync_bank`

## Поточні ризики, які треба врахувати при переносі

1. `pay-sync-bank` і `pay-import-csv` зараз дублюють одну й ту саму backend-операцію.
2. `matchedDoc` у DTO семантично не зовсім відповідає назві поля.
3. Після mutation зараз рефрешиться і Payments, і Dashboard:
   - [src/ui/payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:495)
   - [src/ui/payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:497)

У Tauri це треба явно зберегти через targeted invalidation двох store:

- `payments.ts`
- `dashboard.ts`

## Пов'язані документи

- [tauri-migration-contract-matrix-2026-04-29.md](./tauri-migration-contract-matrix-2026-04-29.md)
- [tauri-migration-roadmap-2026-04-29.md](./tauri-migration-roadmap-2026-04-29.md)
- [tauri-documents-command-spec-2026-04-29.md](./tauri-documents-command-spec-2026-04-29.md)
