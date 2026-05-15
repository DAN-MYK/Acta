# Spec: Редагування та створення контрагентів з форми документів

**Дата:** 2026-05-15  
**Статус:** Затверджено

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
```

**Events (createEventDispatcher):**
- `fieldChange: { field: keyof CounterpartyDraftFormDto, value: string }`
- `save`
- `close`

**Структура UI:**
- `div.modal-overlay` (фіксований, z-index поверх drawer) → клік → `close`
- `div.modal-panel` (~480px, role="dialog", aria-modal="true")
  - Заголовок: «Новий контрагент» (create) або «Редагування контрагента» (edit) + кнопка ×
  - Форма `.cp-editor-grid`: поля `name`, `edrpou`, `ipn`, `iban`, `phone`, `email`, `address` (span-2), `notes` (span-2, textarea)
  - Footer: `[Зберегти]` (btn-primary, disabled якщо loading) + `[Скасувати]` (btn-ghost)
  - Dirty-banner: якщо `isDirty` і натиснуто ×/Скасувати — «Є незбережені зміни. Закрити без збереження?» + `[Так, закрити]` `[Залишитись]`

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
} | null
```

**5 нових функцій:**

| Функція | Логіка |
|---|---|
| `openCpCreate()` | `counterpartyOpenEditor()` → `cpModal = { isOpen: true, mode: 'create', form, snapshot: cloneSnapshot(form), loading: false }` |
| `openCpEdit(id: string)` | `counterpartyOpenEditor(id)` → `cpModal = { isOpen: true, mode: 'edit', form, snapshot: cloneSnapshot(form), loading: false }` |
| `updateCpField(field, value)` | `cpModal.form[field] = value` |
| `saveCp()` | `counterpartySave(cpModal.form)` → знайти ім'я: `result.updatedList.find(cp => cp.id === result.savedId)?.name`; якщо create: `editor.form.counterpartyId = result.savedId`, `editor.form.counterpartyName = знайдене ім'я`; якщо edit: оновити `editor.form.counterpartyName` → `counterparties.load()` → `closeCpModal(true)` |
| `closeCpModal(force?)` | `isEditorFormDirty(snapshot, form)` → якщо dirty і не force → встановити dirty-flag в cpModal; інакше `cpModal = null` |

**Додаткова функція:**

| Функція | Логіка |
|---|---|
| `changeCounterparty(docId, cpId)` | `documentChangeCounterparty(docId, cpId)` → `editor.form.counterpartyId = result.counterpartyId`, `editor.form.counterpartyName = result.counterpartyName` |

`isDirtyCpModal` — derived: `isEditorFormDirty(cpModal.snapshot, cpModal.form)`

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
  selectElement.value = ''; // скинути select
  documents.openCpCreate();
  return;
}
```

#### Існуючі документи

Замінити read-only текст на блок з трьома станами через local variable `isReassigning`:

**Стан 1 — перегляд (default):**
```html
<span>{form.counterpartyName}</span>
<button on:click={() => documents.openCpEdit(form.counterpartyId)}>Редагувати</button>
<button on:click={() => isReassigning = true}>Змінити</button>
```

**Стан 2 — переприсвоєння (`isReassigning`):**
```html
<select bind:value={reassignTargetId}>
  {#each counterpartiesList as cp}
    <option value={cp.id}>{cp.name}</option>
  {/each}
</select>
<button on:click={() => documents.changeCounterparty(form.id, reassignTargetId)}>Зберегти</button>
<button on:click={() => isReassigning = false}>Скасувати</button>
```

#### Рендер модала (у кінці розмітки drawer)

```html
{#if $documents.cpModal?.isOpen}
  <CounterpartyModal
    isOpen={$documents.cpModal.isOpen}
    mode={$documents.cpModal.mode}
    form={$documents.cpModal.form}
    loading={$documents.cpModal.loading}
    isDirty={$isDirtyCpModal}
    on:fieldChange={(e) => documents.updateCpField(e.detail.field, e.detail.value)}
    on:save={() => documents.saveCp()}
    on:close={() => documents.closeCpModal()}
  />
{/if}
```

---

### Backend — нова Tauri-команда

**Файл:** `src-tauri/src/commands/documents.rs`

```rust
#[tauri::command]
pub async fn document_change_counterparty(
    document_id: String,
    counterparty_id: String,
    state: State<'_, AppState>,
) -> Result<ChangeCounterpartyResultDto, String>
```

**SQL:**
```sql
UPDATE documents
SET counterparty_id = $1::uuid, updated_at = now()
WHERE id = $2::uuid
RETURNING id
```

Після UPDATE — окремий SELECT для отримання `counterparty_name` (через JOIN з counterparties).

**DTO (types.ts):**
```typescript
interface ChangeCounterpartyResultDto {
  ok: boolean
  counterpartyId: string
  counterpartyName: string
}
```

Реєстрація команди в `src-tauri/src/main.rs` у `tauri::Builder::invoke_handler`.

---

### Browser Fixtures

**Файл:** `frontend/src/lib/browser-fixtures.ts`

Додати mock для `counterparty_open_editor`:
- Без аргументу (create) → повертає `CounterpartyEditorDto` з порожньою формою (`id: ''`, всі поля `''`, `title: 'Новий контрагент'`)
- З `id` (edit) → повертає заповнену форму на основі існуючих fixture-контрагентів

Додати mock для `document_change_counterparty` → повертає `{ ok: true, counterpartyId, counterpartyName }`.

---

### Тести

**Нові тести:**
- `frontend/src/lib/components/__tests__/CounterpartyModal.test.ts`
  - рендер create/edit режиму
  - dirty-check при спробі закрити
  - dispatch save/close events
- `frontend/src/lib/stores/__tests__/documents.store.test.ts` (оновити)
  - `openCpCreate` → `cpModal.mode === 'create'`
  - `saveCp` create mode → `editor.form.counterpartyId` оновлено
  - `saveCp` edit mode → `editor.form.counterpartyName` оновлено
  - `changeCounterparty` → форма оновлена

---

## Що НЕ змінюється

- CounterpartiesScreen.svelte — без змін
- counterparties store — тільки викликається `counterparties.load()` після save (вже існує)
- Rust struct `Counterparty`, `counterparty_save`, `counterparty_open_editor` — без змін
- Документ після переприсвоєння не перезавантажується — лише оновлюється поле у формі
