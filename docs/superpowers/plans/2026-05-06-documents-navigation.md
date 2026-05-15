# Documents Navigation Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add direction tabs (Всі / Вихідні / Вхідні) and document-type chip filters to the Documents screen, with full direction propagation through create, save, chain-create, and list-filter flows.

**Architecture:** `DocumentDirection` already exists in domain models and DB; the work is threading it into all DTOs (list items, form, create requests) and wiring the filter from the new `DocumentsListRequest.direction` field through each DB query. Frontend adds `activeTab`/`kindFilter` state to the documents store, then the UI renders tabs, chips, a direction badge in list rows, and a radio toggle in the editor.

**Tech Stack:** Rust (anyhow, sqlx, Tauri), Svelte + TypeScript

---

## File Map

| File | Change |
|------|--------|
| `src/models/shared.rs` | Add `#[serde(rename_all = "lowercase")]` to `DocumentDirection` |
| `src/models/act.rs` | Add `direction` to `UpdateAct` |
| `src/models/invoice.rs` | Add `direction` to `UpdateInvoice` |
| `src/models/waybill.rs` | Add `direction` to `UpdateWaybill` |
| `src/db/acts.rs` | Add `direction = $9` to UPDATE SQL |
| `src/db/invoices.rs` | Add `direction = $9` to UPDATE SQL |
| `src/db/waybills.rs` | Add `direction = $8` to UPDATE SQL |
| `src/tauri_api/documents/dto.rs` | Replace `tab` with `direction+kind`; add `direction` to `DocumentItemDto`, `DocumentDraftFormDto`, `CreateDocumentDraftRequest` |
| `src/tauri_api/documents/api.rs` | Add `direction` to `DocumentSnapshot`; wire through all construction sites, create flow, save flow, chain flow, list filter |
| `src/tauri_api/dashboard.rs` | Add `direction` to local `RowDto`, SQL, and mapper |
| `src/tauri_api/counterparties.rs` | Add `direction` to 2 `DocumentItemDto` helper functions |
| `src/tauri_api/shell.rs` | Add `direction: "outgoing"` to `CreateDocumentDraftRequest` |
| `tests/tauri_vertical_slice.rs` | Fix `CreateDocumentDraftRequest` construction sites; add direction filter test |
| `frontend/src/lib/types.ts` | Add `DocumentDirection` type; add `direction` to `DocumentItemDto` and `DocumentDraftFormDto` |
| `frontend/src/lib/api.ts` | Update `documentsList` and `documentCreateDraft` signatures |
| `frontend/src/lib/stores/documents.ts` | Add `activeTab`, `kindFilter`, `reloadList`, `setTab`, `setKindFilter`; update `create` |
| `frontend/src/lib/screens/DocumentsScreen.svelte` | Tab bar, chip filters, direction badge in row, direction radio in editor |

---

## Task 1: `DocumentDirection` — lowercase serde

**Files:**
- Modify: `src/models/shared.rs`

- [ ] **Step 1: Add `#[serde(rename_all = "lowercase")]`**

In `src/models/shared.rs`, change the derive block:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum DocumentDirection {
```

- [ ] **Step 2: Verify compile**

```powershell
cargo build --lib
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```powershell
git add src/models/shared.rs
git commit -m "feat(models): add serde lowercase to DocumentDirection"
```

---

## Task 2: `UpdateAct/Invoice/Waybill` — add `direction` + SQL + `document_save`

**Files:**
- Modify: `src/models/act.rs`, `src/models/invoice.rs`, `src/models/waybill.rs`
- Modify: `src/db/acts.rs`, `src/db/invoices.rs`, `src/db/waybills.rs`
- Modify: `src/tauri_api/documents/api.rs`

- [ ] **Step 1: Add `direction` to `UpdateAct`**

In `src/models/act.rs`, find `pub struct UpdateAct` (line ~155):

```rust
pub struct UpdateAct {
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub direction: DocumentDirection,  // new
    pub date: NaiveDate,
    pub expected_payment_date: Option<NaiveDate>,
    pub notes: Option<String>,
}
```

- [ ] **Step 2: Add `direction` to `UpdateInvoice`**

In `src/models/invoice.rs`, find `pub struct UpdateInvoice` (line ~155):

```rust
pub struct UpdateInvoice {
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub direction: DocumentDirection,  // new
    pub date: NaiveDate,
    pub expected_payment_date: Option<NaiveDate>,
    pub notes: Option<String>,
}
```

- [ ] **Step 3: Add `direction` to `UpdateWaybill`**

In `src/models/waybill.rs`, find `pub struct UpdateWaybill` (line ~142):

```rust
pub struct UpdateWaybill {
    pub number: String,
    pub counterparty_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub direction: DocumentDirection,  // new
    pub date: NaiveDate,
    pub notes: Option<String>,
}
```

- [ ] **Step 4: Update SQL in `db/acts.rs`**

In `src/db/acts.rs`, find `update_with_items` SQL (line ~682). Add `direction = $9` and a new bind:

```rust
let act = sqlx::query_as::<_, Act>(
    r#"
    UPDATE acts
    SET number                 = $2,
        counterparty_id        = $3,
        contract_id            = $4,
        category_id            = $5,
        date                   = $6,
        expected_payment_date  = $7,
        notes                  = $8,
        direction              = $9,
        updated_at             = NOW()
    WHERE id = $1
    RETURNING id, number, counterparty_id, contract_id, category_id, direction,
              date, expected_payment_date, total_amount,
              status, notes, bas_id, created_at, updated_at
    "#,
)
.bind(id)
.bind(&data.number)
.bind(data.counterparty_id)
.bind(data.contract_id)
.bind(data.category_id)
.bind(data.date)
.bind(data.expected_payment_date)
.bind(&data.notes)
.bind(data.direction)  // new $9
.fetch_optional(&mut *tx)
```

- [ ] **Step 5: Update SQL in `db/invoices.rs`**

In `src/db/invoices.rs`, find `update_with_items` SQL (line ~877). Add `direction = $9`:

```rust
let invoice = sqlx::query_as::<_, Invoice>(
    r#"
    UPDATE invoices
    SET number                = $2,
        counterparty_id       = $3,
        contract_id           = $4,
        category_id           = $5,
        date                  = $6,
        expected_payment_date = $7,
        notes                 = $8,
        direction             = $9,
        updated_at            = NOW()
    WHERE id = $1
    RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
              date, expected_payment_date, total_amount, vat_amount,
              status, notes, pdf_path, bas_id, created_at, updated_at
    "#,
)
.bind(id)
.bind(&data.number)
.bind(data.counterparty_id)
.bind(data.contract_id)
.bind(data.category_id)
.bind(data.date)
.bind(data.expected_payment_date)
.bind(&data.notes)
.bind(data.direction)  // new $9
.fetch_optional(&mut *tx)
```

- [ ] **Step 6: Update SQL in `db/waybills.rs`**

In `src/db/waybills.rs`, find `update_with_items` SQL (line ~313). Add `direction = $8` (waybills has no `expected_payment_date`, so `notes` is currently $7):

```rust
let waybill = sqlx::query_as::<_, Waybill>(
    r#"
    UPDATE waybills
    SET number          = $2,
        counterparty_id = $3,
        contract_id     = $4,
        category_id     = $5,
        date            = $6,
        notes           = $7,
        direction       = $8,
        updated_at      = NOW()
    WHERE id = $1
    RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
              date, total_amount, status, notes, pdf_path, bas_id, created_at, updated_at
    "#,
)
.bind(id)
.bind(&data.number)
.bind(data.counterparty_id)
.bind(data.contract_id)
.bind(data.category_id)
.bind(data.date)
.bind(&data.notes)
.bind(data.direction)  // new $8
.fetch_optional(&mut *tx)
```

- [ ] **Step 7: Fix `document_save` construction sites in `api.rs`**

In `src/tauri_api/documents/api.rs`, find `document_save` (line ~998). Add `direction: act.direction` / `invoice.direction` / `waybill.direction` to each `Update*` struct. The existing DB fetch for each document type already provides the model:

**Act branch** (~line 1018):
```rust
db::acts::update_with_items(
    ctx.pool(),
    id,
    UpdateAct {
        number: request.form.number.clone(),
        counterparty_id: act.counterparty_id,
        contract_id: act.contract_id,
        category_id: act.category_id,
        direction: act.direction,  // new — read from DB record
        date,
        expected_payment_date: act.expected_payment_date,
        notes: compose_notes_with_chain_parent(
            &request.form.notes,
            parent_ref.as_deref(),
        ),
    },
    draft_items_to_new_act(request.items)?,
)
```

**Invoice branch** (~line 1048):
```rust
db::invoices::update_with_items(
    ctx.pool(),
    id,
    UpdateInvoice {
        number: request.form.number.clone(),
        counterparty_id: invoice.counterparty_id,
        contract_id: invoice.contract_id,
        category_id: invoice.category_id,
        direction: invoice.direction,  // new
        date,
        expected_payment_date: invoice.expected_payment_date,
        notes: compose_notes_with_chain_parent(
            &request.form.notes,
            parent_ref.as_deref(),
        ),
    },
    draft_items_to_new_invoice(request.items)?,
)
```

**Waybill branch** (~line 1075) — find the `update_with_items` call and add direction:
```rust
db::waybills::update_with_items(
    ctx.pool(),
    id,
    UpdateWaybill {
        number: request.form.number.clone(),
        counterparty_id: waybill.counterparty_id,
        contract_id: waybill.contract_id,
        category_id: waybill.category_id,
        direction: waybill.direction,  // new
        date,
        notes: compose_notes_with_chain_parent(
            &request.form.notes,
            parent_ref.as_deref(),
        ),
    },
    draft_items_to_new_waybill(request.items)?,
)
```

- [ ] **Step 8: Verify compile**

```powershell
cargo build --tests
```

Expected: `Finished` with no errors.

- [ ] **Step 9: Commit**

```powershell
git add src/models/act.rs src/models/invoice.rs src/models/waybill.rs
git add src/db/acts.rs src/db/invoices.rs src/db/waybills.rs
git add src/tauri_api/documents/api.rs
git commit -m "feat(db): add direction to UpdateAct/Invoice/Waybill and SQL"
```

---

## Task 3: DTOs + all construction sites (atomic)

When a required field is added to a struct, ALL struct-literal construction sites must be updated in the same step.

**Files:**
- Modify: `src/tauri_api/documents/dto.rs`
- Modify: `src/tauri_api/documents/api.rs` (many sites)
- Modify: `src/tauri_api/dashboard.rs`
- Modify: `src/tauri_api/counterparties.rs`
- Modify: `src/tauri_api/shell.rs`
- Modify: `tests/tauri_vertical_slice.rs`

- [ ] **Step 1: Update `dto.rs`**

In `src/tauri_api/documents/dto.rs`:

1. **`DocumentsListRequest`** — replace `tab` with `direction` + `kind`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentsListRequest {
    pub query: Option<String>,
    pub direction: Option<DocumentDirection>,
    pub kind: Option<String>,
}
```
Add `use crate::models::DocumentDirection;` at the top if not already imported.

2. **`DocumentItemDto`** — add `direction: String`:
```rust
pub struct DocumentItemDto {
    pub id: String,
    pub kind: DocumentKindDto,
    pub number: String,
    pub date: String,
    pub counterparty: String,
    pub amount_str: String,
    pub status: DocumentStatusDto,
    pub status_label: String,
    pub linked_id: String,
    pub direction: String,  // new: "outgoing" | "incoming"
}
```

3. **`DocumentDraftFormDto`** — add `direction: String`:
```rust
pub struct DocumentDraftFormDto {
    pub id: String,
    pub kind: String,
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub title: String,
    pub number: String,
    pub date: String,
    pub notes: String,
    pub direction: String,  // new: "outgoing" | "incoming"
}
```

4. **`CreateDocumentDraftRequest`** — add `direction: String`:
```rust
pub struct CreateDocumentDraftRequest {
    pub counterparty_id: String,
    pub kind: String,
    pub direction: String,  // new: "outgoing" | "incoming"
}
```

- [ ] **Step 2: Add `direction` to `DocumentSnapshot` in `api.rs`**

In `src/tauri_api/documents/api.rs`, find `struct DocumentSnapshot` (line ~36):

```rust
struct DocumentSnapshot {
    ref_id: String,
    kind: String,
    number: String,
    counterparty_id: Uuid,
    counterparty_name: String,
    date: NaiveDate,
    total_amount: Decimal,
    status: String,
    notes: Option<String>,
    items: Vec<DocumentDraftItemDto>,
    direction: DocumentDirection,  // new
}
```

- [ ] **Step 3: Populate `direction` in `load_document_snapshot` (3 sites)**

In `api.rs`, find `load_document_snapshot` (line ~305). Add `direction` to each branch:

**Act branch** (~line 315):
```rust
Ok(DocumentSnapshot {
    ref_id: document_ref_string("act", id),
    kind: "act".to_string(),
    number: act.number.clone(),
    counterparty_id: act.counterparty_id,
    counterparty_name: load_counterparty_name(pool, company_id, act.counterparty_id).await?,
    date: act.date,
    total_amount: act.total_amount,
    status: act.status.as_str().to_string(),
    notes: act.notes.clone(),
    items: act_items_to_draft(items),
    direction: act.direction,  // new
})
```

**Invoice branch** (~line 333):
```rust
Ok(DocumentSnapshot {
    ref_id: document_ref_string("invoice", id),
    kind: "invoice".to_string(),
    number: invoice.number.clone(),
    counterparty_id: invoice.counterparty_id,
    counterparty_name: load_counterparty_name(pool, company_id, invoice.counterparty_id).await?,
    date: invoice.date,
    total_amount: invoice.total_amount,
    status: invoice.status.as_str().to_string(),
    notes: invoice.notes.clone(),
    items: invoice_items_to_draft(items),
    direction: invoice.direction,  // new
})
```

**Waybill branch** (~line 355):
```rust
Ok(DocumentSnapshot {
    ref_id: document_ref_string("waybill", id),
    kind: "waybill".to_string(),
    number: waybill.number.clone(),
    counterparty_id: waybill.counterparty_id,
    counterparty_name: load_counterparty_name(pool, company_id, waybill.counterparty_id).await?,
    date: waybill.date,
    total_amount: waybill.total_amount,
    status: waybill.status.as_str().to_string(),
    notes: waybill.notes.clone(),
    items: waybill_items_to_draft(items),
    direction: waybill.direction,  // new
})
```

- [ ] **Step 4: Populate `direction` in `snapshot_from_act/invoice/waybill` (3 functions)**

`snapshot_from_act` (~line 382):
```rust
Ok(DocumentSnapshot {
    ref_id: document_ref_string("act", act.id),
    kind: "act".to_string(),
    number: act.number,
    counterparty_id: act.counterparty_id,
    counterparty_name: load_counterparty_name(pool, company_id, act.counterparty_id).await?,
    date: act.date,
    total_amount: act.total_amount,
    status: act.status.as_str().to_string(),
    notes: act.notes,
    items: act_items_to_draft(items),
    direction: act.direction,  // new
})
```

`snapshot_from_invoice` (~line 402):
```rust
Ok(DocumentSnapshot {
    ref_id: document_ref_string("invoice", invoice.id),
    kind: "invoice".to_string(),
    number: invoice.number,
    counterparty_id: invoice.counterparty_id,
    counterparty_name: load_counterparty_name(pool, company_id, invoice.counterparty_id).await?,
    date: invoice.date,
    total_amount: invoice.total_amount,
    status: invoice.status.as_str().to_string(),
    notes: invoice.notes,
    items: invoice_items_to_draft(items),
    direction: invoice.direction,  // new
})
```

`snapshot_from_waybill` (~line 423):
```rust
Ok(DocumentSnapshot {
    ref_id: document_ref_string("waybill", waybill.id),
    kind: "waybill".to_string(),
    number: waybill.number,
    counterparty_id: waybill.counterparty_id,
    counterparty_name: load_counterparty_name(pool, company_id, waybill.counterparty_id).await?,
    date: waybill.date,
    total_amount: waybill.total_amount,
    status: waybill.status.as_str().to_string(),
    notes: waybill.notes,
    items: waybill_items_to_draft(items),
    direction: waybill.direction,  // new
})
```

- [ ] **Step 5: Update `build_existing_document_form` — 3 `DocumentDraftFormDto` sites**

In `api.rs`, find `build_existing_document_form` (line ~471). Each branch constructs `DocumentDraftFormDto`. Add `direction`:

**Act branch** (~line 485):
```rust
form: DocumentDraftFormDto {
    id: format!("act:{id}"),
    kind: "act".to_string(),
    counterparty_id: act.counterparty_id.to_string(),
    counterparty_name,
    title: kind_title("act", false).to_string(),
    number: act.number,
    date: date_to_str(act.date),
    notes: split_visible_notes_and_chain_parent(act.notes.as_deref()).0,
    direction: act.direction.as_str().to_string(),  // new
},
```

**Invoice branch** (~line 512):
```rust
form: DocumentDraftFormDto {
    id: format!("inv:{id}"),
    kind: "invoice".to_string(),
    counterparty_id: invoice.counterparty_id.to_string(),
    counterparty_name,
    title: kind_title("invoice", false).to_string(),
    number: invoice.number,
    date: date_to_str(invoice.date),
    notes: split_visible_notes_and_chain_parent(invoice.notes.as_deref()).0,
    direction: invoice.direction.as_str().to_string(),  // new
},
```

**Waybill branch** (~line 539):
```rust
form: DocumentDraftFormDto {
    id: format!("wbl:{id}"),
    kind: "waybill".to_string(),
    counterparty_id: waybill.counterparty_id.to_string(),
    counterparty_name,
    title: kind_title("waybill", false).to_string(),
    number: waybill.number,
    date: date_to_str(waybill.date),
    notes: split_visible_notes_and_chain_parent(waybill.notes.as_deref()).0,
    direction: waybill.direction.as_str().to_string(),  // new
},
```

- [ ] **Step 6: Update `create_draft_form` — 3 `DocumentDraftFormDto` sites (temporarily hardcode Outgoing)**

In `api.rs`, find `create_draft_form` (line ~558). Add `direction` to each `DocumentDraftFormDto`. Use `DocumentDirection::Outgoing.as_str()` for now (will be wired in Task 4):

**Act branch** (~line 589):
```rust
Ok(DocumentDraftFormDto {
    id: format!("act:{}", act.id),
    kind: "act".to_string(),
    counterparty_id: counterparty_id.to_string(),
    counterparty_name,
    title: kind_title("act", true).to_string(),
    number,
    date: date_to_str(today),
    notes: String::new(),
    direction: DocumentDirection::Outgoing.as_str().to_string(),  // temp — wired in Task 4
})
```

**Invoice branch** (~line 620):
```rust
Ok(DocumentDraftFormDto {
    id: format!("inv:{}", invoice.id),
    kind: "invoice".to_string(),
    counterparty_id: counterparty_id.to_string(),
    counterparty_name,
    title: kind_title("invoice", true).to_string(),
    number,
    date: date_to_str(today),
    notes: String::new(),
    direction: DocumentDirection::Outgoing.as_str().to_string(),  // temp
})
```

**Waybill branch** (~line 650):
```rust
Ok(DocumentDraftFormDto {
    id: format!("wbl:{}", waybill.id),
    kind: "waybill".to_string(),
    counterparty_id: counterparty_id.to_string(),
    counterparty_name,
    title: kind_title("waybill", true).to_string(),
    number,
    date: date_to_str(today),
    notes: String::new(),
    direction: DocumentDirection::Outgoing.as_str().to_string(),  // temp
})
```

- [ ] **Step 7: Update `document_chain_create_draft` — 3 `DocumentDraftFormDto` sites (temporarily hardcode Outgoing)**

In `api.rs`, find `document_chain_create_draft` (line ~1181). Add `direction` to each `DocumentDraftFormDto`. Use `DocumentDirection::Outgoing.as_str()` for now (will be wired in Task 5):

**Act branch** (~line 1237):
```rust
DocumentDraftFormDto {
    id: format!("act:{}", act.id),
    kind: "act".to_string(),
    counterparty_id: source.counterparty_id.to_string(),
    counterparty_name: source.counterparty_name.clone(),
    title: kind_title("act", true).to_string(),
    number,
    date: date_to_str(source.date),
    notes: visible_notes.clone(),
    direction: DocumentDirection::Outgoing.as_str().to_string(),  // temp
}
```

**Invoice branch** (~line 1268):
```rust
DocumentDraftFormDto {
    id: format!("inv:{}", invoice.id),
    kind: "invoice".to_string(),
    counterparty_id: source.counterparty_id.to_string(),
    counterparty_name: source.counterparty_name.clone(),
    title: kind_title("invoice", true).to_string(),
    number,
    date: date_to_str(source.date),
    notes: visible_notes.clone(),
    direction: DocumentDirection::Outgoing.as_str().to_string(),  // temp
}
```

**Waybill branch** (~line 1298):
```rust
DocumentDraftFormDto {
    id: format!("wbl:{}", waybill.id),
    kind: "waybill".to_string(),
    counterparty_id: source.counterparty_id.to_string(),
    counterparty_name: source.counterparty_name.clone(),
    title: kind_title("waybill", true).to_string(),
    number,
    date: date_to_str(source.date),
    notes: visible_notes.clone(),
    direction: DocumentDirection::Outgoing.as_str().to_string(),  // temp
}
```

- [ ] **Step 8: Update `documents_list` — 3 `DocumentItemDto` sites**

In `api.rs`, find `documents_list` (line ~734). Add `direction` to each `DocumentItemDto`:

**Acts loop** (~line 752):
```rust
DocumentItemDto {
    id: format!("act:{}", row.id),
    kind: DocumentKindDto::Act,
    number: row.number,
    date: date_to_str(row.date),
    counterparty: row.counterparty_name,
    amount_str: format_money_ua(row.total_amount),
    status: document_status_from_act(&row.status),
    status_label: row.status.label().to_string(),
    linked_id: String::new(),
    direction: row.direction.as_str().to_string(),  // new
},
```

**Invoices loop** (~line 769):
```rust
DocumentItemDto {
    id: format!("inv:{}", row.id),
    kind: DocumentKindDto::Invoice,
    number: row.number,
    date: date_to_str(row.date),
    counterparty: row.counterparty_name,
    amount_str: format_money_ua(row.total_amount),
    status: document_status_from_invoice(&row.status),
    status_label: row.status.label().to_string(),
    linked_id: String::new(),
    direction: row.direction.as_str().to_string(),  // new
},
```

**Waybills loop** (~line 786):
```rust
DocumentItemDto {
    id: format!("wbl:{}", row.id),
    kind: DocumentKindDto::Waybill,
    number: row.number,
    date: date_to_str(row.date),
    counterparty: row.counterparty_name,
    amount_str: format_money_ua(row.total_amount),
    status: document_status_from_waybill(&row.status),
    status_label: row.status.label().to_string(),
    linked_id: String::new(),
    direction: row.direction.as_str().to_string(),  // new
},
```

- [ ] **Step 9: Update `counterparties.rs` — 2 `DocumentItemDto` helper functions**

In `src/tauri_api/counterparties.rs`, add `direction` to both helpers:

`act_to_document_item` (~line 277):
```rust
fn act_to_document_item(row: &crate::models::act::ActListRow) -> DocumentItemDto {
    DocumentItemDto {
        id: format!("act:{}", row.id),
        kind: DocumentKindDto::Act,
        number: row.number.clone(),
        date: format_date(row.date),
        counterparty: row.counterparty_name.clone(),
        amount_str: format_money_ua(row.total_amount),
        status: DocumentStatusDto::from_act_status(&row.status),
        status_label: row.status.label().to_string(),
        linked_id: String::new(),
        direction: row.direction.as_str().to_string(),  // new
    }
}
```

`invoice_to_document_item` (~line 291):
```rust
fn invoice_to_document_item(row: &crate::models::invoice::InvoiceListRow) -> DocumentItemDto {
    DocumentItemDto {
        id: format!("inv:{}", row.id),
        kind: DocumentKindDto::Invoice,
        number: row.number.clone(),
        date: format_date(row.date),
        counterparty: row.counterparty_name.clone(),
        amount_str: format_money_ua(row.total_amount),
        status: DocumentStatusDto::from_invoice_status(&row.status),
        status_label: row.status.label().to_string(),
        linked_id: String::new(),
        direction: row.direction.as_str().to_string(),  // new
    }
}
```

- [ ] **Step 10: Update `dashboard.rs` — `RowDto`, SQL, mapper**

In `src/tauri_api/dashboard.rs`, find `dashboard_recent_documents` (line ~262):

**`RowDto` struct** (~line 267):
```rust
struct RowDto {
    id: String,
    kind: String,
    number: String,
    date: NaiveDate,
    counterparty: String,
    amount: Decimal,
    status: String,
    direction: String,  // new
}
```

**`FromRow` impl** (~line 277):
```rust
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for RowDto {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            kind: row.try_get("kind")?,
            number: row.try_get("number")?,
            date: row.try_get("date")?,
            counterparty: row.try_get("counterparty")?,
            amount: row.try_get("amount")?,
            status: row.try_get("status")?,
            direction: row.try_get("direction")?,  // new
        })
    }
}
```

**SQL** (~line 291) — add `a.direction::text AS direction` etc. to each branch:
```sql
SELECT * FROM (
    SELECT
        'act:' || a.id::text AS id,
        'act'::text AS kind,
        a.number,
        a.date,
        cp.name AS counterparty,
        a.total_amount AS amount,
        a.status::text AS status,
        a.direction::text AS direction,
        a.created_at
    FROM acts a
    JOIN counterparties cp ON cp.id = a.counterparty_id
    WHERE a.company_id = $1
    UNION ALL
    SELECT
        'inv:' || i.id::text AS id,
        'invoice'::text AS kind,
        i.number,
        i.date,
        cp.name AS counterparty,
        i.total_amount AS amount,
        i.status::text AS status,
        i.direction::text AS direction,
        i.created_at
    FROM invoices i
    JOIN counterparties cp ON cp.id = i.counterparty_id
    WHERE i.company_id = $1
    UNION ALL
    SELECT
        'wbl:' || w.id::text AS id,
        'waybill'::text AS kind,
        w.number,
        w.date,
        cp.name AS counterparty,
        w.total_amount AS amount,
        w.status::text AS status,
        w.direction::text AS direction,
        w.created_at
    FROM waybills w
    JOIN counterparties cp ON cp.id = w.counterparty_id
    WHERE w.company_id = $1
) docs
ORDER BY created_at DESC
LIMIT $2
```

**`DocumentItemDto` mapper** (~line 351):
```rust
Ok(DocumentItemDto {
    id: row.id,
    kind,
    number: row.number,
    date: format_date_ua(row.date),
    counterparty: row.counterparty,
    amount_str: format_money_ua(row.amount),
    status,
    status_label,
    linked_id: String::new(),
    direction: row.direction,  // new
})
```

- [ ] **Step 11: Update `shell.rs`**

In `src/tauri_api/shell.rs`, find `CreateDocumentDraftRequest` construction (~line 292):

```rust
CreateDocumentDraftRequest {
    counterparty_id: counterparty_id.to_string(),
    kind: kind.as_str().to_string(),
    direction: "outgoing".to_string(),  // new — shell always creates outgoing
},
```

- [ ] **Step 12: Fix `tests/tauri_vertical_slice.rs` construction sites**

Search for `CreateDocumentDraftRequest {` in the test file. There are multiple — add `direction: "outgoing".to_string()` to each one.

Example at line ~404:
```rust
acta::tauri_api::documents::CreateDocumentDraftRequest {
    counterparty_id: counterparty.id.to_string(),
    kind: "invoice".to_string(),
    direction: "outgoing".to_string(),  // new
},
```

Do the same for every other `CreateDocumentDraftRequest { ... }` occurrence in the test file (search with grep).

- [ ] **Step 13: Verify compile**

```powershell
cargo build --tests
```

Expected: `Finished` with no errors.

- [ ] **Step 14: Commit**

```powershell
git add src/tauri_api/documents/dto.rs src/tauri_api/documents/api.rs
git add src/tauri_api/dashboard.rs src/tauri_api/counterparties.rs src/tauri_api/shell.rs
git add tests/tauri_vertical_slice.rs
git commit -m "feat(dto): add direction field to all document DTOs and construction sites"
```

---

## Task 4: Wire direction through `create_draft_form`

**Files:**
- Modify: `src/tauri_api/documents/api.rs`

- [ ] **Step 1: Add `direction` param to `create_draft_form` and replace hardcoded values**

In `api.rs`, find `async fn create_draft_form` (line ~558). Add `direction: DocumentDirection` as a parameter and use it:

```rust
async fn create_draft_form(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    counterparty_name: String,
    kind: &str,
    direction: DocumentDirection,  // new param
) -> Result<DocumentDraftFormDto> {
```

Replace `DocumentDirection::Outgoing` with `direction` in each `NewAct`/`NewInvoice`/`NewWaybill` construction in this function (3 sites, lines ~578/610/641).

Also replace the temporary `DocumentDirection::Outgoing.as_str()` in each `DocumentDraftFormDto` with `direction.as_str()` (3 sites from Task 3 Step 6).

- [ ] **Step 2: Parse direction in `document_create_draft` and pass to `create_draft_form`**

In `api.rs`, find `document_create_draft` (line ~968). Parse direction from the request and pass it:

```rust
pub async fn document_create_draft(
    ctx: &AppCtx,
    request: CreateDocumentDraftRequest,
) -> Result<DocumentEditorDto> {
    let counterparty_id = Uuid::parse_str(&request.counterparty_id).with_context(|| {
        format!("Некоректний ідентифікатор контрагента: {}", request.counterparty_id)
    })?;
    let direction = DocumentDirection::try_from(request.direction.clone())
        .map_err(|e| anyhow!("Невідома направленість документа: {e}"))?;
    let counterparty_name =
        load_counterparty_name(ctx.pool(), ctx.company_id(), counterparty_id).await?;
    let form = create_draft_form(
        ctx.pool(),
        ctx.company_id(),
        counterparty_id,
        counterparty_name,
        &request.kind,
        direction,  // new
    )
    .await?;

    Ok(DocumentEditorDto {
        form,
        items: Vec::new(),
        pdf: None,
        show_type_picker: false,
        show_editor: true,
    })
}
```

- [ ] **Step 3: Verify compile**

```powershell
cargo build --tests
```

Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```powershell
git add src/tauri_api/documents/api.rs
git commit -m "feat(api): wire direction param through create_draft_form"
```

---

## Task 5: Wire direction in `document_chain_create_draft`

**Files:**
- Modify: `src/tauri_api/documents/api.rs`

- [ ] **Step 1: Replace hardcoded `DocumentDirection::Outgoing` with `source.direction`**

In `api.rs`, find `document_chain_create_draft` (line ~1181). Replace every `direction: DocumentDirection::Outgoing` in `NewAct`, `NewInvoice`, `NewWaybill` with `direction: source.direction`.

Act branch (~line 1221):
```rust
&NewAct {
    number: number.clone(),
    counterparty_id: source.counterparty_id,
    contract_id: None,
    category_id: None,
    direction: source.direction,  // was DocumentDirection::Outgoing
    ...
},
```

Invoice and Waybill branches: same pattern.

- [ ] **Step 2: Update `DocumentDraftFormDto` chain sites to use `source.direction`**

In the same function, replace the temporary `DocumentDirection::Outgoing.as_str()` (from Task 3 Step 7) with `source.direction.as_str()` in all 3 `DocumentDraftFormDto` constructions:

```rust
DocumentDraftFormDto {
    ...
    direction: source.direction.as_str().to_string(),  // was Outgoing.as_str()
}
```

- [ ] **Step 3: Verify compile**

```powershell
cargo build --tests
```

Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```powershell
git add src/tauri_api/documents/api.rs
git commit -m "feat(api): chain draft inherits direction from source document"
```

---

## Task 6: Wire direction in `document_save` (form → DB)

Currently `document_save` reads direction from the existing DB record. This task makes it use the form's direction instead — enabling the editor radio toggle to actually persist.

**Files:**
- Modify: `src/tauri_api/documents/api.rs`

- [ ] **Step 1: Parse form direction and pass to Update structs**

In `api.rs`, find `document_save` (line ~998). At the top of the function, after parsing `date`, add direction parsing. Since each doc type branch has its own logic, add it per-branch:

**Act branch** (~line 1010): Add direction parse before the `update_with_items` call:
```rust
DocumentRef::Act(id) => {
    let (act, _) = db::acts::get_by_id(ctx.pool(), id)
        .await?
        .ok_or_else(|| anyhow!("Акт не знайдено"))?;
    let parent_ref = split_visible_notes_and_chain_parent(act.notes.as_deref()).1;
    let direction = DocumentDirection::try_from(request.form.direction.clone())
        .map_err(|e| anyhow!("Невідома направленість: {e}"))?;
    db::acts::update_with_items(
        ctx.pool(),
        id,
        UpdateAct {
            number: request.form.number.clone(),
            counterparty_id: act.counterparty_id,
            contract_id: act.contract_id,
            category_id: act.category_id,
            direction,  // was act.direction — now from form
            date,
            expected_payment_date: act.expected_payment_date,
            notes: compose_notes_with_chain_parent(
                &request.form.notes,
                parent_ref.as_deref(),
            ),
        },
        draft_items_to_new_act(request.items)?,
    )
    .await?;
    ...
}
```

**Invoice branch**: same pattern, `invoice.direction` → parsed `direction` from `request.form.direction`.

**Waybill branch**: same pattern.

- [ ] **Step 2: Verify compile**

```powershell
cargo build --tests
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```powershell
git add src/tauri_api/documents/api.rs
git commit -m "feat(api): document_save persists direction from form"
```

---

## Task 7: Wire direction + kind filter in `documents_list`

**Files:**
- Modify: `src/tauri_api/documents/api.rs`

- [ ] **Step 1: Update `documents_list` to pass direction + kind filters**

In `api.rs`, find `documents_list` (line ~734). Replace the current `tokio::join!` block with filtered queries:

```rust
pub async fn documents_list(
    ctx: &AppCtx,
    request: DocumentsListRequest,
) -> Result<DocumentsListDto> {
    let company_id = ctx.company_id();
    let search = request.query.as_deref();
    let direction_filter = request.direction;

    let include_acts     = request.kind.as_deref().map_or(true, |k| k == "act");
    let include_invoices = request.kind.as_deref().map_or(true, |k| k == "invoice");
    let include_waybills = request.kind.as_deref().map_or(true, |k| k == "waybill");

    // All three list_filtered share the same signature:
    // (pool, company_id, status_filter, direction, search_query, counterparty_id, date_from, date_to)
    let (acts, invoices, waybills) = tokio::join!(
        async {
            if include_acts {
                db::acts::list_filtered(ctx.pool(), company_id, None, direction_filter, search, None, None, None).await
            } else {
                Ok(vec![])
            }
        },
        async {
            if include_invoices {
                db::invoices::list_filtered(ctx.pool(), company_id, None, direction_filter, search, None, None, None).await
            } else {
                Ok(vec![])
            }
        },
        async {
            if include_waybills {
                db::waybills::list_filtered(ctx.pool(), company_id, None, direction_filter, search, None, None, None).await
            } else {
                Ok(vec![])
            }
        },
    );
    // rest of function (DocumentItemDto construction, combined sort, return) unchanged

- [ ] **Step 2: Verify compile**

```powershell
cargo build --tests
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```powershell
git add src/tauri_api/documents/api.rs
git commit -m "feat(api): documents_list filters by direction and kind"
```

---

## Task 8: Integration tests for direction behavior

**Files:**
- Modify: `tests/tauri_vertical_slice.rs`

- [ ] **Step 1: Add `documents_direction_filter` test**

Append a new `#[tokio::test]` at the end of `tests/tauri_vertical_slice.rs`:

```rust
#[tokio::test]
async fn documents_direction_filter() -> Result<()> {
    use acta::models::DocumentDirection;
    use acta::tauri_api::documents::{
        CreateDocumentDraftRequest, DocumentsListRequest, documents_list, document_create_draft,
        document_delete,
    };

    let _lock = tauri_vertical_slice_lock().await;
    let ctx = make_test_ctx().await?;

    let counterparty = acta::db::counterparties::list(ctx.pool(), ctx.company_id())
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("потрібен хоча б один контрагент"))?;

    // Create one outgoing invoice and one incoming act
    let outgoing = document_create_draft(
        &ctx,
        CreateDocumentDraftRequest {
            counterparty_id: counterparty.id.to_string(),
            kind: "invoice".to_string(),
            direction: "outgoing".to_string(),
        },
    )
    .await?;

    let incoming = document_create_draft(
        &ctx,
        CreateDocumentDraftRequest {
            counterparty_id: counterparty.id.to_string(),
            kind: "act".to_string(),
            direction: "incoming".to_string(),
        },
    )
    .await?;

    let cleanup: Vec<String> = vec![outgoing.form.id.clone(), incoming.form.id.clone()];

    let result: Result<()> = async {
        // direction field is populated
        assert_eq!(outgoing.form.direction, "outgoing");
        assert_eq!(incoming.form.direction, "incoming");

        // filter by outgoing — only invoice returned
        let outgoing_list = documents_list(
            &ctx,
            DocumentsListRequest { direction: Some(DocumentDirection::Outgoing), ..Default::default() },
        )
        .await?;
        assert!(
            outgoing_list.items.iter().any(|i| i.id == outgoing.form.id),
            "outgoing filter must include outgoing invoice"
        );
        assert!(
            !outgoing_list.items.iter().any(|i| i.id == incoming.form.id),
            "outgoing filter must exclude incoming act"
        );
        assert!(
            outgoing_list.items.iter().all(|i| i.direction == "outgoing"),
            "all items in outgoing filter must have direction=outgoing"
        );

        // filter by incoming — only act returned
        let incoming_list = documents_list(
            &ctx,
            DocumentsListRequest { direction: Some(DocumentDirection::Incoming), ..Default::default() },
        )
        .await?;
        assert!(
            incoming_list.items.iter().any(|i| i.id == incoming.form.id),
            "incoming filter must include incoming act"
        );
        assert!(
            !incoming_list.items.iter().any(|i| i.id == outgoing.form.id),
            "incoming filter must exclude outgoing invoice"
        );

        // filter by kind=act — only act returned
        let act_list = documents_list(
            &ctx,
            DocumentsListRequest { kind: Some("act".to_string()), ..Default::default() },
        )
        .await?;
        assert!(
            act_list.items.iter().all(|i| i.direction == "incoming" || i.direction == "outgoing"),
            "all items must have direction set"
        );
        assert!(
            act_list.items.iter().all(|i| matches!(i.kind, acta::tauri_api::documents::DocumentKindDto::Act)),
            "kind=act filter must only return acts"
        );

        Ok(())
    }
    .await;

    // cleanup
    for id in cleanup {
        let _ = document_delete(&ctx, id).await;
    }

    result
}
```

- [ ] **Step 2: Run the test (requires DATABASE_URL)**

```powershell
$env:DATABASE_URL="postgres://postgres:password@localhost:5432/acta"
cargo test documents_direction_filter -- --nocapture
```

Expected: PASS. If it fails with a connection error, set the correct `DATABASE_URL` for your local DB.

- [ ] **Step 3: Commit**

```powershell
git add tests/tauri_vertical_slice.rs
git commit -m "test: add direction filter integration test"
```

---

## Task 9: TypeScript types

**Files:**
- Modify: `frontend/src/lib/types.ts`

- [ ] **Step 1: Add `DocumentDirection` type alias**

In `frontend/src/lib/types.ts`, add after the existing type aliases at the top:

```ts
export type DocumentDirection = "outgoing" | "incoming";
```

- [ ] **Step 2: Add `direction` to `DocumentItemDto`**

```ts
export interface DocumentItemDto {
  id: string;
  kind: DocumentKind;
  number: string;
  date: string;
  counterparty: string;
  amountStr: string;
  status: DocumentStatus;
  statusLabel: string;
  linkedId: string;
  direction: DocumentDirection;  // new
}
```

- [ ] **Step 3: Add `direction` to `DocumentDraftFormDto`**

```ts
export interface DocumentDraftFormDto {
  id: string;
  kind: string;
  counterpartyId: string;
  counterpartyName: string;
  title: string;
  number: string;
  date: string;
  notes: string;
  direction: DocumentDirection;  // new
}
```

- [ ] **Step 4: Verify TypeScript compiles**

```powershell
cd frontend; npx tsc --noEmit; cd ..
```

Expected: no errors (or only pre-existing errors unrelated to this change).

- [ ] **Step 5: Commit**

```powershell
git add frontend/src/lib/types.ts
git commit -m "feat(types): add DocumentDirection and direction fields to DTOs"
```

---

## Task 10: TypeScript `api.ts`

**Files:**
- Modify: `frontend/src/lib/api.ts`

- [ ] **Step 1: Update `documentsList` signature**

In `frontend/src/lib/api.ts`, replace the existing `documentsList` function:

```ts
export function documentsList(
  query = "",
  direction?: "outgoing" | "incoming",
  kind?: string
): Promise<DocumentsListDto> {
  return appInvoke("documents_list", {
    request: {
      query: query || null,
      direction: direction ?? null,
      kind: kind ?? null
    }
  });
}
```

- [ ] **Step 2: Update `documentCreateDraft` signature**

```ts
export function documentCreateDraft(
  counterpartyId: string,
  kind: string,
  direction: "outgoing" | "incoming" = "outgoing"
): Promise<DocumentEditorDto> {
  return appInvoke("document_create_draft", {
    request: {
      counterpartyId,
      kind,
      direction
    }
  });
}
```

- [ ] **Step 3: Verify TypeScript compiles**

```powershell
cd frontend; npx tsc --noEmit; cd ..
```

Expected: no errors.

- [ ] **Step 4: Commit**

```powershell
git add frontend/src/lib/api.ts
git commit -m "feat(api): update documentsList and documentCreateDraft with direction/kind"
```

---

## Task 11: Documents store

**Files:**
- Modify: `frontend/src/lib/stores/documents.ts`

- [ ] **Step 1: Add `activeTab` and `kindFilter` to state interface**

At the top of the file, update the `DocumentsState` interface:

```ts
import type { DocumentChainDto, DocumentEditorDto, DocumentsListDto, DocumentKind } from "../types";

interface DocumentsState {
  list: DocumentsListDto | null;
  editor: DocumentEditorDto | null;
  editorSnapshot: EditorPayload | null;
  chain: DocumentChainDto | null;
  draftContext: { counterpartyId: string; counterpartyName: string } | null;
  selectedIds: string[];
  initialLoading: boolean;
  loading: boolean;
  error: string | null;
  message: string | null;
  query: string;
  activeTab: "all" | "outgoing" | "incoming";   // new
  kindFilter: DocumentKind | null;              // new
}
```

Update `initialState`:
```ts
const initialState: DocumentsState = {
  list: null,
  editor: null,
  editorSnapshot: null,
  chain: null,
  draftContext: null,
  selectedIds: [],
  initialLoading: true,
  loading: false,
  error: null,
  message: null,
  query: "",
  activeTab: "all",     // new
  kindFilter: null       // new
};
```

- [ ] **Step 2: Add `tabToDirection` helper and `reloadList` private helper**

Add these as local functions inside `createDocumentsStore` (before the `return` statement):

```ts
function tabToDirection(tab: "all" | "outgoing" | "incoming"): "outgoing" | "incoming" | undefined {
  if (tab === "outgoing") return "outgoing";
  if (tab === "incoming") return "incoming";
  return undefined;
}

async function reloadList(state: DocumentsState): Promise<DocumentsListDto> {
  return documentsList(
    state.query,
    tabToDirection(state.activeTab),
    state.kindFilter ?? undefined
  );
}
```

- [ ] **Step 3: Replace all 9 direct `documentsList(...)` calls with `reloadList(state)` or `reloadList(get({subscribe}))`**

Find every occurrence of `documentsList(` in the store and replace. The pattern is: wherever `documentsList(snapshot.query)` or `documentsList(get(...).query)` appears, replace with `reloadList(snapshot)` or `reloadList(get({ subscribe }))`.

Locations (with approximate current line numbers):
1. `load()` — replace `await documentsList(query)` → use state directly since `query` is being set before the call. Pass updated state:
   ```ts
   async load(query = "") {
     update((state) => ({ ...state, loading: true, error: null, message: state.message, query }));
     try {
       const snap = get({ subscribe });
       const list = await reloadList({ ...snap, query });
       ...
   ```
2. `reloadCurrent()` — replace `documentsList(snapshot.query)` → `reloadList(snapshot)`
3. `create()` — replace `documentsList(get({ subscribe }).query)` → `reloadList(get({ subscribe }))`
4. `save()` — replace `documentsList(snapshot.query)` → `reloadList(snapshot)`
5. `advanceStatus()` — replace `documentsList(snapshot.query)` → `reloadList(snapshot)`
6. `deleteCurrent()` — replace `documentsList(snapshot.query)` → `reloadList(snapshot)`
7. `createChainDraft()` — replace `documentsList(snapshot.query)` → `reloadList(snapshot)`
8. `bulkDelete()` — replace `documentsList(snapshot.query)` → `reloadList(snapshot)`
9. `bulkAdvanceStatus()` — replace `documentsList(snapshot.query)` → `reloadList(snapshot)`

- [ ] **Step 4: Add `setTab` and `setKindFilter` methods**

Add to the returned store object:

```ts
setTab(tab: "all" | "outgoing" | "incoming") {
  const snap = get({ subscribe });
  update((state) => ({ ...state, activeTab: tab }));
  const newSnap = { ...snap, activeTab: tab };
  update((state) => ({ ...state, loading: true, error: null }));
  reloadList(newSnap).then((list) => {
    update((state) => ({ ...state, list, loading: false }));
  }).catch((error) => {
    update((state) => ({ ...state, loading: false, error: String(error) }));
  });
},
setKindFilter(kind: DocumentKind | null) {
  const snap = get({ subscribe });
  update((state) => ({ ...state, kindFilter: kind }));
  const newSnap = { ...snap, kindFilter: kind };
  update((state) => ({ ...state, loading: true, error: null }));
  reloadList(newSnap).then((list) => {
    update((state) => ({ ...state, list, loading: false }));
  }).catch((error) => {
    update((state) => ({ ...state, loading: false, error: String(error) }));
  });
},
```

- [ ] **Step 5: Update `create()` to derive direction from `activeTab`**

In `create(counterpartyId, kind)`, derive direction before the API call:

```ts
async create(counterpartyId: string, kind: string) {
  update((state) => ({ ...state, loading: true, error: null, message: null }));
  const snap = get({ subscribe });
  const direction: "outgoing" | "incoming" =
    snap.activeTab === "incoming" ? "incoming" : "outgoing";

  try {
    const [editor, list] = await Promise.all([
      documentCreateDraft(counterpartyId, kind, direction),
      reloadList(snap)
    ]);
    ...
```

- [ ] **Step 6: Verify TypeScript compiles**

```powershell
cd frontend; npx tsc --noEmit; cd ..
```

Expected: no errors.

- [ ] **Step 7: Commit**

```powershell
git add frontend/src/lib/stores/documents.ts
git commit -m "feat(store): add activeTab/kindFilter state and reloadList helper"
```

---

## Task 12: `DocumentsScreen.svelte` UI

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`

- [ ] **Step 1: Import `DocumentKind` type**

Ensure the `<script>` block imports `DocumentKind`:

```ts
import type { DocumentDraftItemDto, DocumentKind } from "../types";
```

(Already imported — verify it's there. If not, add it.)

- [ ] **Step 2: Add direction label map**

In the script section, after `documentKindIcons`:

```ts
const directionLabels: Record<string, string> = {
  outgoing: "↑ Вихідний",
  incoming: "↓ Вхідний"
};
```

- [ ] **Step 3: Update `getCreateButtonLabel` to include direction hint**

Replace the existing `getCreateButtonLabel` function:

```ts
function getCreateButtonLabel(kind: DocumentKind): string {
  const tab = $documents.activeTab;
  const dirSuffix = tab === "incoming" ? " (вхідний)" : tab === "outgoing" ? " (вихідний)" : "";
  if (kind === "invoice") return `Створити рахунок${dirSuffix}`;
  if (kind === "waybill") return `Створити накладну${dirSuffix}`;
  return `Створити акт${dirSuffix}`;
}
```

- [ ] **Step 4: Add tab bar and chip filters in template**

In the template, after the `<div class="panel-header">` closing tag and before `<div class="documents-create-bar">`, add:

```html
<div class="documents-nav-tabs" role="tablist" aria-label="Напрямок документів">
  {#each [
    { value: "all",      label: "Всі" },
    { value: "outgoing", label: "Вихідні" },
    { value: "incoming", label: "Вхідні" }
  ] as tab}
    <button
      role="tab"
      type="button"
      class="nav-tab"
      class:nav-tab-active={$documents.activeTab === tab.value}
      on:click={() => documents.setTab(tab.value as "all" | "outgoing" | "incoming")}
      disabled={$documents.loading}
    >
      {tab.label}
    </button>
  {/each}
</div>

<div class="documents-kind-chips" role="group" aria-label="Тип документа">
  {#each [
    { value: null,       label: "Всі" },
    { value: "act",      label: "Акти" },
    { value: "invoice",  label: "Рахунки" },
    { value: "waybill",  label: "Накладні" }
  ] as chip}
    <button
      type="button"
      class="kind-chip"
      class:kind-chip-active={$documents.kindFilter === chip.value}
      on:click={() => documents.setKindFilter(chip.value as DocumentKind | null)}
      disabled={$documents.loading}
    >
      {chip.label}
    </button>
  {/each}
</div>
```

- [ ] **Step 5: Add direction badge to document list row**

In the template, find the `.doc-row-meta` div (~line 520). After `<span class="doc-status-chip">`, add:

```html
<span class="doc-direction-badge" data-direction={item.direction}>
  {directionLabels[item.direction] ?? item.direction}
</span>
```

- [ ] **Step 6: Add direction radio toggle in editor**

In the template, find the `<div class="editor-grid">` block (~line 667). After the notes `<label>`, add:

```html
<fieldset class="editor-direction-fieldset editor-grid-span">
  <legend>Напрямок</legend>
  <label class="editor-direction-option">
    <input
      type="radio"
      name="direction"
      value="outgoing"
      checked={$documents.editor.form.direction === "outgoing"}
      on:change={() => documents.updateFormField("direction", "outgoing")}
      disabled={$documents.loading}
    />
    ↑ Вихідний
  </label>
  <label class="editor-direction-option">
    <input
      type="radio"
      name="direction"
      value="incoming"
      checked={$documents.editor.form.direction === "incoming"}
      on:change={() => documents.updateFormField("direction", "incoming")}
      disabled={$documents.loading}
    />
    ↓ Вхідний
  </label>
</fieldset>
```

- [ ] **Step 7: Add minimal CSS**

At the bottom of the Svelte file, in the `<style>` block, add:

```css
.documents-nav-tabs {
  display: flex;
  gap: 2px;
  border-bottom: 1px solid var(--color-border);
  padding: 0 16px;
}

.nav-tab {
  padding: 8px 16px;
  border: none;
  background: none;
  cursor: pointer;
  color: var(--color-text-sub);
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
}

.nav-tab-active {
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
  font-weight: 500;
}

.documents-kind-chips {
  display: flex;
  gap: 6px;
  padding: 8px 16px;
}

.kind-chip {
  padding: 4px 12px;
  border-radius: 12px;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  cursor: pointer;
  font-size: 13px;
  color: var(--color-text-sub);
}

.kind-chip-active {
  background: var(--color-primary);
  border-color: var(--color-primary);
  color: #fff;
}

.doc-direction-badge {
  font-size: 11px;
  color: var(--color-text-muted);
}

.doc-direction-badge[data-direction="outgoing"] {
  color: var(--color-success);
}

.doc-direction-badge[data-direction="incoming"] {
  color: var(--color-warning);
}

.editor-direction-fieldset {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 8px 12px;
}

.editor-direction-fieldset legend {
  font-size: 12px;
  color: var(--color-text-sub);
  padding: 0 4px;
}

.editor-direction-option {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-right: 16px;
  cursor: pointer;
}
```

- [ ] **Step 8: Verify TypeScript compiles**

```powershell
cd frontend; npx tsc --noEmit; cd ..
```

Expected: no errors.

- [ ] **Step 9: Commit**

```powershell
git add frontend/src/lib/screens/DocumentsScreen.svelte
git commit -m "feat(ui): add tab bar, kind chips, direction badge, and editor radio toggle"
```

---

## Final: Full build verification

- [ ] **Step 1: Full Rust build with tests**

```powershell
cargo build --tests
```

Expected: `Finished` with no errors.

- [ ] **Step 2: Full TypeScript check**

```powershell
cd frontend; npx tsc --noEmit; cd ..
```

Expected: no errors.

- [ ] **Step 3: Run Rust tests (requires DATABASE_URL)**

```powershell
$env:DATABASE_URL="postgres://postgres:password@localhost:5432/acta"
cargo test -- --nocapture 2>&1 | Select-String -Pattern "FAILED|passed|failed"
```

Expected: all tests passed, 0 failed.
