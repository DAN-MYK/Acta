# Spec: Редагування та створення контрагентів з форми документів

**Дата:** 2026-05-15  
**Статус:** Затверджено (rev 2)

## Контекст

Зараз у формі документа контрагент — read-only поле для існуючих документів і простий dropdown для нових. Користувач не може редагувати дані контрагента або створити нового, не покидаючи екрану Documents.

## Мета

1. З форми редагування документа — відкрити модал для редагування даних контрагента.
2. При створенні документа — можливість створити нового контрагента на льоту через "+ Новий контрагент..." у dropdown.
3. Для існуючих документів — переприсвоїти документ іншому контрагенту.

## UX-рішення

- **Модал** (overlay поверх drawer) — вибраний підхід для редагування/створення.
- **Dropdown-пункт** `+ Новий контрагент...` для нових документів.
- Існуючі документи: кнопки `[Редагувати]` та `[Змінити]` поруч з назвою контрагента.

---

## Архітектура

### Новий компонент: `CounterpartyModal.svelte`

**Файл:** `frontend/src/lib/components/CounterpartyModal.svelte`

Чисто presentational компонент. Не містить бізнес-логіки, не звертається до store або API напряму.

**Props:**
```typescript
export let isOpen: boolean
export let mode: 'create' | 'edit'
export let form: CounterpartyDraftFormDto | null
export let loading: boolean = false
export let isDirty: boolean = false
export let showCloseConfirm: boolean = false  // окремий стан підтвердження закриття
```

**Events (createEventDispatcher):**
- `fieldChange: { field: keyof CounterpartyDraftFormDto, value: string }`
- `save`
- `close`
- `closeConfirmed` — підтверджено «Так, закрити» у dirty-баннері
- `closeCancelled` — натиснуто «Залишитись»

**Структура UI:**
- `div.modal-overlay` (фіксований, z-index поверх drawer) → клік → `close` (не `closeConfirmed` — store вирішує)
- `div.modal-panel` (~480px, role="dialog", aria-modal="true")
  - Заголовок: «Новий контрагент» (create) або «Редагування контрагента» (edit) + кнопка ×
  - Форма `.cp-editor-grid`: поля `name`, `edrpou`, `ipn`, `iban`, `phone`, `email`, `address` (span-2), `notes` (span-2, textarea)
  - Footer: `[Зберегти]` (btn-primary, disabled якщо loading) + `[Скасувати]` (btn-ghost)
  - Dirty-confirm (відображається коли `showCloseConfirm === true`): «Є незбережені зміни. Закрити без збереження?» + `[Так, закрити]` → `closeConfirmed` / `[Залишитись]` → `closeCancelled`

**Логіка dirty-підтвердження** живе у store, не в компоненті:
- `close` → store перевіряє `isDirty`: якщо так → `cpModal.confirmClose = true` (показує confirm); якщо ні → закриває
- `closeConfirmed` → store закриває без dirty-check
- `closeCancelled` → store скидає `cpModal.confirmClose = false`

---

### Documents Store — новий стан `cpModal`

**Файл:** `frontend/src/lib/stores/documents.ts`

Додати до `DocumentsState`:
```typescript
cpModal: {
  isOpen: boolean
  mode: 'create' | 'edit'
  form: CounterpartyDraftFormDto | null
  snapshot: CounterpartyDraftFormDto | null
  loading: boolean
  confirmClose: boolean  // окремий прапорець для показу dirty-confirm
} | null
```

**5 нових функцій:**

| Функція | Логіка |
|---|---|
| `openCpCreate()` | `counterpartyOpenEditor()` → `cpModal = { isOpen: true, mode: 'create', form, snapshot: cloneSnapshot(form), loading: false, confirmClose: false }` |
| `openCpEdit(id: string)` | `counterpartyOpenEditor(id)` → `cpModal = { isOpen: true, mode: 'edit', form, snapshot: cloneSnapshot(form), loading: false, confirmClose: false }` |
| `updateCpField(field, value)` | `cpModal.form[field] = value` |
| `saveCp()` | `counterpartySave(cpModal.form)` → ім'я: `result.updatedList.find(cp => cp.id === result.savedId)?.name`; якщо create: `editor.form.counterpartyId = result.savedId`, `editor.form.counterpartyName = ім'я`, оновити `editorSnapshot.counterpartyId/Name`; якщо edit: оновити `editor.form.counterpartyName`, оновити `editorSnapshot.counterpartyName` → `counterparties.load()` → `cpModal = null` |
| `closeCpModal()` | якщо `isDirty` → `cpModal.confirmClose = true`; інакше → `cpModal = null` |

**Додаткові функції:**

| Функція | Логіка |
|---|---|
| `confirmCloseCpModal()` | `cpModal = null` (без dirty-check) |
| `cancelCloseCpModal()` | `cpModal.confirmClose = false` |
| `changeCounterparty(docId, cpId)` | `documentChangeCounterparty(docId, cpId)` → `editor.form.counterpartyId = result.counterpartyId`, `editor.form.counterpartyName = result.counterpartyName` → **синхронізувати snapshot**: `editorSnapshot.counterpartyId = result.counterpartyId`, `editorSnapshot.counterpartyName = result.counterpartyName` → `isReassigning = false` (через store або подія до screen) |

`isDirtyCpModal` — derived: `isEditorFormDirty(cpModal?.snapshot, cpModal?.form)` (false якщо cpModal null)

---

### DocumentsScreen — UI зміни

**Файл:** `frontend/src/lib/screens/DocumentsScreen.svelte`

#### Нові документи (`$documents.pendingNew === true`)

У `<select>` контрагентів додати останній `<option>`:
```html
<option value="__new__">+ Новий контрагент...</option>
```

У `onEditorCounterpartyChange`:
```typescript
if (value === '__new__') {
  selectElement.value = ''; // скинути select на placeholder
  documents.openCpCreate();
  return;
}
```

#### Існуючі документи

Замінити read-only текст на блок з трьома станами через local variables `isReassigning` та `reassignTargetId`.

При вході в режим reassign ініціалізувати: `reassignTargetId = form.counterpartyId`.

**Стан 1 — перегляд (default):**
```html
<span>{form.counterpartyName}</span>
<button on:click={() => documents.openCpEdit(form.counterpartyId)}>Редагувати</button>
<button on:click={() => { isReassigning = true; reassignTargetId = form.counterpartyId; }}>Змінити</button>
```

**Стан 2 — переприсвоєння (`isReassigning`):**
```html
<select bind:value={reassignTargetId}>
  {#each counterpartiesList as cp}
    <option value={cp.id}>{cp.name}</option>
  {/each}
</select>
<button
  disabled={!reassignTargetId || reassignTargetId === form.counterpartyId}
  on:click={() => documents.changeCounterparty(form.id, reassignTargetId)}
>Зберегти</button>
<button on:click={() => isReassigning = false}>Скасувати</button>
```

`isReassigning` скидається у `false` автоматично після успішного `changeCounterparty` — store оновлює `editor.form`, screen реагує через reactive statement `$: if ($documents.editor?.form.counterpartyId !== prevCounterpartyId) isReassigning = false`.

#### Рендер модала (у кінці розмітки drawer)

```html
{#if $documents.cpModal?.isOpen}
  <CounterpartyModal
    isOpen={$documents.cpModal.isOpen}
    mode={$documents.cpModal.mode}
    form={$documents.cpModal.form}
    loading={$documents.cpModal.loading}
    isDirty={$isDirtyCpModal}
    showCloseConfirm={$documents.cpModal.confirmClose}
    on:fieldChange={(e) => documents.updateCpField(e.detail.field, e.detail.value)}
    on:save={() => documents.saveCp()}
    on:close={() => documents.closeCpModal()}
    on:closeConfirmed={() => documents.confirmCloseCpModal()}
    on:closeCancelled={() => documents.cancelCloseCpModal()}
  />
{/if}
```

---

### Frontend API та типи

#### `frontend/src/lib/api.ts` (після рядка ~130, поряд з іншими document-командами)

```typescript
export function documentChangeCounterparty(
  documentId: string,
  counterpartyId: string,
): Promise<ChangeCounterpartyResultDto> {
  return appInvoke('document_change_counterparty', { documentId, counterpartyId });
}
```

#### `frontend/src/lib/types.ts` (після рядка ~239, поряд з іншими document DTO)

```typescript
export interface ChangeCounterpartyResultDto {
  ok: boolean
  counterpartyId: string
  counterpartyName: string
}
```

---

### Backend — нова Tauri-команда

**Файл:** `src-tauri/src/commands/documents.rs`

```rust
#[tauri::command]
pub async fn document_change_counterparty(
    document_id: String,
    counterparty_id: String,
    state: State<'_, TauriState>,
) -> Result<ChangeCounterpartyResultDto, String>
```

**Логіка (parse_document_ref):**
```rust
let doc_ref = parse_document_ref(&document_id)?; // повертає (DocKind, Uuid)
let cp_uuid = Uuid::parse_str(&counterparty_id).map_err(...)?;

match doc_ref.kind {
    DocKind::Act => sqlx::query!(
        "UPDATE acts SET counterparty_id = $1, updated_at = now() WHERE id = $2",
        cp_uuid, doc_ref.id
    ).execute(&state.pool).await?,
    DocKind::Invoice => sqlx::query!(
        "UPDATE invoices SET counterparty_id = $1, updated_at = now() WHERE id = $2",
        cp_uuid, doc_ref.id
    ).execute(&state.pool).await?,
    DocKind::Waybill => sqlx::query!(
        "UPDATE waybills SET counterparty_id = $1, updated_at = now() WHERE id = $2",
        cp_uuid, doc_ref.id
    ).execute(&state.pool).await?,
}

// Отримати counterparty_name через окремий SELECT
let cp = sqlx::query!("SELECT name FROM counterparties WHERE id = $1", cp_uuid)
    .fetch_one(&state.pool).await?;

Ok(ChangeCounterpartyResultDto { ok: true, counterparty_id: counterparty_id.clone(), counterparty_name: cp.name })
```

**Реєстрація:** `src-tauri/src/lib.rs`, рядок ~43, у `tauri::Builder::invoke_handler` поряд з іншими document-командами.

**Acceptance criterion для тестів:** команда повинна успішно виконуватись для всіх трьох типів (`act:`, `inv:`, `wbl:`). Тест в `tests/` повинен перевірити мінімум один UPDATE для кожного типу або параметризувати один тест по трьох prefix-варіантах.

---

### Browser Fixtures

**Файл:** `frontend/src/lib/browser-fixtures.ts`

Mock для `counterparty_open_editor` розгалужується за `payload?.counterpartyId`:
- `payload.counterpartyId` відсутній або `''` → повертає порожню форму (`id: ''`, всі поля `''`, `title: 'Новий контрагент'`)
- `payload.counterpartyId === 'cp-1'` → повертає форму з даними fixture-контрагента `cp-1`
- інші id → повертає мінімально заповнену форму з `id = payload.counterpartyId`

Mock для `document_change_counterparty(payload)` → повертає `{ ok: true, counterpartyId: payload.counterpartyId, counterpartyName: fixtures.counterparties.find(cp => cp.id === payload.counterpartyId)?.name ?? 'Контрагент' }`.

---

### Тести

**Нові тести:**
- `frontend/src/lib/components/__tests__/CounterpartyModal.test.ts`
  - рендер create-режиму: заголовок «Новий контрагент»
  - рендер edit-режиму: заголовок «Редагування контрагента»
  - `showCloseConfirm=true` → відображає dirty-confirm блок
  - dispatch `closeConfirmed` при кліку «Так, закрити»
  - dispatch `closeCancelled` при кліку «Залишитись»
  - dispatch `save` при кліку «Зберегти»
- `frontend/src/lib/stores/__tests__/documents.store.test.ts` (оновити)
  - `openCpCreate` → `cpModal.mode === 'create'`, `confirmClose === false`
  - `saveCp` create mode → `editor.form.counterpartyId` оновлено, `editorSnapshot.counterpartyId` синхронізовано
  - `saveCp` edit mode → `editor.form.counterpartyName` оновлено, `editorSnapshot.counterpartyName` синхронізовано
  - `closeCpModal` з dirty → `cpModal.confirmClose === true`, `cpModal` не null
  - `closeCpModal` без dirty → `cpModal === null`
  - `confirmCloseCpModal` → `cpModal === null`
  - `changeCounterparty` → `editor.form` оновлено + `editorSnapshot` синхронізовано

---

## Що НЕ змінюється

- `CounterpartiesScreen.svelte` — без змін
- `counterparties` store — тільки викликається `counterparties.load()` після save (вже існує)
- Rust struct `Counterparty`, `counterparty_save`, `counterparty_open_editor` — без змін
- Документ після переприсвоєння не перезавантажується — лише оновлюється поле у формі + editorSnapshot
