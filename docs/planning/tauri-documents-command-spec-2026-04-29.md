# Documents → Tauri command spec — 2026-04-29

## Призначення

Цей документ фіксує цільовий Tauri API для модуля `documents` на базі поточного Slint/Rust контракту.

Основне джерело логіки:

- [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:23)

Skeleton майбутніх Tauri commands уже зафіксований у:

- [src-tauri/src/commands/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src-tauri/src/commands/documents.rs:1)

## Ключові доменні правила

### Ідентифікатор документа

Документи мають лишатися у форматі:

- `act:<uuid>`
- `inv:<uuid>`
- `wbl:<uuid>`

Джерело:

- [parse_document_ref](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:139)

### Формат дати

Дата у формі очікується у форматі `dd.mm.yyyy`.

Джерело:

- [parse_ui_date](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:245)

### Money contract

Кількість і ціна в editor state живуть рядками до моменту валідації в Rust.

Джерело:

- [parse_decimal_input](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:263)
- [draft_items_to_new_act](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:305)
- [draft_items_to_new_invoice](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:319)
- [draft_items_to_new_waybill](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:335)

### Chain parent у notes

Прихований `chain-parent` не можна втратити при збереженні документа.

Джерело:

- [split_visible_notes_and_chain_parent](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:191)
- [compose_notes_with_chain_parent](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:215)

## Команди

### `documents_list(request) -> DocumentsListDto`

Джерело логіки:

- [prepare_documents_data](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:52)

Призначення:

- завантажити список усіх документів;
- повернути повний список і pre-split підмасиви за типами.

Примітка:

У поточній реалізації `tab` ще не фільтрує бекендом.

### `document_open(doc_id) -> DocumentEditorDto`

Джерело логіки:

- [build_existing_document_form](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:536)

Призначення:

- відкрити документ в editor state;
- повернути form + items.

### `document_prepare_new(counterparty_id) -> NewDocumentContextDto`

Джерело логіки:

- [on_doc_new](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1153)

Призначення:

- перевірити контекст створення;
- повернути дані контрагента для type picker.

### `document_create_draft(request) -> DocumentEditorDto`

Джерело логіки:

- [create_draft_form](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:611)
- [on_doc_create_kind_selected](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1534)

Призначення:

- створити новий draft-запис у БД;
- повернути editor state для подальшого редагування.

### `document_save(request) -> SaveDocumentResponse`

Джерело логіки:

- [save_document_form](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:730)

Призначення:

- зберегти шапку документа;
- зберегти позиції;
- зберегти `chain-parent` у `notes`, якщо він є.

### `document_advance_status(doc_id) -> MutationResultDto`

Джерело логіки:

- [on_doc_send](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1079)
- [on_context_send](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1648)
- [on_context_archive](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1674)

Призначення:

- перевести документ на наступний статус.

Примітка:

Зараз `send` і `archive` зводяться до одного виклику `advance_status`.

### `document_delete(doc_id) -> MutationResultDto`

Джерело логіки:

- [on_doc_delete](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1119)
- [on_context_delete](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1701)

### `documents_bulk_advance_status(request) -> BulkMutationResultDto`

Джерело логіки:

- [on_doc_bulk_send](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1286)
- [on_doc_bulk_archive](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1365)

Примітка:

Обидва flows зараз доменно однакові.

### `documents_bulk_delete(request) -> BulkMutationResultDto`

Джерело логіки:

- [on_doc_bulk_delete](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1448)

### `document_chain_get(doc_id) -> DocumentChainDto`

Джерело логіки:

- [load_chain_from_id](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:821)

### `document_chain_create_draft(request) -> DocumentEditorDto`

Джерело логіки:

- [create_chain_draft_from_source](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:905)

Призначення:

- створити похідний draft у ланцюжку;
- повернути editor state.

## DTO

### `DocumentItemDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:76)
- [act_row_to_document_item](/C:/Users/MykhailoDan/apps/Acta/src/ui/helpers.rs:206)
- [invoice_row_to_document_item](/C:/Users/MykhailoDan/apps/Acta/src/ui/helpers.rs:220)
- [waybill_row_to_document_item](/C:/Users/MykhailoDan/apps/Acta/src/ui/helpers.rs:234)

Поля:

- `id: string`
- `kind: "invoice" | "act" | "waybill"`
- `number: string`
- `date: string`
- `counterparty: string`
- `amountStr: string`
- `status: "draft" | "issued" | "signed" | "paid"`
- `linkedId: string`
- `selected: bool`

### `DocumentsListDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:328)

Поля:

- `items`
- `invoiceItems`
- `actItems`
- `waybillItems`
- `totalCount`
- `pageCount`

`selectedIds` не треба переносити в backend DTO, бо це frontend-only state.

### `DocumentDraftFormDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:88)

Поля:

- `id`
- `kind`
- `counterpartyId`
- `counterpartyName`
- `title`
- `number`
- `date`
- `notes`

### `DocumentDraftItemDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:99)

Поля:

- `description`
- `unit`
- `quantity`
- `price`

### `DocumentEditorDto`

Поля:

- `form`
- `items`
- `showTypePicker`
- `showEditor`

Відповідає поточному `set_document_state(...)`:

- [set_document_state](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:250)

### `ChainStepDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:267)
- [load_document_chain](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:831)

Поля:

- `docType`
- `docNumber`
- `amountStr`
- `status`
- `exists`

### `BulkMutationResultDto`

Базується на:

- [OperationResult](/C:/Users/MykhailoDan/apps/Acta/src/ui/helpers.rs:90)

Поля:

- `total`
- `succeeded`
- `failed`
- `errors`
- `message`

## Що лишається у frontend state

Ці сценарії не треба переносити у Rust commands:

- `doc_toggled`
- `doc_selection_cleared`
- `doc_more_actions`
- `doc_page_changed`
- `doc_draft_item_upserted`
- `doc_draft_item_removed`

Поточне джерело:

- [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1274)
- [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1614)

Це локальний UI state, і в Tauri/Svelte має жити у store або component state.

## Мінімальний набір для першого vertical slice

1. `documents_list`
2. `document_open`
3. `document_create_draft`
4. `document_save`
5. `document_advance_status`
6. `document_delete`
7. `document_chain_get`

## Пов'язані файли

- [src-tauri/src/commands/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src-tauri/src/commands/documents.rs:1)
- [tauri-migration-audit-2026-04-29.md](./tauri-migration-audit-2026-04-29.md)
- [tauri-migration-roadmap-2026-04-29.md](./tauri-migration-roadmap-2026-04-29.md)
- [tauri-migration-contract-matrix-2026-04-29.md](./tauri-migration-contract-matrix-2026-04-29.md)
