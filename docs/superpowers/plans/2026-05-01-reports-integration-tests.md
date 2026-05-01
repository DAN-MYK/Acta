# Reports Integration Tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Покрити інтеграційними тестами всі SQL-функції в `src/db/reports.rs` — єдиний непокритий шар у модулі звітів.

**Architecture:** Тести додаються до існуючого `tests/db_integration.rs`. Кожен тест сам створює тестові дані з унікальним суфіксом і видаляє їх у кінці. `AppCtx::new(pool, DEFAULT_COMPANY_ID)` використовується як контекст для функцій з `db::reports`. Тести пропускаються якщо БД недоступна (патерн `let Some(pool) = test_pool().await? else { return Ok(()); }`).

**Tech Stack:** Rust, tokio, sqlx, PostgreSQL, rust_decimal, chrono, anyhow.

---

## Поточний стан

Функції `src/db/reports.rs` вже реалізовані і зв'язані з Tauri:
- `compute_opening_balance` — баланс до початку періоду
- `load_bank_rows` — cashflow з `payments`, групування по контрагентах або компаніях
- `load_pnl_rows` — P&L з `acts` + `invoices`, групування по категоріях
- `load_receivables_rows` — відкриті документи з overdue calculation
- `load_payables_rows` — заплановані виплати з `payment_schedule`

Жодного інтеграційного тесту немає. Всі SQL виконуються тільки в runtime.

## Цільова структура файлів

- **Modify:** `tests/db_integration.rs` — 5 нових `#[tokio::test]` в кінці файлу
- **No new files** — всі тести в існуючому integration test файлі

---

## Task 1: Хелпер create_test_act та тест для load_pnl_rows

**Files:**
- Modify: `tests/db_integration.rs`

- [ ] **Step 1: Додати хелпер create_test_act в кінець блоку хелперів (після create_test_contract, ~рядок 100)**

```rust
async fn create_test_act(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    number: &str,
    amount: Decimal,
    status: &str,
    category_id: Option<Uuid>,
    date: chrono::NaiveDate,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO acts
           (id, company_id, counterparty_id, number, date, total_amount, status, category_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7::act_status, $8)"#,
    )
    .bind(id)
    .bind(company_id)
    .bind(counterparty_id)
    .bind(number)
    .bind(date)
    .bind(amount)
    .bind(status)
    .bind(category_id)
    .execute(pool)
    .await?;
    Ok(id)
}
```

- [ ] **Step 2: Додати хелпер create_test_category**

```rust
async fn create_test_category(
    pool: &PgPool,
    company_id: Uuid,
    name: &str,
    kind: &str,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO categories (id, company_id, name, kind)
           VALUES ($1, $2, $3, $4::category_kind)"#,
    )
    .bind(id)
    .bind(company_id)
    .bind(name)
    .bind(kind)
    .execute(pool)
    .await?;
    Ok(id)
}
```

- [ ] **Step 3: Написати тест load_pnl_rows_groups_by_category_and_excludes_draft**

```rust
#[tokio::test]
async fn load_pnl_rows_groups_by_category_and_excludes_draft() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_pnl_rows;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);

    let cp = create_test_counterparty(&pool, &suffix, &format!("ІТ PNL CP {suffix}"), None, None).await?;
    let cat_income_id = create_test_category(&pool, DEFAULT_COMPANY_ID, &format!("Послуги {suffix}"), "income").await?;

    // Акт зі статусом issued → має з'явитись у P&L
    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("PNL-{suffix}-1"),
        dec!(10000),
        "issued",
        Some(cat_income_id),
        today,
    ).await?;

    // Акт зі статусом draft → НЕ повинен з'явитись
    let draft_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("PNL-{suffix}-draft"),
        dec!(99999),
        "draft",
        Some(cat_income_id),
        today,
    ).await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: format!("Послуги {suffix}"),
    };

    let rows = load_pnl_rows(&ctx, &filter).await?;

    assert_eq!(rows.len(), 1, "має бути рівно 1 категорія після фільтра");
    assert_eq!(rows[0].income, dec!(10000));
    assert_eq!(rows[0].expense, dec!(0));

    // Cleanup
    sqlx::query("DELETE FROM acts WHERE id IN ($1, $2)").bind(act_id).bind(draft_id).execute(&pool).await?;
    sqlx::query("DELETE FROM categories WHERE id = $1").bind(cat_income_id).execute(&pool).await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1").bind(cp.id).execute(&pool).await?;

    Ok(())
}
```

- [ ] **Step 4: Запустити тест**

```bash
cargo test load_pnl_rows_groups_by_category_and_excludes_draft -- --nocapture
```

Очікувано: PASS (або SKIP якщо DB не налаштована).

- [ ] **Step 5: Commit**

```bash
git add tests/db_integration.rs
git commit -m "test(reports): add integration test for pnl rows SQL"
```

---

## Task 2: Тест для compute_opening_balance

**Files:**
- Modify: `tests/db_integration.rs`

- [ ] **Step 1: Додати хелпер create_test_payment**

Після хелперів вище, перед тестами:

```rust
async fn create_test_payment(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Option<Uuid>,
    amount: Decimal,
    direction: &str,
    date: chrono::NaiveDate,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO payments
           (id, company_id, counterparty_id, amount, direction, date)
           VALUES ($1, $2, $3, $4, $5::payment_direction, $6)"#,
    )
    .bind(id)
    .bind(company_id)
    .bind(counterparty_id)
    .bind(amount)
    .bind(direction)
    .bind(date)
    .execute(pool)
    .await?;
    Ok(id)
}
```

- [ ] **Step 2: Написати тест compute_opening_balance_sums_payments_before_period**

```rust
#[tokio::test]
async fn compute_opening_balance_sums_payments_before_period() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::compute_opening_balance;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(10);
    let before_period = today - Duration::days(20);

    // Платіж ДО period_start → включається у opening balance
    let p1 = create_test_payment(&pool, DEFAULT_COMPANY_ID, None, dec!(5000), "income", before_period).await?;
    // Платіж ДО period_start, expense → зменшує opening balance
    let p2 = create_test_payment(&pool, DEFAULT_COMPANY_ID, None, dec!(1000), "expense", before_period).await?;
    // Платіж В межах period → НЕ входить в opening balance
    let p3 = create_test_payment(&pool, DEFAULT_COMPANY_ID, None, dec!(9999), "income", today).await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: String::new(),
    };

    let balance = compute_opening_balance(&ctx, &filter).await?;

    // 5000 income - 1000 expense = 4000 (плюс будь-які інші платежі до period_start в тестовій БД)
    assert!(
        balance >= dec!(4000),
        "opening balance має включати 5000-1000=4000 від тестових платежів"
    );

    // Cleanup
    sqlx::query("DELETE FROM payments WHERE id IN ($1, $2, $3)")
        .bind(p1).bind(p2).bind(p3).execute(&pool).await?;

    Ok(())
}
```

- [ ] **Step 3: Запустити тест**

```bash
cargo test compute_opening_balance_sums_payments_before_period -- --nocapture
```

Очікувано: PASS або SKIP.

- [ ] **Step 4: Commit**

```bash
git add tests/db_integration.rs
git commit -m "test(reports): add integration test for opening balance calculation"
```

---

## Task 3: Тест для load_bank_rows

**Files:**
- Modify: `tests/db_integration.rs`

- [ ] **Step 1: Написати тест load_bank_rows_groups_payments_by_counterparty**

```rust
#[tokio::test]
async fn load_bank_rows_groups_payments_by_counterparty() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_bank_rows;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);

    let cp = create_test_counterparty(&pool, &suffix, &format!("ІТ Bank CP {suffix}"), None, None).await?;

    let p1 = create_test_payment(&pool, DEFAULT_COMPANY_ID, Some(cp.id), dec!(3000), "income", today).await?;
    let p2 = create_test_payment(&pool, DEFAULT_COMPANY_ID, Some(cp.id), dec!(1000), "expense", today).await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: format!("ІТ Bank CP {suffix}"),
    };

    let rows = load_bank_rows(&ctx, &filter).await?;

    assert_eq!(rows.len(), 1, "має бути один рядок по контрагенту після query фільтра");
    assert_eq!(rows[0].income, dec!(3000));
    assert_eq!(rows[0].expense, dec!(1000));
    assert!(rows[0].label.contains(&suffix));

    // Cleanup
    sqlx::query("DELETE FROM payments WHERE id IN ($1, $2)").bind(p1).bind(p2).execute(&pool).await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1").bind(cp.id).execute(&pool).await?;

    Ok(())
}
```

- [ ] **Step 2: Запустити тест**

```bash
cargo test load_bank_rows_groups_payments_by_counterparty -- --nocapture
```

Очікувано: PASS або SKIP.

- [ ] **Step 3: Commit**

```bash
git add tests/db_integration.rs
git commit -m "test(reports): add integration test for bank rows grouping"
```

---

## Task 4: Тест для load_receivables_rows з overdue calculation

**Files:**
- Modify: `tests/db_integration.rs`

- [ ] **Step 1: Додати хелпер create_test_invoice**

```rust
async fn create_test_invoice(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    number: &str,
    amount: Decimal,
    status: &str,
    expected_payment_date: Option<chrono::NaiveDate>,
    date: chrono::NaiveDate,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO invoices
           (id, company_id, counterparty_id, number, date, total_amount, status, expected_payment_date)
           VALUES ($1, $2, $3, $4, $5, $6, $7::invoice_status, $8)"#,
    )
    .bind(id)
    .bind(company_id)
    .bind(counterparty_id)
    .bind(number)
    .bind(date)
    .bind(amount)
    .bind(status)
    .bind(expected_payment_date)
    .execute(pool)
    .await?;
    Ok(id)
}
```

- [ ] **Step 2: Написати тест load_receivables_rows_calculates_overdue_days**

```rust
#[tokio::test]
async fn load_receivables_rows_calculates_overdue_days() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_receivables_rows;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(60);
    let overdue_expected = today - Duration::days(10); // очікувана дата 10 днів тому

    let cp = create_test_counterparty(&pool, &suffix, &format!("ІТ Recv CP {suffix}"), None, None).await?;

    // Рахунок зі статусом issued та простроченою expected_payment_date
    let inv_id = create_test_invoice(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("INV-{suffix}"),
        dec!(8000),
        "issued",
        Some(overdue_expected),
        period_start + Duration::days(1),
    ).await?;

    // Рахунок зі статусом paid → НЕ повинен з'явитись
    let paid_id = create_test_invoice(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("INV-{suffix}-PAID"),
        dec!(5000),
        "paid",
        None,
        period_start + Duration::days(1),
    ).await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: format!("ІТ Recv CP {suffix}"),
    };

    let rows = load_receivables_rows(&ctx, &filter).await?;

    assert_eq!(rows.len(), 1, "тільки issued рахунок має бути в дебіторці");
    assert_eq!(rows[0].amount, dec!(8000));
    assert!(rows[0].overdue_days >= 10, "прострочка має бути >= 10 днів");

    // Cleanup
    sqlx::query("DELETE FROM invoices WHERE id IN ($1, $2)").bind(inv_id).bind(paid_id).execute(&pool).await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1").bind(cp.id).execute(&pool).await?;

    Ok(())
}
```

- [ ] **Step 3: Запустити тест**

```bash
cargo test load_receivables_rows_calculates_overdue_days -- --nocapture
```

Очікувано: PASS або SKIP.

- [ ] **Step 4: Commit**

```bash
git add tests/db_integration.rs
git commit -m "test(reports): add integration test for receivables overdue calculation"
```

---

## Task 5: Тест для load_payables_rows та фінальна перевірка компіляції

**Files:**
- Modify: `tests/db_integration.rs`

- [ ] **Step 1: Додати хелпер create_test_payment_schedule**

```rust
async fn create_test_payment_schedule(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Option<Uuid>,
    title: &str,
    amount: Decimal,
    scheduled_date: chrono::NaiveDate,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO payment_schedule
           (id, company_id, counterparty_id, title, amount, direction, scheduled_date, is_completed, recurrence)
           VALUES ($1, $2, $3, $4, $5, 'expense'::payment_direction, $6, FALSE, 'once')"#,
    )
    .bind(id)
    .bind(company_id)
    .bind(counterparty_id)
    .bind(title)
    .bind(amount)
    .bind(scheduled_date)
    .execute(pool)
    .await?;
    Ok(id)
}
```

- [ ] **Step 2: Написати тест load_payables_rows_returns_expense_schedule_entries**

```rust
#[tokio::test]
async fn load_payables_rows_returns_expense_schedule_entries() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_payables_rows;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);

    let cp = create_test_counterparty(&pool, &suffix, &format!("ІТ Payable CP {suffix}"), None, None).await?;

    let title = format!("Оренда ІТ {suffix}");
    let ps_id = create_test_payment_schedule(
        &pool, DEFAULT_COMPANY_ID, Some(cp.id),
        &title, dec!(6000), today,
    ).await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: format!("Оренда ІТ {suffix}"),
    };

    let rows = load_payables_rows(&ctx, &filter).await?;

    assert_eq!(rows.len(), 1, "має бути один запис зі schedule");
    assert_eq!(rows[0].amount, dec!(6000));
    assert!(rows[0].title.contains(&suffix));

    // Cleanup
    sqlx::query("DELETE FROM payment_schedule WHERE id = $1").bind(ps_id).execute(&pool).await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1").bind(cp.id).execute(&pool).await?;

    Ok(())
}
```

- [ ] **Step 3: Перевірити повну компіляцію**

```bash
cargo build --tests 2>&1 | tail -5
```

Очікувано: `Finished ... [unoptimized + debuginfo] target(s) in ...`

- [ ] **Step 4: Запустити всі нові тести разом**

```bash
cargo test load_pnl_rows load_bank_rows compute_opening_balance load_receivables_rows load_payables_rows -- --nocapture
```

Очікувано: всі 5 тестів PASS або SKIP.

- [ ] **Step 5: Commit**

```bash
git add tests/db_integration.rs
git commit -m "test(reports): add integration test for payables rows from payment_schedule"
```

---

## Self-Review

### Spec coverage
- ✅ `compute_opening_balance` — Task 2
- ✅ `load_bank_rows` scope=active — Task 3
- ✅ `load_pnl_rows` з фільтром draft — Task 1
- ✅ `load_receivables_rows` з overdue — Task 4
- ✅ `load_payables_rows` — Task 5
- ⏭ `load_bank_rows` scope=all — залишено на майбутнє (потребує другої компанії, складніший тест)
- ⏭ Top counterparties (Phase 5) — окремий план або backlog

### Placeholder scan
Всі steps містять реальний код, команди та очікувані результати.

### Type consistency
- `ResolvedReportsFilter` використовується однаково у всіх тестах
- `DEFAULT_COMPANY_ID` — константа з main test file
- `dec!(...)` macro з `rust_decimal_macros` — вже є в Cargo.toml
