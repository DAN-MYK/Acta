# Акти коригування — Design Spec

**Дата:** 2026-05-21  
**Статус:** Затверджено, готово до реалізації

---

## Контекст

Акти коригування — новий тип документу, що дозволяє коригувати суму вже підписаного або оплаченого акту (як збільшення, так і зменшення). Окрема таблиця, окрема вкладка у Documents screen, агрегуються з оригінальним актом у звітах.

---

## Секція 1: Схема БД

### Нова міграція `030_adjustment_acts.sql`

**Таблиця `adjustment_acts`:**

```sql
CREATE TYPE adjustment_act_status AS ENUM ('draft', 'issued', 'signed', 'applied');

CREATE TABLE adjustment_acts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id        UUID NOT NULL REFERENCES companies(id),
    original_act_id   UUID NOT NULL REFERENCES acts(id),
    counterparty_id   UUID NOT NULL REFERENCES counterparties(id),
    number            VARCHAR(50) NOT NULL,
    date              DATE NOT NULL,
    direction         VARCHAR(8) NOT NULL DEFAULT 'outgoing' CHECK (direction IN ('outgoing', 'incoming')),
    total_amount      DECIMAL(15,2) NOT NULL,  -- може бути від'ємним (зменшення)
    status            adjustment_act_status NOT NULL DEFAULT 'draft',
    notes             TEXT,
    bas_id            VARCHAR(100) UNIQUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE adjustment_act_items (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    adjustment_act_id UUID NOT NULL REFERENCES adjustment_acts(id) ON DELETE CASCADE,
    description       TEXT NOT NULL,
    quantity          DECIMAL(15,4) NOT NULL,
    unit_price        DECIMAL(15,2) NOT NULL,
    total_price       DECIMAL(15,2) NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_adjustment_acts_company ON adjustment_acts(company_id);
CREATE INDEX idx_adjustment_acts_original ON adjustment_acts(original_act_id);
CREATE INDEX idx_adjustment_acts_counterparty ON adjustment_acts(counterparty_id);
CREATE INDEX idx_adjustment_act_items_parent ON adjustment_act_items(adjustment_act_id);
```

**Зміни в таблиці `acts`:**

```sql
ALTER TABLE acts ADD COLUMN is_adjusted BOOLEAN NOT NULL DEFAULT FALSE;
```

**Нумерація:** `КОР-РРРР-NNN` (окрема серія від актів).

**Підтримка `is_adjusted` на рівні застосунку** (не тригер):
- Встановити `TRUE` коли перший корегуючий акт переходить у статус `applied`
- Встановити `FALSE` коли видалено останній корегуючий акт (перевірити COUNT)

---

## Секція 2: Rust-моделі та DB-шар

### `src/models/adjustment_act.rs` (новий файл)

```rust
pub enum AdjustmentActStatus { Draft, Issued, Signed, Applied }

pub struct AdjustmentAct {
    pub id: Uuid,
    pub company_id: Uuid,
    pub original_act_id: Uuid,
    pub counterparty_id: Uuid,
    pub number: String,
    pub date: NaiveDate,
    pub direction: String,
    pub total_amount: Decimal,
    pub status: AdjustmentActStatus,
    pub notes: Option<String>,
    pub bas_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct AdjustmentActItem {
    pub id: Uuid,
    pub adjustment_act_id: Uuid,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub total_price: Decimal,
}

pub struct AdjustmentActListRow {
    pub id: Uuid,
    pub company_id: Uuid,
    pub original_act_id: Uuid,
    pub original_act_number: String,
    pub counterparty_id: Uuid,
    pub counterparty_name: String,
    pub number: String,
    pub date: NaiveDate,
    pub total_amount: Decimal,
    pub status: AdjustmentActStatus,
}

pub struct NewAdjustmentAct { /* поля без id/timestamps */ }
pub struct NewAdjustmentActItem { /* поля без id */ }
pub struct UpdateAdjustmentAct { /* редаговані поля */ }
```

**Зміни в існуючих моделях:**
- `src/models/act.rs` → `ActListRow`: додати `pub is_adjusted: bool`

**Зміни в `src/models/mod.rs`:**
- Додати `pub mod adjustment_act;` та re-exports

### `src/db/adjustment_acts.rs` (новий файл)

Функції (всі `_scoped` — перевіряють `company_id`):

| Функція | Опис |
|---|---|
| `generate_next_number(pool, company_id, year)` | `КОР-РРРР-NNN`, rsplit_once('-') паттерн |
| `create(pool, company_id, original_act_id, ...)` | INSERT з транзакцією |
| `get_full(pool, company_id, id)` | SELECT з items |
| `list_filtered(pool, company_id, filter)` | QueryBuilder з пагінацією |
| `update_with_items_scoped(pool, company_id, id, ...)` | DELETE items + INSERT в транзакції |
| `change_status_scoped(pool, company_id, id, new_status)` | UPDATE + підтримка is_adjusted |
| `delete_scoped(pool, company_id, id)` | DELETE + підтримка is_adjusted |
| `list_for_act(pool, company_id, original_act_id)` | всі корегування до акту |

**Підтримка `is_adjusted` в `change_status_scoped`:**
```rust
if new_status == Applied {
    sqlx::query("UPDATE acts SET is_adjusted = TRUE WHERE id = $1 AND company_id = $2")...
}
```

**Підтримка `is_adjusted` в `delete_scoped`:**
```rust
let remaining = sqlx::query_scalar::<_, i64>(
    "SELECT COUNT(*) FROM adjustment_acts WHERE original_act_id = $1 AND id != $2 AND status = 'applied'"
)...;
if remaining == 0 {
    sqlx::query("UPDATE acts SET is_adjusted = FALSE WHERE id = $1")...
}
```

**Зміни в `src/db/acts.rs`:**
- `list_filtered` SQL: додати `a.is_adjusted` у SELECT

**Зміни в `src/db/mod.rs`:**
- Додати `pub mod adjustment_acts;`
- Додати `let _ = adjustment_acts::list_filtered;` у compile-smoke test

### Інтеграційні тести

Новий файл `tests/db_integration/adjustment_acts.rs`, підключений у `tests/db_integration.rs` як `mod adjustment_acts;`.

Тести:
- `test_adjustment_act_create_and_get`
- `test_adjustment_act_numbering`
- `test_adjustment_act_status_advance_sets_is_adjusted`
- `test_adjustment_act_delete_clears_is_adjusted`
- `test_adjustment_acts_list_for_act`

---

## Секція 3: Tauri-команди

### Нові команди в `src-tauri/src/commands/documents.rs`:

```rust
#[tauri::command]
pub async fn adjustment_acts_list(
    state: State<'_, TauriState>,
    request: DocumentsListRequest,
) -> CommandResult<DocumentsListDto> { ... }

#[tauri::command]
pub async fn act_adjustments_list(
    state: State<'_, TauriState>,
    act_id: String,
) -> CommandResult<Vec<DocumentItemDto>> { ... }
```

### Зміни в `src/tauri_api/documents/dto.rs`:

- `DocumentKindDto` → додати `AdjustmentAct`
- `DocumentStatusDto` → додати `Applied`
- `DocumentsListDto` → додати `adjustment_act_items: Vec<DocumentItemDto>`
- `CreateDocumentDraftRequest` → додати `original_act_id: Option<String>`
- `DocumentDraftFormDto` → додати `original_act_id: Option<String>`, `original_act_number: Option<String>`
- `DocumentItemDto` → `linked_id: Option<String>` для `adj:uuid → act:uuid` зв'язку

### Зміни в `src/tauri_api/documents/api.rs`:

- `parse_document_ref` → додати `"adj:" prefix → DocumentRef::AdjustmentAct(uuid)`
- `documents_list` → 4-way `tokio::join!` (додати adjustment_acts)
- `document_open`, `document_save`, `document_advance_status`, `document_delete` → додати `DocumentRef::AdjustmentAct` гілки
- `document_chain_get` для `adj:uuid` → повертає 1-кроковий chain (тільки поточний статус adj)

---

## Секція 4: Svelte UI

### `frontend/src/lib/types.ts`:

```typescript
type DocumentKind = "invoice" | "act" | "waybill" | "adjustment_act";
type DocumentStatus = "draft" | "issued" | "signed" | "paid" | "delivered" | "applied";
// DocumentsListDto: додати adjustmentActItems: DocumentItemDto[]
```

### `frontend/src/lib/config/documents.ts`:

- `DOCUMENT_KIND_META` → додати запис `adjustment_act` (label: "Коригування", icon, colors)
- `DOCUMENT_KIND_FILTER_OPTIONS` → додати `{ value: "adjustment_act", label: "Коригування" }` (ТІЛЬКИ тут, не в create picker)
- `DOCUMENT_KIND_OPTIONS` → **НЕ додавати** (adj акт створюється тільки з drawer оригінального акту)
- `resolveDocumentKindMeta` → додати `if (normalized === "adjustment_act")` гілку
- `supportsDocumentPdfGeneration` → додати `|| kind === "adjustment_act"`
- `DOCUMENT_STATUS_OPTIONS` → додати `{ value: "applied", label: "Застосовано" }`
- `getDocumentChainTargets` → без змін (повертає `[]` для adj_act за замовчуванням)

### `frontend/src/lib/screens/DocumentsScreen.svelte`:

- Кнопка "Створити" → disabled коли `kindFilter === "adjustment_act"` (adj не можна створити напряму)
- Drawer для `adj:uuid`:
  - Поле "Оригінальний акт" — read-only, показує `original_act_number`
  - Поле "Напрямок" — read-only (успадковується від оригіналу)
  - Кнопка "Змінити контрагента" — прихована
- Drawer для `act:uuid` → кнопка "+ Коригування" → викликає `store.createAdjustmentActDraft(act.id)`
- `getCurrentChainStatus()` → без змін (для adj повертає 1-кроковий chain з `document_chain_get`)

### `frontend/src/lib/stores/documents.ts`:

Новий метод `createAdjustmentActDraft(actId: string)`:
```typescript
async createAdjustmentActDraft(actId: string) {
    const draft = await api.document_create_draft({ kind: "adjustment_act", original_act_id: actId });
    // відкрити drawer з новим adj draft
}
```

### `frontend/src/lib/browser-fixtures.ts`:

- `documentsList()` → додати `adjustmentActItems: []`

---

## Секція 5: PDF-генерація

**Підхід:** Typst CLI (як для звичайних актів), без збереження шляху в БД.

**Шлях файлу:** `storage/documents/adjustment_acts/РРРР/КОР-2026-001.pdf`

### Новий шаблон `templates/adjustment_act.typ`:

Аналогічний до `templates/act.typ`, але:
- Заголовок "АКТ КОРИГУВАННЯ №{number} до АКТ №{original_act_number}"
- Таблиця позицій коригування
- Блок підписів аналогічний до акту

### Зміни в `src/pdf/generator.rs`:

```rust
pub struct PdfAdjustmentActData {
    pub number: String,
    pub date: NaiveDate,
    pub original_act_number: String,
    pub company_name: String,
    pub counterparty_name: String,
    pub items: Vec<PdfActItem>,  // перевикористати
    pub total_amount: Decimal,
    pub notes: Option<String>,
}

fn ensure_adj_output_dir(year: i32) -> Result<PathBuf>; // storage/documents/adjustment_acts/YYYY/
pub async fn generate_adjustment_act_pdf(data: &PdfAdjustmentActData) -> Result<PathBuf>;
```

### Зміни в `src/tauri_api/documents/pdf.rs`:

- `load_existing_pdf_path` → `DocumentRef::AdjustmentAct(_)` → `None`
- `document_ref_uuid` → додати `AdjustmentAct` гілку
- `generate_document_pdf` → додати `DocumentRef::AdjustmentAct` → виклик `generate_adjustment_act_pdf`

---

## Секція 6: Вплив на звіти та Dashboard

**Принцип:** Скрізь де показується `a.total_amount` для актів — замінити на **ефективну суму**:

```
effective_amount = a.total_amount + COALESCE(SUM(adj.total_amount) WHERE adj.status != 'draft', 0)
```

### SQL Паттерн А — скалярний підзапит (для рядкових запитів / UNION ALL):

```sql
a.total_amount + COALESCE(
    (SELECT SUM(aa.total_amount) FROM adjustment_acts aa
     WHERE aa.original_act_id = a.id AND aa.status != 'draft'),
    0
) AS amount
```

### SQL Паттерн Б — CTE + LEFT JOIN (для агрегатних запитів з GROUP BY):

```sql
WITH adj_sums AS (
    SELECT original_act_id, SUM(total_amount) AS adj_total
    FROM adjustment_acts WHERE status != 'draft'
    GROUP BY original_act_id
)
-- FROM acts a LEFT JOIN adj_sums adj ON adj.original_act_id = a.id
-- використовувати: a.total_amount + COALESCE(adj.adj_total, 0)
```

### `src/db/reports.rs` — 4 функції (Паттерн А):

| Функція | Зміна |
|---|---|
| `load_pnl_rows` | `a.total_amount AS amount` у CTE `docs` (acts-гілка) |
| `load_receivables_rows` | `a.total_amount AS amount` у SELECT acts-гілки |
| `load_top_counterparties_receivables` | `a.total_amount AS amount` у CTE `docs` (acts-гілка) |
| `load_top_counterparties_pnl` | `a.total_amount AS amount` у CTE `docs` (acts-гілка) |

### `src/db/dashboard.rs` — 7 функцій:

| Функція | Паттерн | Де змінюється |
|---|---|---|
| `get_kpi_summary` | Б | `SUM(total_amount) FILTER (...)` — `revenue_this_month` і `unpaid_total` |
| `revenue_by_month` | Б | `SUM(total_amount) FILTER (WHERE status = 'paid')` |
| `expenses_by_month` | А | `a.total_amount AS amount` у CTE `expense_docs` (acts-гілка) |
| `category_breakdown` | А | `a.total_amount AS amount` у CTE `expense_docs` (acts-гілка) |
| `upcoming_payments` | А | `a.total_amount AS amount` |
| `get_recent_acts` | А | `a.total_amount AS amount` |
| `inbox_items` | А | `a.total_amount AS amount` |

### Не змінюються:

- `load_bank_rows`, `compute_opening_balance`, `load_payables_rows` — оперують платежами/payment_schedule
- `load_top_counterparties_bank`, `load_top_counterparties_payables` — аналогічно
- `acts_status_distribution` — COUNT по статусах, сума не потрібна

---

## Нові файли

| Файл | Призначення |
|---|---|
| `migrations/030_adjustment_acts.sql` | Нова таблиця + is_adjusted на acts |
| `src/models/adjustment_act.rs` | Rust-моделі |
| `src/db/adjustment_acts.rs` | CRUD-функції |
| `templates/adjustment_act.typ` | Typst шаблон PDF |
| `tests/db_integration/adjustment_acts.rs` | Інтеграційні тести |
| `docs/superpowers/specs/2026-05-21-adjustment-acts-design.md` | Цей документ |

## Файли, що модифікуються

| Файл | Зміна |
|---|---|
| `src/models/act.rs` | `ActListRow.is_adjusted: bool` |
| `src/models/mod.rs` | `pub mod adjustment_act;` |
| `src/db/acts.rs` | `a.is_adjusted` у `list_filtered` SELECT |
| `src/db/mod.rs` | `pub mod adjustment_acts;` + compile-smoke |
| `src/tauri_api/documents/dto.rs` | Нові варіанти enum, нові поля |
| `src/tauri_api/documents/api.rs` | `adj:` prefix, 4-way join, нові гілки |
| `src/tauri_api/documents/pdf.rs` | AdjustmentAct гілки у всіх match |
| `src/pdf/generator.rs` | `PdfAdjustmentActData`, `generate_adjustment_act_pdf` |
| `src-tauri/src/commands/documents.rs` | 2 нові команди |
| `src/db/reports.rs` | 4 функції — ефективна сума |
| `src/db/dashboard.rs` | 7 функцій — ефективна сума |
| `frontend/src/lib/types.ts` | Нові варіанти типів |
| `frontend/src/lib/config/documents.ts` | Метадані adj_act |
| `frontend/src/lib/screens/DocumentsScreen.svelte` | Adj-специфічна логіка |
| `frontend/src/lib/stores/documents.ts` | `createAdjustmentActDraft` |
| `frontend/src/lib/browser-fixtures.ts` | `adjustmentActItems: []` |
