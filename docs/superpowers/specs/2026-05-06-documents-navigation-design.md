# Documents Screen Navigation Redesign

**Date:** 2026-05-06  
**Status:** Approved

## Problem

The Documents screen shows all document types (acts, invoices, waybills) in a single flat list with no separation by type or direction (incoming/outgoing). This is inconvenient because:
- Users cannot quickly see "what I issued to clients" vs "what I received from suppliers"
- Mixing all document types makes scanning the list slow

## Decisions

### Navigation structure

Three top-level tabs: **Всі | Вихідні | Вхідні**

Direction (Outgoing/Incoming) is the primary split because it maps directly to the Revenue vs Expenses mental model in Ukrainian management accounting. The "Всі" tab is available for global search and cross-direction queries.

### Secondary filter

Chip filters below the tabs: **Всі | Акти | Рахунки | Накладні**

Document type is secondary — acts, invoices, and waybills for the same counterparty form a chain and should not be separated by top-level navigation.

**Chips are list-filter only.** They do NOT affect the create flow. The create bar retains its own kind selector (currently a `<select>` in `DocumentsScreen.svelte`). This avoids the ambiguity of "Всі chip active → what kind to create?".

### Direction on create

Direction is inferred automatically from the active tab:
- "Вихідні" tab → new document is Outgoing
- "Вхідні" tab → new document is Incoming
- "Всі" tab → defaults to Outgoing (most common case)

The "Створити" button label dynamically reflects the selected kind from the create-bar selector AND the inferred direction from the active tab, e.g. _"Створити рахунок (вхідний)"_.

### Direction in editor

A radio toggle (Вихідний / Вхідний) is shown in the document editor so the user can correct direction after creation. This must be fully wired through save (see Save flow below).

### Direction badge in list

Each row in the document list shows a small badge: **↑ Вихідний** or **↓ Вхідний**. This makes direction scannable without opening the document.

## Architecture

### Backend (already exists)

`DocumentDirection` enum (`Outgoing` / `Incoming`) is already implemented in the Rust domain model (`src/models/shared.rs`) and stored in the DB for acts, invoices, and waybills. The `list_filtered` functions in `db/acts.rs`, `db/invoices.rs`, and `db/waybills.rs` already accept `direction: Option<DocumentDirection>` and apply it as a SQL filter.

### Changes required

#### 1. List filter request — replace `tab` with `direction` + `kind`

`DocumentsListRequest` currently has `tab: Option<String>` which the backend ignores entirely (`api.rs:734`). Replace it:

```rust
// src/tauri_api/documents/dto.rs
pub struct DocumentsListRequest {
    pub query: Option<String>,
    pub direction: Option<DocumentDirection>,  // replaces tab
    pub kind: Option<String>,                  // "act" | "invoice" | "waybill" | null
}
```

`api.ts` `documentsList` signature updates accordingly:

```ts
export function documentsList(
  query = "",
  direction?: "outgoing" | "incoming",
  kind?: string
): Promise<DocumentsListDto>
```

In `documents_list` (api.rs), pass `direction` and `kind` filter to each `list_filtered` call. When `kind` is set, skip the calls for the other two entity types entirely (e.g. `kind = "act"` → only call `db::acts::list_filtered`, return empty for invoices/waybills).

#### 2. `DocumentItemDto` — add `direction` field

```rust
// src/tauri_api/documents/dto.rs
pub struct DocumentItemDto {
    // ... existing fields ...
    pub direction: String,  // "outgoing" | "incoming"
}
```

`DocumentItemDto` is constructed in multiple places in `api.rs` — not only in `documents_list`, but also in dashboard and counterparties mappers. Every construction site must set `direction` from `row.direction.as_str()`.

#### 3. `DocumentDraftFormDto` — add `direction` field

```rust
// src/tauri_api/documents/dto.rs
pub struct DocumentDraftFormDto {
    // ... existing fields ...
    pub direction: String,  // "outgoing" | "incoming"
}
```

`DocumentDraftFormDto` is constructed in **seven** places in `api.rs`:
- `document_open` for acts (line 485), invoices (line 512), waybills (line 539)
- `create_draft_form` for acts (line 589), invoices (line 620), waybills (line 650)
- `document_chain_create_draft` for acts (line 1227), invoices (line 1258), waybills (line 1288)

Each must populate `direction` from the corresponding model field (`act.direction.as_str()`, etc.).

#### 4. Create flow — `CreateDocumentDraftRequest` + `direction`

```rust
pub struct CreateDocumentDraftRequest {
    pub counterparty_id: String,
    pub kind: String,
    pub direction: String,  // "outgoing" | "incoming"
}
```

Two callers must be updated:

**Documents store** (`documents.ts`): `create()` derives direction from `activeTab` (falls back to `"outgoing"` when tab is `"all"`) and passes it to the invoke.

**Shell / command palette** (`src/tauri_api/shell.rs`, line 292): No tab context is available here. Policy: always default to `"outgoing"`. The palette creates documents from search context where direction is unknown, so Outgoing is the safer default. Update `CreateDocumentDraftRequest { ..., direction: "outgoing".to_string() }`.

In `create_draft_form` (api.rs), the `direction` parameter replaces the hardcoded `DocumentDirection::Outgoing` in all three `New*` struct constructions.

#### 5. Save flow — direction must persist end-to-end

Currently `document_save` does not update direction. Full chain of changes required:

`SaveDocumentRequest` already embeds `DocumentDraftFormDto` as `form`, so `direction` travels with it automatically once step 3 is done.

**Domain update structs** (`src/models/`): add `direction: DocumentDirection` to `UpdateAct`, `UpdateInvoice`, `UpdateWaybill`.

**SQL UPDATE statements** (`src/db/acts.rs:682`, `src/db/invoices.rs`, `src/db/waybills.rs`): add `direction = $N` to each `SET` clause.

**`document_save` in api.rs (~line 951)**: parse `req.form.direction` → `DocumentDirection`, include it in the `UpdateAct` / `UpdateInvoice` / `UpdateWaybill` struct.

#### 6. Chain draft direction — inherit from source (not hardcoded)

Currently `document_chain_create_draft` hardcodes `DocumentDirection::Outgoing` for all three document types (api.rs lines 1216, 1248, 1279). This is wrong — a chain draft should inherit the direction of its source document.

Fix: load the source document's direction before constructing the chain and pass it through to `NewAct` / `NewInvoice` / `NewWaybill`. The source direction is already available from the DB query that fetches the source document earlier in `document_chain_create_draft`.

#### 7. TypeScript types (`frontend/src/lib/types.ts`)

```ts
export type DocumentDirection = "outgoing" | "incoming";
```

- Add `direction: DocumentDirection` to `DocumentItemDto`
- Add `direction: DocumentDirection` to `DocumentDraftFormDto`

#### 8. Documents store (`frontend/src/lib/stores/documents.ts`)

- Add state: `activeTab: "all" | "outgoing" | "incoming"` (default `"all"`)
- Add state: `kindFilter: DocumentKind | null` (default `null`)
- `setTab(tab)`: updates `activeTab`, reloads list
- `setKindFilter(kind | null)`: updates `kindFilter`, reloads list
- `load()`: passes `direction` and `kind` to `documentsList` invoke
- `create(counterpartyId, kind)`: derives direction from `activeTab`

#### 9. DocumentsScreen UI (`frontend/src/lib/screens/DocumentsScreen.svelte`)

- Tab bar: Всі / Вихідні / Вхідні → calls `documents.setTab()`
- Kind chips: Всі / Акти / Рахунки / Накладні → calls `documents.setKindFilter()` (list filter only)
- Create bar: retains existing kind `<select>` (`createKind`); button label becomes `"Створити {kind} ({direction})"`
- Document row: direction badge ↑/↓ alongside existing status chip
- Editor: radio toggle Вихідний/Вхідний → `documents.updateFormField("direction", value)`
