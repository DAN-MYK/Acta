# Documents Filter Expansion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-05-08-documents-filter-expansion-design.md`
**Goal:** Replace the standalone "Пошук документів" search input with a richer document filter (period, multi-status, counterparty, amount, overdue) plus topline quick-presets, active-filter chips, and a counter badge.

**Architecture:** SQL `QueryBuilder` фрагменти в трьох таблицях (acts/invoices/waybills) приймають нові параметри (`statuses: &[String]`, `amount_min/max: Decimal`, `overdue_only: bool`); single-object Tauri DTO; Svelte store розширюється новими setters з автоскиданням `activePresetId`; UI рендерить inline-розгорнуту filter-panel + presets row + active chips. Money-amount передається як **major-units** decimal string (нормалізується на UI, парситься у `Decimal` на Rust-стороні), щоб збігтися з `total_amount` у БД.

**Tech Stack:** Rust + sqlx + chrono + rust_decimal | Svelte + TypeScript + vitest | Tauri 2.

---

## File Structure

**Backend (Rust)**
- `src/db/acts.rs` — `list_filtered` нова сигнатура: multi-status, amount range, overdue_only.
- `src/db/invoices.rs` — те саме.
- `src/db/waybills.rs` — multi-status + amount range (без overdue_only — нема `expected_payment_date`).
- `src/tauri_api/counterparties.rs` — оновити 2 callers `list_filtered`.
- `src/tauri_api/documents/api.rs` — оновити 3 callers + waybill-skip при `overdue_only`.
- `src/tauri_api/documents/dto.rs` — нові поля `DocumentsListRequest`.
- `tests/db_integration/acts.rs` — нові тести (amount, overdue).
- `tests/db_integration/invoices.rs` — оновити 3 наявні `list_filtered` callers; додати тести (amount, overdue).
- `tests/db_integration/waybills.rs` — нові тести (amount, multi-status).
- `tests/tauri_vertical_slice/documents.rs` — vertical slice для повної комбінації фільтрів.

**Frontend**
- `frontend/src/lib/types.ts` — `DocumentStatus` + розширити DTO type.
- `frontend/src/lib/api.ts` — `documentsList` single-object argument.
- `frontend/src/lib/stores/documents.ts` — нові поля state, setters, `applyPreset`, `applyFilters`, `clearAllFilters`.
- `frontend/src/lib/stores/__tests__/documents.test.ts` — новий store-тест.
- `frontend/src/lib/config/ui.ts` — `DOCUMENT_FILTER_PRESETS`, `DOCUMENT_STATUS_OPTIONS`, `DOCUMENTS_FILTER_COPY`.
- `frontend/src/lib/screens/DocumentsScreen.svelte` — UI.
- `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts` — оновити.
- `frontend/src/styles/documents.css` — стилі.

---

## Phase 1 — Backend `list_filtered` signature migration

### Task 1: Migrate `list_filtered` signatures across 3 db modules + all callers

**Files:**
- Modify: `src/db/acts.rs:139-148`, `src/db/invoices.rs:97-106`, `src/db/waybills.rs:85-94`
- Modify callers: `src/tauri_api/counterparties.rs:336-355`, `src/tauri_api/documents/api.rs:800-814`, `tests/db_integration/invoices.rs:74-84`, `:656-666`, `:670-680`

This is a single-commit refactor — change three `list_filtered` signatures and patch every caller in the same commit so the project compiles. New parameters added: `statuses: Option<&[String]>` (replaces `status_filter`), `amount_min: Option<Decimal>`, `amount_max: Option<Decimal>`. For `acts.rs` and `invoices.rs` also add `overdue_only: bool`. Keep `status_filter` semantics by translating: callers passing `Some(InvoiceStatus::Issued)` become `Some(&["issued".to_string()][..])`.

- [ ] **Step 1: Update `src/db/acts.rs:139-148` signature**

Replace:
```rust
pub async fn list_filtered(
    pool: &PgPool,
    company_id: Uuid,
    status_filter: Option<ActStatus>,
    direction: Option<DocumentDirection>,
    search_query: Option<&str>,
    counterparty_id: Option<Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
) -> Result<Vec<ActListRow>> {
```

with:
```rust
pub async fn list_filtered(
    pool: &PgPool,
    company_id: Uuid,
    statuses: Option<&[String]>,
    direction: Option<DocumentDirection>,
    search_query: Option<&str>,
    counterparty_id: Option<Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
    amount_min: Option<rust_decimal::Decimal>,
    amount_max: Option<rust_decimal::Decimal>,
    overdue_only: bool,
    today: chrono::NaiveDate,
) -> Result<Vec<ActListRow>> {
```

Inside the function: replace existing `if let Some(status) = status_filter { qb.push(" AND a.status = ").push_bind(status.as_str()); }` with the multi-status block:

```rust
if let Some(statuses) = statuses.filter(|s| !s.is_empty()) {
    let owned: Vec<String> = statuses.to_vec();
    qb.push(" AND a.status::text = ANY(").push_bind(owned).push("::text[])");
}
```

After all existing `WHERE`-builders (and after the `date_to` block) append:
```rust
if let Some(min) = amount_min {
    qb.push(" AND a.total_amount >= ").push_bind(min);
}
if let Some(max) = amount_max {
    qb.push(" AND a.total_amount <= ").push_bind(max);
}
if overdue_only {
    qb.push(" AND a.expected_payment_date IS NOT NULL")
      .push(" AND a.expected_payment_date < ").push_bind(today)
      .push(" AND a.status::text IN ('issued','signed')");
}
```

- [ ] **Step 2: Update the public `list` wrapper in `src/db/acts.rs:121`**

The wrapper at `acts.rs:121` calls `list_filtered`. Find it (it forwards to `list_filtered` with all `None`s) and add the four new arguments: `None, None, false, chrono::Utc::now().date_naive()`.

- [ ] **Step 3: Apply the same pattern to `src/db/invoices.rs:97-106`**

Same signature change for invoices: alias `i.`, same blocks for `amount_min/amount_max/overdue_only`. The existing `list` wrapper at `invoices.rs:79` also forwards — append four args.

- [ ] **Step 4: Apply to `src/db/waybills.rs:85-94`**

Same signature change for waybills BUT **omit `overdue_only`** parameter and SQL block — waybill schema lacks `expected_payment_date`. Add only: `statuses: Option<&[String]>`, `amount_min`, `amount_max`. Wrapper at `waybills.rs:70` forwards with two new `None`s.

- [ ] **Step 5: Patch caller `src/tauri_api/counterparties.rs:336`**

Replace the `db::acts::list_filtered(...)` block (lines 336-345):
```rust
db::acts::list_filtered(
    ctx.pool(),
    company_id,
    None,
    None,
    None,
    Some(counterparty_id),
    None,
    None,
    None,
    None,
    false,
    chrono::Utc::now().date_naive(),
),
```

And the `db::invoices::list_filtered(...)` block (lines 346-355) the same way (4 new args at the end).

- [ ] **Step 6: Patch callers `src/tauri_api/documents/api.rs:800,807,814`**

Each call currently ends with `..., counterparty_filter, None, None).await`. Append `, None, None, false, chrono::Utc::now().date_naive()` for acts/invoices, and `, None, None` for waybills. Note: at this stage we do NOT yet wire the new `request.amount_min/amount_max/overdue_only` — that happens in Task 7. Right now we just preserve current behaviour while the signature compiles.

- [ ] **Step 7: Patch test callers `tests/db_integration/invoices.rs:74,656,670`**

Line 74 (passes `None` for status):
```rust
let listed = db::invoices::list_filtered(
    &pool,
    DEFAULT_COMPANY_ID,
    None,
    None,
    Some("IT-INV-"),
    None,
    None,
    None,
    None,
    None,
    false,
    chrono::Utc::now().date_naive(),
)
.await?;
```

Lines 656 and 670 (pass `Some(models::InvoiceStatus::Issued)`):
```rust
let issued_only = db::invoices::list_filtered(
    &pool,
    DEFAULT_COMPANY_ID,
    Some(&["issued".to_string()]),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    false,
    chrono::Utc::now().date_naive(),
)
.await?;
```

(Line 670 same pattern, plus its `Some("FILTER-ISSUED")` search.)

- [ ] **Step 8: Compile**

Run: `cargo build --tests`
Expected: `Finished` (no errors). If a caller is missing, the compiler will name it — fix and re-run.

- [ ] **Step 9: Run existing tests to confirm no regression**

Run: `cargo test --lib`
Expected: all green. (DB-gated tests skipped without `TEST_DATABASE_URL`.)

If `TEST_DATABASE_URL` is set:
Run: `TEST_DATABASE_URL=$TEST_DATABASE_URL cargo test --test db_integration -- invoices`
Expected: all 3 patched tests pass with new args.

- [ ] **Step 10: Commit**

```bash
git add src/db/acts.rs src/db/invoices.rs src/db/waybills.rs \
        src/tauri_api/counterparties.rs src/tauri_api/documents/api.rs \
        tests/db_integration/invoices.rs
git commit -m "refactor(db): migrate list_filtered to multi-status + amount + overdue params

Adds:
- statuses: Option<&[String]> (replaces single status enum)
- amount_min, amount_max: Option<Decimal>
- overdue_only: bool, today: NaiveDate (acts/invoices only — waybills lack expected_payment_date)

All call sites updated; semantics preserved at this stage."
```

---

## Phase 2 — Backend new-feature TDD

### Task 2: TDD amount range filter for acts

**Files:**
- Test: `tests/db_integration/acts.rs` (new test in existing file; if file doesn't exist, create with the same module pattern as `tests/db_integration/invoices.rs`)
- Modify: `src/db/acts.rs:139-...` (already extended in Task 1 — verify the SQL block)

- [ ] **Step 1: Locate or create `tests/db_integration/acts.rs`**

Run: `ls tests/db_integration/`
If `acts.rs` exists, append to it; if not, create with the canonical preamble (copy module imports from `invoices.rs:1-30`).

- [ ] **Step 2: Add failing test `list_filtered_amount_range`**

Append to `tests/db_integration/acts.rs`:
```rust
#[sqlx::test(migrations = "./migrations")]
async fn list_filtered_amount_range(pool: sqlx::PgPool) -> anyhow::Result<()> {
    use rust_decimal_macros::dec;

    let cp = seed_counterparty(&pool, "AMT-CP").await?;
    seed_act(&pool, &cp, "AMT-A-500", dec!(500.00)).await?;
    seed_act(&pool, &cp, "AMT-A-5000", dec!(5000.00)).await?;
    seed_act(&pool, &cp, "AMT-A-50000", dec!(50000.00)).await?;

    let mid = db::acts::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        None,
        None,
        Some("AMT-A-"),
        Some(cp.id),
        None,
        None,
        Some(dec!(1000)),
        Some(dec!(10000)),
        false,
        chrono::Utc::now().date_naive(),
    )
    .await?;

    assert_eq!(mid.len(), 1);
    assert_eq!(mid[0].number, "AMT-A-5000");

    sqlx::query("DELETE FROM acts WHERE counterparty_id = $1").bind(cp.id).execute(&pool).await?;
    Ok(())
}
```

If `seed_act` helper doesn't exist in this file, define it inline mirroring how `tests/db_integration/invoices.rs` builds invoices via `db::acts::create(...)` + `NewAct { number, counterparty_id: cp.id, contract_id: None, category_id: None, direction: DocumentDirection::Outgoing, date: chrono::Utc::now().date_naive(), expected_payment_date: None, status: ActStatus::Draft, notes: None, bas_id: None, items: vec![NewActItem { description: "x".into(), quantity: dec!(1), unit: "шт".into(), unit_price: amount }] }`.

- [ ] **Step 3: Run the test — expect FAIL**

Run: `TEST_DATABASE_URL=$TEST_DATABASE_URL cargo test --test db_integration -- list_filtered_amount_range`
Expected: FAIL — at this point Task 1's SQL block exists but the test exercises it for the first time. If the SQL has a typo, this is where we catch it.

If FAIL is "no such test" — re-check file location.
If FAIL is "expected 1 row, got 3" — SQL block in `src/db/acts.rs` is missing or malformed; re-check Task 1 Step 1.

- [ ] **Step 4: If SQL block from Task 1 is correct, the test should pass**

Re-run: `TEST_DATABASE_URL=$TEST_DATABASE_URL cargo test --test db_integration -- list_filtered_amount_range`
Expected: PASS. (Task 1 already wrote the SQL — this test is the verification.)

- [ ] **Step 5: Commit**

```bash
git add tests/db_integration/acts.rs
git commit -m "test(db): cover amount range filter in acts list_filtered"
```

### Task 3: TDD multi-status filter for acts

**Files:**
- Test: `tests/db_integration/acts.rs` (append)

- [ ] **Step 1: Append failing test `list_filtered_multi_status`**

```rust
#[sqlx::test(migrations = "./migrations")]
async fn list_filtered_multi_status(pool: sqlx::PgPool) -> anyhow::Result<()> {
    use rust_decimal_macros::dec;

    let cp = seed_counterparty(&pool, "MS-CP").await?;
    let draft = seed_act(&pool, &cp, "MS-DRAFT", dec!(100)).await?;
    let issued = seed_act(&pool, &cp, "MS-ISSUED", dec!(100)).await?;
    db::acts::change_status(&pool, issued.id, models::ActStatus::Issued).await?.expect("issued");
    let paid = seed_act(&pool, &cp, "MS-PAID", dec!(100)).await?;
    db::acts::change_status(&pool, paid.id, models::ActStatus::Issued).await?;
    db::acts::change_status(&pool, paid.id, models::ActStatus::Signed).await?;
    db::acts::change_status(&pool, paid.id, models::ActStatus::Paid).await?;

    let filtered = db::acts::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(&["draft".to_string(), "paid".to_string()]),
        None,
        Some("MS-"),
        Some(cp.id),
        None,
        None,
        None,
        None,
        false,
        chrono::Utc::now().date_naive(),
    )
    .await?;

    let numbers: Vec<&str> = filtered.iter().map(|r| r.number.as_str()).collect();
    assert_eq!(numbers.len(), 2);
    assert!(numbers.contains(&"MS-DRAFT"));
    assert!(numbers.contains(&"MS-PAID"));
    assert!(!numbers.contains(&"MS-ISSUED"));

    sqlx::query("DELETE FROM acts WHERE counterparty_id = $1").bind(cp.id).execute(&pool).await?;
    Ok(())
}
```

- [ ] **Step 2: Run test — expect PASS (Task 1 already wrote the multi-status SQL block)**

Run: `TEST_DATABASE_URL=$TEST_DATABASE_URL cargo test --test db_integration -- list_filtered_multi_status`
Expected: PASS.

If FAIL with "expected 2, got 0" — `WHERE status::text = ANY($N::text[])` may have a typo; verify Task 1 Step 1 SQL.
If FAIL with "expected 2, got 3" — `if let Some(statuses) = statuses.filter(|s| !s.is_empty())` is missing the `.filter()` so empty list isn't treated as "no filter"; recheck.

- [ ] **Step 3: Commit**

```bash
git add tests/db_integration/acts.rs
git commit -m "test(db): cover multi-status filter in acts list_filtered"
```

### Task 4: TDD overdue_only for acts

**Files:**
- Test: `tests/db_integration/acts.rs` (append)

- [ ] **Step 1: Append failing test `list_filtered_overdue_only`**

```rust
#[sqlx::test(migrations = "./migrations")]
async fn list_filtered_overdue_only(pool: sqlx::PgPool) -> anyhow::Result<()> {
    use rust_decimal_macros::dec;

    let today = chrono::NaiveDate::from_ymd_opt(2026, 5, 8).unwrap();
    let past = chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
    let future = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

    let cp = seed_counterparty(&pool, "OVD-CP").await?;

    // 1. paid + overdue payment date — should NOT match (paid is excluded)
    let paid = seed_act_with_due(&pool, &cp, "OVD-PAID", dec!(100), Some(past)).await?;
    db::acts::change_status(&pool, paid.id, models::ActStatus::Issued).await?;
    db::acts::change_status(&pool, paid.id, models::ActStatus::Signed).await?;
    db::acts::change_status(&pool, paid.id, models::ActStatus::Paid).await?;

    // 2. issued + overdue payment date — SHOULD match
    let overdue = seed_act_with_due(&pool, &cp, "OVD-ISSUED", dec!(100), Some(past)).await?;
    db::acts::change_status(&pool, overdue.id, models::ActStatus::Issued).await?;

    // 3. issued + future payment date — should NOT match
    let future_due = seed_act_with_due(&pool, &cp, "OVD-FUTURE", dec!(100), Some(future)).await?;
    db::acts::change_status(&pool, future_due.id, models::ActStatus::Issued).await?;

    // 4. draft + overdue payment date — should NOT match (drafts not yet issued)
    let draft = seed_act_with_due(&pool, &cp, "OVD-DRAFT", dec!(100), Some(past)).await?;
    let _ = draft;

    let result = db::acts::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        None,
        None,
        Some("OVD-"),
        Some(cp.id),
        None,
        None,
        None,
        None,
        true,
        today,
    )
    .await?;

    let numbers: Vec<&str> = result.iter().map(|r| r.number.as_str()).collect();
    assert_eq!(numbers, vec!["OVD-ISSUED"]);

    sqlx::query("DELETE FROM acts WHERE counterparty_id = $1").bind(cp.id).execute(&pool).await?;
    Ok(())
}
```

If `seed_act_with_due` helper doesn't exist, add it: same as `seed_act` but accepts `expected_payment_date: Option<NaiveDate>` and passes it through to `NewAct`.

- [ ] **Step 2: Run test**

Run: `TEST_DATABASE_URL=$TEST_DATABASE_URL cargo test --test db_integration -- list_filtered_overdue_only`
Expected: PASS.

If FAIL with `OVD-DRAFT` in result — SQL is `NOT IN ('paid','delivered')` instead of `IN ('issued','signed')`. Re-check Task 1 Step 1 — it must be the whitelist form.
If FAIL with `OVD-PAID` in result — same root cause.
If FAIL with `OVD-FUTURE` in result — `expected_payment_date < today` comparison missing or wrong direction.

- [ ] **Step 3: Commit**

```bash
git add tests/db_integration/acts.rs
git commit -m "test(db): cover overdue_only whitelist in acts list_filtered"
```

### Task 5: Mirror tests for invoices (`amount`, `multi_status`, `overdue`)

**Files:**
- Test: `tests/db_integration/invoices.rs` (append three tests at the bottom of the file)

- [ ] **Step 1: Append three tests mirroring Tasks 2-4**

Use the same structure but call `db::invoices::list_filtered` and `db::invoices::create` / `models::InvoiceStatus`. Numbering prefixes: `INV-AMT-*`, `INV-MS-*`, `INV-OVD-*`. Same assertions.

- [ ] **Step 2: Run all three**

Run: `TEST_DATABASE_URL=$TEST_DATABASE_URL cargo test --test db_integration -- list_filtered_amount_range list_filtered_multi_status list_filtered_overdue_only`
Expected: ALL PASS (both acts and invoices versions). The test runner deduplicates by file::name, so namespace each by full path: `db_integration::invoices::list_filtered_amount_range`.

If duplicate-name collision: prefix invoices versions with `invoices_` (e.g. `invoices_list_filtered_amount_range`).

- [ ] **Step 3: Commit**

```bash
git add tests/db_integration/invoices.rs
git commit -m "test(db): cover amount + multi-status + overdue in invoices list_filtered"
```

### Task 6: Tests for waybills (`amount`, `multi_status`)

**Files:**
- Test: `tests/db_integration/waybills.rs` (append two tests; create file if absent following acts pattern)

- [ ] **Step 1: Append `list_filtered_amount_range` for waybills**

Same structure as Task 2 but `db::waybills::list_filtered`, `db::waybills::create`, `models::WaybillStatus`. **Note the signature has only 11 args** (no `overdue_only`, no `today`):
```rust
db::waybills::list_filtered(
    &pool, DEFAULT_COMPANY_ID,
    None, None, Some("WBL-AMT-"), Some(cp.id),
    None, None,
    Some(dec!(1000)), Some(dec!(10000)),
).await?;
```

- [ ] **Step 2: Append `list_filtered_multi_status` for waybills**

Use statuses `["draft", "delivered"]` (delivered exists for waybills). Seed three waybills, advance two through their state machines.

- [ ] **Step 3: Run both**

Run: `TEST_DATABASE_URL=$TEST_DATABASE_URL cargo test --test db_integration -- waybills`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add tests/db_integration/waybills.rs
git commit -m "test(db): cover amount + multi-status in waybills list_filtered"
```

---

## Phase 3 — Tauri DTO + `documents_list`

### Task 7: Extend `DocumentsListRequest` and wire into `documents_list`

**Files:**
- Modify: `src/tauri_api/documents/dto.rs` (find `DocumentsListRequest`)
- Modify: `src/tauri_api/documents/api.rs:774-906`
- Test: `tests/tauri_vertical_slice/documents.rs` (append vertical slice)

- [ ] **Step 1: Locate `DocumentsListRequest` struct**

Run: `Grep DocumentsListRequest src/tauri_api/documents/dto.rs`
Note the line range and existing fields.

- [ ] **Step 2: Extend the struct**

Replace it with (preserving any existing serde attributes — assume `#[serde(rename_all = "camelCase", default)]` on the struct):
```rust
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DocumentsListRequest {
    pub query: Option<String>,
    pub direction: Option<DocumentDirection>,
    pub kind: Option<String>,
    pub counterparty_id: Option<String>,
    pub date_from: Option<chrono::NaiveDate>,
    pub date_to: Option<chrono::NaiveDate>,
    pub statuses: Option<Vec<String>>,
    pub amount_min: Option<rust_decimal::Decimal>,
    pub amount_max: Option<rust_decimal::Decimal>,
    pub overdue_only: Option<bool>,
}
```

If the existing struct has additional fields not listed here, keep them — only add the new ones.

- [ ] **Step 3: Update `documents_list` function to wire new fields**

In `src/tauri_api/documents/api.rs:774-820` replace the body of the function so the three `db::*::list_filtered(...)` calls receive the new arguments:

```rust
pub async fn documents_list(
    ctx: &AppCtx,
    request: DocumentsListRequest,
) -> Result<DocumentsListDto> {
    let company_id = ctx.company_id();
    let search = request.query.as_deref();
    let direction_filter = request.direction;
    let counterparty_filter = request
        .counterparty_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            Uuid::parse_str(value)
                .with_context(|| format!("Некоректний фільтр контрагента: {value}"))
        })
        .transpose()?;

    let statuses_owned: Option<Vec<String>> = request
        .statuses
        .filter(|v| !v.is_empty());
    let statuses_slice: Option<&[String]> = statuses_owned.as_deref();

    let amount_min = request.amount_min;
    let amount_max = request.amount_max;
    let overdue_only = request.overdue_only.unwrap_or(false);
    let today = chrono::Utc::now().date_naive();

    let include_acts = request.kind.as_deref().map_or(true, |k| k == "act");
    let include_invoices = request.kind.as_deref().map_or(true, |k| k == "invoice");
    // overdue не має сенсу для waybill — пропускаємо запит до БД.
    let include_waybills =
        request.kind.as_deref().map_or(true, |k| k == "waybill") && !overdue_only;

    let (acts, invoices, waybills) = tokio::join!(
        async {
            if include_acts {
                db::acts::list_filtered(
                    ctx.pool(), company_id,
                    statuses_slice, direction_filter, search, counterparty_filter,
                    request.date_from, request.date_to,
                    amount_min, amount_max, overdue_only, today,
                ).await
            } else {
                Ok(vec![])
            }
        },
        async {
            if include_invoices {
                db::invoices::list_filtered(
                    ctx.pool(), company_id,
                    statuses_slice, direction_filter, search, counterparty_filter,
                    request.date_from, request.date_to,
                    amount_min, amount_max, overdue_only, today,
                ).await
            } else {
                Ok(vec![])
            }
        },
        async {
            if include_waybills {
                db::waybills::list_filtered(
                    ctx.pool(), company_id,
                    statuses_slice, direction_filter, search, counterparty_filter,
                    request.date_from, request.date_to,
                    amount_min, amount_max,
                ).await
            } else {
                Ok(vec![])
            }
        },
    );

    // ... ↓ keep the existing combined-vec assembly verbatim from line ~821 onwards ...
}
```

The "keep verbatim" part is the loop that builds `combined: Vec<(NaiveDate, DocumentItemDto)>` and the final `Ok(DocumentsListDto { ... })` — do not modify it.

- [ ] **Step 4: Compile**

Run: `cargo build --tests`
Expected: clean. If `Decimal`/`NaiveDate` aren't imported in `api.rs`, they already are (line 6: `use chrono::{NaiveDate, Utc};`, line 7: `use rust_decimal::Decimal;`). For dto.rs, add `use chrono;` and `use rust_decimal;` at top if missing.

- [ ] **Step 5: Add vertical-slice test `tests/tauri_vertical_slice/documents.rs`**

Append a test that exercises the full filter combination:
```rust
#[sqlx::test(migrations = "./migrations")]
async fn documents_list_combined_filters(pool: sqlx::PgPool) -> anyhow::Result<()> {
    use rust_decimal_macros::dec;

    let ctx = test_ctx(pool).await?;
    let cp = seed_counterparty_in_ctx(&ctx, "VS-CP").await?;
    seed_invoice_in_ctx(&ctx, &cp, "VS-INV-LO", dec!(500), chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()).await?;
    let target = seed_invoice_in_ctx(&ctx, &cp, "VS-INV-HIT", dec!(5000), chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()).await?;
    db::invoices::change_status(ctx.pool(), target.id, models::InvoiceStatus::Issued).await?;
    seed_invoice_in_ctx(&ctx, &cp, "VS-INV-HI", dec!(50000), chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()).await?;

    let request = DocumentsListRequest {
        query: None,
        direction: None,
        kind: Some("invoice".into()),
        counterparty_id: Some(cp.id.to_string()),
        date_from: Some(chrono::NaiveDate::from_ymd_opt(2026, 4, 15).unwrap()),
        date_to: Some(chrono::NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
        statuses: Some(vec!["issued".into()]),
        amount_min: Some(dec!(1000)),
        amount_max: Some(dec!(10000)),
        overdue_only: Some(false),
    };

    let result = documents_list(&ctx, request).await?;
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].number, "VS-INV-HIT");
    Ok(())
}
```

(Helpers `test_ctx`, `seed_counterparty_in_ctx`, `seed_invoice_in_ctx` should already exist in `tests/tauri_vertical_slice/`. If named differently, adapt to the local convention — grep `seed_invoice` in the directory.)

- [ ] **Step 6: Run vertical slice**

Run: `TEST_DATABASE_URL=$TEST_DATABASE_URL cargo test --test tauri_vertical_slice -- documents_list_combined_filters`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/tauri_api/documents/dto.rs src/tauri_api/documents/api.rs \
        tests/tauri_vertical_slice/documents.rs
git commit -m "feat(documents): wire date/status/amount/overdue filters through Tauri DTO"
```

- [ ] **Step 8: Run `cargo sqlx prepare` as verification**

Run: `cargo sqlx prepare`
Expected: usually no diff (we used `QueryBuilder`, not `query!` macros). If diff appears, `git add .sqlx && git commit -m "chore(sqlx): refresh prepared metadata"`.

---

## Phase 4 — Frontend types and API surface

### Task 8: Add `DocumentStatus` type and extend the request DTO

**Files:**
- Modify: `frontend/src/lib/types.ts`

- [ ] **Step 1: Locate `DocumentsListDto` and any existing list-request types**

Run: `Grep "DocumentsList\|DocumentKind\|DocumentDirection" frontend/src/lib/types.ts`

- [ ] **Step 2: Add `DocumentStatus`**

Append to the file (near other document-related types):
```ts
export type DocumentStatus =
  | "draft"
  | "issued"
  | "signed"
  | "paid"
  | "delivered";

export interface DocumentsListRequest {
  direction?: "outgoing" | "incoming";
  kind?: string;
  counterpartyId?: string;
  dateFrom?: string;        // "YYYY-MM-DD"
  dateTo?: string;          // "YYYY-MM-DD"
  statuses?: DocumentStatus[];
  amountMin?: string;       // major-units decimal string ("1000.00")
  amountMax?: string;
  overdueOnly?: boolean;
}
```

Do NOT include `query` — frontend no longer sends it.

- [ ] **Step 3: Type-check**

Run: `cd frontend && npm run check`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/types.ts
git commit -m "feat(frontend): add DocumentStatus and DocumentsListRequest types"
```

### Task 9: Refactor `documentsList` to single-object argument

**Files:**
- Modify: `frontend/src/lib/api.ts:92-106`
- Modify: every caller of `documentsList` (will be just the store after Phase 5)

- [ ] **Step 1: Replace the function**

In `frontend/src/lib/api.ts:92-106`:
```ts
import type { DocumentsListDto, DocumentsListRequest } from "./types";

export function documentsList(
  request: DocumentsListRequest = {}
): Promise<DocumentsListDto> {
  return appInvoke("documents_list", {
    request: {
      query: null,
      direction: request.direction ?? null,
      kind: request.kind ?? null,
      counterpartyId: request.counterpartyId ?? null,
      dateFrom: request.dateFrom ?? null,
      dateTo: request.dateTo ?? null,
      statuses: request.statuses && request.statuses.length > 0 ? request.statuses : null,
      amountMin: request.amountMin ?? null,
      amountMax: request.amountMax ?? null,
      overdueOnly: request.overdueOnly ?? false,
    }
  });
}
```

- [ ] **Step 2: Find all callers**

Run: `Grep "documentsList\(" frontend/src`
Expected: only `frontend/src/lib/stores/documents.ts:85` and possibly `browser-fixtures.ts`.

- [ ] **Step 3: Patch caller in `frontend/src/lib/stores/documents.ts:84-91`**

Replace:
```ts
async function reloadList(state: DocumentsState): Promise<DocumentsListDto> {
  return documentsList(
    state.query,
    tabToDirection(state.activeTab),
    state.kindFilter ?? undefined,
    state.counterpartyFilterId ?? undefined
  );
}
```

with (provisional — Task 11 will replace state shape, but this keeps it compiling now):
```ts
async function reloadList(state: DocumentsState): Promise<DocumentsListDto> {
  return documentsList({
    direction: tabToDirection(state.activeTab),
    kind: state.kindFilter ?? undefined,
    counterpartyId: state.counterpartyFilterId ?? undefined,
  });
}
```

- [ ] **Step 4: Type-check**

Run: `cd frontend && npm run check`
Expected: clean.

- [ ] **Step 5: Run frontend tests to confirm nothing broke yet**

Run: `cd frontend && npm run test:frontend`
Expected: most pass; one or two screen tests may still reference `mocks.load` — they'll be fixed in Task 13.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/api.ts frontend/src/lib/stores/documents.ts
git commit -m "refactor(frontend): documentsList accepts single-object request"
```

---

## Phase 5 — Frontend store

### Task 10: Extend `DocumentsState` with new filter fields and remove `query`

**Files:**
- Modify: `frontend/src/lib/stores/documents.ts:31-63` (state shape + `initialState`), `:95-117` (load), `:204-207` (`onDocumentSearch`-related — actually in `DocumentsScreen.svelte`, but `state.query` here)
- Test: `frontend/src/lib/stores/__tests__/documents.test.ts` (new file)

- [ ] **Step 1: Create the new test file**

Path: `frontend/src/lib/stores/__tests__/documents.test.ts`. Start with the canonical preamble (mirror `payments.test.ts` or any existing store test):
```ts
/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", () => {
  return {
    documentsList: vi.fn(),
    documentOpen: vi.fn(),
    documentChainGet: vi.fn(),
    documentCreateDraft: vi.fn(),
    documentSave: vi.fn(),
    documentDelete: vi.fn(),
    documentAdvanceStatus: vi.fn(),
    documentChainCreateDraft: vi.fn(),
    documentGeneratePdf: vi.fn(),
    documentPdfApplyTextReplace: vi.fn(),
    documentPdfAttachExisting: vi.fn(),
    documentPdfOpenCurrent: vi.fn(),
    documentsBulkDelete: vi.fn(),
    documentsBulkAdvanceStatus: vi.fn(),
  };
});

import * as api from "../../api";
import { documentsStore } from "../documents";

const documentsListMock = api.documentsList as ReturnType<typeof vi.fn>;

const emptyList = {
  items: [],
  invoiceItems: [],
  actItems: [],
  waybillItems: [],
  totalCount: 0,
  pageCount: 0
};

beforeEach(() => {
  documentsListMock.mockReset();
  documentsListMock.mockResolvedValue(emptyList);
  // documentsStore is a singleton — manually reset its filters between tests
  documentsStore.clearAllFilters();
});
```

- [ ] **Step 2: Write the failing test for filter shape**

Append to the test file:
```ts
describe("documentsStore filter state", () => {
  it("starts without filters and without a search query field", async () => {
    let snapshot: any;
    const unsub = documentsStore.subscribe((state) => { snapshot = state; });
    expect(snapshot.dateFrom).toBeNull();
    expect(snapshot.dateTo).toBeNull();
    expect(snapshot.statusFilter).toEqual([]);
    expect(snapshot.amountMin).toBeNull();
    expect(snapshot.amountMax).toBeNull();
    expect(snapshot.overdueOnly).toBe(false);
    expect(snapshot.activePresetId).toBeNull();
    expect("query" in snapshot).toBe(false);
    unsub();
  });
});
```

- [ ] **Step 3: Run — expect FAIL**

Run: `cd frontend && npx vitest run --config vitest.config.mjs -t "starts without filters"`
Expected: FAIL (current state still has `query: ""`, lacks new fields).

- [ ] **Step 4: Update `DocumentsState` in `frontend/src/lib/stores/documents.ts:31-46`**

Replace the interface:
```ts
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
  activeTab: "all" | "outgoing" | "incoming";
  kindFilter: DocumentKind | null;
  counterpartyFilterId: string | null;
  dateFrom: string | null;
  dateTo: string | null;
  statusFilter: string[];
  amountMin: string | null;
  amountMax: string | null;
  overdueOnly: boolean;
  activePresetId: string | null;
}
```

Update `initialState` (around line 48):
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
  activeTab: "all",
  kindFilter: null,
  counterpartyFilterId: null,
  dateFrom: null,
  dateTo: null,
  statusFilter: [],
  amountMin: null,
  amountMax: null,
  overdueOnly: false,
  activePresetId: null,
};
```

- [ ] **Step 5: Update `reloadList` to send all filters**

Replace the body in `frontend/src/lib/stores/documents.ts:84-91`:
```ts
async function reloadList(state: DocumentsState): Promise<DocumentsListDto> {
  return documentsList({
    direction: tabToDirection(state.activeTab),
    kind: state.kindFilter ?? undefined,
    counterpartyId: state.counterpartyFilterId ?? undefined,
    dateFrom: state.dateFrom ?? undefined,
    dateTo: state.dateTo ?? undefined,
    statuses: state.statusFilter.length > 0 ? (state.statusFilter as any) : undefined,
    amountMin: state.amountMin ?? undefined,
    amountMax: state.amountMax ?? undefined,
    overdueOnly: state.overdueOnly || undefined,
  });
}
```

- [ ] **Step 6: Remove `state.query` from `load(...)` and signature**

Replace `load` method (lines ~95-117):
```ts
async load() {
  update((state) => ({
    ...state,
    loading: true,
    error: null,
  }));

  try {
    const snap = get({ subscribe });
    const list = await reloadList(snap);
    update((state) => ({
      ...state,
      list,
      selectedIds: state.selectedIds.filter((id) => list.items.some((item) => item.id === id)),
      initialLoading: false,
      loading: false
    }));
  } catch (error) {
    update((state) => ({ ...state, loading: false, error: String(error) }));
  }
},
```

(Removed `query` parameter entirely; UI no longer calls `load(query)` — Task 13 removes the input.)

- [ ] **Step 7: Type-check + run the test**

Run: `cd frontend && npm run check && npx vitest run --config vitest.config.mjs -t "starts without filters"`
Expected: PASS.

If `clearAllFilters` doesn't exist yet (Task 11 adds it), comment out that line in `beforeEach` for now and uncomment in Task 11.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/lib/stores/documents.ts frontend/src/lib/stores/__tests__/documents.test.ts
git commit -m "feat(documents store): extend state with date/status/amount/overdue filters"
```

### Task 11: Add filter setters with `activePresetId` auto-clear

**Files:**
- Modify: `frontend/src/lib/stores/documents.ts`
- Test: `frontend/src/lib/stores/__tests__/documents.test.ts`

- [ ] **Step 1: Write failing test for `setDateRange` clearing the preset**

Append to test file:
```ts
describe("filter setters clear active preset", () => {
  it("setDateRange clears activePresetId", async () => {
    documentsStore.applyPreset("this-month");
    let s: any;
    const unsub = documentsStore.subscribe((state) => { s = state; });
    expect(s.activePresetId).toBe("this-month");

    await documentsStore.setDateRange("2026-04-01", "2026-04-30");
    expect(s.activePresetId).toBeNull();
    expect(s.dateFrom).toBe("2026-04-01");
    unsub();
  });

  it("setStatusFilter clears activePresetId", async () => {
    documentsStore.applyPreset("drafts");
    let s: any;
    const unsub = documentsStore.subscribe((state) => { s = state; });
    expect(s.activePresetId).toBe("drafts");

    await documentsStore.setStatusFilter(["issued"]);
    expect(s.activePresetId).toBeNull();
    unsub();
  });

  it("setAmountRange clears activePresetId", async () => {
    documentsStore.applyPreset("unpaid");
    await documentsStore.setAmountRange("100", "5000");
    let s: any;
    const unsub = documentsStore.subscribe((state) => { s = state; });
    expect(s.activePresetId).toBeNull();
    unsub();
  });
});
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cd frontend && npx vitest run --config vitest.config.mjs -t "filter setters clear active preset"`
Expected: FAIL — `applyPreset` and `setDateRange/Status/Amount` don't exist.

- [ ] **Step 3: Add the setters in `frontend/src/lib/stores/documents.ts`**

Append inside the returned object (alongside existing methods like `setKindFilter`):

```ts
setDateRange(from: string | null, to: string | null) {
  update((state) => ({
    ...state,
    dateFrom: from,
    dateTo: to,
    activePresetId: null,
    loading: true,
    error: null,
  }));
  const seq = ++filterSeq;
  const snap = get({ subscribe });
  reloadList(snap).then((list) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, list, loading: false }));
  }).catch((error) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, loading: false, error: String(error) }));
  });
},

setStatusFilter(statuses: string[]) {
  update((state) => ({
    ...state,
    statusFilter: statuses,
    activePresetId: null,
    loading: true,
    error: null,
  }));
  const seq = ++filterSeq;
  const snap = get({ subscribe });
  reloadList(snap).then((list) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, list, loading: false }));
  }).catch((error) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, loading: false, error: String(error) }));
  });
},

setAmountRange(min: string | null, max: string | null) {
  update((state) => ({
    ...state,
    amountMin: min,
    amountMax: max,
    activePresetId: null,
    loading: true,
    error: null,
  }));
  const seq = ++filterSeq;
  const snap = get({ subscribe });
  reloadList(snap).then((list) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, list, loading: false }));
  }).catch((error) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, loading: false, error: String(error) }));
  });
},
```

Also patch existing `setCounterpartyFilter` to clear `activePresetId`:
```ts
setCounterpartyFilter(counterpartyId: string | null) {
  update((state) => ({
    ...state,
    counterpartyFilterId: counterpartyId,
    activePresetId: null,                        // ← NEW LINE
    loading: true,
    error: null
  }));
  // ... rest unchanged
},
```

- [ ] **Step 4: Run the failing tests — they still fail because `applyPreset` is missing**

Run: `cd frontend && npx vitest run --config vitest.config.mjs -t "filter setters clear active preset"`
Expected: FAIL — `applyPreset` is undefined.

- [ ] **Step 5: Commit progress (setters in place; preset action follows)**

Skip commit — bundle with Task 12. Continue.

### Task 12: Add `applyPreset`, `applyFilters`, `clearAllFilters`

**Files:**
- Modify: `frontend/src/lib/stores/documents.ts`
- Test: `frontend/src/lib/stores/__tests__/documents.test.ts`

- [ ] **Step 1: Write failing test for `applyPreset("unpaid")`**

Append:
```ts
describe("applyPreset", () => {
  it("applyPreset('unpaid') sets statuses to ['issued','signed'] and reloads list once", async () => {
    documentsListMock.mockClear();
    await documentsStore.applyPreset("unpaid");

    let s: any;
    const unsub = documentsStore.subscribe((state) => { s = state; });
    expect(s.statusFilter).toEqual(["issued", "signed"]);
    expect(s.activePresetId).toBe("unpaid");
    expect(s.overdueOnly).toBe(false);
    expect(documentsListMock).toHaveBeenCalledTimes(1);
    unsub();
  });

  it("applyPreset('overdue') sets overdueOnly=true and clears statusFilter", async () => {
    await documentsStore.applyPreset("overdue");
    let s: any;
    const unsub = documentsStore.subscribe((state) => { s = state; });
    expect(s.overdueOnly).toBe(true);
    expect(s.statusFilter).toEqual([]);
    expect(s.activePresetId).toBe("overdue");
    unsub();
  });

  it("applyPreset('all') resets all filter fields", async () => {
    await documentsStore.setDateRange("2026-04-01", "2026-04-30");
    await documentsStore.setStatusFilter(["draft"]);
    await documentsStore.applyPreset("all");
    let s: any;
    const unsub = documentsStore.subscribe((state) => { s = state; });
    expect(s.dateFrom).toBeNull();
    expect(s.dateTo).toBeNull();
    expect(s.statusFilter).toEqual([]);
    expect(s.activePresetId).toBe("all");
    unsub();
  });
});

describe("applyFilters", () => {
  it("applyFilters batches all draft fields and reloads once", async () => {
    documentsListMock.mockClear();
    await documentsStore.applyFilters({
      dateFrom: "2026-04-01",
      dateTo: "2026-05-01",
      statusFilter: ["issued"],
      amountMin: "100",
      amountMax: "5000",
      counterpartyFilterId: "cp-1",
    });

    expect(documentsListMock).toHaveBeenCalledTimes(1);
    const args = documentsListMock.mock.calls[0][0];
    expect(args.dateFrom).toBe("2026-04-01");
    expect(args.statuses).toEqual(["issued"]);
    expect(args.amountMin).toBe("100");
    expect(args.counterpartyId).toBe("cp-1");
  });
});

describe("clearAllFilters", () => {
  it("resets every filter field including activePresetId and overdueOnly", async () => {
    await documentsStore.applyPreset("overdue");
    await documentsStore.setAmountRange("100", "1000");
    documentsStore.clearAllFilters();
    let s: any;
    const unsub = documentsStore.subscribe((state) => { s = state; });
    expect(s.dateFrom).toBeNull();
    expect(s.dateTo).toBeNull();
    expect(s.statusFilter).toEqual([]);
    expect(s.amountMin).toBeNull();
    expect(s.amountMax).toBeNull();
    expect(s.counterpartyFilterId).toBeNull();
    expect(s.overdueOnly).toBe(false);
    expect(s.activePresetId).toBeNull();
    unsub();
  });
});
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cd frontend && npx vitest run --config vitest.config.mjs --testPathPattern documents.test`
Expected: FAILs for `applyPreset`, `applyFilters`, `clearAllFilters` (undefined).

- [ ] **Step 3: Add `applyPreset` (uses preset definitions from `ui.ts` — Task 14 will define them; for now inline a minimal map)**

In `documents.ts`, near the top of the factory, add:
```ts
import { DOCUMENT_FILTER_PRESETS } from "../config/ui";
```

(If `ui.ts` doesn't yet export it, the import will type-fail. Stash this task and run Task 14 first if needed; but the plan order assumes Task 14 happens before Task 12 in execution. Reorder if executing-plans flags it.)

Add inside the factory:
```ts
async applyPreset(presetId: string) {
  const today = new Date();
  const preset = DOCUMENT_FILTER_PRESETS.find((p) => p.id === presetId);
  if (!preset) return;
  const draft = preset.build(today);

  update((state) => ({
    ...state,
    dateFrom: draft.dateFrom,
    dateTo: draft.dateTo,
    statusFilter: draft.statusFilter,
    amountMin: draft.amountMin,
    amountMax: draft.amountMax,
    overdueOnly: draft.overdueOnly,
    activePresetId: presetId,
    loading: true,
    error: null,
  }));
  const seq = ++filterSeq;
  const snap = get({ subscribe });
  return reloadList(snap).then((list) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, list, loading: false }));
  }).catch((error) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, loading: false, error: String(error) }));
  });
},

async applyFilters(draft: {
  dateFrom: string | null;
  dateTo: string | null;
  statusFilter: string[];
  amountMin: string | null;
  amountMax: string | null;
  counterpartyFilterId: string | null;
}) {
  update((state) => ({
    ...state,
    dateFrom: draft.dateFrom,
    dateTo: draft.dateTo,
    statusFilter: draft.statusFilter,
    amountMin: draft.amountMin,
    amountMax: draft.amountMax,
    counterpartyFilterId: draft.counterpartyFilterId,
    activePresetId: null,
    loading: true,
    error: null,
  }));
  const seq = ++filterSeq;
  const snap = get({ subscribe });
  return reloadList(snap).then((list) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, list, loading: false }));
  }).catch((error) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, loading: false, error: String(error) }));
  });
},

clearAllFilters() {
  update((state) => ({
    ...state,
    dateFrom: null,
    dateTo: null,
    statusFilter: [],
    amountMin: null,
    amountMax: null,
    counterpartyFilterId: null,
    overdueOnly: false,
    activePresetId: null,
    loading: true,
    error: null,
  }));
  const seq = ++filterSeq;
  const snap = get({ subscribe });
  reloadList(snap).then((list) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, list, loading: false }));
  }).catch((error) => {
    if (seq !== filterSeq) return;
    update((state) => ({ ...state, loading: false, error: String(error) }));
  });
},
```

- [ ] **Step 4: Run all store tests — expect PASS**

Run: `cd frontend && npx vitest run --config vitest.config.mjs --testPathPattern documents.test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/stores/documents.ts frontend/src/lib/stores/__tests__/documents.test.ts
git commit -m "feat(documents store): add applyPreset, applyFilters, clearAllFilters with preset auto-clear"
```

---

## Phase 6 — UI configuration (presets, copy, status options)

### Task 13: Add presets, status options, copy to `ui.ts`

**Files:**
- Modify: `frontend/src/lib/config/ui.ts`

- [ ] **Step 1: Append `DOCUMENT_STATUS_OPTIONS`**

Find the existing `DOCUMENT_KIND_FILTER_OPTIONS` block in `ui.ts` and append nearby:
```ts
import type { DocumentStatus } from "../types";

export const DOCUMENT_STATUS_OPTIONS: Array<{ value: DocumentStatus; label: string }> = [
  { value: "draft",     label: "Чернетка" },
  { value: "issued",    label: "Виставлено" },
  { value: "signed",    label: "Підписано" },
  { value: "paid",      label: "Оплачено" },
  { value: "delivered", label: "Доставлено" },
];
```

- [ ] **Step 2: Append `DOCUMENT_FILTER_PRESETS`**

```ts
export interface DocumentFilterPresetSnapshot {
  dateFrom: string | null;
  dateTo: string | null;
  statusFilter: string[];
  amountMin: string | null;
  amountMax: string | null;
  overdueOnly: boolean;
}

export interface DocumentFilterPreset {
  id: string;
  label: string;
  build(today: Date): DocumentFilterPresetSnapshot;
}

function isoDate(d: Date): string {
  return d.toISOString().slice(0, 10);
}

const empty = (): DocumentFilterPresetSnapshot => ({
  dateFrom: null, dateTo: null,
  statusFilter: [],
  amountMin: null, amountMax: null,
  overdueOnly: false,
});

export const DOCUMENT_FILTER_PRESETS: DocumentFilterPreset[] = [
  { id: "all",        label: "Усі",          build: () => empty() },
  { id: "drafts",     label: "Чернетки",     build: () => ({ ...empty(), statusFilter: ["draft"] }) },
  { id: "unpaid",     label: "Неоплачені",   build: () => ({ ...empty(), statusFilter: ["issued", "signed"] }) },
  { id: "overdue",    label: "Прострочені",  build: () => ({ ...empty(), overdueOnly: true }) },
  { id: "this-month", label: "Цього місяця", build: (today) => ({
      ...empty(),
      dateFrom: isoDate(new Date(today.getFullYear(), today.getMonth(), 1)),
      dateTo: isoDate(today),
  }) },
];
```

- [ ] **Step 3: Append `DOCUMENTS_FILTER_COPY`**

```ts
export const DOCUMENTS_FILTER_COPY = {
  filterButton: "Фільтр",
  filterButtonWithCount: (n: number) => `Фільтр · ${n}`,
  clearAll: "Очистити",
  apply: "Застосувати",
  reset: "Скинути",
  activeFiltersLabel: "Активні:",
  presetsLabel: "Швидкі:",
  periodLabel: "Період",
  periodFrom: "Від",
  periodTo: "До",
  periodSubpresets: { today: "Сьогодні", week: "Тиждень", month: "Місяць", quarter: "Квартал", year: "Рік" },
  statusLabel: "Статус",
  counterpartyLabel: "Контрагент",
  counterpartyAll: "Усі контрагенти",
  amountLabel: "Сума, грн",
  amountFrom: "Від",
  amountTo: "До",
  errors: {
    dateRangeInvalid: "Кінцева дата раніше за початкову",
    amountRangeInvalid: "Максимальна сума менша за мінімальну",
    amountInvalidFormat: "Некоректна сума",
  },
};
```

- [ ] **Step 4: Type-check**

Run: `cd frontend && npm run check`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/config/ui.ts
git commit -m "feat(ui config): add document filter presets, status options, copy"
```

---

## Phase 7 — Screen UI

### Task 14: Remove "Пошук документів" input + handler + state references

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte:204-207, 371-390`
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts:31-45, 211-247, 466-489, 543-557, 627-649`

- [ ] **Step 1: Remove the input from the template**

In `DocumentsScreen.svelte:371-390`, replace the entire `<div class="documents-filter-toolbar">...</div>` block with:
```svelte
<div class="documents-filter-toolbar">
  <button
    class="btn-secondary"
    data-testid="documents-filter-button"
    type="button"
    aria-expanded={filtersOpen}
    on:click={toggleFilters}
    disabled={$documents.loading}
  >
    <span>{filterButtonLabel}</span>
  </button>
  {#if activeFilterCount > 0}
    <button
      class="btn-ghost"
      type="button"
      data-testid="documents-clear-filters"
      on:click={onClearAllFilters}
      disabled={$documents.loading}
    >
      {DOCUMENTS_FILTER_COPY.clearAll}
    </button>
  {/if}
</div>
```

(`filterButtonLabel`, `activeFilterCount`, `onClearAllFilters` come in Task 16. For now this leaves a small `Cannot find name` — we accept this single-step regression and fix in Task 16.)

- [ ] **Step 2: Remove `onDocumentSearch` function (lines ~204-207)**

Delete entirely:
```ts
function onDocumentSearch(event: Event) {
  const input = event.currentTarget as HTMLInputElement;
  void documents.load(input.value);
}
```

- [ ] **Step 3: Remove `placeholder="Пошук документів"` test assertion**

In `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`, find the test `routes create, search and editor actions into the documents store` (line ~466). Rename it to `routes create and editor actions into the documents store` and remove the search assertions:
```ts
it("routes create and editor actions into the documents store", async () => {
  const { component, target } = renderDocuments();

  (target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).click();
  buttonByText(target, "Додати позицію").click();
  buttonByText(target, "Зберегти").click();
  buttonByText(target, "Відкрити PDF").click();
  (target.querySelector('[data-testid="documents-chain-create-act"]') as HTMLButtonElement).click();
  await tick();

  expect(mocks.create).toHaveBeenCalledWith("counterparty-1", "act");
  expect(mocks.addItem).toHaveBeenCalled();
  expect(mocks.save).toHaveBeenCalled();
  expect(mocks.openCurrentPdf).toHaveBeenCalled();
  expect(mocks.createChainDraft).toHaveBeenCalledWith("act");

  component.$destroy();
});
```

Remove `mocks.load` from the `mocks` hoisted object (line ~70) and from the `vi.mock("../../stores/documents", ...)` block (line ~100), and from the `mockReset` loop (line ~286).

- [ ] **Step 4: Remove `query: ""` from every state object in tests**

Find all 5 occurrences of `query: ""` (in `setDocumentsState`, `setDocumentsStateWithoutDraftContext`, the empty-list test, the unreadable-PDF test, and the skeleton test) and delete those lines.

- [ ] **Step 5: Type-check + run** (will fail at this step until Task 16 wires the new variables)

Skip running the screen test now; commit and proceed to Task 15.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte \
        frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
git commit -m "refactor(documents): remove standalone search input"
```

### Task 15: Add presets row above the toolbar

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`

- [ ] **Step 1: Import preset config**

Add to the import block at the top of the `<script>`:
```ts
import {
  DOCUMENT_FILTER_PRESETS,
  DOCUMENT_STATUS_OPTIONS,
  DOCUMENTS_FILTER_COPY,
} from "../config/ui";
```

- [ ] **Step 2: Add preset-click handler**

After existing `function toggleFilters()`:
```ts
function onPresetClick(presetId: string) {
  void documents.applyPreset(presetId);
}
```

- [ ] **Step 3: Insert the presets row above `documents-filter-toolbar`**

Above the `documents-filter-toolbar` div (around line 371):
```svelte
<div class="documents-presets-row" role="group" aria-label="Швидкі фільтри">
  <span class="documents-presets-label">{DOCUMENTS_FILTER_COPY.presetsLabel}</span>
  {#each DOCUMENT_FILTER_PRESETS as preset}
    <button
      type="button"
      class="kind-chip"
      class:kind-chip-active={$documents.activePresetId === preset.id}
      data-testid={`documents-preset-${preset.id}`}
      on:click={() => onPresetClick(preset.id)}
      disabled={$documents.loading}
    >
      {preset.label}
    </button>
  {/each}
</div>
```

- [ ] **Step 4: Compile-check**

Run: `cd frontend && npm run check`
Expected: still failing on `filterButtonLabel`/`activeFilterCount`/`onClearAllFilters` from Task 14. Fixed in Task 16.

Skip commit — bundle with Task 16.

### Task 16: Active filter chips, counter badge, clear-all wiring

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`

- [ ] **Step 1: Add reactive computed `activeFilterCount` and `filterButtonLabel`**

Add after the existing `$:` blocks:
```ts
$: activeFilterCount = (() => {
  const s = $documents;
  let n = 0;
  if (s.dateFrom || s.dateTo) n++;
  if (s.statusFilter.length > 0) n++;
  if (s.counterpartyFilterId) n++;
  if (s.amountMin || s.amountMax) n++;
  if (s.overdueOnly) n++;
  return n;
})();

$: filterButtonLabel = activeFilterCount > 0
  ? DOCUMENTS_FILTER_COPY.filterButtonWithCount(activeFilterCount)
  : DOCUMENTS_FILTER_COPY.filterButton;

function statusLabelOf(code: string): string {
  return DOCUMENT_STATUS_OPTIONS.find((o) => o.value === code)?.label ?? code;
}

function formatPeriodChip(from: string | null, to: string | null): string {
  if (from && to) return `${DOCUMENTS_FILTER_COPY.periodLabel}: ${from} – ${to}`;
  if (from)      return `${DOCUMENTS_FILTER_COPY.periodLabel}: від ${from}`;
  if (to)        return `${DOCUMENTS_FILTER_COPY.periodLabel}: до ${to}`;
  return DOCUMENTS_FILTER_COPY.periodLabel;
}

function onClearAllFilters() {
  documents.clearAllFilters();
}

function onRemovePeriodChip()       { documents.setDateRange(null, null); }
function onRemoveStatusChip()       { documents.setStatusFilter([]); }
function onRemoveCounterpartyChip() { documents.setCounterpartyFilter(null); }
function onRemoveAmountChip()       { documents.setAmountRange(null, null); }
function onRemoveOverdueChip()      { documents.applyPreset("all"); }
```

- [ ] **Step 2: Insert active-chips row between toolbar and create-bar**

After the `{#if filtersOpen}` panel block (around line 417, before `documents-create-bar`):
```svelte
{#if activeFilterCount > 0}
  <div class="documents-active-filters" data-testid="documents-active-filters">
    <span class="documents-active-label">{DOCUMENTS_FILTER_COPY.activeFiltersLabel}</span>

    {#if $documents.dateFrom || $documents.dateTo}
      <button class="active-chip" type="button" on:click={onRemovePeriodChip} aria-label="Прибрати фільтр період">
        <span>{formatPeriodChip($documents.dateFrom, $documents.dateTo)}</span>
        <span aria-hidden="true">×</span>
      </button>
    {/if}

    {#if $documents.statusFilter.length > 0}
      <button class="active-chip" type="button" on:click={onRemoveStatusChip} aria-label="Прибрати фільтр статус">
        <span>{DOCUMENTS_FILTER_COPY.statusLabel}: {$documents.statusFilter.map(statusLabelOf).join(", ")}</span>
        <span aria-hidden="true">×</span>
      </button>
    {/if}

    {#if $documents.counterpartyFilterId}
      <button class="active-chip" type="button" on:click={onRemoveCounterpartyChip} aria-label="Прибрати фільтр контрагент">
        <span>{DOCUMENTS_FILTER_COPY.counterpartyLabel}: {
          ($counterparties.screen?.items ?? []).find((c) => c.id === $documents.counterpartyFilterId)?.name ?? ""
        }</span>
        <span aria-hidden="true">×</span>
      </button>
    {/if}

    {#if $documents.amountMin || $documents.amountMax}
      <button class="active-chip" type="button" on:click={onRemoveAmountChip} aria-label="Прибрати фільтр сума">
        <span>{DOCUMENTS_FILTER_COPY.amountLabel}: {$documents.amountMin ?? "0"} – {$documents.amountMax ?? "∞"}</span>
        <span aria-hidden="true">×</span>
      </button>
    {/if}

    {#if $documents.overdueOnly}
      <button class="active-chip" type="button" on:click={onRemoveOverdueChip} aria-label="Прибрати фільтр прострочені">
        <span>Прострочені</span>
        <span aria-hidden="true">×</span>
      </button>
    {/if}
  </div>
{/if}
```

- [ ] **Step 3: Type-check**

Run: `cd frontend && npm run check`
Expected: clean (filter panel itself still has the old single-counterparty form — Task 17 will expand it).

- [ ] **Step 4: Run existing screen tests**

Run: `cd frontend && npx vitest run --config vitest.config.mjs --testPathPattern DocumentsScreen.test`
Expected: 1 failure remains (`keeps counterparty selection inside the document filters` — counterparty select still works as before). All others pass.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte
git commit -m "feat(documents): add presets row, active filter chips, counter badge"
```

### Task 17: Expand the filter panel with date, status, amount

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte:392-416` (the `{#if filtersOpen}` block)

- [ ] **Step 1: Replace the panel with the expanded layout**

Replace the entire `{#if filtersOpen}` block with:
```svelte
{#if filtersOpen}
  <div class="documents-filter-panel" data-testid="documents-filter-panel">
    <fieldset class="filter-panel-section">
      <legend>{DOCUMENTS_FILTER_COPY.periodLabel}</legend>
      <div class="filter-panel-subpresets">
        <button type="button" class="kind-chip" on:click={() => onDateSubpreset('today')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.today}</button>
        <button type="button" class="kind-chip" on:click={() => onDateSubpreset('week')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.week}</button>
        <button type="button" class="kind-chip" on:click={() => onDateSubpreset('month')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.month}</button>
        <button type="button" class="kind-chip" on:click={() => onDateSubpreset('quarter')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.quarter}</button>
        <button type="button" class="kind-chip" on:click={() => onDateSubpreset('year')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.year}</button>
      </div>
      <div class="filter-panel-grid-2">
        <label>{DOCUMENTS_FILTER_COPY.periodFrom}<input type="date" bind:value={panelDateFrom} /></label>
        <label>{DOCUMENTS_FILTER_COPY.periodTo}<input type="date" bind:value={panelDateTo} /></label>
      </div>
      {#if dateRangeError}
        <p class="filter-error" role="alert">{dateRangeError}</p>
      {/if}
    </fieldset>

    <fieldset class="filter-panel-section">
      <legend>{DOCUMENTS_FILTER_COPY.statusLabel}</legend>
      <div class="filter-panel-statuses">
        {#each DOCUMENT_STATUS_OPTIONS as opt}
          <label class="status-checkbox">
            <input type="checkbox" value={opt.value}
              checked={panelStatuses.includes(opt.value)}
              on:change={(e) => toggleStatus(opt.value, (e.currentTarget as HTMLInputElement).checked)} />
            {opt.label}
          </label>
        {/each}
      </div>
    </fieldset>

    <fieldset class="filter-panel-section">
      <legend>{DOCUMENTS_FILTER_COPY.counterpartyLabel}</legend>
      <select
        bind:value={filterCounterpartyId}
        disabled={$documents.loading}
        data-testid="documents-counterparty-filter"
        aria-label="Фільтр за контрагентом"
      >
        <option value="">{DOCUMENTS_FILTER_COPY.counterpartyAll}</option>
        {#each $counterparties.screen?.items ?? [] as cp}
          <option value={cp.id}>{cp.name}</option>
        {/each}
      </select>
    </fieldset>

    <fieldset class="filter-panel-section">
      <legend>{DOCUMENTS_FILTER_COPY.amountLabel}</legend>
      <div class="filter-panel-grid-2">
        <label>{DOCUMENTS_FILTER_COPY.amountFrom}<input type="text" inputmode="decimal" bind:value={panelAmountMin} placeholder="0,00" /></label>
        <label>{DOCUMENTS_FILTER_COPY.amountTo}<input type="text" inputmode="decimal" bind:value={panelAmountMax} placeholder="0,00" /></label>
      </div>
      {#if amountRangeError}
        <p class="filter-error" role="alert">{amountRangeError}</p>
      {/if}
    </fieldset>

    <div class="documents-filter-actions">
      <button class="btn-ghost" type="button" on:click={resetPanelDraft} disabled={$documents.loading}>
        {DOCUMENTS_FILTER_COPY.reset}
      </button>
      <button class="btn-primary" type="button" on:click={applyPanel} disabled={$documents.loading || !!dateRangeError || !!amountRangeError}>
        {DOCUMENTS_FILTER_COPY.apply}
      </button>
    </div>
  </div>
{/if}
```

- [ ] **Step 2: Add panel-draft state and helpers in the `<script>` block**

Just below `let filterCounterpartyId = "";` add:
```ts
let panelDateFrom: string = "";
let panelDateTo: string = "";
let panelStatuses: string[] = [];
let panelAmountMin: string = "";
let panelAmountMax: string = "";

$: if (filtersOpen) {
  // sync draft from current store state when panel opens
  panelDateFrom = $documents.dateFrom ?? "";
  panelDateTo = $documents.dateTo ?? "";
  panelStatuses = [...$documents.statusFilter];
  panelAmountMin = $documents.amountMin ?? "";
  panelAmountMax = $documents.amountMax ?? "";
}

$: dateRangeError = (panelDateFrom && panelDateTo && panelDateFrom > panelDateTo)
  ? DOCUMENTS_FILTER_COPY.errors.dateRangeInvalid
  : null;

$: amountRangeError = computeAmountError(panelAmountMin, panelAmountMax);

function computeAmountError(minStr: string, maxStr: string): string | null {
  const norm = (s: string) => s.trim().replace(/\s+/g, "").replace(",", ".");
  const minNum = minStr ? Number(norm(minStr)) : null;
  const maxNum = maxStr ? Number(norm(maxStr)) : null;
  if (minStr && (minNum === null || Number.isNaN(minNum))) return DOCUMENTS_FILTER_COPY.errors.amountInvalidFormat;
  if (maxStr && (maxNum === null || Number.isNaN(maxNum))) return DOCUMENTS_FILTER_COPY.errors.amountInvalidFormat;
  if (minNum !== null && maxNum !== null && minNum > maxNum) return DOCUMENTS_FILTER_COPY.errors.amountRangeInvalid;
  return null;
}

function toggleStatus(code: string, on: boolean) {
  panelStatuses = on
    ? Array.from(new Set([...panelStatuses, code]))
    : panelStatuses.filter((s) => s !== code);
}

function onDateSubpreset(kind: 'today' | 'week' | 'month' | 'quarter' | 'year') {
  const today = new Date();
  const iso = (d: Date) => d.toISOString().slice(0, 10);
  if (kind === 'today') {
    panelDateFrom = iso(today); panelDateTo = iso(today); return;
  }
  if (kind === 'week') {
    const start = new Date(today); start.setDate(today.getDate() - 6);
    panelDateFrom = iso(start); panelDateTo = iso(today); return;
  }
  if (kind === 'month') {
    panelDateFrom = iso(new Date(today.getFullYear(), today.getMonth(), 1));
    panelDateTo = iso(today); return;
  }
  if (kind === 'quarter') {
    const q = Math.floor(today.getMonth() / 3);
    panelDateFrom = iso(new Date(today.getFullYear(), q * 3, 1));
    panelDateTo = iso(today); return;
  }
  if (kind === 'year') {
    panelDateFrom = iso(new Date(today.getFullYear(), 0, 1));
    panelDateTo = iso(today); return;
  }
}

function resetPanelDraft() {
  panelDateFrom = "";
  panelDateTo = "";
  panelStatuses = [];
  panelAmountMin = "";
  panelAmountMax = "";
  filterCounterpartyId = "";
}

function normalizeAmount(s: string): string | null {
  const n = s.trim().replace(/\s+/g, "").replace(",", ".");
  return n.length === 0 ? null : n;
}

function applyPanel() {
  if (dateRangeError || amountRangeError) return;
  void documents.applyFilters({
    dateFrom: panelDateFrom || null,
    dateTo: panelDateTo || null,
    statusFilter: [...panelStatuses],
    amountMin: normalizeAmount(panelAmountMin),
    amountMax: normalizeAmount(panelAmountMax),
    counterpartyFilterId: filterCounterpartyId || null,
  });
  filtersOpen = false;
}
```

- [ ] **Step 3: Remove the now-obsolete `applyCounterpartyFilter` function**

Delete `applyCounterpartyFilter` and `resetDocumentFilters` (lines ~225-237). The new `applyPanel` and `documents.clearAllFilters` cover their roles.

- [ ] **Step 4: Type-check**

Run: `cd frontend && npm run check`
Expected: clean.

- [ ] **Step 5: Run frontend tests**

Run: `cd frontend && npm run test:frontend`
Expected: most pass; one residual test `keeps counterparty selection inside the document filters` likely passes (the counterparty select is still present and `setCounterpartyFilter` still exists).

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte
git commit -m "feat(documents): expand filter panel with date, status, amount controls"
```

---

## Phase 8 — Styles

### Task 18: Stylesheet additions for new UI elements

**Files:**
- Modify: `frontend/src/styles/documents.css`

- [ ] **Step 1: Append styles**

Append at the bottom of `documents.css`:
```css
.documents-presets-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
}

.documents-presets-label,
.documents-active-label {
  font-size: 12px;
  color: var(--acta-color-text-muted);
  margin-right: 4px;
}

.documents-active-filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  padding: 6px 16px;
  border-bottom: 1px solid var(--acta-color-border);
}

.active-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 8px;
  border-radius: var(--acta-radius-pill);
  border: 1px solid var(--acta-color-border);
  background: var(--acta-color-bg-subtle);
  font-size: 12px;
  cursor: pointer;
}

.active-chip:hover {
  border-color: var(--acta-color-accent);
}

.documents-filter-panel {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  padding: 16px;
  border: 1px solid var(--acta-color-border);
  border-radius: 6px;
  margin: 0 16px 12px;
}

.filter-panel-section {
  border: none;
  padding: 0;
}

.filter-panel-section legend {
  font-size: 12px;
  color: var(--acta-color-text-muted);
  padding: 0;
  margin-bottom: 6px;
}

.filter-panel-grid-2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.filter-panel-subpresets {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 8px;
}

.filter-panel-statuses {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
}

.status-checkbox {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  font-size: 13px;
}

.documents-filter-actions {
  grid-column: 1 / -1;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.filter-error {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--acta-color-danger);
}

@media (max-width: 980px) {
  .documents-filter-panel {
    grid-template-columns: 1fr;
  }
}
```

Also remove the legacy `.documents-list-search` and `.documents-list-search::placeholder` blocks (lines 23-34).

- [ ] **Step 2: Visual smoke**

Run: `cd src-tauri && cargo tauri dev`
(In another terminal if available; otherwise after this task.) Click Documents tab, click `Фільтр`, verify layout looks like the spec ASCII.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/styles/documents.css
git commit -m "style(documents): styles for presets row, active chips, expanded filter panel"
```

---

## Phase 9 — Screen tests for new behavior

### Task 19: Add screen tests for presets, panel, chips, counter

**Files:**
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`

- [ ] **Step 1: Add `applyPreset`, `applyFilters`, `clearAllFilters`, `setDateRange`, `setStatusFilter`, `setAmountRange` to mocks**

In the `mocks = vi.hoisted(...)` block, add:
```ts
applyPreset: vi.fn(),
applyFilters: vi.fn(),
clearAllFilters: vi.fn(),
setDateRange: vi.fn(),
setStatusFilter: vi.fn(),
setAmountRange: vi.fn(),
```

And register them in the `vi.mock(...)` for `documentsStore`.

Update the new `documentsState` initial value to include `dateFrom: null, dateTo: null, statusFilter: [], amountMin: null, amountMax: null, overdueOnly: false, activePresetId: null`.

Update both `setDocumentsState` helper and `setDocumentsStateWithoutDraftContext` to include those fields.

- [ ] **Step 2: Add 4 new tests**

Append to the `describe`:
```ts
it("renders preset chips and applies preset on click", async () => {
  const { component, target } = renderDocuments();
  (target.querySelector('[data-testid="documents-preset-unpaid"]') as HTMLButtonElement).click();
  await tick();
  expect(mocks.applyPreset).toHaveBeenCalledWith("unpaid");
  component.$destroy();
});

it("shows filter counter badge when filters are active", () => {
  mocks.documentsState.set({
    list: makeList(),
    editor: null,
    chain: null,
    draftContext: null,
    selectedIds: [],
    initialLoading: false,
    loading: false,
    error: null,
    message: null,
    activeTab: "all",
    kindFilter: null,
    counterpartyFilterId: "counterparty-1",
    dateFrom: "2026-04-01",
    dateTo: "2026-05-01",
    statusFilter: ["draft"],
    amountMin: null,
    amountMax: null,
    overdueOnly: false,
    activePresetId: null,
  });

  const { component, target } = renderDocuments();
  const filterButton = target.querySelector('[data-testid="documents-filter-button"]') as HTMLButtonElement;
  expect(filterButton.textContent).toContain("· 3");
  component.$destroy();
});

it("renders active filter chips with × removal", async () => {
  mocks.documentsState.set({
    list: makeList(),
    editor: null,
    chain: null,
    draftContext: null,
    selectedIds: [],
    initialLoading: false,
    loading: false,
    error: null,
    message: null,
    activeTab: "all",
    kindFilter: null,
    counterpartyFilterId: null,
    dateFrom: "2026-04-01",
    dateTo: "2026-05-01",
    statusFilter: [],
    amountMin: null,
    amountMax: null,
    overdueOnly: false,
    activePresetId: null,
  });

  const { component, target } = renderDocuments();
  const chipsBlock = target.querySelector('[data-testid="documents-active-filters"]')!;
  const periodChip = Array.from(chipsBlock.querySelectorAll("button")).find((b) => b.textContent?.includes("Період"))!;
  periodChip.click();
  await tick();
  expect(mocks.setDateRange).toHaveBeenCalledWith(null, null);
  component.$destroy();
});

it("opens filter panel and applies all draft fields with one applyFilters call", async () => {
  const { component, target } = renderDocuments();
  (target.querySelector('[data-testid="documents-filter-button"]') as HTMLButtonElement).click();
  await tick();

  const panel = target.querySelector('[data-testid="documents-filter-panel"]')!;
  const dateFromInput = panel.querySelectorAll('input[type="date"]')[0] as HTMLInputElement;
  dateFromInput.value = "2026-04-01";
  dateFromInput.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();

  const amountFromInput = panel.querySelectorAll('input[inputmode="decimal"]')[0] as HTMLInputElement;
  amountFromInput.value = "1000";
  amountFromInput.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();

  const draftCheckbox = Array.from(panel.querySelectorAll('input[type="checkbox"]')).find((cb) => (cb as HTMLInputElement).value === "draft") as HTMLInputElement;
  draftCheckbox.click();
  await tick();

  buttonByText(target, "Застосувати").click();
  await tick();

  expect(mocks.applyFilters).toHaveBeenCalledTimes(1);
  expect(mocks.applyFilters.mock.calls[0][0]).toMatchObject({
    dateFrom: "2026-04-01",
    statusFilter: ["draft"],
    amountMin: "1000",
  });
  component.$destroy();
});

it("clear-all button calls clearAllFilters", async () => {
  mocks.documentsState.set({
    list: makeList(),
    editor: null, chain: null, draftContext: null, selectedIds: [],
    initialLoading: false, loading: false, error: null, message: null,
    activeTab: "all", kindFilter: null, counterpartyFilterId: "cp-1",
    dateFrom: null, dateTo: null, statusFilter: [], amountMin: null, amountMax: null,
    overdueOnly: false, activePresetId: null,
  });
  const { component, target } = renderDocuments();
  (target.querySelector('[data-testid="documents-clear-filters"]') as HTMLButtonElement).click();
  expect(mocks.clearAllFilters).toHaveBeenCalled();
  component.$destroy();
});
```

- [ ] **Step 3: Run all tests**

Run: `cd frontend && npm run test:frontend`
Expected: all green.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
git commit -m "test(documents): cover presets, counter badge, active chips, panel apply"
```

---

## Phase 10 — Verification

### Task 20: Full verification + smoke

- [ ] **Step 1: Compile everything**

Run: `cargo build --tests`
Expected: `Finished`.

- [ ] **Step 2: Rust tests**

Run: `cargo test --lib`
Expected: all green.

If `TEST_DATABASE_URL` is set:
Run: `TEST_DATABASE_URL=$TEST_DATABASE_URL cargo test --test db_integration --test tauri_vertical_slice`
Expected: all green.

- [ ] **Step 3: Frontend tests**

Run: `cd frontend && npm run check && npm run test:frontend`
Expected: all green.

- [ ] **Step 4: Smoke `cargo tauri dev`**

Run: `cd src-tauri && cargo tauri dev`
Manual checklist:
1. Open Documents tab. The "Пошук документів" input is gone.
2. Topline shows `Швидкі: [Усі] [Чернетки] [Неоплачені] [Прострочені] [Цього місяця]`.
3. Click `Неоплачені` → list refreshes; `Неоплачені` chip is highlighted; counter on `Фільтр` shows `· 1`; under toolbar appears `Активні: [Статус: Виставлено, Підписано ×]`.
4. Click the `×` on the status chip → list resets; `Неоплачені` no longer highlighted (preset cleared).
5. Click `Фільтр` → panel opens with 4 sections. Pick `Місяць` sub-preset → date inputs populate; pick a status checkbox; type `1000` in amount-from; click `Застосувати` → list reloads; counter shows `· 3`; chips appear.
6. Click `Очистити` → counter disappears, chips disappear, list shows everything.
7. Pick `Прострочені` preset → if any `kind=waybill` is currently selected via chip filter, the list is empty (waybills skip overdue). Switch back to `Усі типи` → list shows overdue acts/invoices only.
8. Try `dateFrom > dateTo` in the panel → red inline error appears, `Застосувати` is disabled.
9. Type `abc` in amount-from → error `Некоректна сума`; `Застосувати` disabled.

Document any visual issue and patch in a follow-up commit.

- [ ] **Step 5: Final commit (if smoke fixes were needed)**

If patches were needed:
```bash
git add -A
git commit -m "fix(documents): smoke fixes for filter UI"
```

---

## Self-Review Notes

- **Spec coverage:** All sections of the spec map to tasks: search removal (Task 14), presets (Tasks 13/15), expanded panel (Task 17), active chips + counter (Task 16), backend SQL (Tasks 1-6), DTO (Task 7), store (Tasks 10-12), styles (Task 18), tests (Tasks 2-6, 10-12, 19), verification (Task 20).
- **Type consistency:** `DocumentStatus` defined once in `types.ts` and reused in `ui.ts`, store, screen. `DocumentFilterPresetSnapshot` matches state-update payload in `applyPreset`. `applyFilters` draft shape matches what `applyPanel` passes.
- **Order dependency note:** Task 12 imports `DOCUMENT_FILTER_PRESETS` from `ui.ts` which is defined in Task 13. If executing strictly sequentially, run Task 13 *before* Task 12. The plan numbering reflects narrative flow (store → config → screen). Re-order during execution: 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, **13**, **12**, 14, 15, 16, 17, 18, 19, 20.
