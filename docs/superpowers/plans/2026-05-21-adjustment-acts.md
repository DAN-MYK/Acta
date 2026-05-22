# Акти коригування — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Акти коригування — a new document type for correcting acts, with full CRUD, PDF generation, `is_adjusted` flag maintenance, and reports integration via `status = 'applied'` effective amounts.

**Architecture:** Separate `adjustment_acts` + `adjustment_act_items` tables with FK to `acts.id`. New `adj:uuid` document reference prefix integrates into the existing unified `tauri_api/documents` layer — all existing commands (`document_open`, `document_save`, `document_advance_status`, `document_delete`, `document_generate_pdf`) gain a new `DocumentRef::AdjustmentAct` branch. Reports use a correlated scalar subquery or CTE+JOIN for effective amounts, counting only `status = 'applied'` adjustments.

**Tech Stack:** Rust + SQLx (runtime-style async queries, no `query_as!` macro to avoid `cargo sqlx prepare` dependency), Typst CLI for PDF, PostgreSQL ENUMs, Svelte 5 + Tauri 2 frontend.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `migrations/030_adjustment_acts.sql` | Schema: tables, indexes, UNIQUE, is_adjusted |
| Create | `src/models/adjustment_act.rs` | Rust structs + AdjustmentActStatus enum |
| Modify | `src/models/act.rs:121` | Add `is_adjusted: bool` to ActListRow |
| Modify | `src/models/mod.rs` | pub mod + re-exports |
| Create | `src/db/adjustment_acts.rs` | Full CRUD: generate_next_number, create, get_full, list_filtered, list_for_act, update_with_items_scoped, change_status_scoped, delete_scoped |
| Modify | `src/db/acts.rs:162-164` | Add `a.is_adjusted` to list_filtered SELECT |
| Modify | `src/db/mod.rs` | pub mod adjustment_acts + smoke test |
| Create | `tests/db_integration/adjustment_acts.rs` | 7 integration tests |
| Modify | `tests/db_integration.rs:230-245` | Register mod adjustment_acts |
| Modify | `src/tauri_api/documents/dto.rs` | DTOs: new kind/status variants, new fields |
| Modify | `src/tauri_api/documents/api.rs` | DocumentRef::AdjustmentAct branches everywhere |
| Modify | `src/tauri_api/documents/pdf.rs` | AdjustmentAct branches in all match |
| Create | `src/db/adjustment_acts.rs` (covered above) | — |
| Modify | `src-tauri/src/commands/documents.rs` | act_adjustments_list command |
| Modify | `src-tauri/src/lib.rs:85` | Register act_adjustments_list |
| Create | `templates/adjustment_act.typ` | Typst PDF template |
| Modify | `src/pdf/generator.rs` | PdfAdjustmentActData + generate_adjustment_act_pdf |
| Modify | `frontend/src/lib/types.ts` | DocumentKind + DocumentStatus + DocumentsListDto |
| Modify | `frontend/src/lib/browser-fixtures.ts` | adjustmentActItems: [] |
| Modify | `frontend/src/lib/api.ts` | documentCreateAdjustmentActDraft + actAdjustmentsList |
| Modify | `frontend/src/lib/config/documents.ts` | DOCUMENT_KIND_META + filter options + status |
| Modify | `frontend/src/lib/stores/documents.ts` | createAdjustmentActDraft method |
| Modify | `frontend/src/lib/screens/DocumentsScreen.svelte` | adj-specific drawer UI |
| Modify | `src/db/reports.rs` | 4 functions: effective amounts |
| Modify | `src/db/dashboard.rs` | 7 functions: effective amounts |

---

## Task 1: DB Migration

**Files:**
- Create: `migrations/030_adjustment_acts.sql`

- [ ] **Create migration file**

```sql
-- migrations/030_adjustment_acts.sql
CREATE TYPE adjustment_act_status AS ENUM ('draft', 'issued', 'signed', 'applied');

CREATE TABLE adjustment_acts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id        UUID NOT NULL REFERENCES companies(id),
    original_act_id   UUID NOT NULL REFERENCES acts(id),
    counterparty_id   UUID NOT NULL REFERENCES counterparties(id),
    number            VARCHAR(50) NOT NULL,
    date              DATE NOT NULL,
    direction         VARCHAR(8) NOT NULL DEFAULT 'outgoing'
                        CHECK (direction IN ('outgoing', 'incoming')),
    total_amount      DECIMAL(15,2) NOT NULL,
    status            adjustment_act_status NOT NULL DEFAULT 'draft',
    notes             TEXT,
    bas_id            VARCHAR(100) UNIQUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(company_id, number)
);

CREATE TABLE adjustment_act_items (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    adjustment_act_id UUID NOT NULL REFERENCES adjustment_acts(id) ON DELETE CASCADE,
    description       TEXT NOT NULL,
    quantity          DECIMAL(15,4) NOT NULL,
    unit_price        DECIMAL(15,2) NOT NULL,
    total_price       DECIMAL(15,2) NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_adjustment_acts_company      ON adjustment_acts(company_id);
CREATE INDEX idx_adjustment_acts_original     ON adjustment_acts(original_act_id);
CREATE INDEX idx_adjustment_acts_counterparty ON adjustment_acts(counterparty_id);
CREATE INDEX idx_adjustment_act_items_parent  ON adjustment_act_items(adjustment_act_id);

ALTER TABLE acts ADD COLUMN is_adjusted BOOLEAN NOT NULL DEFAULT FALSE;
```

- [ ] **Run migration**

```powershell
sqlx migrate run
```

Expected output: `Applied 030/up adjustment_acts`

- [ ] **Verify schema**

```powershell
sqlx migrate info
```

Expected: migration 030 shows as Applied.

- [ ] **Commit**

```bash
git add migrations/030_adjustment_acts.sql
git commit -m "feat(db): add adjustment_acts tables and is_adjusted flag on acts"
```

---

## Task 2: Rust Models

**Files:**
- Create: `src/models/adjustment_act.rs`
- Modify: `src/models/act.rs` (line 121: ActListRow)
- Modify: `src/models/mod.rs`

- [ ] **Create `src/models/adjustment_act.rs`**

```rust
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::DocumentDirection;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "adjustment_act_status", rename_all = "lowercase")]
pub enum AdjustmentActStatus {
    Draft,
    Issued,
    Signed,
    Applied,
}

impl AdjustmentActStatus {
    pub fn can_transition_to(&self, next: &AdjustmentActStatus) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Issued)
                | (Self::Issued, Self::Signed)
                | (Self::Signed, Self::Applied)
        )
    }

    pub fn next(&self) -> Option<AdjustmentActStatus> {
        match self {
            Self::Draft => Some(Self::Issued),
            Self::Issued => Some(Self::Signed),
            Self::Signed => Some(Self::Applied),
            Self::Applied => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Draft => "Чернетка",
            Self::Issued => "Виставлено",
            Self::Signed => "Підписано",
            Self::Applied => "Застосовано",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Issued => "issued",
            Self::Signed => "signed",
            Self::Applied => "applied",
        }
    }
}

impl std::fmt::Display for AdjustmentActStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AdjustmentAct {
    pub id: Uuid,
    pub company_id: Uuid,
    pub original_act_id: Uuid,
    pub counterparty_id: Uuid,
    pub number: String,
    pub date: NaiveDate,
    pub direction: DocumentDirection,
    pub total_amount: Decimal,
    pub status: AdjustmentActStatus,
    pub notes: Option<String>,
    pub bas_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AdjustmentActItem {
    pub id: Uuid,
    pub adjustment_act_id: Uuid,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub total_price: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
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
    pub direction: DocumentDirection,
    pub status: AdjustmentActStatus,
}

pub struct NewAdjustmentActItem {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

pub struct UpdateAdjustmentAct {
    pub number: String,
    pub date: NaiveDate,
    pub notes: Option<String>,
}
```

- [ ] **Modify `src/models/act.rs` — add `is_adjusted` to ActListRow**

Find the struct at line 121 and replace:

```rust
// BEFORE (line 120-129):
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ActListRow {
    pub id: Uuid,
    pub number: String,
    pub direction: DocumentDirection,
    pub date: NaiveDate,
    pub counterparty_name: String,
    pub total_amount: Decimal,
    pub status: ActStatus,
}

// AFTER:
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ActListRow {
    pub id: Uuid,
    pub number: String,
    pub direction: DocumentDirection,
    pub date: NaiveDate,
    pub counterparty_name: String,
    pub total_amount: Decimal,
    pub status: ActStatus,
    pub is_adjusted: bool,
}
```

- [ ] **Modify `src/models/mod.rs` — add module and re-exports**

Add after line 14 (`pub mod waybill;`):
```rust
pub mod adjustment_act;
```

Add after line 17 (`pub use act::{...}`):
```rust
#[allow(unused_imports)]
pub use adjustment_act::{
    AdjustmentAct, AdjustmentActItem, AdjustmentActListRow, AdjustmentActStatus,
    NewAdjustmentActItem, UpdateAdjustmentAct,
};
```

Also add to the existing `reexports_are_available_for_consumers` test — add a usage of `AdjustmentActStatus`:
```rust
// In models/mod.rs #[cfg(test)] block, add to imports:
use super::AdjustmentActStatus;

// Add assertion:
assert_eq!(AdjustmentActStatus::Applied.as_str(), "applied");
```

- [ ] **Compile check**

```powershell
cargo build --lib 2>&1 | Select-String -Pattern "error"
```

Expected: no errors.

- [ ] **Commit**

```bash
git add src/models/adjustment_act.rs src/models/act.rs src/models/mod.rs
git commit -m "feat(models): add AdjustmentAct models and is_adjusted on ActListRow"
```

---

## Task 3: DB Layer — adjustment_acts CRUD

**Files:**
- Create: `src/db/adjustment_acts.rs`
- Modify: `src/db/acts.rs` (list_filtered SELECT)
- Modify: `src/db/mod.rs`

- [ ] **Create `src/db/adjustment_acts.rs`**

```rust
use anyhow::Result;
use chrono::Datelike;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::adjustment_act::{
    AdjustmentAct, AdjustmentActItem, AdjustmentActListRow, AdjustmentActStatus,
    NewAdjustmentActItem, UpdateAdjustmentAct,
};
use crate::models::DocumentDirection;

/// Генерує наступний номер у форматі "КОР-РРРР-NNN".
/// Той самий rsplit_once('-') паттерн що і в acts::generate_next_number.
pub async fn generate_next_number(pool: &PgPool, company_id: Uuid) -> Result<String> {
    use sqlx::Row;
    let year = chrono::Utc::now().year();

    let rows = sqlx::query(
        "SELECT number FROM adjustment_acts WHERE company_id = $1 AND EXTRACT(YEAR FROM date)::int = $2"
    )
    .bind(company_id)
    .bind(year as i32)
    .fetch_all(pool)
    .await?;

    let max_seq = rows
        .iter()
        .filter_map(|r| {
            let num: Option<String> = r.try_get("number").ok();
            num.and_then(|n| n.rsplit_once('-').and_then(|(_, s)| s.parse::<u32>().ok()))
        })
        .max()
        .unwrap_or(0);

    Ok(format!("КОР-{year}-{:03}", max_seq + 1))
}

/// Створює акт коригування — чернетку з нульовою сумою.
/// Верифікує що `original_act_id` належить до `company_id`.
/// Копіює `counterparty_id` і `direction` з оригінального акту — не довіряє клієнту.
pub async fn create(pool: &PgPool, company_id: Uuid, original_act_id: Uuid) -> Result<AdjustmentAct> {
    use sqlx::Row;

    let original = sqlx::query(
        "SELECT counterparty_id, direction FROM acts WHERE id = $1 AND company_id = $2"
    )
    .bind(original_act_id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("Оригінальний акт не знайдено в межах компанії"))?;

    let counterparty_id: Uuid = original.get("counterparty_id");
    let direction: DocumentDirection = original.get("direction");
    let number = generate_next_number(pool, company_id).await?;
    let date = chrono::Utc::now().date_naive();

    let mut tx = pool.begin().await?;

    let row = sqlx::query(
        r#"INSERT INTO adjustment_acts
           (company_id, original_act_id, counterparty_id, number, date, direction, total_amount, status)
           VALUES ($1, $2, $3, $4, $5, $6, 0, 'draft')
           RETURNING id, company_id, original_act_id, counterparty_id, number, date,
                     direction, total_amount, status, notes, bas_id, created_at, updated_at"#
    )
    .bind(company_id)
    .bind(original_act_id)
    .bind(counterparty_id)
    .bind(&number)
    .bind(date)
    .bind(direction.as_str())
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(AdjustmentAct {
        id: row.get("id"),
        company_id: row.get("company_id"),
        original_act_id: row.get("original_act_id"),
        counterparty_id: row.get("counterparty_id"),
        number: row.get("number"),
        date: row.get("date"),
        direction: row.get("direction"),
        total_amount: row.get("total_amount"),
        status: row.get("status"),
        notes: row.get("notes"),
        bas_id: row.get("bas_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub async fn get_full(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
) -> Result<Option<(AdjustmentAct, Vec<AdjustmentActItem>)>> {
    let adj = sqlx::query_as::<_, AdjustmentAct>(
        r#"SELECT id, company_id, original_act_id, counterparty_id, number, date,
                  direction, total_amount, status, notes, bas_id, created_at, updated_at
           FROM adjustment_acts WHERE id = $1 AND company_id = $2"#
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let Some(adj) = adj else { return Ok(None) };

    let items = sqlx::query_as::<_, AdjustmentActItem>(
        r#"SELECT id, adjustment_act_id, description, quantity, unit_price, total_price,
                  created_at, updated_at
           FROM adjustment_act_items WHERE adjustment_act_id = $1 ORDER BY created_at"#
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(Some((adj, items)))
}

/// Оновлює заголовок + позиції (DELETE+INSERT в транзакції).
/// total_amount перераховується з items.
pub async fn update_with_items_scoped(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
    update: UpdateAdjustmentAct,
    items: Vec<NewAdjustmentActItem>,
) -> Result<Option<()>> {
    let total_amount: Decimal = items.iter().map(|i| i.quantity * i.unit_price).sum();

    let mut tx = pool.begin().await?;

    let result = sqlx::query(
        r#"UPDATE adjustment_acts
           SET number = $1, date = $2, total_amount = $3, notes = $4, updated_at = NOW()
           WHERE id = $5 AND company_id = $6"#
    )
    .bind(&update.number)
    .bind(update.date)
    .bind(total_amount)
    .bind(&update.notes)
    .bind(id)
    .bind(company_id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }

    sqlx::query("DELETE FROM adjustment_act_items WHERE adjustment_act_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    for item in &items {
        let total_price = item.quantity * item.unit_price;
        sqlx::query(
            r#"INSERT INTO adjustment_act_items
               (adjustment_act_id, description, quantity, unit_price, total_price)
               VALUES ($1, $2, $3, $4, $5)"#
        )
        .bind(id)
        .bind(&item.description)
        .bind(item.quantity)
        .bind(item.unit_price)
        .bind(total_price)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(Some(()))
}

/// Переводить акт коригування на наступний статус.
/// При переході у Applied: виставляє is_adjusted = TRUE на оригінальному акті.
pub async fn change_status_scoped(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
) -> Result<Option<()>> {
    let adj = sqlx::query_as::<_, AdjustmentAct>(
        r#"SELECT id, company_id, original_act_id, counterparty_id, number, date,
                  direction, total_amount, status, notes, bas_id, created_at, updated_at
           FROM adjustment_acts WHERE id = $1 AND company_id = $2"#
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let Some(adj) = adj else { return Ok(None) };

    let next = adj
        .status
        .next()
        .ok_or_else(|| anyhow::anyhow!("Акт коригування вже у фінальному статусі Applied"))?;

    let mut tx = pool.begin().await?;

    sqlx::query(
        "UPDATE adjustment_acts SET status = $1::adjustment_act_status, updated_at = NOW() WHERE id = $2"
    )
    .bind(next.as_str())
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if matches!(next, AdjustmentActStatus::Applied) {
        sqlx::query(
            "UPDATE acts SET is_adjusted = TRUE, updated_at = NOW() WHERE id = $1"
        )
        .bind(adj.original_act_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(Some(()))
}

/// Видаляє акт коригування.
/// Якщо він був Applied і це останній applied — знімає is_adjusted на оригінальному акті.
pub async fn delete_scoped(pool: &PgPool, company_id: Uuid, id: Uuid) -> Result<bool> {
    let adj = sqlx::query_as::<_, AdjustmentAct>(
        r#"SELECT id, company_id, original_act_id, counterparty_id, number, date,
                  direction, total_amount, status, notes, bas_id, created_at, updated_at
           FROM adjustment_acts WHERE id = $1 AND company_id = $2"#
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let Some(adj) = adj else { return Ok(false) };
    let was_applied = matches!(adj.status, AdjustmentActStatus::Applied);
    let original_act_id = adj.original_act_id;

    let mut tx = pool.begin().await?;

    let result = sqlx::query(
        "DELETE FROM adjustment_acts WHERE id = $1 AND company_id = $2"
    )
    .bind(id)
    .bind(company_id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(false);
    }

    if was_applied {
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM adjustment_acts WHERE original_act_id = $1 AND status = 'applied'"
        )
        .bind(original_act_id)
        .fetch_one(&mut *tx)
        .await?;

        if remaining == 0 {
            sqlx::query(
                "UPDATE acts SET is_adjusted = FALSE, updated_at = NOW() WHERE id = $1"
            )
            .bind(original_act_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(true)
}

pub async fn list_for_act(
    pool: &PgPool,
    company_id: Uuid,
    original_act_id: Uuid,
) -> Result<Vec<AdjustmentActListRow>> {
    sqlx::query_as::<_, AdjustmentActListRow>(
        r#"SELECT aa.id, aa.company_id, aa.original_act_id,
                  a.number AS original_act_number,
                  aa.counterparty_id,
                  c.name AS counterparty_name,
                  aa.number, aa.date, aa.total_amount, aa.direction, aa.status
           FROM adjustment_acts aa
           JOIN acts a ON a.id = aa.original_act_id
           JOIN counterparties c ON c.id = aa.counterparty_id
           WHERE aa.company_id = $1 AND aa.original_act_id = $2
           ORDER BY aa.date DESC, aa.number"#
    )
    .bind(company_id)
    .bind(original_act_id)
    .fetch_all(pool)
    .await
    .map_err(anyhow::Error::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn list_filtered(
    pool: &PgPool,
    company_id: Uuid,
    statuses: Option<&[String]>,
    direction: Option<DocumentDirection>,
    search_query: Option<&str>,
    counterparty_id: Option<Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
    amount_min: Option<Decimal>,
    amount_max: Option<Decimal>,
) -> Result<Vec<AdjustmentActListRow>> {
    let search_query = search_query.map(str::trim).filter(|q| !q.is_empty());
    let has_search = search_query.is_some();

    let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        r#"SELECT aa.id, aa.company_id, aa.original_act_id,
                  a.number AS original_act_number,
                  aa.counterparty_id,
                  c.name AS counterparty_name,
                  aa.number, aa.date, aa.total_amount, aa.direction, aa.status
           FROM adjustment_acts aa
           JOIN acts a ON a.id = aa.original_act_id
           JOIN counterparties c ON c.id = aa.counterparty_id
           WHERE aa.company_id = "#,
    );
    qb.push_bind(company_id);

    if let Some(statuses) = statuses.filter(|s| !s.is_empty()) {
        let owned: Vec<String> = statuses.to_vec();
        qb.push(" AND aa.status::text = ANY(")
            .push_bind(owned)
            .push("::text[])");
    }
    if let Some(dir) = direction {
        qb.push(" AND aa.direction = ").push_bind(dir.as_str());
    }
    if let Some(q) = search_query {
        let pattern = super::ilike_pattern(q);
        qb.push(" AND (aa.number ILIKE ")
            .push_bind(pattern.clone())
            .push(r" ESCAPE '\' OR c.name ILIKE ")
            .push_bind(pattern)
            .push(r" ESCAPE '\')");
    }
    if let Some(cp_id) = counterparty_id {
        qb.push(" AND aa.counterparty_id = ").push_bind(cp_id);
    }
    if let Some(df) = date_from {
        qb.push(" AND aa.date >= ").push_bind(df);
    }
    if let Some(dt) = date_to {
        qb.push(" AND aa.date <= ").push_bind(dt);
    }
    if let Some(min) = amount_min {
        qb.push(" AND aa.total_amount >= ").push_bind(min);
    }
    if let Some(max) = amount_max {
        qb.push(" AND aa.total_amount <= ").push_bind(max);
    }
    qb.push(" ORDER BY aa.date DESC, aa.number");
    if has_search {
        qb.push(" LIMIT 100");
    }

    qb.build_query_as::<AdjustmentActListRow>()
        .fetch_all(pool)
        .await
        .map_err(anyhow::Error::from)
}
```

- [ ] **Modify `src/db/acts.rs` — add `is_adjusted` to list_filtered SELECT**

In `list_filtered` (around line 161), change the QueryBuilder initialization:

```rust
// BEFORE:
let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
    r#"SELECT a.id, a.number, a.direction, a.date,
           c.name AS counterparty_name,
           a.total_amount, a.status
    FROM acts a
    JOIN counterparties c ON c.id = a.counterparty_id
    WHERE a.company_id = "#,
);

// AFTER:
let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
    r#"SELECT a.id, a.number, a.direction, a.date,
           c.name AS counterparty_name,
           a.total_amount, a.status, a.is_adjusted
    FROM acts a
    JOIN counterparties c ON c.id = a.counterparty_id
    WHERE a.company_id = "#,
);
```

- [ ] **Modify `src/db/mod.rs` — register adjustment_acts**

Add after `pub mod waybills;` (line 25):
```rust
pub mod adjustment_acts;
```

In the `#[cfg(test)]` block, update the `use super::{...}` import to include `adjustment_acts`:
```rust
use super::{
    acts, adjustment_acts, categories, companies, contracts, counterparties, dashboard,
    document_templates, ilike_pattern, invoices, payments, reports, search, tasks, waybills,
};
```

Add to `db_submodules_are_available` test:
```rust
let _ = adjustment_acts::list_filtered;
```

- [ ] **Compile check**

```powershell
cargo build --lib 2>&1 | Select-String -Pattern "error"
```

Expected: no errors.

- [ ] **Commit**

```bash
git add src/db/adjustment_acts.rs src/db/acts.rs src/db/mod.rs
git commit -m "feat(db): add adjustment_acts CRUD with is_adjusted maintenance"
```

---

## Task 4: Integration Tests

**Files:**
- Create: `tests/db_integration/adjustment_acts.rs`
- Modify: `tests/db_integration.rs`

- [ ] **Create `tests/db_integration/adjustment_acts.rs`**

```rust
use rust_decimal_macros::dec;

use super::*;

#[tokio::test]
async fn test_adjustment_act_create_and_get() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-ТОВ {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-КОР-{suffix}"), dec!(10000.00), "issued", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let adj = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id)
        .await.unwrap();

    assert_eq!(adj.original_act_id, act_id);
    assert_eq!(adj.counterparty_id, cp.id);
    assert!(adj.number.starts_with("КОР-"), "number must start with КОР-");
    assert_eq!(
        adj.status,
        acta::models::adjustment_act::AdjustmentActStatus::Draft
    );
    assert_eq!(adj.total_amount, dec!(0));

    let (fetched, items) = acta::db::adjustment_acts::get_full(
        &pool, DEFAULT_COMPANY_ID, adj.id,
    ).await.unwrap().unwrap();

    assert_eq!(fetched.id, adj.id);
    assert!(items.is_empty(), "fresh draft must have no items");
}

#[tokio::test]
async fn test_adjustment_act_numbering() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-NUM {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-NUM-{suffix}"), dec!(5000.00), "draft", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let adj1 = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();
    let adj2 = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    let seq1: u32 = adj1.number.rsplit_once('-').unwrap().1.parse().unwrap();
    let seq2: u32 = adj2.number.rsplit_once('-').unwrap().1.parse().unwrap();
    assert!(seq2 > seq1, "sequential adj acts must have ascending numbers");
}

#[tokio::test]
async fn test_adjustment_act_status_advance_sets_is_adjusted() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-STATUS {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-STATUS-{suffix}"), dec!(3000.00), "signed", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let is_adj_before: bool = sqlx::query_scalar(
        "SELECT is_adjusted FROM acts WHERE id = $1"
    )
    .bind(act_id)
    .fetch_one(&pool)
    .await.unwrap();
    assert!(!is_adj_before, "is_adjusted must start as false");

    let adj = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    // Draft → Issued → Signed → Applied (3 transitions)
    for _ in 0..3 {
        acta::db::adjustment_acts::change_status_scoped(&pool, DEFAULT_COMPANY_ID, adj.id)
            .await.unwrap().expect("transition must succeed");
    }

    let is_adj_after: bool = sqlx::query_scalar(
        "SELECT is_adjusted FROM acts WHERE id = $1"
    )
    .bind(act_id)
    .fetch_one(&pool)
    .await.unwrap();
    assert!(is_adj_after, "is_adjusted must be TRUE after adj reaches Applied");

    let status: String = sqlx::query_scalar(
        "SELECT status::text FROM adjustment_acts WHERE id = $1"
    )
    .bind(adj.id)
    .fetch_one(&pool)
    .await.unwrap();
    assert_eq!(status, "applied");
}

#[tokio::test]
async fn test_adjustment_act_delete_clears_is_adjusted() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-DEL {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-DEL-{suffix}"), dec!(7000.00), "paid", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let adj = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    // Advance to Applied
    for _ in 0..3 {
        acta::db::adjustment_acts::change_status_scoped(&pool, DEFAULT_COMPANY_ID, adj.id)
            .await.unwrap().unwrap();
    }

    let is_adj: bool = sqlx::query_scalar("SELECT is_adjusted FROM acts WHERE id = $1")
        .bind(act_id).fetch_one(&pool).await.unwrap();
    assert!(is_adj);

    let deleted = acta::db::adjustment_acts::delete_scoped(&pool, DEFAULT_COMPANY_ID, adj.id)
        .await.unwrap();
    assert!(deleted);

    let is_adj_after: bool = sqlx::query_scalar("SELECT is_adjusted FROM acts WHERE id = $1")
        .bind(act_id).fetch_one(&pool).await.unwrap();
    assert!(!is_adj_after, "is_adjusted must be FALSE when last applied adj deleted");
}

#[tokio::test]
async fn test_adjustment_acts_list_for_act() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-LIST {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-LIST-{suffix}"), dec!(5000.00), "issued", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let _adj1 = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();
    let _adj2 = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    let rows = acta::db::adjustment_acts::list_for_act(&pool, DEFAULT_COMPANY_ID, act_id)
        .await.unwrap();

    assert_eq!(rows.len(), 2, "must return both adj acts for the act");
    for row in &rows {
        assert_eq!(row.original_act_id, act_id);
        assert_eq!(row.counterparty_id, cp.id);
    }
}

#[tokio::test]
async fn test_issued_signed_adj_does_not_affect_effective_amount() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-EFF {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-EFF-{suffix}"), dec!(10000.00), "signed", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let adj = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    // Simulate a -1000 adjustment
    sqlx::query("UPDATE adjustment_acts SET total_amount = -1000 WHERE id = $1")
        .bind(adj.id)
        .execute(&pool)
        .await.unwrap();

    // Advance to Signed only (not Applied)
    for _ in 0..2 {
        acta::db::adjustment_acts::change_status_scoped(&pool, DEFAULT_COMPANY_ID, adj.id)
            .await.unwrap().unwrap();
    }

    let effective: rust_decimal::Decimal = sqlx::query_scalar(
        r#"SELECT a.total_amount + COALESCE(
               (SELECT SUM(aa.total_amount) FROM adjustment_acts aa
                WHERE aa.original_act_id = a.id AND aa.status = 'applied'),
               0)
           FROM acts a WHERE a.id = $1"#
    )
    .bind(act_id)
    .fetch_one(&pool)
    .await.unwrap();

    assert_eq!(
        effective, dec!(10000.00),
        "signed adj must NOT affect effective_amount — only applied ones do"
    );
}

#[tokio::test]
async fn test_duplicate_adj_number_rejected_by_constraint() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-UNIQ {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-UNIQ-{suffix}"), dec!(1000.00), "draft", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let adj = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    // Attempt to bypass generate_next_number and insert duplicate number
    let result = sqlx::query(
        r#"INSERT INTO adjustment_acts
           (company_id, original_act_id, counterparty_id, number, date, direction, total_amount, status)
           SELECT company_id, original_act_id, counterparty_id, $1, date, direction, 0, 'draft'
           FROM adjustment_acts WHERE id = $2"#
    )
    .bind(&adj.number)
    .bind(adj.id)
    .execute(&pool)
    .await;

    assert!(result.is_err(), "UNIQUE(company_id, number) must reject duplicate number");
    let err_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        err_msg.contains("unique") || err_msg.contains("duplicate"),
        "error must mention uniqueness violation"
    );
}
```

- [ ] **Register in `tests/db_integration.rs`**

Add before the final closing (after `#[path = "db_integration/waybills.rs"]`):
```rust
#[path = "db_integration/adjustment_acts.rs"]
mod adjustment_acts;
```

- [ ] **Run integration tests**

```powershell
$env:TEST_DATABASE_URL="postgres://postgres:password@localhost:5432/acta"
cargo test --test db_integration adjustment_act 2>&1 | tail -30
```

Expected: `7 passed`, no failures.

- [ ] **Commit**

```bash
git add tests/db_integration/adjustment_acts.rs tests/db_integration.rs
git commit -m "test(db): add 7 integration tests for adjustment_acts"
```

---

## Task 5: DTO Changes

**Files:**
- Modify: `src/tauri_api/documents/dto.rs`

- [ ] **Add `AdjustmentAct` to `DocumentKindDto`**

```rust
// BEFORE:
pub enum DocumentKindDto {
    Invoice,
    Act,
    Waybill,
}

// AFTER:
pub enum DocumentKindDto {
    Invoice,
    Act,
    Waybill,
    AdjustmentAct,
}
```

- [ ] **Add `Applied` to `DocumentStatusDto` and conversion method**

```rust
// BEFORE:
pub enum DocumentStatusDto {
    Draft, Issued, Signed, Paid, Delivered,
}

// AFTER:
pub enum DocumentStatusDto {
    Draft, Issued, Signed, Paid, Delivered, Applied,
}
```

Add method to `DocumentStatusDto` impl block:
```rust
pub fn from_adjustment_act_status(
    status: &crate::models::adjustment_act::AdjustmentActStatus,
) -> Self {
    use crate::models::adjustment_act::AdjustmentActStatus as S;
    match status {
        S::Draft => Self::Draft,
        S::Issued => Self::Issued,
        S::Signed => Self::Signed,
        S::Applied => Self::Applied,
    }
}
```

- [ ] **Update `DocumentsListDto` — add `adjustment_act_items`**

```rust
// BEFORE:
pub struct DocumentsListDto {
    pub items: Vec<DocumentItemDto>,
    pub invoice_items: Vec<DocumentItemDto>,
    pub act_items: Vec<DocumentItemDto>,
    pub waybill_items: Vec<DocumentItemDto>,
    pub total_count: i32,
    pub page_count: i32,
}

// AFTER:
pub struct DocumentsListDto {
    pub items: Vec<DocumentItemDto>,
    pub invoice_items: Vec<DocumentItemDto>,
    pub act_items: Vec<DocumentItemDto>,
    pub waybill_items: Vec<DocumentItemDto>,
    pub adjustment_act_items: Vec<DocumentItemDto>,
    pub total_count: i32,
    pub page_count: i32,
}
```

- [ ] **Update `CreateDocumentDraftRequest` — direction optional, add original_act_id**

```rust
// BEFORE:
pub struct CreateDocumentDraftRequest {
    pub counterparty_id: Option<String>,
    pub kind: String,
    pub direction: String,
}

// AFTER:
pub struct CreateDocumentDraftRequest {
    pub counterparty_id: Option<String>,
    pub kind: String,
    pub direction: Option<String>, // None for adjustment_act; required for others
    pub original_act_id: Option<String>, // Some(uuid) only for adjustment_act
}
```

- [ ] **Update `DocumentDraftFormDto` — add original_act fields**

```rust
// BEFORE:
pub struct DocumentDraftFormDto {
    pub id: String,
    pub kind: String,
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub title: String,
    pub number: String,
    pub date: String,
    pub notes: String,
    pub direction: String,
}

// AFTER:
pub struct DocumentDraftFormDto {
    pub id: String,
    pub kind: String,
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub title: String,
    pub number: String,
    pub date: String,
    pub notes: String,
    pub direction: String,
    #[serde(default)]
    pub original_act_id: Option<String>,
    #[serde(default)]
    pub original_act_number: Option<String>,
}
```

- [ ] **Compile check**

```powershell
cargo build --lib 2>&1 | Select-String -Pattern "error"
```

Expected: compile errors because `DocumentsListDto { ... }` literal in `api.rs` is missing the new field and `DocumentRef` match is non-exhaustive. These will be fixed in Task 6.

- [ ] **Commit** (after Task 6 compile passes)

---

## Task 6: API Layer — DocumentRef + list + open/create/save/advance/delete/chain

**Files:**
- Modify: `src/tauri_api/documents/api.rs`

- [ ] **Add `AdjustmentAct` to `DocumentRef` enum**

```rust
// BEFORE (line 29-33):
pub(super) enum DocumentRef {
    Act(Uuid),
    Invoice(Uuid),
    Waybill(Uuid),
}

// AFTER:
pub(super) enum DocumentRef {
    Act(Uuid),
    Invoice(Uuid),
    Waybill(Uuid),
    AdjustmentAct(Uuid),
}
```

- [ ] **Add imports at top of api.rs**

Add to the `use` block at the top:
```rust
use crate::models::adjustment_act::{
    AdjustmentActStatus, NewAdjustmentActItem, UpdateAdjustmentAct,
};
```

- [ ] **Extend `parse_document_ref`**

```rust
// BEFORE (last two lines):
id.strip_prefix("wbl:")
    .and_then(|value| Uuid::parse_str(value).ok())
    .map(DocumentRef::Waybill)

// AFTER:
if let Some(uuid) = id
    .strip_prefix("wbl:")
    .and_then(|value| Uuid::parse_str(value).ok())
{
    return Some(DocumentRef::Waybill(uuid));
}

id.strip_prefix("adj:")
    .and_then(|value| Uuid::parse_str(value).ok())
    .map(DocumentRef::AdjustmentAct)
```

- [ ] **Extend `document_ref_string`**

```rust
// BEFORE:
fn document_ref_string(kind: &str, id: Uuid) -> String {
    match kind {
        "act" => format!("act:{id}"),
        "invoice" => format!("inv:{id}"),
        "waybill" => format!("wbl:{id}"),
        _ => id.to_string(),
    }
}

// AFTER:
fn document_ref_string(kind: &str, id: Uuid) -> String {
    match kind {
        "act" => format!("act:{id}"),
        "invoice" => format!("inv:{id}"),
        "waybill" => format!("wbl:{id}"),
        "adjustment_act" => format!("adj:{id}"),
        _ => id.to_string(),
    }
}
```

- [ ] **Extend `documents_list` — 4th branch in tokio::join! and result loop**

Replace lines 799–952 of `documents_list`. The key additions are:

After `let include_waybills = ...` add:
```rust
let include_adj_acts =
    request.kind.as_deref().map_or(true, |k| k == "adjustment_act") && !overdue_only;
```

Extend `tokio::join!` to 4 calls (wrap existing 3 in a tuple and add 4th):
```rust
let (acts, invoices, waybills, adj_acts) = tokio::join!(
    async {
        if include_acts {
            db::acts::list_filtered(
                ctx.pool(), company_id, statuses_slice, direction_filter,
                search, counterparty_filter, date_from, date_to,
                amount_min, amount_max, overdue_only, today,
            ).await
        } else { Ok(vec![]) }
    },
    async {
        if include_invoices {
            db::invoices::list_filtered(
                ctx.pool(), company_id, statuses_slice, direction_filter,
                search, counterparty_filter, date_from, date_to,
                amount_min, amount_max, overdue_only, today,
            ).await
        } else { Ok(vec![]) }
    },
    async {
        if include_waybills {
            db::waybills::list_filtered(
                ctx.pool(), company_id, statuses_slice, direction_filter,
                search, counterparty_filter, date_from, date_to,
                amount_min, amount_max,
            ).await
        } else { Ok(vec![]) }
    },
    async {
        if include_adj_acts {
            db::adjustment_acts::list_filtered(
                ctx.pool(), company_id, statuses_slice, direction_filter,
                search, counterparty_filter, date_from, date_to,
                amount_min, amount_max,
            ).await
        } else { Ok(vec![]) }
    },
);
```

After the `for row in waybills?` loop, add:
```rust
for row in adj_acts? {
    combined.push((
        row.date,
        DocumentItemDto {
            id: format!("adj:{}", row.id),
            kind: DocumentKindDto::AdjustmentAct,
            number: row.number,
            date: date_to_str(row.date),
            counterparty: row.counterparty_name,
            amount_str: format_money_ua(row.total_amount),
            status: DocumentStatusDto::from_adjustment_act_status(&row.status),
            status_label: row.status.label().to_string(),
            linked_id: row.original_act_id.to_string(),
            direction: row.direction.as_str().to_string(),
        },
    ));
}
```

Add `adjustment_act_items` partition after `waybill_items`:
```rust
let adjustment_act_items = items
    .iter()
    .filter(|item| matches!(item.kind, DocumentKindDto::AdjustmentAct))
    .cloned()
    .collect::<Vec<_>>();
```

In the `Ok(DocumentsListDto { ... })`:
```rust
Ok(DocumentsListDto {
    total_count: items.len() as i32,
    page_count: 1,
    items,
    invoice_items,
    act_items,
    waybill_items,
    adjustment_act_items,
})
```

- [ ] **Extend `build_existing_document_form` — add AdjustmentAct branch**

Add to the match in `build_existing_document_form`:
```rust
DocumentRef::AdjustmentAct(id) => {
    let (adj, items) = db::adjustment_acts::get_full(pool, company_id, id)
        .await?
        .ok_or_else(|| anyhow!("Акт коригування не знайдено"))?;
    let counterparty_name =
        load_counterparty_name(pool, company_id, adj.counterparty_id).await?;
    let original_act_number: String = sqlx::query_scalar(
        "SELECT number FROM acts WHERE id = $1"
    )
    .bind(adj.original_act_id)
    .fetch_one(pool)
    .await?;

    Ok(DocumentEditorDto {
        form: DocumentDraftFormDto {
            id: format!("adj:{id}"),
            kind: "adjustment_act".to_string(),
            counterparty_id: adj.counterparty_id.to_string(),
            counterparty_name,
            title: "Акт коригування".to_string(),
            number: adj.number,
            date: date_to_str(adj.date),
            notes: adj.notes.unwrap_or_default(),
            direction: adj.direction.as_str().to_string(),
            original_act_id: Some(adj.original_act_id.to_string()),
            original_act_number: Some(original_act_number),
        },
        items: items
            .into_iter()
            .map(|item| DocumentDraftItemDto {
                description: item.description,
                unit: String::new(),
                quantity: item.quantity.to_string(),
                price: item.unit_price.to_string(),
            })
            .collect(),
        pdf: None,
        show_type_picker: false,
        show_editor: true,
    })
}
```

- [ ] **Extend `document_create_draft` — handle adjustment_act kind**

Replace the function body:
```rust
pub async fn document_create_draft(
    ctx: &AppCtx,
    request: CreateDocumentDraftRequest,
) -> Result<DocumentEditorDto> {
    // Special path for adjustment acts — direction and counterparty come from the original act
    if request.kind == "adjustment_act" {
        let original_act_id_str = request
            .original_act_id
            .as_deref()
            .ok_or_else(|| anyhow!("original_act_id є обов'язковим для adjustment_act"))?;
        let original_act_id = Uuid::parse_str(original_act_id_str)
            .with_context(|| format!("Некоректний original_act_id: {original_act_id_str}"))?;

        let adj = db::adjustment_acts::create(ctx.pool(), ctx.company_id(), original_act_id)
            .await?;

        return build_existing_document_form(
            ctx.storage_dir(),
            ctx.pool(),
            ctx.company_id(),
            DocumentRef::AdjustmentAct(adj.id),
        )
        .await;
    }

    // Existing path for act / invoice / waybill
    let direction_str = request
        .direction
        .as_deref()
        .ok_or_else(|| anyhow!("direction є обов'язковим для документів цього типу"))?;
    let direction = DocumentDirection::try_from(direction_str.to_string())
        .map_err(|_| anyhow!("Невідома направленість документа"))?;
    let (counterparty_id, counterparty_name) =
        resolve_draft_counterparty(ctx, request.counterparty_id).await?;
    let form = create_draft_form(
        ctx.pool(),
        ctx.company_id(),
        counterparty_id,
        counterparty_name,
        &request.kind,
        direction,
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

- [ ] **Extend `document_save` — add AdjustmentAct branch**

Add to the match in `document_save`:
```rust
DocumentRef::AdjustmentAct(id) => {
    let update = UpdateAdjustmentAct {
        number: request.form.number.clone(),
        date,
        notes: optional_string(&request.form.notes),
    };
    let items: Vec<NewAdjustmentActItem> = request
        .items
        .into_iter()
        .map(|item| {
            Ok(NewAdjustmentActItem {
                description: item.description,
                quantity: parse_decimal_input(&item.quantity, "Кількість")?,
                unit_price: parse_decimal_input(&item.price, "Ціна")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    db::adjustment_acts::update_with_items_scoped(
        ctx.pool(), ctx.company_id(), id, update, items,
    )
    .await?
    .ok_or_else(|| anyhow!("Акт коригування не знайдено"))?;

    Ok(SaveDocumentResponse {
        document_id: request.form.id,
        kind: "adjustment_act".to_string(),
        message: "Акт коригування збережено".to_string(),
    })
}
```

- [ ] **Extend `document_advance_status` — add AdjustmentAct branch**

```rust
DocumentRef::AdjustmentAct(id) => {
    db::adjustment_acts::change_status_scoped(ctx.pool(), ctx.company_id(), id)
        .await?
        .ok_or_else(|| anyhow!("Акт коригування не знайдено"))?;
    "Статус акту коригування оновлено"
}
```

- [ ] **Extend `document_delete` — add AdjustmentAct branch**

```rust
DocumentRef::AdjustmentAct(id) => {
    if !db::adjustment_acts::delete_scoped(ctx.pool(), ctx.company_id(), id).await? {
        return Err(anyhow!("Акт коригування не знайдено"));
    }
}
```

- [ ] **Extend `document_chain_get` — handle AdjustmentAct (1-step chain)**

In `load_document_chain`, add a guard at the top of the function before the existing logic:

```rust
// For adjustment acts the chain concept doesn't apply — return single-step chain
if let DocumentRef::AdjustmentAct(id) = source {
    let adj = db::adjustment_acts::get_full(pool, company_id, id)
        .await?
        .ok_or_else(|| anyhow!("Акт коригування не знайдено"))?
        .0;

    return Ok(vec![ChainStepDto {
        doc_type: "adjustment_act".to_string(),
        doc_number: adj.number,
        amount_str: format_money_ua(adj.total_amount),
        status: adj.status.as_str().to_string(),
        exists: true,
    }]);
}
```

- [ ] **Compile check**

```powershell
cargo build --lib 2>&1 | Select-String -Pattern "error"
```

Expected: no errors.

- [ ] **Commit**

```bash
git add src/tauri_api/documents/dto.rs src/tauri_api/documents/api.rs
git commit -m "feat(api): add AdjustmentAct to document layer — parse, list, open, create, save, advance, delete, chain"
```

---

## Task 7: Tauri Command + Wiring

**Files:**
- Modify: `src-tauri/src/commands/documents.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `frontend/src/lib/api.ts`

- [ ] **Add `act_adjustments_list` to `src-tauri/src/commands/documents.rs`**

Add at the end of the file:
```rust
#[tauri::command]
pub async fn act_adjustments_list(
    state: State<'_, TauriState>,
    act_id: String,
) -> CommandResult<Vec<acta::tauri_api::documents::DocumentItemDto>> {
    let company_id = state.ctx.company_id();
    let pool = state.ctx.pool();

    let uuid = uuid::Uuid::parse_str(&act_id)
        .map_err(|_| format!("Некоректний act_id: {act_id}"))?;

    let rows = acta::db::adjustment_acts::list_for_act(pool, company_id, uuid)
        .await
        .map_err(|e| e.to_string())?;

    use acta::tauri_api::documents::{DocumentItemDto, DocumentKindDto, DocumentStatusDto};

    Ok(rows
        .into_iter()
        .map(|row| {
            let amount_str = {
                let v = row.total_amount.round_dp(2);
                let s = format!("{:.2}", v).replace('.', ",");
                let (whole, frac) = s.split_once(',').unwrap_or((&s, "00"));
                let grouped = whole.chars().rev().collect::<Vec<_>>()
                    .chunks(3)
                    .map(|c| c.iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars().rev().collect::<String>();
                format!("{grouped},{frac}")
            };
            DocumentItemDto {
                id: format!("adj:{}", row.id),
                kind: DocumentKindDto::AdjustmentAct,
                number: row.number,
                date: row.date.format("%d.%m.%Y").to_string(),
                counterparty: row.counterparty_name,
                amount_str,
                status: DocumentStatusDto::from_adjustment_act_status(&row.status),
                status_label: row.status.label().to_string(),
                linked_id: row.original_act_id.to_string(),
                direction: row.direction.as_str().to_string(),
            }
        })
        .collect())
}
```

- [ ] **Register command in `src-tauri/src/lib.rs`**

After line 85 (`commands::documents::document_pdf_open_current,`), add:
```rust
commands::documents::act_adjustments_list,
```

- [ ] **Add API wrapper to `frontend/src/lib/api.ts`**

Add after `documentChainCreateDraft`:
```typescript
export function documentCreateAdjustmentActDraft(
  originalActId: string
): Promise<DocumentEditorDto> {
  return appInvoke("document_create_draft", {
    request: {
      kind: "adjustment_act",
      originalActId,
      direction: null,
      counterpartyId: null,
    }
  });
}

export function actAdjustmentsList(actId: string): Promise<DocumentItemDto[]> {
  return appInvoke("act_adjustments_list", { actId });
}
```

- [ ] **Compile check**

```powershell
cargo build --tests 2>&1 | Select-String -Pattern "error"
```

Expected: no errors.

- [ ] **Commit**

```bash
git add src-tauri/src/commands/documents.rs src-tauri/src/lib.rs frontend/src/lib/api.ts
git commit -m "feat(tauri): add act_adjustments_list command and API wrapper"
```

---

## Task 8: PDF Generation

**Files:**
- Create: `templates/adjustment_act.typ`
- Modify: `src/pdf/generator.rs`
- Modify: `src/tauri_api/documents/pdf.rs`

- [ ] **Create `templates/adjustment_act.typ`**

```typst
// АКТ КОРИГУВАННЯ — Typst шаблон
// Дані передаються через: --input 'data=<JSON рядок>'

#import sys: inputs

#let raw = inputs.at("data", default: "{}")
#let d = json(bytes(raw))

#let company = d.company
#let client  = d.client
#let items   = d.items

#set page(
  paper:  "a4",
  margin: (top: 20mm, bottom: 20mm, left: 20mm, right: 20mm),
)
#set text(font: ("Libertinus Serif", "FreeSerif", "DejaVu Serif"), size: 10pt, lang: "uk")
#set par(justify: false)

#let label-style = text.with(size: 8pt, fill: luma(100))
#let value-style = text.with(size: 10pt)
#let bold        = text.with(weight: "bold")

#align(center)[
  #text(size: 13pt, weight: "bold")[
    АКТ КОРИГУВАННЯ № #d.number від #d.date р.
  ]
  #v(2mm)
  #text(size: 10pt, fill: luma(80))[
    до Акту виконаних робіт № #d.original_act_number
  ]
]

#v(6mm)

#let reqs-cell(header, name, edrpou, iban, address: none) = [
  #block(
    stroke: 0.5pt + luma(160), inset: (x: 5mm, y: 4mm),
    radius: 2pt, width: 100%,
  )[
    #bold[#header] \
    #v(1mm)
    #label-style[Найменування:] \
    #value-style[#name] \
    #v(1mm)
    #label-style[ЄДРПОУ/ІПН:] \
    #value-style[#edrpou] \
    #v(1mm)
    #label-style[IBAN:] \
    #value-style[#iban]
    #if address != none [
      #v(1mm)
      #label-style[Адреса:] \
      #value-style[#address]
    ]
  ]
]

#grid(columns: (1fr, 1fr), gutter: 6mm,
  reqs-cell("Виконавець", company.name, company.edrpou, company.iban, address: company.address),
  reqs-cell("Замовник",   client.name,  client.edrpou,  client.iban,  address: client.address),
)

#v(6mm)

#table(
  columns: (auto, 1fr, auto, auto, auto, auto),
  inset: (x: 4mm, y: 3mm),
  stroke: 0.5pt + luma(160),
  fill: (x, y) => if y == 0 { luma(235) } else { white },
  align: (center, left, center, center, right, right),
  [*№*], [*Найменування*], [*Кіл.*], [*Од.*], [*Ціна*], [*Сума*],
  ..items.map(i => (
    str(i.num), i.name, i.qty, i.unit, i.price, i.amount
  )).flatten()
)

#v(4mm)
#align(right)[
  *Разом до коригування:*  #d.total грн \
  #text(size: 9pt)[#d.total_words]
]

#if d.notes != "" [
  #v(4mm)
  #label-style[Примітки:] #value-style[#d.notes]
]

#v(10mm)
#grid(columns: (1fr, 1fr), gutter: 6mm,
  [
    *Від виконавця:* \
    #v(8mm)
    #line(length: 100%, stroke: 0.5pt)
    #text(size: 8pt)[підпис / П.І.Б.]
  ],
  [
    *Від замовника:* \
    #v(8mm)
    #line(length: 100%, stroke: 0.5pt)
    #text(size: 8pt)[підпис / П.І.Б.]
  ],
)
```

- [ ] **Add to `src/pdf/generator.rs`**

Add after `PdfActData` struct (after line 57):
```rust
#[derive(Debug, Serialize)]
pub struct PdfAdjustmentActData {
    pub number: String,
    pub original_act_number: String,
    pub date: String,
    pub company: PdfCompany,
    pub client: PdfCompany,
    pub items: Vec<PdfActItem>,
    pub total: String,
    pub total_words: String,
    pub notes: String,
}
```

Add after `ensure_output_dir` function:
```rust
/// `storage/documents/adjustment_acts/{рік}/{number}.pdf`
pub fn ensure_adj_output_dir(storage_dir: &Path, number: &str) -> Result<PathBuf> {
    let year = chrono::Utc::now().year();
    let dir = storage_dir.join("documents").join("adjustment_acts").join(year.to_string());
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Не вдалось створити директорію: {}", dir.display()))?;
    let safe_number = number.replace('/', "_");
    Ok(dir.join(format!("{safe_number}.pdf")))
}

pub fn generate_adjustment_act_pdf(
    data: &PdfAdjustmentActData,
    template_path: &Path,
    output_path: &Path,
) -> Result<()> {
    let json = serde_json::to_string(data)
        .context("Серіалізація PdfAdjustmentActData у JSON")?;

    let output = std::process::Command::new("typst")
        .args([
            "compile",
            template_path.to_str().context("Невалідний шлях до шаблону adjustment_act.typ")?,
            output_path.to_str().context("Невалідний шлях до output PDF")?,
            "--input",
            &format!("data={json}"),
        ])
        .output()
        .context("Не вдалось запустити typst")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("typst завершився з помилкою:\n{stderr}");
    }

    tracing::info!(path = %output_path.display(), "PDF акту коригування згенеровано");
    Ok(())
}
```

- [ ] **Extend `src/tauri_api/documents/pdf.rs`**

Add to `load_existing_pdf_path` match:
```rust
DocumentRef::AdjustmentAct(_) => None,
```

Add to `load_document_kind_and_number` match:
```rust
DocumentRef::AdjustmentAct(id) => {
    let (adj, _) = db::adjustment_acts::get_full(pool, company_id, id)
        .await?
        .ok_or_else(|| anyhow!("Акт коригування не знайдено"))?;
    Ok(("adjustment_act".to_string(), adj.number))
}
```

Change `document_ref_uuid` to:
```rust
pub(super) fn document_ref_uuid(doc_ref: DocumentRef) -> Uuid {
    match doc_ref {
        DocumentRef::Act(id)
        | DocumentRef::Invoice(id)
        | DocumentRef::Waybill(id)
        | DocumentRef::AdjustmentAct(id) => id,
    }
}
```

Extend `generate_document_pdf` — add before `DocumentRef::Waybill(_)` bailout:
```rust
DocumentRef::AdjustmentAct(id) => {
    let (adj, items) = db::adjustment_acts::get_full(pool, company_id, id)
        .await?
        .ok_or_else(|| anyhow!("Акт коригування не знайдено"))?;

    let original_number: String = sqlx::query_scalar(
        "SELECT number FROM acts WHERE id = $1"
    )
    .bind(adj.original_act_id)
    .fetch_one(pool)
    .await?;

    let (company_res, counterparty_res) = tokio::join!(
        db::companies::get_by_id(pool, company_id),
        db::counterparties::get_by_id(pool, company_id, adj.counterparty_id)
    );
    let company = company_res?.ok_or_else(|| anyhow!("Компанію не знайдено"))?;
    let counterparty = counterparty_res?.ok_or_else(|| anyhow!("Контрагента не знайдено"))?;

    use crate::pdf::generator::{
        PdfAdjustmentActData, PdfActItem, amount_to_words,
        ensure_adj_output_dir, generate_adjustment_act_pdf,
    };

    let data = PdfAdjustmentActData {
        number: adj.number.clone(),
        original_act_number: original_number,
        date: adj.date.format("%d.%m.%Y").to_string(),
        company: to_pdf_company(&company),
        client: counterparty_to_pdf_company(&counterparty),
        items: items
            .iter()
            .enumerate()
            .map(|(i, item)| PdfActItem {
                num: (i + 1) as u32,
                name: item.description.clone(),
                qty: format!("{:.4}", item.quantity),
                unit: String::new(),
                price: format!("{:.2}", item.unit_price),
                amount: format!("{:.2}", item.total_price),
            })
            .collect(),
        total: format!("{:.2}", adj.total_amount),
        total_words: amount_to_words(&adj.total_amount),
        notes: adj.notes.clone().unwrap_or_default(),
    };

    let path = ensure_adj_output_dir(ctx.storage_dir(), &adj.number)?;
    let template = ctx.template_dir().join("adjustment_act.typ");
    let out = path.clone();
    tokio::task::spawn_blocking(move || generate_adjustment_act_pdf(&data, &template, &out))
        .await
        .context("PDF thread error")??;
    path
}
```

Also update `persist_existing_pdf_path` match to add:
```rust
DocumentRef::AdjustmentAct(_) => {
    anyhow::bail!("Для актів коригування flow існуючого PDF не підтримується")
}
```

- [ ] **Compile check**

```powershell
cargo build --lib 2>&1 | Select-String -Pattern "error"
```

Expected: no errors.

- [ ] **Commit**

```bash
git add templates/adjustment_act.typ src/pdf/generator.rs src/tauri_api/documents/pdf.rs
git commit -m "feat(pdf): add adjustment_act PDF generation with Typst template"
```

---

## Task 9: Frontend Types + Fixtures

**Files:**
- Modify: `frontend/src/lib/types.ts`
- Modify: `frontend/src/lib/browser-fixtures.ts`

- [ ] **Update `frontend/src/lib/types.ts`**

Find `DocumentKind` and add `"adjustment_act"`:
```typescript
// BEFORE:
export type DocumentKind = "invoice" | "act" | "waybill";

// AFTER:
export type DocumentKind = "invoice" | "act" | "waybill" | "adjustment_act";
```

Find `DocumentStatus` and add `"applied"`:
```typescript
// BEFORE:
export type DocumentStatus = "draft" | "issued" | "signed" | "paid" | "delivered";

// AFTER:
export type DocumentStatus = "draft" | "issued" | "signed" | "paid" | "delivered" | "applied";
```

Find `DocumentsListDto` and add `adjustmentActItems`:
```typescript
// BEFORE:
export interface DocumentsListDto {
  items: DocumentItemDto[];
  invoiceItems: DocumentItemDto[];
  actItems: DocumentItemDto[];
  waybillItems: DocumentItemDto[];
  totalCount: number;
  pageCount: number;
}

// AFTER:
export interface DocumentsListDto {
  items: DocumentItemDto[];
  invoiceItems: DocumentItemDto[];
  actItems: DocumentItemDto[];
  waybillItems: DocumentItemDto[];
  adjustmentActItems: DocumentItemDto[];
  totalCount: number;
  pageCount: number;
}
```

Find `DocumentDraftFormDto` and add optional fields:
```typescript
// Add to interface:
originalActId?: string;
originalActNumber?: string;
```

- [ ] **Update `frontend/src/lib/browser-fixtures.ts`**

Find `documentsList()` function — add `adjustmentActItems: []` to the returned object:
```typescript
// In the DocumentsListDto literal, add:
adjustmentActItems: [],
```

- [ ] **Compile check**

```powershell
cd frontend && npm run check 2>&1 | Select-String -Pattern "Error"
```

Expected: type errors in `config/documents.ts` and `DocumentsScreen.svelte` because `"adjustment_act"` is not yet in `DOCUMENT_KIND_META`. These will be fixed in Task 10.

- [ ] **Commit** (after Task 10 passes)

---

## Task 10: Frontend Config

**Files:**
- Modify: `frontend/src/lib/config/documents.ts`

- [ ] **Add `adjustment_act` to `DOCUMENT_KIND_META`**

Find `DOCUMENT_KIND_META` and add:
```typescript
adjustment_act: {
  label: "Коригування",
  labelShort: "КОР",
  icon: "↔",
  colorClass: "kind-adj",
},
```

- [ ] **Add to `DOCUMENT_KIND_FILTER_OPTIONS` only (NOT to `DOCUMENT_KIND_OPTIONS`)**

```typescript
// DOCUMENT_KIND_FILTER_OPTIONS — add:
{ value: "adjustment_act", label: "Коригування" },

// DOCUMENT_KIND_OPTIONS — do NOT add (adj acts created only from act drawer, not create picker)
```

- [ ] **Extend `resolveDocumentKindMeta` — add `adjustment_act` branch**

The function uses string matching. Add before the `default` return:
```typescript
if (normalized === "adjustment_act") return DOCUMENT_KIND_META.adjustment_act;
```

- [ ] **Extend `supportsDocumentPdfGeneration`**

```typescript
// BEFORE:
export function supportsDocumentPdfGeneration(kind: DocumentKind): boolean {
  return kind === "act" || kind === "invoice";
}

// AFTER:
export function supportsDocumentPdfGeneration(kind: DocumentKind): boolean {
  return kind === "act" || kind === "invoice" || kind === "adjustment_act";
}
```

- [ ] **Add `applied` to `DOCUMENT_STATUS_OPTIONS`**

```typescript
// Add to array:
{ value: "applied", label: "Застосовано" },
```

- [ ] **Compile check**

```powershell
cd frontend && npm run check 2>&1 | Select-String -Pattern "Error"
```

Expected: no type errors from config.

- [ ] **Commit types + config together**

```bash
git add frontend/src/lib/types.ts frontend/src/lib/browser-fixtures.ts frontend/src/lib/config/documents.ts
git commit -m "feat(frontend): add adjustment_act types, config metadata, and status"
```

---

## Task 11: Frontend Store

**Files:**
- Modify: `frontend/src/lib/stores/documents.ts`

- [ ] **Add `createAdjustmentActDraft` method to the documents store**

Find the store's public API (where `openNewEditor`, `create` etc. are defined) and add:

```typescript
async createAdjustmentActDraft(actId: string): Promise<void> {
  const draft = await documentCreateAdjustmentActDraft(actId);
  // Open the new adj act draft in the editor (same pattern as openNewEditor)
  // The exact implementation depends on how the store tracks pendingNew/openEditor.
  // Pattern: set the editor to the returned draft, mark dirty=false, show drawer.
  this._openEditorFromDto(draft);
},
```

The exact pattern for `_openEditorFromDto` depends on the existing store structure. Look at how `create` opens the editor and replicate it for the adj draft.

- [ ] **Compile check**

```powershell
cd frontend && npm run check 2>&1 | Select-String -Pattern "Error"
```

Expected: no errors.

- [ ] **Commit**

```bash
git add frontend/src/lib/stores/documents.ts
git commit -m "feat(store): add createAdjustmentActDraft method to documents store"
```

---

## Task 12: DocumentsScreen — Adj-Specific UI

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`

- [ ] **Disable Create button for adjustment_act kind filter**

Find the Create button in the header area:
```svelte
<!-- Find the Create button and add disabled condition: -->
<button
  class="btn-primary"
  disabled={$store.kindFilter === "adjustment_act"}
  onclick={() => store.openNewEditor()}
>
  Створити
</button>
```

- [ ] **Add "+ Коригування" button in the act drawer**

In the drawer section where `kind === "act"` (or `doc.kind === "act"`), add:
```svelte
{#if doc.kind === "act"}
  <button
    class="btn-secondary"
    onclick={() => store.createAdjustmentActDraft(doc.id.replace("act:", ""))}
  >
    + Коригування
  </button>
{/if}
```

- [ ] **Show "Оригінальний акт" read-only field for adj acts**

In the drawer form area, add conditional:
```svelte
{#if form.kind === "adjustment_act" && form.originalActNumber}
  <div class="field-row">
    <label>Оригінальний акт</label>
    <span class="field-value-readonly">{form.originalActNumber}</span>
  </div>
{/if}
```

- [ ] **Make direction read-only for adj acts**

Find the direction selector/display. Add condition:
```svelte
{#if form.kind === "adjustment_act"}
  <span class="field-value-readonly">{form.direction === "outgoing" ? "Вихідний" : "Вхідний"}</span>
{:else}
  <!-- existing direction selector -->
{/if}
```

- [ ] **Hide "Змінити контрагента" button for adj acts**

Find the counterparty change button:
```svelte
{#if form.kind !== "adjustment_act"}
  <button class="btn-link" onclick={...}>Змінити</button>
{/if}
```

- [ ] **Svelte check**

```powershell
cd frontend && npm run check 2>&1 | Select-String -Pattern "Error"
```

Expected: no errors.

- [ ] **Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte
git commit -m "feat(ui): add adjustment act drawer UI and Create button guard"
```

---

## Task 13: Reports + Dashboard — Effective Amounts

**Files:**
- Modify: `src/db/reports.rs` (4 functions)
- Modify: `src/db/dashboard.rs` (7 functions)

### reports.rs

The SQL **Паттерн А** (scalar subquery) for all 4 functions:
```sql
a.total_amount + COALESCE(
    (SELECT SUM(aa.total_amount) FROM adjustment_acts aa
     WHERE aa.original_act_id = a.id AND aa.status = 'applied'),
    0
) AS amount
```

- [ ] **Update `load_pnl_rows` — acts CTE branch**

Find `a.total_amount AS amount` inside the `WITH docs AS (` CTE in `load_pnl_rows` (line ~205) and replace with the scalar subquery above.

- [ ] **Update `load_receivables_rows` — acts SELECT branch**

Find `a.total_amount AS amount` in the acts SELECT of `load_receivables_rows` (line ~311) and replace with scalar subquery.

- [ ] **Update `load_top_counterparties_receivables` — docs CTE acts branch**

Find `a.total_amount AS amount` in the CTE of `load_top_counterparties_receivables` (line ~638) and replace.

- [ ] **Update `load_top_counterparties_pnl` — docs CTE acts branch**

Find `a.total_amount AS amount` in the CTE of `load_top_counterparties_pnl` (line ~833) and replace.

### dashboard.rs

The SQL **Паттерн Б** (CTE + LEFT JOIN) for aggregate functions:
```sql
WITH adj_sums AS (
    SELECT original_act_id, SUM(total_amount) AS adj_total
    FROM adjustment_acts WHERE status = 'applied'
    GROUP BY original_act_id
)
-- FROM acts a LEFT JOIN adj_sums adj ON adj.original_act_id = a.id
-- use: a.total_amount + COALESCE(adj.adj_total, 0)
```

- [ ] **Update `get_kpi_summary` — Паттерн Б**

Replace the SQL in `get_kpi_summary` with:
```sql
WITH adj_sums AS (
    SELECT original_act_id, SUM(total_amount) AS adj_total
    FROM adjustment_acts WHERE status = 'applied'
    GROUP BY original_act_id
)
SELECT
    COALESCE(SUM(a.total_amount + COALESCE(adj.adj_total, 0)) FILTER (
        WHERE a.status = 'paid'
          AND date_trunc('month', a.date) = date_trunc('month', CURRENT_DATE)
    ), 0) AS revenue_this_month,

    COALESCE(SUM(a.total_amount + COALESCE(adj.adj_total, 0)) FILTER (
        WHERE a.status IN ('issued', 'signed')
    ), 0) AS unpaid_total,

    COUNT(*) FILTER (
        WHERE date_trunc('month', a.date) = date_trunc('month', CURRENT_DATE)
    ) AS acts_this_month,

    (SELECT COUNT(*) FROM counterparties
     WHERE company_id = $1 AND is_archived = FALSE
    ) AS active_counterparties

FROM acts a
LEFT JOIN adj_sums adj ON adj.original_act_id = a.id
WHERE a.company_id = $1
```

- [ ] **Update `revenue_by_month` — Паттерн Б**

Replace SQL with:
```sql
WITH adj_sums AS (
    SELECT original_act_id, SUM(total_amount) AS adj_total
    FROM adjustment_acts WHERE status = 'applied'
    GROUP BY original_act_id
)
SELECT
    EXTRACT(MONTH FROM date_trunc('month', a.date))::int AS month_num,
    EXTRACT(YEAR  FROM date_trunc('month', a.date))::int AS year_num,
    COALESCE(SUM(
        CASE WHEN a.status = 'paid'
             THEN a.total_amount + COALESCE(adj.adj_total, 0)
             ELSE 0 END
    ), 0) AS amount
FROM acts a
LEFT JOIN adj_sums adj ON adj.original_act_id = a.id
WHERE a.company_id = $1
  AND a.date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'
GROUP BY date_trunc('month', a.date)
ORDER BY date_trunc('month', a.date) ASC
```

- [ ] **Update `expenses_by_month` — Паттерн А in CTE acts branch**

In `expense_docs` CTE, change the acts SELECT to:
```sql
SELECT a.date,
       a.total_amount + COALESCE(
           (SELECT SUM(aa.total_amount) FROM adjustment_acts aa
            WHERE aa.original_act_id = a.id AND aa.status = 'applied'),
           0
       ) AS amount
FROM acts a
JOIN categories c ON c.id = a.category_id
WHERE a.company_id = $1
  AND c.kind = 'expense'
  AND a.date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'
```

- [ ] **Update `category_breakdown` — Паттерн А in CTE acts branch**

Same pattern as `expenses_by_month`:
```sql
SELECT c.name AS label,
       a.total_amount + COALESCE(
           (SELECT SUM(aa.total_amount) FROM adjustment_acts aa
            WHERE aa.original_act_id = a.id AND aa.status = 'applied'),
           0
       ) AS amount
FROM acts a
JOIN categories c ON c.id = a.category_id
WHERE a.company_id = $1
  AND c.kind = 'expense'
  AND a.date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'
```

- [ ] **Update `upcoming_payments` — Паттерн А**

Change `a.total_amount AS amount` to:
```sql
a.total_amount + COALESCE(
    (SELECT SUM(aa.total_amount) FROM adjustment_acts aa
     WHERE aa.original_act_id = a.id AND aa.status = 'applied'),
    0
) AS amount
```

- [ ] **Update `get_recent_acts` — Паттерн А**

Change `a.total_amount AS amount` to the scalar subquery.

- [ ] **Update `inbox_items` — Паттерн А**

Change `a.total_amount AS amount` to the scalar subquery in the acts branch of the UNION ALL.

- [ ] **Compile check**

```powershell
cargo build --lib 2>&1 | Select-String -Pattern "error"
```

Expected: no errors.

- [ ] **Run full test suite**

```powershell
$env:TEST_DATABASE_URL="postgres://postgres:password@localhost:5432/acta"
cargo test 2>&1 | tail -20
```

Expected: all tests pass including adjustment_act integration tests.

- [ ] **Commit**

```bash
git add src/db/reports.rs src/db/dashboard.rs
git commit -m "feat(reports): use effective_amount (incl. applied adjustments) in all report queries"
```

---

## Self-Review

**Spec coverage check:**
- ✅ Migration 030: Task 1
- ✅ `is_adjusted` flag: Task 1 + Task 3 (change_status/delete)
- ✅ Rust models: Task 2
- ✅ DB CRUD with security: Task 3 (create verifies company_id, copies counterparty/direction)
- ✅ Integration tests (7 total incl. issued/signed + constraint): Task 4
- ✅ DTOs: Task 5
- ✅ `adj:` prefix + all DocumentRef branches: Task 6
- ✅ `act_adjustments_list` with full wiring: Task 7
- ✅ PDF template + generator + pdf.rs: Task 8
- ✅ Frontend types + fixtures: Task 9
- ✅ Frontend config (filter only, not create picker): Task 10
- ✅ `createAdjustmentActDraft` store method: Task 11
- ✅ Drawer UI (adj fields, "+Коригування", disabled create): Task 12
- ✅ Reports (4) + Dashboard (7) effective amounts with `status = 'applied'`: Task 13

**Placeholder check:** Task 11 mentions `_openEditorFromDto` as a pattern — the actual method name depends on how the existing store opens editors. Read `documents.ts` store before implementing and mirror the existing pattern exactly.

**Type consistency:**
- `AdjustmentActStatus` used in models, db, api, commands — consistent
- `DocumentRef::AdjustmentAct` used in api.rs and pdf.rs — consistent
- `DocumentKindDto::AdjustmentAct` used in api.rs and commands.rs — consistent
- `adj:` prefix used in `parse_document_ref`, `document_ref_string`, and frontend `id` format — consistent
- `linked_id = original_act_id.to_string()` in list results — consistent with spec

---

**Plan complete and saved to `docs/superpowers/plans/2026-05-21-adjustment-acts.md`.**
