# Epic 8.1 Design — Stringly-Typed → Enums (Full Rust + Slint)

**Date:** 2026-04-23  
**Task:** Epic 8.1 — Прибрати stringly-typed state там, де можливі enum-и  
**Acceptance Criteria:**
1. Статуси, типи, фільтри та інші структуровані поля не живуть як магічні рядки без потреби.
2. Зменшено кількість fragile string comparisons.

---

## Overview

Convert `String` fields that represent fixed enumerations (direction, category kind) from both **Rust** and **Slint** sides to native enum types. No DB schema changes needed — VARCHAR columns remain, conversion happens at the boundary via `sqlx::Type` traits.

**Approach:** Slint native `enum` + Rust `enum` + automatic mapping via enum variant matching.

---

## Tier 1 — High Priority (MUST-HAVE for Epic 8.1)

### 1. DocumentDirection Enum

**Rust side** — new file `src/models/shared.rs` (or inline in an existing module):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum DocumentDirection {
    #[sqlx(rename = "outgoing")]
    Outgoing,
    #[sqlx(rename = "incoming")]
    Incoming,
}

impl DocumentDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Outgoing => "outgoing",
            Self::Incoming => "incoming",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Outgoing => "Вихідний",
            Self::Incoming => "Вхідний",
        }
    }
}

impl std::fmt::Display for DocumentDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl TryFrom<String> for DocumentDirection {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "outgoing" => Ok(Self::Outgoing),
            "incoming" => Ok(Self::Incoming),
            _ => Err(format!("Unknown direction: {}", s)),
        }
    }
}
```

**Slint side** — add to `ui/types.slint`:

```slint
export enum DocumentDirection {
    outgoing,
    incoming,
}
```

**Structs affected:**
- `Act.direction: String` → `Act.direction: DocumentDirection`
- `ActListRow.direction: String` → `ActListRow.direction: DocumentDirection`
- `Invoice.direction: String` → `Invoice.direction: DocumentDirection`
- `InvoiceListRow.direction: String` → `InvoiceListRow.direction: DocumentDirection`
- `Waybill.direction: String` → `Waybill.direction: DocumentDirection`
- `WaybillListRow.direction: String` → `WaybillListRow.direction: DocumentDirection`

**DB layer** — no migration. Functions in `src/db/acts.rs`, `invoices.rs`, `waybills.rs`:
- `insert_act(pool, direction: DocumentDirection, ...)` — sqlx binds the enum as string "outgoing"/"incoming"
- `update_act(pool, ..., direction: DocumentDirection, ...)` — same
- Read queries already work: `row.direction` is automatically converted to `DocumentDirection` via `sqlx::Type`

**UI presenter** — `src/ui/acts.rs`, `invoices.rs`, `waybills.rs`:
- `apply_acts_to_ui(ui, acts)` — `act.direction` is already `DocumentDirection`, passes to Slint unchanged (enum match happens automatically)

---

### 2. CategoryKind Enum

**Rust side** — in `src/models/category.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum CategoryKind {
    #[sqlx(rename = "income")]
    Income,
    #[sqlx(rename = "expense")]
    Expense,
}

impl CategoryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Income => "Дохід",
            Self::Expense => "Видаток",
        }
    }
}

// + Display, TryFrom<String> (same pattern as DocumentDirection)
```

**Slint side** — add to `ui/types.slint`:

```slint
export enum CategoryKind {
    income,
    expense,
}
```

**Structs affected:**
- `Category.kind: String` → `Category.kind: CategoryKind`
- `CategorySelectItem.kind: String` → `CategorySelectItem.kind: CategoryKind`

**DB layer** — `src/db/categories.rs`:
- `create_category(pool, kind: CategoryKind, ...)` → auto-bind as "income"/"expense"
- `get_categories(pool)` → `row.kind` auto-converts to `CategoryKind`

**UI presenter** — `src/ui/categories.rs`:
- Same pattern as DocumentDirection

---

## Tier 2 — Medium Priority (NICE-TO-HAVE, can defer)

### 3. ChainStepStatus Enum

Used in payment chain visualization (`ChainStep.status` in `types.slint`). Currently stores "draft"|"issued"|"signed"|"paid"|"overdue"|"".

**Slint side only** (this is UI-only state):

```slint
export enum ChainStepStatus {
    draft,
    issued,
    signed,
    paid,
    overdue,
    none,  // empty/initial state
}
```

Update in `ui/types.slint`:
- `ChainStep.status: string` → `ChainStep.status: ChainStepStatus`

Update in `ui/components.slint` (payment chain comparisons):
```slint
// Before:
if step.status == "paid" || step.status == "overdue" { ... }

// After:
if step.status == ChainStepStatus.paid || step.status == ChainStepStatus.overdue { ... }
```

**No Rust side needed** — this state is computed in UI from act/invoice data, not persisted in DB.

---

### 4. InboxItemKind Enum

Currently stores "overdue"|"unsigned"|"act-needed"|"unmatched" in Slint.

**Slint side only:**

```slint
export enum InboxItemKind {
    overdue,
    unsigned,
    act-needed,
    unmatched,
}
```

Update `ui/types.slint`:
- `InboxItem.kind: string` → `InboxItem.kind: InboxItemKind`

---

## Architecture & Data Flow

```
DB (PostgreSQL)
├─ acts.direction VARCHAR ──┐
├─ invoices.direction VARCHAR ──┐
├─ waybills.direction VARCHAR ──┤
│                           │
└─ categories.kind VARCHAR ─┐
                             │
                             ▼
                    sqlx reads row
                    (VARCHAR → Rust enum via Type trait)
                             │
                             ▼
                    Rust struct
                    (Act { direction: DocumentDirection, ... })
                             │
                             ▼
                    Presenter mapping
                    (apply_acts_to_ui)
                             │
                             ▼
                    Slint struct
                    (Act { direction: DocumentDirection, ... })
                             │
                             ▼
                    Slint comparisons
                    (step.direction == DocumentDirection.outgoing)
```

---

## Implementation Checklist

**Tier 1:**

- [ ] Create `DocumentDirection` enum in Rust (src/models/)
- [ ] Add `DocumentDirection` enum to Slint (ui/types.slint)
- [ ] Update Act, Invoice, Waybill struct fields (Rust + Slint)
- [ ] Update DB layer functions (acts.rs, invoices.rs, waybills.rs)
- [ ] Update presenter functions (src/ui/acts.rs, invoices.rs, waybills.rs)
- [ ] Create `CategoryKind` enum in Rust (src/models/category.rs)
- [ ] Add `CategoryKind` enum to Slint (ui/types.slint)
- [ ] Update Category, CategorySelectItem fields (Rust + Slint)
- [ ] Update DB layer functions (categories.rs)
- [ ] Update presenter functions (src/ui/categories.rs)
- [ ] Run `cargo sqlx prepare` (update .sqlx cache)
- [ ] Test: load acts/invoices/waybills with mixed directions
- [ ] Test: load categories with mixed kinds
- [ ] No type mismatches on Rust↔Slint boundary

**Tier 2:**

- [ ] Add ChainStepStatus enum to Slint
- [ ] Update ChainStep field + all comparisons in components.slint
- [ ] Add InboxItemKind enum to Slint
- [ ] Update InboxItem field + comparisons in relevant .slint files

---

## Notes

1. **No DB Migration:** VARCHAR columns stay as-is. The `#[sqlx(type_name = "VARCHAR")]` attribute tells sqlx to treat the Rust enum as VARCHAR in queries. The `rename_all = "lowercase"` maps enum variant names (PascalCase) to DB values (lowercase).

2. **Enum Variant Naming:** Rust uses PascalCase (Outgoing, Incoming), Slint uses snake_case (outgoing, incoming). Slint compiler requires exact variant names. The mapping happens automatically via enum discriminants.

3. **sqlx prepare:** After adding/modifying `sqlx::query_as!` macros, run `cargo sqlx prepare` to update `.sqlx/*.json` cache. This is required even though we're using `sqlx::Type` derive macro.

4. **Slint Enum Comparison:** `step.status == DocumentDirection.outgoing` compiles to a discriminant check (e.g., == 0), so it's type-safe.

5. **Task 8.2 (Naming Callbacks):** Separate work — focuses on callback/property naming consistency in Slint, not on enum types.

---

## Success Criteria

✅ All 6 direction fields in Act/Invoice/Waybill models use `DocumentDirection` enum  
✅ All 2 category kind fields use `CategoryKind` enum  
✅ No `.unwrap()` or unsafe string parsing for these fields  
✅ String comparisons (`== "outgoing"`) replaced with enum comparisons (`== DocumentDirection.Outgoing`)  
✅ Slint types match Rust types (no lossy conversion)  
✅ All tests pass, no regressions  
