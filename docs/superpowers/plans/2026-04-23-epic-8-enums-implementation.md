# Epic 8.1 Implementation Plan — Stringly-Typed → Enums

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert magic string fields (direction, category kind) to native Rust and Slint enums, eliminating fragile string comparisons.

**Architecture:** Rust enums with sqlx::Type traits handle DB↔Rust conversion; Slint enums handle UI type safety; presenter layer maps Rust→Slint automatically.

**Tech Stack:** Rust, sqlx, Slint 1.9, PostgreSQL

---

## Phase 1: Create Rust Enums

### Task 1: Create DocumentDirection Enum

**Files:**
- Create: `src/models/shared.rs`

- [ ] **Step 1: Create new file with DocumentDirection enum**

Create `src/models/shared.rs`:

```rust
use sqlx::FromRow;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_str() {
        assert_eq!(DocumentDirection::Outgoing.as_str(), "outgoing");
        assert_eq!(DocumentDirection::Incoming.as_str(), "incoming");
    }

    #[test]
    fn test_label() {
        assert_eq!(DocumentDirection::Outgoing.label(), "Вихідний");
        assert_eq!(DocumentDirection::Incoming.label(), "Вхідний");
    }

    #[test]
    fn test_display() {
        assert_eq!(DocumentDirection::Outgoing.to_string(), "Вихідний");
        assert_eq!(DocumentDirection::Incoming.to_string(), "Вхідний");
    }

    #[test]
    fn test_try_from_valid() {
        assert_eq!(
            DocumentDirection::try_from("outgoing".to_string()),
            Ok(DocumentDirection::Outgoing)
        );
        assert_eq!(
            DocumentDirection::try_from("incoming".to_string()),
            Ok(DocumentDirection::Incoming)
        );
    }

    #[test]
    fn test_try_from_invalid() {
        assert!(DocumentDirection::try_from("invalid".to_string()).is_err());
    }
}
```

- [ ] **Step 2: Add module export to src/models/mod.rs**

In `src/models/mod.rs`, add at the top with other module declarations:

```rust
pub mod shared;
pub use shared::DocumentDirection;
```

- [ ] **Step 3: Run tests to verify enum works**

```bash
cargo test --lib models::shared --
```

Expected output: All 5 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/models/shared.rs src/models/mod.rs
git commit -m "feat: create DocumentDirection enum with tests"
```

---

### Task 2: Add CategoryKind Enum to category.rs

**Files:**
- Modify: `src/models/category.rs:1-50`
- Test: `src/models/category.rs` (inline tests)

- [ ] **Step 1: Add CategoryKind enum to src/models/category.rs**

At the top of the file, before the `Category` struct definition:

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

impl std::fmt::Display for CategoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl TryFrom<String> for CategoryKind {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "income" => Ok(Self::Income),
            "expense" => Ok(Self::Expense),
            _ => Err(format!("Unknown category kind: {}", s)),
        }
    }
}
```

- [ ] **Step 2: Add tests for CategoryKind in src/models/category.rs**

Add to the `#[cfg(test)]` module at the end of `category.rs`:

```rust
#[test]
fn test_category_kind_as_str() {
    assert_eq!(CategoryKind::Income.as_str(), "income");
    assert_eq!(CategoryKind::Expense.as_str(), "expense");
}

#[test]
fn test_category_kind_label() {
    assert_eq!(CategoryKind::Income.label(), "Дохід");
    assert_eq!(CategoryKind::Expense.label(), "Видаток");
}

#[test]
fn test_category_kind_try_from() {
    assert_eq!(
        CategoryKind::try_from("income".to_string()),
        Ok(CategoryKind::Income)
    );
    assert_eq!(
        CategoryKind::try_from("expense".to_string()),
        Ok(CategoryKind::Expense)
    );
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test --lib models::category --
```

Expected: All CategoryKind tests pass (plus existing Category tests).

- [ ] **Step 4: Commit**

```bash
git add src/models/category.rs
git commit -m "feat: add CategoryKind enum with tests"
```

---

## Phase 2: Update Rust Model Structs

### Task 3: Update Act/ActListRow direction field

**Files:**
- Modify: `src/models/act.rs:80-130` (struct definitions)

- [ ] **Step 1: Find Act struct and change direction field type**

In `src/models/act.rs`, locate the `Act` struct (around line 80-100). Change:

```rust
// Before:
pub struct Act {
    pub id: Uuid,
    pub company_id: Uuid,
    pub direction: String,  // ← change this
    // ... other fields
}

// After:
pub struct Act {
    pub id: Uuid,
    pub company_id: Uuid,
    pub direction: DocumentDirection,  // ← now enum
    // ... other fields
}
```

- [ ] **Step 2: Change ActListRow direction field**

In the same file, locate `ActListRow` struct. Change:

```rust
// Before:
pub struct ActListRow {
    // ...
    pub direction: String,  // ← change this
}

// After:
pub struct ActListRow {
    // ...
    pub direction: DocumentDirection,  // ← now enum
}
```

- [ ] **Step 3: Add DocumentDirection import at top of act.rs**

At the top of `src/models/act.rs`, add:

```rust
use crate::models::DocumentDirection;
```

- [ ] **Step 4: Run compiler check**

```bash
cargo check
```

Expected: Errors in DB layer (next phase) — that's fine, we'll fix them.

- [ ] **Step 5: Commit**

```bash
git add src/models/act.rs
git commit -m "refactor: change Act.direction from String to DocumentDirection enum"
```

---

### Task 4: Update Invoice/InvoiceListRow direction field

**Files:**
- Modify: `src/models/invoice.rs:80-130`

- [ ] **Step 1: Change Invoice direction field**

In `src/models/invoice.rs`, locate the `Invoice` struct and change:

```rust
// Before:
pub struct Invoice {
    pub direction: String,

// After:
pub struct Invoice {
    pub direction: DocumentDirection,
```

- [ ] **Step 2: Change InvoiceListRow direction field**

```rust
// Before:
pub struct InvoiceListRow {
    pub direction: String,

// After:
pub struct InvoiceListRow {
    pub direction: DocumentDirection,
```

- [ ] **Step 3: Add DocumentDirection import**

At the top of `src/models/invoice.rs`:

```rust
use crate::models::DocumentDirection;
```

- [ ] **Step 4: Run compiler check**

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add src/models/invoice.rs
git commit -m "refactor: change Invoice.direction from String to DocumentDirection enum"
```

---

### Task 5: Update Waybill/WaybillListRow direction field

**Files:**
- Modify: `src/models/waybill.rs:75-125`

- [ ] **Step 1: Change Waybill direction field**

```rust
// Before:
pub struct Waybill {
    pub direction: String,

// After:
pub struct Waybill {
    pub direction: DocumentDirection,
```

- [ ] **Step 2: Change WaybillListRow direction field**

```rust
// Before:
pub struct WaybillListRow {
    pub direction: String,

// After:
pub struct WaybillListRow {
    pub direction: DocumentDirection,
```

- [ ] **Step 3: Add DocumentDirection import**

At the top of `src/models/waybill.rs`:

```rust
use crate::models::DocumentDirection;
```

- [ ] **Step 4: Run compiler check**

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add src/models/waybill.rs
git commit -m "refactor: change Waybill.direction from String to DocumentDirection enum"
```

---

### Task 6: Update Category/CategorySelectItem kind field

**Files:**
- Modify: `src/models/category.rs:40-65` (struct definitions)

- [ ] **Step 1: Find Category struct, change kind field**

In `src/models/category.rs`, locate the `Category` struct and change:

```rust
// Before:
pub struct Category {
    pub id: Uuid,
    pub kind: String,  // ← change
    // ...
}

// After:
pub struct Category {
    pub id: Uuid,
    pub kind: CategoryKind,  // ← now enum
    // ...
}
```

- [ ] **Step 2: Update CategorySelectItem kind field**

```rust
// Before:
pub struct CategorySelectItem {
    pub kind: String,

// After:
pub struct CategorySelectItem {
    pub kind: CategoryKind,
```

- [ ] **Step 3: Run compiler check**

```bash
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add src/models/category.rs
git commit -m "refactor: change Category.kind from String to CategoryKind enum"
```

---

## Phase 3: Update DB Layer

### Task 7: Update src/db/acts.rs for DocumentDirection

**Files:**
- Modify: `src/db/acts.rs:1-200` (all functions using direction)

- [ ] **Step 1: Add DocumentDirection import**

At the top of `src/db/acts.rs`:

```rust
use crate::models::DocumentDirection;
```

- [ ] **Step 2: Update insert_act function signature**

Find the `insert_act` function. Change the parameter from:

```rust
// Before:
pub async fn insert_act(
    pool: &PgPool,
    company_id: Uuid,
    direction: &str,  // ← change
    // ... other params
) -> Result<Uuid> {

// After:
pub async fn insert_act(
    pool: &PgPool,
    company_id: Uuid,
    direction: DocumentDirection,  // ← now enum
    // ... other params
) -> Result<Uuid> {
```

In the function body, the `direction` parameter is already the right type. The `sqlx::query!` macro will automatically handle the enum→string conversion via the `sqlx::Type` trait.

- [ ] **Step 3: Update update_act function signature**

Similarly, find `update_act` function and change:

```rust
// Before:
pub async fn update_act(
    pool: &PgPool,
    id: Uuid,
    direction: &str,  // ← change

// After:
pub async fn update_act(
    pool: &PgPool,
    id: Uuid,
    direction: DocumentDirection,  // ← now enum
```

- [ ] **Step 4: Check read functions**

For `get_act` and `list_acts` functions: **no changes needed**. The `sqlx::query_as!` macro automatically converts VARCHAR columns to enum via `sqlx::Type`.

- [ ] **Step 5: Run cargo check and tests**

```bash
cargo check
cargo test --lib db::acts --
```

Expected: Tests pass (sqlx handles enum conversion automatically).

- [ ] **Step 6: Commit**

```bash
git add src/db/acts.rs
git commit -m "refactor: update db::acts to use DocumentDirection enum"
```

---

### Task 8: Update src/db/invoices.rs for DocumentDirection

**Files:**
- Modify: `src/db/invoices.rs:1-200`

- [ ] **Step 1: Add DocumentDirection import**

```rust
use crate::models::DocumentDirection;
```

- [ ] **Step 2: Update insert_invoice signature**

```rust
// Before:
pub async fn insert_invoice(
    pool: &PgPool,
    company_id: Uuid,
    direction: &str,

// After:
pub async fn insert_invoice(
    pool: &PgPool,
    company_id: Uuid,
    direction: DocumentDirection,
```

- [ ] **Step 3: Update update_invoice signature**

```rust
// Before:
pub async fn update_invoice(
    pool: &PgPool,
    id: Uuid,
    direction: &str,

// After:
pub async fn update_invoice(
    pool: &PgPool,
    id: Uuid,
    direction: DocumentDirection,
```

- [ ] **Step 4: Run tests**

```bash
cargo test --lib db::invoices --
```

- [ ] **Step 5: Commit**

```bash
git add src/db/invoices.rs
git commit -m "refactor: update db::invoices to use DocumentDirection enum"
```

---

### Task 9: Update src/db/waybills.rs for DocumentDirection

**Files:**
- Modify: `src/db/waybills.rs:1-200`

- [ ] **Step 1: Add DocumentDirection import**

```rust
use crate::models::DocumentDirection;
```

- [ ] **Step 2: Update insert_waybill signature**

```rust
// Before:
pub async fn insert_waybill(
    pool: &PgPool,
    company_id: Uuid,
    direction: &str,

// After:
pub async fn insert_waybill(
    pool: &PgPool,
    company_id: Uuid,
    direction: DocumentDirection,
```

- [ ] **Step 3: Update update_waybill signature**

```rust
// Before:
pub async fn update_waybill(
    pool: &PgPool,
    id: Uuid,
    direction: &str,

// After:
pub async fn update_waybill(
    pool: &PgPool,
    id: Uuid,
    direction: DocumentDirection,
```

- [ ] **Step 4: Run tests**

```bash
cargo test --lib db::waybills --
```

- [ ] **Step 5: Commit**

```bash
git add src/db/waybills.rs
git commit -m "refactor: update db::waybills to use DocumentDirection enum"
```

---

### Task 10: Update src/db/categories.rs for CategoryKind

**Files:**
- Modify: `src/db/categories.rs:1-150`

- [ ] **Step 1: Add CategoryKind import**

At the top of `src/db/categories.rs`:

```rust
use crate::models::CategoryKind;
```

- [ ] **Step 2: Update create_category function signature**

Find the `create_category` function:

```rust
// Before:
pub async fn create_category(
    pool: &PgPool,
    company_id: Uuid,
    kind: &str,  // ← change

// After:
pub async fn create_category(
    pool: &PgPool,
    company_id: Uuid,
    kind: CategoryKind,  // ← now enum
```

- [ ] **Step 3: Update update_category function signature**

```rust
// Before:
pub async fn update_category(
    pool: &PgPool,
    id: Uuid,
    kind: &str,

// After:
pub async fn update_category(
    pool: &PgPool,
    id: Uuid,
    kind: CategoryKind,
```

- [ ] **Step 4: Run tests**

```bash
cargo test --lib db::categories --
```

- [ ] **Step 5: Commit**

```bash
git add src/db/categories.rs
git commit -m "refactor: update db::categories to use CategoryKind enum"
```

---

### Task 11: Run cargo sqlx prepare to update .sqlx cache

**Files:**
- Modify: `.sqlx/*.json` (auto-updated)

- [ ] **Step 1: Run sqlx prepare**

```bash
cargo sqlx prepare -- --lib
```

Expected: `.sqlx/` directory updated with new metadata for the enum fields.

- [ ] **Step 2: Verify no errors**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add .sqlx/
git commit -m "chore: update sqlx cache after enum changes"
```

---

## Phase 4: Update Slint Types and Structures

### Task 12: Add DocumentDirection and CategoryKind enums to Slint

**Files:**
- Modify: `ui/types.slint:1-50`

- [ ] **Step 1: Add DocumentDirection enum to ui/types.slint**

Near the top of `ui/types.slint`, add:

```slint
export enum DocumentDirection {
    outgoing,
    incoming,
}
```

- [ ] **Step 2: Add CategoryKind enum to ui/types.slint**

Right after DocumentDirection:

```slint
export enum CategoryKind {
    income,
    expense,
}
```

- [ ] **Step 3: Verify Slint compiles**

```bash
cargo check
```

Expected: Slint compiles without errors (enums are now available for use).

- [ ] **Step 4: Commit**

```bash
git add ui/types.slint
git commit -m "feat: add DocumentDirection and CategoryKind enums to Slint"
```

---

### Task 13: Update Act struct fields in Slint

**Files:**
- Modify: `ui/types.slint:120-160` (struct definitions)

- [ ] **Step 1: Find Act struct in types.slint, change direction field**

Locate the `Act` struct definition:

```slint
// Before:
export struct Act {
    direction: string,

// After:
export struct Act {
    direction: DocumentDirection,
```

- [ ] **Step 2: Update ActListRow direction field**

```slint
// Before:
export struct ActListRow {
    direction: string,

// After:
export struct ActListRow {
    direction: DocumentDirection,
```

- [ ] **Step 3: Run cargo check**

```bash
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add ui/types.slint
git commit -m "refactor: change Act/ActListRow.direction to DocumentDirection enum in Slint"
```

---

### Task 14: Update Invoice struct fields in Slint

**Files:**
- Modify: `ui/types.slint:160-200`

- [ ] **Step 1: Change Invoice direction field**

```slint
// Before:
export struct Invoice {
    direction: string,

// After:
export struct Invoice {
    direction: DocumentDirection,
```

- [ ] **Step 2: Change InvoiceListRow direction field**

```slint
// Before:
export struct InvoiceListRow {
    direction: string,

// After:
export struct InvoiceListRow {
    direction: DocumentDirection,
```

- [ ] **Step 3: Run cargo check**

```bash
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add ui/types.slint
git commit -m "refactor: change Invoice/InvoiceListRow.direction to DocumentDirection enum in Slint"
```

---

### Task 15: Update Waybill struct fields in Slint

**Files:**
- Modify: `ui/types.slint:200-240`

- [ ] **Step 1: Change Waybill direction field**

```slint
// Before:
export struct Waybill {
    direction: string,

// After:
export struct Waybill {
    direction: DocumentDirection,
```

- [ ] **Step 2: Change WaybillListRow direction field**

```slint
// Before:
export struct WaybillListRow {
    direction: string,

// After:
export struct WaybillListRow {
    direction: DocumentDirection,
```

- [ ] **Step 3: Run cargo check**

```bash
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add ui/types.slint
git commit -m "refactor: change Waybill/WaybillListRow.direction to DocumentDirection enum in Slint"
```

---

### Task 16: Update Category struct fields in Slint

**Files:**
- Modify: `ui/types.slint:240-280`

- [ ] **Step 1: Change Category kind field**

```slint
// Before:
export struct Category {
    kind: string,

// After:
export struct Category {
    kind: CategoryKind,
```

- [ ] **Step 2: Change CategorySelectItem kind field**

```slint
// Before:
export struct CategorySelectItem {
    kind: string,

// After:
export struct CategorySelectItem {
    kind: CategoryKind,
```

- [ ] **Step 3: Run cargo check**

```bash
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add ui/types.slint
git commit -m "refactor: change Category/CategorySelectItem.kind to CategoryKind enum in Slint"
```

---

## Phase 5: Update Presenter Layer

### Task 17: Update src/ui/acts.rs presenter

**Files:**
- Modify: `src/ui/acts.rs:50-150` (apply_acts_to_ui function)

- [ ] **Step 1: No code changes needed**

The `apply_acts_to_ui` function already passes `act.direction` (which is now `DocumentDirection`) to Slint. Slint enums map automatically.

Run a test to verify data flows correctly:

```bash
cargo build
```

Expected: No compilation errors.

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "chore: verify acts presenter works with DocumentDirection enum"
```

---

### Task 18: Update src/ui/invoices.rs presenter

**Files:**
- Modify: `src/ui/invoices.rs:50-150`

- [ ] **Step 1: Verify presenter code**

Run build:

```bash
cargo build
```

Expected: No errors related to direction field.

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "chore: verify invoices presenter works with DocumentDirection enum"
```

---

### Task 19: Update src/ui/waybills.rs presenter

**Files:**
- Modify: `src/ui/waybills.rs:50-150`

- [ ] **Step 1: Verify presenter code**

```bash
cargo build
```

Expected: No errors.

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "chore: verify waybills presenter works with DocumentDirection enum"
```

---

### Task 20: Update src/ui/categories.rs presenter

**Files:**
- Modify: `src/ui/categories.rs:50-150`

- [ ] **Step 1: Verify presenter code**

```bash
cargo build
```

Expected: No errors.

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "chore: verify categories presenter works with CategoryKind enum"
```

---

## Phase 6: Integration Testing & Final Build

### Task 21: Full build and integration test

**Files:**
- Test: Manual verification

- [ ] **Step 1: Run full test suite**

```bash
cargo test --lib
```

Expected: All tests pass, no regressions.

- [ ] **Step 2: Run full build**

```bash
cargo build --release
```

Expected: No errors or warnings.

- [ ] **Step 3: Manual smoke test (if applicable)**

If the app can run locally:

```bash
cargo run
```

Verify:
- Acts load with correct direction display (Вихідний/Вхідний)
- Invoices load with correct direction display
- Waybills load with correct direction display
- Categories load with correct kind display (Дохід/Видаток)

- [ ] **Step 4: Final commit message**

```bash
git log --oneline -10
```

Verify that commits follow the pattern and log is clean.

---

## Success Criteria (Self-Review)

✅ DocumentDirection enum created in `src/models/shared.rs` with all methods  
✅ CategoryKind enum created in `src/models/category.rs` with all methods  
✅ All 6 direction fields (Act, Invoice, Waybill × ListRow) changed to `DocumentDirection`  
✅ All 2 category kind fields (Category, CategorySelectItem) changed to `CategoryKind`  
✅ DB layer functions updated to accept enums instead of `&str`  
✅ Slint enums defined and struct fields updated  
✅ `cargo sqlx prepare` run and `.sqlx/` committed  
✅ All tests pass (`cargo test --lib`)  
✅ No compilation errors (`cargo build`)  
✅ No string comparisons (`== "outgoing"`) remain in Rust code  

---

## Notes for Implementation

1. **Enum variant naming:** Rust uses PascalCase (Outgoing, Incoming); Slint uses snake_case (outgoing, incoming). The mapping is automatic via discriminants.

2. **sqlx::Type derivation:** The `#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]` attribute handles the conversion. No custom `FromRow` needed.

3. **No DB migration:** VARCHAR columns stay unchanged. The enum is purely a Rust↔Slint layer improvement.

4. **TryFrom implementation:** Used for converting strings to enums if needed at app boundaries, though most conversions happen automatically via sqlx.

5. **Presenter layer:** No changes needed — `act.direction` is already the enum, Slint receives it as-is.
