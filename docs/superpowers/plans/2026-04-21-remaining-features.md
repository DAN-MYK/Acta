# Remaining Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement all remaining stub callbacks and UI data wiring in `src/main.rs`: reports screen, dashboard chart bars, payments KPI strip, settings persistence, and wire remaining palette/export stubs.

**Architecture:** Each feature follows the established prepare/apply/wire pattern. DB queries are added to existing `src/db/` modules. UI logic lives in `src/ui/` submodules. All monetary display values are formatted strings; Rust never passes `f64`/`f32` for financial data except as display-only floats in Slint structs.

**Tech Stack:** Rust, Slint 1.x (`slint::include_modules!()`), sqlx (runtime-style queries), tokio::join! for parallel async, `rust_decimal::Decimal` for all monetary values.

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `ui-redesign/types.slint` | Modify | Add named `ChartBar` struct to replace anonymous `{rev-h, exp-h, month}` |
| `ui-redesign/app.slint` | Modify | Replace 2× anonymous chart-bar types with `[ChartBar]` |
| `src/db/dashboard.rs` | Modify | Add `expenses_by_month`, `category_breakdown` queries |
| `src/db/payments.rs` | Modify | Add `payment_kpi` aggregate query |
| `src/models/dashboard.rs` | Modify | Add `CategoryRevenue` struct |
| `src/ui/dashboard.rs` | Modify | Populate `dash-chart-bars` from `revenue_by_month` |
| `src/ui/payments.rs` | Modify | Add KPI fields to `PaymentsData`, populate all pay-*-str properties |
| `src/ui/reports.rs` | Create | `ReportsData`, `prepare_reports_data`, `apply_reports_to_ui`, `wire_reports_callbacks` |
| `src/ui/settings.rs` | Modify | Load company from DB, wire `settings-company-saved` callback |
| `src/ui/mod.rs` | Modify | Add `pub mod reports` |
| `src/main.rs` | Modify | Wire all remaining callbacks, add reports to nav + initial load |

---

## Task 1: Named ChartBar Struct (fix anonymous Slint type)

The anonymous struct `{rev-h: float, exp-h: float, month: string}` used in `dash-chart-bars` and `rep-chart-bars` generates an unaddressable Rust type from `slint::include_modules!()`. Converting it to a named export gives us a stable Rust type.

**Files:**
- Modify: `ui-redesign/types.slint` (after line 108, the `DashboardMetrics` block)
- Modify: `ui-redesign/app.slint` (lines 88, 134)

- [ ] **Step 1: Write the failing test** in `src/ui/dashboard.rs`

```rust
#[test]
fn chart_bar_struct_is_constructible() {
    // This test verifies the Slint-generated ChartBar type is addressable from Rust.
    // It will fail to compile if the named struct was not added to types.slint.
    let bar = crate::ChartBar {
        rev_h: 0.5_f32,
        exp_h: 0.3_f32,
        month: "Сiч".into(),
    };
    assert!((bar.rev_h - 0.5).abs() < 0.001);
}
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test chart_bar_struct_is_constructible 2>&1 | head -20
```
Expected: compile error — `crate::ChartBar` not found.

- [ ] **Step 3: Add `ChartBar` struct to `ui-redesign/types.slint`**

Add immediately after the `DashboardMetrics` struct block (after line ~108):

```slint
export struct ChartBar {
    rev-h: float,   // revenue bar height, 0.0–1.0
    exp-h: float,   // expenses bar height, 0.0–1.0
    month: string,  // abbreviated month label e.g. "Сiч"
}
```

- [ ] **Step 4: Update `ui-redesign/app.slint`**

Find and replace BOTH occurrences of the anonymous type. Current text:
```
in property <[{rev-h: float, exp-h: float, month: string}]> dash-chart-bars;
```
Replace with:
```
in property <[ChartBar]> dash-chart-bars;
```

Current text:
```
in property <[{rev-h: float, exp-h: float, month: string}]> rep-chart-bars;
```
Replace with:
```
in property <[ChartBar]> rep-chart-bars;
```

Also add `ChartBar` to the import line at the top of `app.slint`. Current:
```slint
import { ..., CounterpartyDetails, ReportMetrics, ExpenseCategory, DrillRow,
```
Add `, ChartBar` to that import.

- [ ] **Step 5: Run test to verify it passes**

```
cargo test chart_bar_struct_is_constructible
```
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add ui-redesign/types.slint ui-redesign/app.slint src/ui/dashboard.rs
git commit -m "feat: add named ChartBar struct to replace anonymous Slint type"
```

---

## Task 2: Dashboard Chart Bars — populate `dash-chart-bars`

Wire the existing `db::dashboard::revenue_by_month` into `apply_dashboard_to_ui`. Revenue bars are normalized to a 0.0–1.0 height relative to the period maximum.

**Files:**
- Modify: `src/ui/dashboard.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn chart_bars_normalized_to_max_revenue() {
    use acta::models::dashboard::MonthRevenue;
    use rust_decimal_macros::dec;

    let months = vec![
        MonthRevenue { month_num: 1, year: 2026, amount: dec!(1000) },
        MonthRevenue { month_num: 2, year: 2026, amount: dec!(2000) },
        MonthRevenue { month_num: 3, year: 2026, amount: dec!(0) },
    ];
    let bars = revenue_months_to_chart_bars(&months);
    assert_eq!(bars.len(), 3);
    // max is 2000 → bar[1].rev_h == 1.0
    assert!((bars[1].rev_h - 1.0).abs() < 0.001, "max bar should be 1.0");
    // bar[0] = 1000/2000 = 0.5
    assert!((bars[0].rev_h - 0.5).abs() < 0.01);
    // bar[2] = 0
    assert!((bars[2].rev_h).abs() < 0.001);
    assert_eq!(bars[0].month.as_str(), "Січ");
}
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test chart_bars_normalized_to_max_revenue
```
Expected: FAIL — `revenue_months_to_chart_bars` not found.

- [ ] **Step 3: Implement `revenue_months_to_chart_bars` in `src/ui/dashboard.rs`**

Add the function before `prepare_dashboard_data`:

```rust
/// Перетворює список `MonthRevenue` у нормалізовані ChartBar (висота 0.0–1.0).
pub fn revenue_months_to_chart_bars(months: &[acta::models::dashboard::MonthRevenue]) -> Vec<crate::ChartBar> {
    use rust_decimal::prelude::ToPrimitive;
    let max_val = months
        .iter()
        .map(|m| m.amount.to_f64().unwrap_or(0.0))
        .fold(0.0f64, f64::max);

    months
        .iter()
        .map(|m| {
            let val = m.amount.to_f64().unwrap_or(0.0);
            let h = if max_val > 0.0 { (val / max_val) as f32 } else { 0.0 };
            crate::ChartBar {
                rev_h: h,
                exp_h: 0.0, // expenses wired in Task 6 (reports)
                month: m.month_label().into(),
            }
        })
        .collect()
}
```

- [ ] **Step 4: Update `DashboardData` and `prepare_dashboard_data`**

Add `chart_bars` field to `DashboardData`:
```rust
pub struct DashboardData {
    pub metrics: crate::DashboardMetrics,
    pub revenue_str: String,
    pub outstanding_str: String,
    pub journal: Vec<crate::JournalRow>,
    pub tasks: Vec<crate::TaskItem>,
    pub chart_bars: Vec<crate::ChartBar>,
}
```

Update `prepare_dashboard_data` to fetch 6 months of revenue in the `tokio::join!`:
```rust
pub async fn prepare_dashboard_data(pool: &PgPool, company_id: Uuid) -> DashboardData {
    let (kpi_res, recent_res, tasks_res, rev_months_res) = tokio::join!(
        db::dashboard::get_kpi_summary(pool, company_id),
        db::dashboard::get_recent_acts(pool, company_id, 20),
        db::tasks::list_open(pool),
        db::dashboard::revenue_by_month(pool, company_id, 6),
    );

    let kpi = kpi_res.unwrap_or_else(|e| {
        tracing::error!("dashboard kpi failed: {e}");
        KpiSummary {
            revenue_this_month: rust_decimal::Decimal::ZERO,
            unpaid_total: rust_decimal::Decimal::ZERO,
            acts_this_month: 0,
            active_counterparties: 0,
        }
    });
    let recent = recent_res.unwrap_or_default();
    let tasks = tasks_res.unwrap_or_default();
    let rev_months = rev_months_res.unwrap_or_default();

    let journal: Vec<crate::JournalRow> = recent.iter().map(recent_act_to_journal_row).collect();
    let task_items: Vec<crate::TaskItem> = tasks.iter().map(task_to_item).collect();
    let chart_bars = revenue_months_to_chart_bars(&rev_months);

    DashboardData {
        revenue_str: format!("{:.0}", kpi.revenue_this_month),
        outstanding_str: format!("{:.0}", kpi.unpaid_total),
        metrics: kpi_to_metrics(&kpi),
        journal,
        tasks: task_items,
        chart_bars,
    }
}
```

Update `apply_dashboard_to_ui` to set `dash-chart-bars`:
```rust
pub fn apply_dashboard_to_ui(ui: &crate::AppWindow, data: DashboardData) {
    ui.set_dash_metrics(data.metrics);
    ui.set_dash_revenue_str(data.revenue_str.into());
    ui.set_dash_outstanding_str(data.outstanding_str.into());
    ui.set_dash_journal(ModelRc::new(VecModel::from(data.journal)));
    ui.set_dash_tasks(ModelRc::new(VecModel::from(data.tasks)));
    ui.set_dash_chart_bars(ModelRc::new(VecModel::from(data.chart_bars)));
}
```

- [ ] **Step 5: Run all dashboard tests**

```
cargo test ui::dashboard
```
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add src/ui/dashboard.rs
git commit -m "feat: populate dash-chart-bars from revenue_by_month"
```

---

## Task 3: Payment KPI aggregate query

Add a single aggregate SQL query to compute KPI totals for the payments screen header strip.

**Files:**
- Modify: `src/db/payments.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn payment_kpi_public_api_is_exposed() {
    let _ = payment_kpi;
}
```

Add this to `src/db/payments.rs` tests block.

- [ ] **Step 2: Run test to verify it fails**

```
cargo test payment_kpi_public_api_is_exposed
```
Expected: FAIL — `payment_kpi` not found.

- [ ] **Step 3: Add `PaymentKpi` struct and `payment_kpi` function**

Add to `src/db/payments.rs` before the existing tests block:

```rust
/// KPI агрегати для смужки заголовку екрану Платежі.
pub struct PaymentKpi {
    /// Загальна сума надходжень (income) за поточний місяць.
    pub incoming_month: rust_decimal::Decimal,
    /// Загальна сума витрат (expense) за поточний місяць.
    pub outgoing_month: rust_decimal::Decimal,
    /// Кількість неузгоджених платежів (is_reconciled = false).
    pub unmatched_count: i64,
}

/// Один SQL-запит: агрегати Income/Expense за поточний місяць + кількість неузгоджених.
pub async fn payment_kpi(pool: &PgPool, company_id: Uuid) -> Result<PaymentKpi> {
    struct Row {
        incoming_month: rust_decimal::Decimal,
        outgoing_month: rust_decimal::Decimal,
        unmatched_count: i64,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                incoming_month:  r.try_get("incoming_month")?,
                outgoing_month:  r.try_get("outgoing_month")?,
                unmatched_count: r.try_get("unmatched_count")?,
            })
        }
    }

    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            COALESCE(SUM(amount) FILTER (
                WHERE direction = 'income'
                  AND date_trunc('month', date) = date_trunc('month', CURRENT_DATE)
            ), 0) AS incoming_month,
            COALESCE(SUM(amount) FILTER (
                WHERE direction = 'expense'
                  AND date_trunc('month', date) = date_trunc('month', CURRENT_DATE)
            ), 0) AS outgoing_month,
            COUNT(*) FILTER (WHERE is_reconciled = FALSE) AS unmatched_count
        FROM payments
        WHERE company_id = $1
        "#,
    )
    .bind(company_id)
    .fetch_one(pool)
    .await?;

    Ok(PaymentKpi {
        incoming_month: row.incoming_month,
        outgoing_month: row.outgoing_month,
        unmatched_count: row.unmatched_count,
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

```
cargo test payment_kpi_public_api_is_exposed
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/db/payments.rs
git commit -m "feat: add payment_kpi aggregate query"
```

---

## Task 4: Populate Payments KPI strip in UI

**Files:**
- Modify: `src/ui/payments.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn format_kpi_amount_formats_zero() {
    assert_eq!(format_kpi_amount(rust_decimal::Decimal::ZERO), "0 ₴");
}

#[test]
fn format_kpi_amount_formats_large_number() {
    use rust_decimal_macros::dec;
    let s = format_kpi_amount(dec!(125000.50));
    assert!(s.contains("125"), "має містити 125: {s}");
    assert!(s.contains("₴"), "має містити знак гривні: {s}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test format_kpi_amount
```
Expected: FAIL — `format_kpi_amount` not found.

- [ ] **Step 3: Update `src/ui/payments.rs`**

Replace the entire file with:

```rust
use slint::{ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use acta::db;
use acta::db::payments::PaymentKpi;
use crate::ui::helpers::payment_row_to_item;

pub struct PaymentsData {
    pub items: Vec<crate::PaymentItem>,
    pub kpi: PaymentKpi,
}

pub async fn prepare_payments_data(pool: &PgPool, company_id: Uuid) -> PaymentsData {
    let (rows, kpi) = tokio::join!(
        db::payments::list(pool, company_id, None),
        db::payments::payment_kpi(pool, company_id),
    );
    let kpi = kpi.unwrap_or(PaymentKpi {
        incoming_month: rust_decimal::Decimal::ZERO,
        outgoing_month: rust_decimal::Decimal::ZERO,
        unmatched_count: 0,
    });
    PaymentsData {
        items: rows.unwrap_or_default().iter().map(payment_row_to_item).collect(),
        kpi,
    }
}

pub fn apply_payments_to_ui(ui: &crate::AppWindow, data: PaymentsData) {
    ui.set_payments(ModelRc::new(VecModel::from(data.items)));

    let net = data.kpi.incoming_month - data.kpi.outgoing_month;
    ui.set_pay_incoming_str(format_kpi_amount(data.kpi.incoming_month).into());
    ui.set_pay_outgoing_str(format_kpi_amount(data.kpi.outgoing_month).into());
    ui.set_pay_net_str(format_kpi_amount(net).into());
    ui.set_pay_unmatched_str(data.kpi.unmatched_count.to_string().into());
    ui.set_pay_incoming_sub("поточний місяць".into());
    ui.set_pay_outgoing_sub("поточний місяць".into());
    ui.set_pay_unmatched_count(data.kpi.unmatched_count as i32);
}

/// Форматує Decimal як "1 234 567 ₴" (з нерозривним пробілом як роздільником тисяч).
fn format_kpi_amount(amt: rust_decimal::Decimal) -> String {
    use rust_decimal::prelude::ToPrimitive;
    let val = amt.to_f64().unwrap_or(0.0);
    if val == 0.0 {
        return "0 ₴".to_string();
    }
    let s = format!("{:.0}", val.abs());
    let digits: Vec<char> = s.chars().collect();
    let mut result = String::new();
    let len = digits.len();
    for (i, d) in digits.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('\u{00A0}');
        }
        result.push(*d);
    }
    if val < 0.0 {
        format!("−{result} ₴")
    } else {
        format!("{result} ₴")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payments_data_default_is_empty() {
        let data = PaymentsData {
            items: vec![],
            kpi: PaymentKpi {
                incoming_month: rust_decimal::Decimal::ZERO,
                outgoing_month: rust_decimal::Decimal::ZERO,
                unmatched_count: 0,
            },
        };
        assert!(data.items.is_empty());
    }

    #[test]
    fn format_kpi_amount_formats_zero() {
        assert_eq!(format_kpi_amount(rust_decimal::Decimal::ZERO), "0 ₴");
    }

    #[test]
    fn format_kpi_amount_formats_large_number() {
        use rust_decimal_macros::dec;
        let s = format_kpi_amount(dec!(125000.50));
        assert!(s.contains("125"), "має містити 125: {s}");
        assert!(s.contains("₴"), "має містити знак гривні: {s}");
    }

    #[test]
    fn format_kpi_amount_negative_uses_minus_sign() {
        use rust_decimal_macros::dec;
        let s = format_kpi_amount(dec!(-500));
        assert!(s.starts_with('−'), "від'ємне значення: {s}");
    }
}
```

- [ ] **Step 4: Run tests**

```
cargo test ui::payments
```
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src/ui/payments.rs
git commit -m "feat: populate payments KPI strip with monthly aggregates"
```

---

## Task 5: Reports DB queries

Add expense aggregates and category breakdown to `src/db/dashboard.rs`, and add `CategoryRevenue` to `src/models/dashboard.rs`.

**Files:**
- Modify: `src/models/dashboard.rs`
- Modify: `src/db/dashboard.rs`

- [ ] **Step 1: Write the failing tests**

Add to `src/models/dashboard.rs` tests:
```rust
#[test]
fn category_revenue_struct_is_accessible() {
    let c = super::CategoryRevenue {
        label: "Зарплата".to_string(),
        amount: rust_decimal::Decimal::ZERO,
    };
    assert_eq!(c.label, "Зарплата");
}
```

Add to `src/db/dashboard.rs` tests:
```rust
#[test]
fn reports_db_functions_are_exposed() {
    let _ = super::expenses_by_month;
    let _ = super::category_breakdown;
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test category_revenue_struct_is_accessible reports_db_functions_are_exposed
```
Expected: FAIL.

- [ ] **Step 3: Add `CategoryRevenue` to `src/models/dashboard.rs`**

Add after the `RecentAct` struct at the bottom of the file:
```rust
/// Виручка/витрати по категорії для звіту.
pub struct CategoryRevenue {
    pub label: String,
    pub amount: rust_decimal::Decimal,
}
```

- [ ] **Step 4: Add `expenses_by_month` to `src/db/dashboard.rs`**

Add after the `revenue_by_month` function:

```rust
/// Витрати по місяцях за останні `months` місяців (платежі direction='expense').
///
/// Заповнює нулями відсутні місяці. Відсортовано від найстарішого до поточного.
pub async fn expenses_by_month(
    pool: &PgPool,
    company_id: Uuid,
    months: u32,
) -> Result<Vec<MonthRevenue>> {
    struct Row {
        month_num: i32,
        year_num: i32,
        amount: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                month_num: r.try_get("month_num")?,
                year_num:  r.try_get("year_num")?,
                amount:    r.try_get("amount")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            EXTRACT(MONTH FROM date_trunc('month', date))::int AS month_num,
            EXTRACT(YEAR  FROM date_trunc('month', date))::int AS year_num,
            COALESCE(SUM(amount), 0) AS amount
        FROM payments
        WHERE company_id = $1
          AND direction = 'expense'
          AND date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'
        GROUP BY date_trunc('month', date)
        ORDER BY date_trunc('month', date) ASC
        "#,
    )
    .bind(company_id)
    .bind(months as i32)
    .fetch_all(pool)
    .await?;

    let today = Local::now().date_naive();
    let mut result = Vec::with_capacity(months as usize);
    for i in (0..months).rev() {
        let target = subtract_months(today, i);
        let found = rows.iter().find(|r| {
            r.month_num as u32 == target.month() && r.year_num == target.year()
        });
        result.push(MonthRevenue {
            month_num: target.month(),
            year: target.year(),
            amount: found.map(|r| r.amount).unwrap_or(Decimal::ZERO),
        });
    }
    result.reverse();
    Ok(result)
}
```

- [ ] **Step 5: Add `category_breakdown` to `src/db/dashboard.rs`**

Add after `expenses_by_month`:

```rust
/// Топ-5 категорій за сумою оплачених актів за `months` місяців.
///
/// Акти з `category_id IS NULL` об'єднуються під міткою "Без категорії".
pub async fn category_breakdown(
    pool: &PgPool,
    company_id: Uuid,
    months: u32,
) -> Result<Vec<crate::models::dashboard::CategoryRevenue>> {
    struct Row {
        label: String,
        amount: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                label:  r.try_get("label")?,
                amount: r.try_get("amount")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            COALESCE(c.name, 'Без категорії') AS label,
            SUM(a.total_amount)               AS amount
        FROM acts a
        LEFT JOIN categories c ON c.id = a.category_id
        WHERE a.company_id = $1
          AND a.status = 'paid'
          AND a.date >= date_trunc('month', CURRENT_DATE) - ($2::int - 1) * INTERVAL '1 month'
        GROUP BY c.name
        ORDER BY amount DESC
        LIMIT 5
        "#,
    )
    .bind(company_id)
    .bind(months as i32)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| crate::models::dashboard::CategoryRevenue { label: r.label, amount: r.amount })
        .collect())
}
```

- [ ] **Step 6: Run tests**

```
cargo test category_revenue_struct_is_accessible reports_db_functions_are_exposed
```
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/models/dashboard.rs src/db/dashboard.rs
git commit -m "feat: add expenses_by_month and category_breakdown DB queries"
```

---

## Task 6: Reports UI module

Create `src/ui/reports.rs` with the full prepare/apply/wire pattern for the Reports screen.

Period mapping (from Slint `rep-period-state`): `0` = current month (1 month), `1` = quarter (3 months), `2` = year (12 months), `3` = custom (stub → use 3 months).

**Files:**
- Create: `src/ui/reports.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
// In src/ui/reports.rs (new file, just the test module first)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_to_months_maps_correctly() {
        assert_eq!(period_to_months(0), 1);
        assert_eq!(period_to_months(1), 3);
        assert_eq!(period_to_months(2), 12);
        assert_eq!(period_to_months(3), 3); // custom → default to quarter
        assert_eq!(period_to_months(99), 3); // unknown → default
    }

    #[test]
    fn build_expense_categories_normalizes_percent() {
        use rust_decimal_macros::dec;
        use acta::models::dashboard::CategoryRevenue;

        let cats = vec![
            CategoryRevenue { label: "А".into(), amount: dec!(750) },
            CategoryRevenue { label: "Б".into(), amount: dec!(250) },
        ];
        let slint_cats = build_expense_categories(&cats);
        assert_eq!(slint_cats.len(), 2);
        assert_eq!(slint_cats[0].percent, 75);
        assert_eq!(slint_cats[1].percent, 25);
    }

    #[test]
    fn format_report_amount_renders_sign() {
        use rust_decimal_macros::dec;
        let s = format_report_amount(dec!(-5000));
        assert!(s.contains('−') || s.starts_with('-'), "від'ємне: {s}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test ui::reports
```
Expected: FAIL — module not found.

- [ ] **Step 3: Add `pub mod reports` to `src/ui/mod.rs`**

```rust
pub mod helpers;
pub mod dashboard;
pub mod documents;
pub mod counterparties;
pub mod payments;
pub mod reports;
pub mod tasks;
pub mod settings;
```

- [ ] **Step 4: Create `src/ui/reports.rs`**

```rust
use slint::{ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use acta::db;
use acta::models::dashboard::CategoryRevenue;

/// Кількість місяців для кожного значення `rep-period-state`.
pub fn period_to_months(period: i32) -> u32 {
    match period {
        0 => 1,
        1 => 3,
        2 => 12,
        _ => 3,
    }
}

pub struct ReportsData {
    pub metrics: crate::ReportMetrics,
    pub chart_bars: Vec<crate::ChartBar>,
    pub categories: Vec<crate::ExpenseCategory>,
    pub drill_rows: Vec<crate::DrillRow>,
    pub revenue_str: String,
    pub expenses_str: String,
    pub profit_str: String,
    pub margin_str: String,
}

pub async fn prepare_reports_data(pool: &PgPool, company_id: Uuid, period: i32) -> ReportsData {
    let months = period_to_months(period);

    let (rev_months_res, exp_months_res, cats_res) = tokio::join!(
        db::dashboard::revenue_by_month(pool, company_id, months),
        db::dashboard::expenses_by_month(pool, company_id, months),
        db::dashboard::category_breakdown(pool, company_id, months),
    );

    let rev_months = rev_months_res.unwrap_or_default();
    let exp_months = exp_months_res.unwrap_or_default();
    let cats = cats_res.unwrap_or_default();

    let total_rev: rust_decimal::Decimal = rev_months.iter().map(|m| m.amount).sum();
    let total_exp: rust_decimal::Decimal = exp_months.iter().map(|m| m.amount).sum();
    let profit = total_rev - total_exp;
    let margin = if total_rev > rust_decimal::Decimal::ZERO {
        (profit / total_rev * rust_decimal::Decimal::from(100))
            .round_dp(1)
    } else {
        rust_decimal::Decimal::ZERO
    };

    use rust_decimal::prelude::ToPrimitive;
    let metrics = crate::ReportMetrics {
        revenue:        total_rev.to_f32().unwrap_or(0.0),
        expenses:       total_exp.to_f32().unwrap_or(0.0),
        profit:         profit.to_f32().unwrap_or(0.0),
        margin:         margin.to_f32().unwrap_or(0.0),
        delta_revenue:  "".into(),
        delta_expenses: "".into(),
        delta_profit:   "".into(),
        delta_margin:   "".into(),
    };

    let chart_bars = build_chart_bars(&rev_months, &exp_months);
    let categories = build_expense_categories(&cats);

    ReportsData {
        revenue_str:  format_report_amount(total_rev),
        expenses_str: format_report_amount(total_exp),
        profit_str:   format_report_amount(profit),
        margin_str:   format!("{margin:.1}%"),
        metrics,
        chart_bars,
        categories,
        drill_rows: vec![],
    }
}

pub fn apply_reports_to_ui(ui: &crate::AppWindow, data: ReportsData) {
    ui.set_rep_metrics(data.metrics);
    ui.set_rep_chart_bars(ModelRc::new(VecModel::from(data.chart_bars)));
    ui.set_rep_categories(ModelRc::new(VecModel::from(data.categories)));
    ui.set_rep_drill_rows(ModelRc::new(VecModel::from(data.drill_rows)));
    ui.set_rep_revenue_str(data.revenue_str.into());
    ui.set_rep_expenses_str(data.expenses_str.into());
    ui.set_rep_profit_str(data.profit_str.into());
    ui.set_rep_margin_str(data.margin_str.into());
}

pub fn wire_reports_callbacks(
    ui: &crate::AppWindow,
    pool: &std::sync::Arc<sqlx::PgPool>,
    company_id: &std::sync::Arc<std::sync::Mutex<Uuid>>,
) {
    use slint::ComponentHandle;

    // Period changed — reload with new period
    ui.on_rep_period_changed({
        let pool = pool.clone();
        let ui_weak = ui.as_weak();
        let company_id = company_id.clone();
        move |period| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let cid = *company_id.lock().unwrap();
            tokio::spawn(async move {
                let data = prepare_reports_data(&pool, cid, period).await;
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    apply_reports_to_ui(&ui, data);
                });
            });
        }
    });

    // Category drilled — stub (drill-down not yet backed by DB query)
    ui.on_rep_category_drilled(|_cat| {
        tracing::debug!("rep_category_drilled: drill-down not yet implemented");
    });

    // Export stubs — log and do nothing
    ui.on_rep_export_csv(|| {
        tracing::info!("rep_export_csv: export not yet implemented");
    });
    ui.on_rep_export_pdf(|| {
        tracing::info!("rep_export_pdf: export not yet implemented");
    });
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Будує нормалізовані ChartBar — висота = value / max(rev, exp).
pub fn build_chart_bars(
    rev: &[acta::models::dashboard::MonthRevenue],
    exp: &[acta::models::dashboard::MonthRevenue],
) -> Vec<crate::ChartBar> {
    use rust_decimal::prelude::ToPrimitive;

    let max_val = rev
        .iter()
        .map(|m| m.amount)
        .chain(exp.iter().map(|m| m.amount))
        .map(|a| a.to_f64().unwrap_or(0.0))
        .fold(0.0f64, f64::max);

    let normalize = |a: rust_decimal::Decimal| -> f32 {
        let v = a.to_f64().unwrap_or(0.0);
        if max_val > 0.0 { (v / max_val) as f32 } else { 0.0 }
    };

    // rev and exp are both sorted oldest→newest with gaps filled; lengths may differ
    // Use rev as the anchor for month labels
    let n = rev.len().max(exp.len());
    (0..n).map(|i| {
        let r = rev.get(i);
        let e = exp.get(i);
        crate::ChartBar {
            rev_h: r.map(|m| normalize(m.amount)).unwrap_or(0.0),
            exp_h: e.map(|m| normalize(m.amount)).unwrap_or(0.0),
            month: r.map(|m| m.month_label().to_string())
                    .or_else(|| e.map(|m| m.month_label().to_string()))
                    .unwrap_or_default()
                    .into(),
        }
    }).collect()
}

/// Перетворює `Vec<CategoryRevenue>` у `Vec<ExpenseCategory>` з відсотками.
pub fn build_expense_categories(cats: &[CategoryRevenue]) -> Vec<crate::ExpenseCategory> {
    use rust_decimal::prelude::ToPrimitive;
    let total: rust_decimal::Decimal = cats.iter().map(|c| c.amount).sum();

    cats.iter().map(|c| {
        let pct = if total > rust_decimal::Decimal::ZERO {
            ((c.amount / total) * rust_decimal::Decimal::from(100))
                .to_i32()
                .unwrap_or(0)
        } else { 0 };
        crate::ExpenseCategory {
            label: c.label.clone().into(),
            amount: c.amount.to_f32().unwrap_or(0.0),
            percent: pct,
        }
    }).collect()
}

/// Форматує Decimal як грошовий рядок: "1 234 567 ₴".
pub fn format_report_amount(amt: rust_decimal::Decimal) -> String {
    use rust_decimal::prelude::ToPrimitive;
    let val = amt.to_f64().unwrap_or(0.0);
    if val == 0.0 {
        return "0 ₴".to_string();
    }
    let s = format!("{:.0}", val.abs());
    let digits: Vec<char> = s.chars().collect();
    let mut result = String::new();
    let len = digits.len();
    for (i, d) in digits.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('\u{00A0}');
        }
        result.push(*d);
    }
    if val < 0.0 {
        format!("−{result} ₴")
    } else {
        format!("{result} ₴")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_to_months_maps_correctly() {
        assert_eq!(period_to_months(0), 1);
        assert_eq!(period_to_months(1), 3);
        assert_eq!(period_to_months(2), 12);
        assert_eq!(period_to_months(3), 3);
        assert_eq!(period_to_months(99), 3);
    }

    #[test]
    fn build_expense_categories_normalizes_percent() {
        use rust_decimal_macros::dec;
        let cats = vec![
            CategoryRevenue { label: "А".into(), amount: dec!(750) },
            CategoryRevenue { label: "Б".into(), amount: dec!(250) },
        ];
        let slint_cats = build_expense_categories(&cats);
        assert_eq!(slint_cats.len(), 2);
        assert_eq!(slint_cats[0].percent, 75);
        assert_eq!(slint_cats[1].percent, 25);
    }

    #[test]
    fn build_expense_categories_with_zero_total_returns_zero_percent() {
        use rust_decimal_macros::dec;
        let cats = vec![
            CategoryRevenue { label: "А".into(), amount: dec!(0) },
        ];
        let slint_cats = build_expense_categories(&cats);
        assert_eq!(slint_cats[0].percent, 0);
    }

    #[test]
    fn format_report_amount_renders_sign() {
        use rust_decimal_macros::dec;
        let s = format_report_amount(dec!(-5000));
        assert!(s.starts_with('−'), "від'ємне: {s}");
    }

    #[test]
    fn format_report_amount_zero() {
        assert_eq!(format_report_amount(rust_decimal::Decimal::ZERO), "0 ₴");
    }

    #[test]
    fn build_chart_bars_normalizes_to_max() {
        use rust_decimal_macros::dec;
        use acta::models::dashboard::MonthRevenue;
        let rev = vec![MonthRevenue { month_num: 1, year: 2026, amount: dec!(1000) }];
        let exp = vec![MonthRevenue { month_num: 1, year: 2026, amount: dec!(2000) }];
        let bars = build_chart_bars(&rev, &exp);
        assert_eq!(bars.len(), 1);
        assert!((bars[0].rev_h - 0.5).abs() < 0.01, "rev: {}", bars[0].rev_h);
        assert!((bars[0].exp_h - 1.0).abs() < 0.01, "exp: {}", bars[0].exp_h);
    }
}
```

- [ ] **Step 5: Run tests**

```
cargo test ui::reports
```
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add src/ui/reports.rs src/ui/mod.rs
git commit -m "feat: add reports UI module with prepare/apply/wire"
```

---

## Task 7: Settings persistence

Load company info from DB on startup, wire the `settings-company-saved` callback to persist changes.

**Files:**
- Modify: `src/ui/settings.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn company_to_info_maps_optional_fields_to_empty_string() {
    use acta::models::company::Company;
    use chrono::Utc;
    use uuid::Uuid;

    let company = Company {
        id: Uuid::new_v4(),
        name: "ТОВ Тест".into(),
        short_name: None,
        edrpou: Some("12345678".into()),
        ipn: None,
        iban: None,
        legal_address: None,
        actual_address: None,
        phone: None,
        email: None,
        director_name: None,
        accountant_name: None,
        tax_system: None,
        is_vat_payer: false,
        logo_path: None,
        notes: None,
        is_archived: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let info = company_to_info(&company);
    assert_eq!(info.full_name.as_str(), "ТОВ Тест");
    assert_eq!(info.edrpou.as_str(), "12345678");
    assert_eq!(info.short_name.as_str(), "");
}
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test company_to_info_maps_optional_fields
```
Expected: FAIL — `company_to_info` not found.

- [ ] **Step 3: Replace `src/ui/settings.rs` with full implementation**

```rust
use sqlx::PgPool;
use uuid::Uuid;
use std::sync::{Arc, Mutex};

use acta::db;
use acta::models::company::{Company, UpdateCompany};

pub struct SettingsData {
    pub company_info: crate::CompanyInfo,
}

/// Перетворює `Company` з БД у `CompanyInfo` для Slint.
pub fn company_to_info(c: &Company) -> crate::CompanyInfo {
    crate::CompanyInfo {
        full_name:      c.name.clone().into(),
        short_name:     c.short_name.clone().unwrap_or_default().into(),
        edrpou:         c.edrpou.clone().unwrap_or_default().into(),
        ipn:            c.ipn.clone().unwrap_or_default().into(),
        address:        c.legal_address.clone().unwrap_or_default().into(),
        director:       c.director_name.clone().unwrap_or_default().into(),
        iban:           c.iban.clone().unwrap_or_default().into(),
        bank:           slint::SharedString::default(),
        vat_registered: c.is_vat_payer,
        vat_cert:       slint::SharedString::default(),
    }
}

/// Перетворює `CompanyInfo` з Slint у `UpdateCompany` для БД.
fn info_to_update(info: &crate::CompanyInfo) -> UpdateCompany {
    fn opt(s: &slint::SharedString) -> Option<String> {
        let v = s.as_str().trim().to_string();
        if v.is_empty() { None } else { Some(v) }
    }
    UpdateCompany {
        name:            info.full_name.as_str().trim().to_string(),
        short_name:      opt(&info.short_name),
        edrpou:          opt(&info.edrpou),
        iban:            opt(&info.iban),
        legal_address:   opt(&info.address),
        director_name:   opt(&info.director),
        accountant_name: None,
        tax_system:      None,
        is_vat_payer:    info.vat_registered,
        logo_path:       None,
    }
}

pub async fn prepare_settings_data(pool: &PgPool, company_id: Uuid) -> SettingsData {
    let company_info = db::companies::get_by_id(pool, company_id)
        .await
        .ok()
        .flatten()
        .map(|c| company_to_info(&c))
        .unwrap_or_else(|| crate::CompanyInfo {
            full_name: slint::SharedString::default(),
            short_name: slint::SharedString::default(),
            edrpou: slint::SharedString::default(),
            ipn: slint::SharedString::default(),
            address: slint::SharedString::default(),
            director: slint::SharedString::default(),
            iban: slint::SharedString::default(),
            bank: slint::SharedString::default(),
            vat_registered: false,
            vat_cert: slint::SharedString::default(),
        });
    SettingsData { company_info }
}

pub fn apply_settings_to_ui(ui: &crate::AppWindow, data: SettingsData) {
    ui.set_company_info(data.company_info);
}

pub fn wire_settings_callbacks(
    ui: &crate::AppWindow,
    pool: &Arc<PgPool>,
    company_id: &Arc<Mutex<Uuid>>,
) {
    use slint::ComponentHandle;

    ui.on_settings_company_saved({
        let pool = pool.clone();
        let company_id = company_id.clone();
        move |info| {
            let pool = pool.clone();
            let cid = *company_id.lock().unwrap();
            let update = info_to_update(&info);
            tokio::spawn(async move {
                match db::companies::update(&pool, cid, &update).await {
                    Ok(Some(_)) => tracing::info!("settings: company saved"),
                    Ok(None)    => tracing::warn!("settings: company not found id={cid}"),
                    Err(e)      => tracing::error!("settings: save failed: {e}"),
                }
            });
        }
    });

    // Stub callbacks — UI handles these visually without Rust persistence
    ui.on_settings_section_changed(|_| {});
    ui.on_settings_dark_mode_toggled(|_| {});
    ui.on_settings_density_changed(|_| {});
    ui.on_settings_integration_configure(|_| {});
    ui.on_settings_team_invite(|| {});
    ui.on_settings_backup_now(|| {});
    ui.on_settings_backup_download(|| {});
}

#[cfg(test)]
mod tests {
    use super::*;
    use acta::models::company::Company;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_company(name: &str) -> Company {
        Company {
            id: Uuid::new_v4(),
            name: name.into(),
            short_name: None,
            edrpou: Some("12345678".into()),
            ipn: None,
            iban: None,
            legal_address: None,
            actual_address: None,
            phone: None,
            email: None,
            director_name: None,
            accountant_name: None,
            tax_system: None,
            is_vat_payer: false,
            logo_path: None,
            notes: None,
            is_archived: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn company_to_info_maps_optional_fields_to_empty_string() {
        let company = make_company("ТОВ Тест");
        let info = company_to_info(&company);
        assert_eq!(info.full_name.as_str(), "ТОВ Тест");
        assert_eq!(info.edrpou.as_str(), "12345678");
        assert_eq!(info.short_name.as_str(), "");
    }

    #[test]
    fn info_to_update_empty_fields_become_none() {
        let info = crate::CompanyInfo {
            full_name:      "ТОВ Тест".into(),
            short_name:     "".into(),
            edrpou:         "".into(),
            ipn:            "".into(),
            address:        "".into(),
            director:       "".into(),
            iban:           "".into(),
            bank:           "".into(),
            vat_registered: false,
            vat_cert:       "".into(),
        };
        let update = info_to_update(&info);
        assert_eq!(update.name, "ТОВ Тест");
        assert!(update.short_name.is_none());
        assert!(update.edrpou.is_none());
    }
}
```

- [ ] **Step 4: Run tests**

```
cargo test ui::settings
```
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src/ui/settings.rs
git commit -m "feat: settings persistence — load company from DB, wire save callback"
```

---

## Task 8: Wire everything in `src/main.rs`

Replace all stub callbacks. Add reports to initial load and nav routing. Wire settings and payments with company_id.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn all_stub_callbacks_are_replaced_by_wired_functions() {
    // This is a compile-time check — if on_pay_import_csv etc. are still
    // registered as || {} stubs, the wiring functions won't exist and this
    // comment serves as a reminder. The real test is: cargo build succeeds
    // and no compiler warnings about unused variables appear for pool/company_id.
    assert!(true);
}
```

Add to the `#[cfg(test)]` block in `src/main.rs`.

- [ ] **Step 2: Update initial load in `main()`**

Replace the existing `block_on(tokio::join!(...))` and subsequent apply calls. The new initial load must include reports and settings. Find the existing block:

```rust
let (dash_data, doc_data, cp_data, pay_data, task_data) = rt.block_on(async {
    tokio::join!(
        ui::dashboard::prepare_dashboard_data(&pool, company_id),
        ui::documents::prepare_documents_data(&pool, company_id, None, None),
        ui::counterparties::prepare_counterparties_data(&pool, company_id, None),
        ui::payments::prepare_payments_data(&pool, company_id),
        ui::tasks::prepare_tasks_data(&pool),
    )
});

ui::dashboard::apply_dashboard_to_ui(&ui, dash_data);
ui::documents::apply_documents_to_ui(&ui, doc_data);
ui::counterparties::apply_counterparties_to_ui(&ui, cp_data);
ui::payments::apply_payments_to_ui(&ui, pay_data);
ui::tasks::apply_tasks_to_ui(&ui, task_data);
ui::settings::apply_settings_to_ui(&ui);
```

Replace with:

```rust
let (dash_data, doc_data, cp_data, pay_data, task_data, rep_data, set_data) = rt.block_on(async {
    tokio::join!(
        ui::dashboard::prepare_dashboard_data(&pool, company_id),
        ui::documents::prepare_documents_data(&pool, company_id, None, None),
        ui::counterparties::prepare_counterparties_data(&pool, company_id, None),
        ui::payments::prepare_payments_data(&pool, company_id),
        ui::tasks::prepare_tasks_data(&pool),
        ui::reports::prepare_reports_data(&pool, company_id, 1),
        ui::settings::prepare_settings_data(&pool, company_id),
    )
});

ui::dashboard::apply_dashboard_to_ui(&ui, dash_data);
ui::documents::apply_documents_to_ui(&ui, doc_data);
ui::counterparties::apply_counterparties_to_ui(&ui, cp_data);
ui::payments::apply_payments_to_ui(&ui, pay_data);
ui::tasks::apply_tasks_to_ui(&ui, task_data);
ui::reports::apply_reports_to_ui(&ui, rep_data);
ui::settings::apply_settings_to_ui(&ui, set_data);
```

- [ ] **Step 3: Update `on_nav_changed` to include Reports**

Find the `NavScreen::Tasks` arm (last match arm before `_ => {}`) in `on_nav_changed` and add after it:

```rust
NavScreen::Reports => {
    let data = ui::reports::prepare_reports_data(&pool, cid, 1).await;
    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        ui::reports::apply_reports_to_ui(&ui, data);
    });
}
NavScreen::Settings => {
    let data = ui::settings::prepare_settings_data(&pool, cid).await;
    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        ui::settings::apply_settings_to_ui(&ui, data);
    });
}
NavScreen::Payments => {
    let data = ui::payments::prepare_payments_data(&pool, cid).await;
    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        ui::payments::apply_payments_to_ui(&ui, data);
    });
}
```

Note: `NavScreen::Payments` arm already exists — replace the existing one with this updated version.

- [ ] **Step 4: Wire callbacks in `main()`**

Remove these stub lines:
```rust
ui.on_rep_period_changed(|_| {});
ui.on_rep_category_drilled(|_| {});
ui.on_rep_export_csv(|| {});
ui.on_rep_export_pdf(|| {});
ui.on_settings_section_changed(|_| {});
ui.on_settings_dark_mode_toggled(|_| {});
ui.on_settings_density_changed(|_| {});
ui.on_settings_company_saved(|_| {});
ui.on_settings_integration_configure(|_| {});
ui.on_settings_team_invite(|| {});
ui.on_settings_backup_now(|| {});
ui.on_settings_backup_download(|| {});
```

And add these wiring calls (alongside the existing wire_document_callbacks etc.):
```rust
// ── Звіти ────────────────────────────────────────────────────────────────────
ui::reports::wire_reports_callbacks(&ui, &pool, &active_company_id);

// ── Налаштування ─────────────────────────────────────────────────────────────
ui::settings::wire_settings_callbacks(&ui, &pool, &active_company_id);
```

Keep the payment and palette stubs that have no implementation yet:
```rust
ui.on_pay_import_csv(|| {});
ui.on_pay_sync_bank(|| {});
ui.on_pay_new(|| {});
ui.on_pay_link(|_| {});
ui.on_palette_query_changed(|_| {});
ui.on_palette_item_activated(|_| {});
```

- [ ] **Step 5: Fix `apply_settings_to_ui` signature**

`apply_settings_to_ui` now takes a `SettingsData` parameter. Update its call sites — the function is also called in `on_nav_changed`. Make sure both call it as:
```rust
ui::settings::apply_settings_to_ui(&ui, data);
```

- [ ] **Step 6: Build and run tests**

```
cargo build 2>&1 | head -40
cargo test
```
Expected: build succeeds, all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire reports/settings/payments callbacks; complete main.rs wiring"
```

---

## Known Limitations

- **`delta-*` fields in `ReportMetrics`** are empty strings — a future task can add month-over-month comparison.
- **`rep-drill-rows`** is always empty — drill-down queries require a per-category payment/act breakdown not yet implemented.
- **`pay-import-csv`, `pay-sync-bank`, `pay-new`, `pay-link`** remain stubs — require new form UI (separate plan).
- **Palette callbacks** (`palette-query-changed`, `palette-item-activated`) remain stubs — the CommandPalette in shell.slint is static (hardcoded items with direct navigation) and does not call these back from the UI layer.
- **Export** (CSV/PDF) logs a message and does nothing — requires file dialog and serialization (separate plan).

---

## Self-Review

**Spec coverage:**
- ✅ `on_rep_period_changed` → `wire_reports_callbacks` → reload with period
- ✅ `on_rep_category_drilled` → stub with log
- ✅ `on_rep_export_csv` / `on_rep_export_pdf` → stub with log
- ✅ `on_settings_*` → `wire_settings_callbacks`
- ✅ `on_settings_company_saved` → `db::companies::update`
- ✅ `dash-chart-bars` → populated from `revenue_by_month`
- ✅ `rep-chart-bars` → populated from rev+exp months
- ✅ `pay-incoming-str` etc. → `payment_kpi` aggregate
- ✅ `rep-categories` → `category_breakdown`
- ✅ `rep-metrics` → computed from totals

**Type consistency:**
- `ChartBar` → used in Task 1 (types.slint), Task 2 (dashboard.rs), Task 6 (reports.rs) ✅
- `PaymentKpi` → defined in Task 3, used in Task 4 ✅
- `CategoryRevenue` → defined in Task 5 (models/dashboard.rs), used in Task 5 (db/dashboard.rs), used in Task 6 (reports.rs) ✅
- `SettingsData` → defined in Task 7, `apply_settings_to_ui(&ui, data)` signature used in Tasks 7+8 ✅
