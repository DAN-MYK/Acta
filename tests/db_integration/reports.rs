use super::*;

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

    let cp = create_test_counterparty(&pool, &suffix, &format!("ІТ PNL CP {suffix}"), None, None)
        .await?;
    let cat_income_id = create_test_category(
        &pool,
        DEFAULT_COMPANY_ID,
        &format!("Послуги {suffix}"),
        "income",
    )
    .await?;

    let act_id = create_test_act(
        &pool,
        DEFAULT_COMPANY_ID,
        cp.id,
        &format!("PNL-{suffix}-1"),
        dec!(10000),
        "issued",
        Some(cat_income_id),
        today,
    )
    .await?;

    let act_id2 = create_test_act(
        &pool,
        DEFAULT_COMPANY_ID,
        cp.id,
        &format!("PNL-{suffix}-2"),
        dec!(5000),
        "issued",
        Some(cat_income_id),
        today,
    )
    .await?;

    let draft_id = create_test_act(
        &pool,
        DEFAULT_COMPANY_ID,
        cp.id,
        &format!("PNL-{suffix}-draft"),
        dec!(99999),
        "draft",
        Some(cat_income_id),
        today,
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: format!("Послуги {suffix}"),
        selected_counterparty_id: None,
    };

    let rows = load_pnl_rows(&ctx, &filter).await?;

    assert_eq!(rows.len(), 1, "має бути рівно 1 категорія після фільтра");
    assert_eq!(
        rows[0].income,
        dec!(15000),
        "10000 + 5000 = агрегація по категорії"
    );
    assert_eq!(rows[0].expense, dec!(0));

    sqlx::query("DELETE FROM acts WHERE id IN ($1, $2, $3)")
        .bind(act_id)
        .bind(act_id2)
        .bind(draft_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(cat_income_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

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

    // Payment BEFORE period_start → included in opening balance
    let p1 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        None,
        dec!(5000),
        "income",
        before_period,
    )
    .await?;
    // Expense BEFORE period_start → reduces opening balance
    let p2 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        None,
        dec!(1000),
        "expense",
        before_period,
    )
    .await?;
    // Payment WITHIN period → NOT in opening balance
    let p3 =
        create_test_payment(&pool, DEFAULT_COMPANY_ID, None, dec!(9999), "income", today).await?;
    // Payment on the boundary date (date_from itself) — must be excluded from opening balance
    // because the SQL uses strict `date < date_from`
    let p4 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        None,
        dec!(777),
        "income",
        period_start,
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: String::new(),
        selected_counterparty_id: None,
    };

    let balance_before = compute_opening_balance(&ctx, &filter).await?;

    // Remove within-period payments (p3) and boundary-date payment (p4),
    // verify neither affects opening balance
    sqlx::query("DELETE FROM payments WHERE id IN ($1, $2)")
        .bind(p3)
        .bind(p4)
        .execute(&pool)
        .await?;

    let balance_after = compute_opening_balance(&ctx, &filter).await?;
    assert_eq!(
        balance_before, balance_after,
        "payment within period and on boundary date must not affect opening balance"
    );

    // The balance must include 5000 income - 1000 expense = net +4000 from our test payments
    // (there may be other payments in the DB, so we check delta)
    // We verify by comparing before and after removing p1 and p2
    sqlx::query("DELETE FROM payments WHERE id IN ($1, $2)")
        .bind(p1)
        .bind(p2)
        .execute(&pool)
        .await?;
    let balance_without_test_payments = compute_opening_balance(&ctx, &filter).await?;
    let delta = balance_before - balance_without_test_payments;
    assert_eq!(
        delta,
        dec!(4000),
        "test payments net 5000-1000=4000 must be in opening balance"
    );

    Ok(())
}

#[tokio::test]
async fn compute_opening_balance_respects_selected_counterparty_id() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::compute_opening_balance;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(10);
    let before_period = today - Duration::days(20);

    let cp_a = create_test_counterparty(
        &pool,
        &suffix,
        &format!("ІТ Opening A {suffix}"),
        None,
        None,
    )
    .await?;
    let cp_b = create_test_counterparty(
        &pool,
        &suffix,
        &format!("ІТ Opening B {suffix}"),
        None,
        None,
    )
    .await?;

    let p1 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp_a.id),
        dec!(5000),
        "income",
        before_period,
    )
    .await?;
    let p2 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp_b.id),
        dec!(1200),
        "income",
        before_period,
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: String::new(),
        selected_counterparty_id: Some(cp_a.id.to_string()),
    };

    let balance_with_cp_a = compute_opening_balance(&ctx, &filter).await?;

    sqlx::query("DELETE FROM payments WHERE id = $1")
        .bind(p1)
        .execute(&pool)
        .await?;
    let balance_without_cp_a_payment = compute_opening_balance(&ctx, &filter).await?;
    assert_eq!(
        balance_with_cp_a - balance_without_cp_a_payment,
        dec!(5000),
        "opening balance має враховувати лише платежі вибраного контрагента"
    );

    sqlx::query("DELETE FROM payments WHERE id = $1")
        .bind(p2)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id IN ($1, $2)")
        .bind(cp_a.id)
        .bind(cp_b.id)
        .execute(&pool)
        .await?;

    Ok(())
}

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

    let cp = create_test_counterparty(&pool, &suffix, &format!("ІТ Bank CP {suffix}"), None, None)
        .await?;

    let p1 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp.id),
        dec!(3000),
        "income",
        today,
    )
    .await?;
    let p2 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp.id),
        dec!(1000),
        "expense",
        today,
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: format!("ІТ Bank CP {suffix}"),
        selected_counterparty_id: None,
    };

    let rows = load_bank_rows(&ctx, &filter).await?;

    assert_eq!(
        rows.len(),
        1,
        "має бути один рядок по контрагенту після query фільтра"
    );
    assert_eq!(rows[0].income, dec!(3000));
    assert_eq!(rows[0].expense, dec!(1000));
    assert!(rows[0].label.contains(&suffix));
    assert_eq!(rows[0].key, cp.id.to_string());

    sqlx::query("DELETE FROM payments WHERE id IN ($1, $2)")
        .bind(p1)
        .bind(p2)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

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
    let overdue_expected = today - Duration::days(10);

    let cp = create_test_counterparty(&pool, &suffix, &format!("ІТ Recv CP {suffix}"), None, None)
        .await?;

    // Рахунок зі статусом issued та простроченою expected_payment_date
    let inv_id = create_test_invoice(
        &pool,
        DEFAULT_COMPANY_ID,
        cp.id,
        &format!("INV-{suffix}"),
        dec!(8000),
        "issued",
        Some(overdue_expected),
        period_start + Duration::days(1),
    )
    .await?;

    // Рахунок зі статусом paid → НЕ повинен з'явитись у дебіторці
    let paid_id = create_test_invoice(
        &pool,
        DEFAULT_COMPANY_ID,
        cp.id,
        &format!("INV-{suffix}-PAID"),
        dec!(5000),
        "paid",
        None,
        period_start + Duration::days(1),
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: format!("ІТ Recv CP {suffix}"),
        selected_counterparty_id: None,
    };

    let rows = load_receivables_rows(&ctx, &filter).await?;

    assert_eq!(rows.len(), 1, "тільки issued рахунок має бути в дебіторці");
    assert_eq!(rows[0].amount, dec!(8000));
    assert!(rows[0].overdue_days >= 10, "прострочка має бути >= 10 днів");

    sqlx::query("DELETE FROM invoices WHERE id IN ($1, $2)")
        .bind(inv_id)
        .bind(paid_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

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

    let cp = create_test_counterparty(
        &pool,
        &suffix,
        &format!("ІТ Payable CP {suffix}"),
        None,
        None,
    )
    .await?;

    let title = format!("Оренда ІТ {suffix}");
    let ps_id = create_test_payment_schedule(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp.id),
        &title,
        dec!(6000),
        today,
    )
    .await?;

    // Виконаний запис — НЕ повинен з'явитись у кредиторці (is_completed = TRUE)
    let completed_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO payment_schedule
           (id, company_id, counterparty_id, title, amount, direction, scheduled_date, is_completed, recurrence)
           VALUES ($1, $2, $3, $4, $5, 'expense'::payment_direction, $6, TRUE, 'none')"#,
    )
    .bind(completed_id)
    .bind(DEFAULT_COMPANY_ID)
    .bind(Some(cp.id))
    .bind(format!("Оренда ІТ {suffix} DONE"))
    .bind(dec!(9999))
    .bind(today)
    .execute(&pool)
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: format!("Оренда ІТ {suffix}"),
        selected_counterparty_id: None,
    };

    let rows = load_payables_rows(&ctx, &filter).await?;

    assert_eq!(
        rows.len(),
        1,
        "має бути один запис: виконаний excluded через is_completed=TRUE"
    );
    assert_eq!(rows[0].amount, dec!(6000));
    assert!(rows[0].title.contains(&suffix));
    assert_eq!(
        rows[0].overdue_days, 0,
        "scheduled_date=today → overdue_days=0"
    );

    sqlx::query("DELETE FROM payment_schedule WHERE id IN ($1, $2)")
        .bind(ps_id)
        .bind(completed_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn load_top_counterparties_bank_ranks_counterparties_by_flow() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_top_counterparties_bank;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);

    let cp_a =
        create_test_counterparty(&pool, &suffix, &format!("ТОВ A {suffix}"), None, None).await?;
    let cp_b =
        create_test_counterparty(&pool, &suffix, &format!("ТОВ B {suffix}"), None, None).await?;

    let p1 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp_a.id),
        dec!(10000),
        "income",
        today,
    )
    .await?;
    let p2 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp_a.id),
        dec!(1000),
        "expense",
        today,
    )
    .await?;
    let p3 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp_b.id),
        dec!(5000),
        "income",
        today,
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: String::new(),
        selected_counterparty_id: None,
    };

    let rows = load_top_counterparties_bank(&ctx, &filter).await?;

    // знаходимо наші рядки серед усіх (можуть бути інші дані в БД)
    let row_a = rows
        .iter()
        .find(|r| r.counterparty_id == cp_a.id.to_string())
        .expect("рядок для cp_a має бути присутній");
    let row_b = rows
        .iter()
        .find(|r| r.counterparty_id == cp_b.id.to_string())
        .expect("рядок для cp_b має бути присутній");

    assert_eq!(row_a.counterparty_name, format!("ТОВ A {suffix}"));
    assert_eq!(row_a.primary_amount, dec!(11000));
    assert_eq!(row_b.primary_amount, dec!(5000));

    // cp_a має бути вище cp_b у рейтингу
    let pos_a = rows
        .iter()
        .position(|r| r.counterparty_id == cp_a.id.to_string())
        .unwrap();
    let pos_b = rows
        .iter()
        .position(|r| r.counterparty_id == cp_b.id.to_string())
        .unwrap();
    assert!(
        pos_a < pos_b,
        "cp_a (11000) має бути вище cp_b (5000) у рейтингу"
    );

    sqlx::query("DELETE FROM payments WHERE id IN ($1, $2, $3)")
        .bind(p1)
        .bind(p2)
        .bind(p3)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id IN ($1, $2)")
        .bind(cp_a.id)
        .bind(cp_b.id)
        .execute(&pool)
        .await?;
    Ok(())
}

#[tokio::test]
async fn load_top_counterparties_bank_respects_bank_name_query_in_active_scope() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_top_counterparties_bank;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);
    let bank_name = format!("Mono fallback {suffix}");

    let payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: today,
            amount: dec!(4200.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name: Some(bank_name.clone()),
            bank_ref: Some(format!("bank-fallback-{suffix}")),
            description: Some("Платіж без контрагента".to_string()),
        },
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: bank_name.clone(),
        selected_counterparty_id: None,
    };

    let rows = load_top_counterparties_bank(&ctx, &filter).await?;

    assert_eq!(
        rows.len(),
        1,
        "fallback bank-name рядок має входити в рейтинг для активної компанії"
    );
    assert_eq!(rows[0].counterparty_name, bank_name);
    assert_eq!(rows[0].primary_amount, dec!(4200.00));

    sqlx::query("DELETE FROM payments WHERE id = $1")
        .bind(payment.id)
        .execute(&pool)
        .await?;
    Ok(())
}

#[tokio::test]
async fn load_top_counterparties_receivables_respects_query_slice() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_top_counterparties_receivables;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);

    let cp_a = create_test_counterparty(
        &pool,
        &suffix,
        &format!("ТОВ Receivable A {suffix}"),
        None,
        None,
    )
    .await?;
    let cp_b = create_test_counterparty(
        &pool,
        &suffix,
        &format!("ТОВ Receivable B {suffix}"),
        None,
        None,
    )
    .await?;

    let doc_number_a = format!("INV-QA-{suffix}");
    let doc_number_b = format!("INV-QB-{suffix}");
    let inv_a = create_test_invoice(
        &pool,
        DEFAULT_COMPANY_ID,
        cp_a.id,
        &doc_number_a,
        dec!(8000),
        "issued",
        None,
        today,
    )
    .await?;
    let inv_b = create_test_invoice(
        &pool,
        DEFAULT_COMPANY_ID,
        cp_b.id,
        &doc_number_b,
        dec!(12000),
        "issued",
        None,
        today,
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: doc_number_a.clone(),
        selected_counterparty_id: None,
    };

    let rows = load_top_counterparties_receivables(&ctx, &filter).await?;

    assert_eq!(
        rows.len(),
        1,
        "рейтинг має відображати лише slice, що збігся по query"
    );
    assert_eq!(rows[0].counterparty_id, cp_a.id.to_string());
    assert_eq!(rows[0].primary_amount, dec!(8000));

    sqlx::query("DELETE FROM invoices WHERE id IN ($1, $2)")
        .bind(inv_a)
        .bind(inv_b)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id IN ($1, $2)")
        .bind(cp_a.id)
        .bind(cp_b.id)
        .execute(&pool)
        .await?;
    Ok(())
}

#[tokio::test]
async fn load_bank_rows_respects_selected_counterparty_id() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_bank_rows;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);

    let cp_a =
        create_test_counterparty(&pool, &suffix, &format!("ТОВ Drill A {suffix}"), None, None)
            .await?;
    let cp_b =
        create_test_counterparty(&pool, &suffix, &format!("ТОВ Drill B {suffix}"), None, None)
            .await?;

    let p1 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp_a.id),
        dec!(7000),
        "income",
        today,
    )
    .await?;
    let p2 = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp_b.id),
        dec!(3000),
        "income",
        today,
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: String::new(),
        selected_counterparty_id: Some(cp_a.id.to_string()),
    };

    let rows = load_bank_rows(&ctx, &filter).await?;

    assert!(
        rows.iter().all(|r| r.key == cp_a.id.to_string()),
        "усі рядки мають бути лише для cp_a, знайдено: {:?}",
        rows.iter().map(|r| &r.key).collect::<Vec<_>>()
    );
    assert!(
        rows.iter().any(|r| r.key == cp_a.id.to_string()),
        "cp_a має бути в результатах"
    );

    sqlx::query("DELETE FROM payments WHERE id IN ($1, $2)")
        .bind(p1)
        .bind(p2)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id IN ($1, $2)")
        .bind(cp_a.id)
        .bind(cp_b.id)
        .execute(&pool)
        .await?;
    Ok(())
}

#[tokio::test]
async fn load_bank_rows_respects_bank_name_selector() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_bank_rows;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);
    let selected_bank_name = format!("Mono direct {suffix}");
    let selected_id = format!("bank-name:{selected_bank_name}");

    let selected_payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: today,
            amount: dec!(3100.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name: Some(selected_bank_name.clone()),
            bank_ref: Some(format!("selected-direct-{suffix}")),
            description: Some("selected direct".to_string()),
        },
    )
    .await?;
    let other_payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: today,
            amount: dec!(700.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name: Some(format!("Other direct {suffix}")),
            bank_ref: Some(format!("other-direct-{suffix}")),
            description: Some("other direct".to_string()),
        },
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: String::new(),
        selected_counterparty_id: Some(selected_id.clone()),
    };

    let rows = load_bank_rows(&ctx, &filter).await?;

    assert_eq!(
        rows.len(),
        1,
        "bank-name selector має лишати лише один fallback рядок"
    );
    assert_eq!(rows[0].key, selected_id);
    assert_eq!(rows[0].label, selected_bank_name);
    assert_eq!(rows[0].income, dec!(3100.00));

    for payment_id in [selected_payment.id, other_payment.id] {
        sqlx::query("DELETE FROM payments WHERE id = $1")
            .bind(payment_id)
            .execute(&pool)
            .await?;
    }

    Ok(())
}

#[tokio::test]
async fn load_top_counterparties_bank_respects_counterparty_query_in_all_scope() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_top_counterparties_bank;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);
    let counterparty_name = format!("Scope All CP {suffix}");
    let cp = create_test_counterparty(&pool, &suffix, &counterparty_name, None, None).await?;

    let payment_id = create_test_payment(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(cp.id),
        dec!(6100),
        "income",
        today,
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::All,
        date_from: period_start,
        date_to: today,
        query: counterparty_name.clone(),
        selected_counterparty_id: None,
    };

    let rows = load_top_counterparties_bank(&ctx, &filter).await?;

    assert_eq!(
        rows.len(),
        1,
        "у scope=all query по назві контрагента має лишати рівно один рядок рейтингу"
    );
    assert_eq!(rows[0].counterparty_id, cp.id.to_string());
    assert_eq!(rows[0].counterparty_name, counterparty_name);

    sqlx::query("DELETE FROM payments WHERE id = $1")
        .bind(payment_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn reports_load_keeps_selected_counterparty_when_it_is_outside_top_counterparties(
) -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::tauri_api::reports::{reports_load, ReportsLoadRequest};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);

    let mut counterparty_ids = Vec::new();
    let mut payment_ids = Vec::new();

    for index in 0..9 {
        let cp = create_test_counterparty(
            &pool,
            &suffix,
            &format!("ТОВ Reports Pick {} {suffix}", index + 1),
            None,
            Some(format!("reports-pick-{suffix}-{index}")),
        )
        .await?;
        let payment_id = create_test_payment(
            &pool,
            DEFAULT_COMPANY_ID,
            Some(cp.id),
            dec!(1000) * Decimal::from(9 - index),
            "income",
            today,
        )
        .await?;

        counterparty_ids.push((cp.id, cp.name));
        payment_ids.push(payment_id);
    }

    let selected_counterparty = counterparty_ids
        .last()
        .cloned()
        .expect("має бути створений дев'ятий контрагент");
    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);

    let screen = reports_load(
        &ctx,
        ReportsLoadRequest {
            tab: Some("bank".to_string()),
            scope: Some("active".to_string()),
            date_from: Some(period_start.format("%Y-%m-%d").to_string()),
            date_to: Some(today.format("%Y-%m-%d").to_string()),
            query: Some(String::new()),
            selected_counterparty_id: Some(selected_counterparty.0.to_string()),
        },
    )
    .await?;

    assert_eq!(
        screen.top_counterparties.len(),
        8,
        "топ має обмежуватись 8 рядками"
    );
    assert!(
        screen
            .top_counterparties
            .iter()
            .all(|row| row.counterparty_id != selected_counterparty.0.to_string()),
        "обраний контрагент має лишитись поза топ-8 у цьому сценарії"
    );
    assert_eq!(
        screen.selected_counterparty,
        Some(acta::tauri_api::reports::SelectedCounterpartyDto {
            id: selected_counterparty.0.to_string(),
            name: selected_counterparty.1.clone(),
        })
    );
    assert!(
        !screen.bank_rows.is_empty(),
        "drill-down не має повертати порожній bank_rows для вибраного контрагента"
    );
    assert!(
        screen
            .bank_rows
            .iter()
            .all(|row| row.key == selected_counterparty.0.to_string()),
        "drill-down має залишити у bank_rows лише вибраного контрагента"
    );

    for payment_id in payment_ids {
        sqlx::query("DELETE FROM payments WHERE id = $1")
            .bind(payment_id)
            .execute(&pool)
            .await?;
    }
    for (counterparty_id, _) in counterparty_ids {
        sqlx::query("DELETE FROM counterparties WHERE id = $1")
            .bind(counterparty_id)
            .execute(&pool)
            .await?;
    }

    Ok(())
}

#[tokio::test]
async fn reports_load_keeps_selected_counterparty_from_other_company_in_all_scope() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::models::company::NewCompany;
    use acta::tauri_api::reports::{reports_load, ReportsLoadRequest, SelectedCounterpartyDto};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);

    let foreign_company = db::companies::create(
        &pool,
        &NewCompany {
            name: format!("ІТ Foreign Reports {suffix}"),
            short_name: Some(format!("FR {suffix}")),
            edrpou: None,
            ipn: None,
            iban: None,
            legal_address: None,
            director_name: None,
            tax_system: None,
            is_vat_payer: false,
        },
    )
    .await?;

    let selected_counterparty = db::counterparties::create(
        &pool,
        foreign_company.id,
        &models::NewCounterparty {
            name: format!("ІТ Cross Company Selected {suffix}"),
            edrpou: None,
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("cross-company-selected-{suffix}")),
        },
    )
    .await?;

    let selected_payment = create_test_payment(
        &pool,
        foreign_company.id,
        Some(selected_counterparty.id),
        dec!(500),
        "income",
        today,
    )
    .await?;

    let mut default_counterparty_ids = Vec::new();
    let mut default_payment_ids = Vec::new();
    for index in 0..8 {
        let cp = create_test_counterparty(
            &pool,
            &format!("{suffix}-{index}"),
            &format!("ІТ Local Top {index} {suffix}"),
            None,
            None,
        )
        .await?;
        let payment_id = create_test_payment(
            &pool,
            DEFAULT_COMPANY_ID,
            Some(cp.id),
            dec!(1000) * Decimal::from(9 - index),
            "income",
            today,
        )
        .await?;
        default_counterparty_ids.push(cp.id);
        default_payment_ids.push(payment_id);
    }

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let screen = reports_load(
        &ctx,
        ReportsLoadRequest {
            tab: Some("bank".to_string()),
            scope: Some("all".to_string()),
            date_from: Some(period_start.format("%Y-%m-%d").to_string()),
            date_to: Some(today.format("%Y-%m-%d").to_string()),
            query: Some(String::new()),
            selected_counterparty_id: Some(selected_counterparty.id.to_string()),
        },
    )
    .await?;

    assert_eq!(
        screen.top_counterparties.len(),
        8,
        "топ має лишатись обмеженим 8 рядками"
    );
    assert!(
        screen
            .top_counterparties
            .iter()
            .all(|row| row.counterparty_id != selected_counterparty.id.to_string()),
        "обраний контрагент з іншої компанії має лишатись поза top-8 у цьому сценарії"
    );
    assert_eq!(
        screen.selected_counterparty,
        Some(SelectedCounterpartyDto {
            id: selected_counterparty.id.to_string(),
            name: selected_counterparty.name.clone(),
        })
    );

    sqlx::query("DELETE FROM payments WHERE id = $1")
        .bind(selected_payment)
        .execute(&pool)
        .await?;
    for payment_id in default_payment_ids {
        sqlx::query("DELETE FROM payments WHERE id = $1")
            .bind(payment_id)
            .execute(&pool)
            .await?;
    }
    for counterparty_id in default_counterparty_ids {
        sqlx::query("DELETE FROM counterparties WHERE id = $1")
            .bind(counterparty_id)
            .execute(&pool)
            .await?;
    }
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(selected_counterparty.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(foreign_company.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn reports_load_filters_bank_fallback_selector_end_to_end() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::tauri_api::reports::{reports_load, ReportsLoadRequest};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);
    let before_period = today - Duration::days(45);
    let selected_bank_name = format!("Mono fallback {suffix}");
    let other_bank_name = format!("Other fallback {suffix}");
    let selected_id = format!("bank-name:{selected_bank_name}");

    let selected_before = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: before_period,
            amount: dec!(1000.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name: Some(selected_bank_name.clone()),
            bank_ref: Some(format!("selected-before-{suffix}")),
            description: Some("opening selected".to_string()),
        },
    )
    .await?;
    let other_before = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: before_period,
            amount: dec!(2500.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name: Some(other_bank_name.clone()),
            bank_ref: Some(format!("other-before-{suffix}")),
            description: Some("opening other".to_string()),
        },
    )
    .await?;
    let selected_income = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: today,
            amount: dec!(4200.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name: Some(selected_bank_name.clone()),
            bank_ref: Some(format!("selected-income-{suffix}")),
            description: Some("period selected income".to_string()),
        },
    )
    .await?;
    let selected_expense = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: today,
            amount: dec!(200.00),
            direction: models::payment::PaymentDirection::Expense,
            counterparty_id: None,
            bank_name: Some(selected_bank_name.clone()),
            bank_ref: Some(format!("selected-expense-{suffix}")),
            description: Some("period selected expense".to_string()),
        },
    )
    .await?;
    let other_income = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: today,
            amount: dec!(999.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name: Some(other_bank_name.clone()),
            bank_ref: Some(format!("other-income-{suffix}")),
            description: Some("period other income".to_string()),
        },
    )
    .await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let screen = reports_load(
        &ctx,
        ReportsLoadRequest {
            tab: Some("bank".to_string()),
            scope: Some("active".to_string()),
            date_from: Some(period_start.format("%Y-%m-%d").to_string()),
            date_to: Some(today.format("%Y-%m-%d").to_string()),
            query: Some(String::new()),
            selected_counterparty_id: Some(selected_id.clone()),
        },
    )
    .await?;

    assert_eq!(
        screen.selected_counterparty,
        Some(acta::tauri_api::reports::SelectedCounterpartyDto {
            id: selected_id.clone(),
            name: selected_bank_name.clone(),
        })
    );
    assert_eq!(screen.summary.opening_balance_str, "1 000,00 грн");
    assert_eq!(
        screen.bank_rows.len(),
        1,
        "fallback drill-down має лишити один bank row"
    );
    assert_eq!(screen.bank_rows[0].key, selected_id);
    assert_eq!(screen.bank_rows[0].label, selected_bank_name);
    assert_eq!(screen.bank_rows[0].income_str, "4 200,00 грн");
    assert_eq!(screen.bank_rows[0].expense_str, "200,00 грн");

    for payment_id in [
        selected_before.id,
        other_before.id,
        selected_income.id,
        selected_expense.id,
        other_income.id,
    ] {
        sqlx::query("DELETE FROM payments WHERE id = $1")
            .bind(payment_id)
            .execute(&pool)
            .await?;
    }

    Ok(())
}
