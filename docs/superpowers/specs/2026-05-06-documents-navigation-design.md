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

### Direction on create

Direction is inferred automatically from the active tab:
- "Вихідні" tab → new document is Outgoing
- "Вхідні" tab → new document is Incoming
- "Всі" tab → defaults to Outgoing (most common case)

The "Створити" button dynamically reflects both the selected kind chip and the inferred direction, e.g. _"Створити рахунок (вхідний)"_.

### Direction in editor

A toggle (Вихідний / Вхідний) is shown in the document editor so the user can correct direction if they created it on the wrong tab.

### Direction badge in list

Each row in the document list shows a small badge: **↑ Вихідний** or **↓ Вхідний**. This makes direction scannable without opening the document.

## Architecture

### Backend (already exists)

`DocumentDirection` enum (`Outgoing` / `Incoming`) is already implemented in the Rust domain model (`src/models/shared.rs`) and stored in the DB for acts, invoices, and waybills.

### Changes required

**Rust / Tauri API (`src/tauri_api/documents.rs`):**
- `DocumentsListRequest`: add optional `direction: Option<DocumentDirection>` filter
- `DocumentItemDto`: add `direction: String` field (serialized as `"outgoing"` / `"incoming"`)
- `CreateDocumentDraftRequest`: add `direction: DocumentDirection` field (replaces any hardcoded default)

**TypeScript types (`frontend/src/lib/types.ts`):**
- Add `DocumentDirection = "outgoing" | "incoming"` type
- Add `direction: DocumentDirection` to `DocumentItemDto`
- Add `direction: DocumentDirection` to `DocumentDraftFormDto` (for editor toggle)

**Documents store (`frontend/src/lib/stores/documents.ts`):**
- Add state: `activeTab: "all" | "outgoing" | "incoming"` (default `"all"`)
- Add state: `kindFilter: DocumentKind | null` (default `null` = all)
- `setTab(tab)`: updates `activeTab`, reloads list
- `setKindFilter(kind | null)`: updates `kindFilter`, reloads list
- `load()`: passes `direction` and `kind` filters to `documents_list` invoke
- `create(counterpartyId, kind)`: derives direction from `activeTab` (defaults to `"outgoing"` when `activeTab === "all"`)

**DocumentsScreen UI (`frontend/src/lib/screens/DocumentsScreen.svelte`):**
- Tab bar: Всі / Вихідні / Вхідні (calls `documents.setTab()`)
- Kind chips: Всі / Акти / Рахунки / Накладні (calls `documents.setKindFilter()`)
- Create button label: dynamically composed from `createKind` + inferred direction
- Document row: direction badge ↑/↓ alongside the existing status chip
- Editor: Вихідний/Вхідний radio toggle, calls `documents.updateFormField("direction", ...)`

## Out of scope

- Separate numbering sequences per direction (e.g. АКТ-В vs АКТ-П) — not requested
- Automatic direction detection from BAS import — already handled by existing import logic
- Chain document direction inheritance — chains always share the direction of their source document; no UI change needed
