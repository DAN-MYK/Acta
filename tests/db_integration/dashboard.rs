use super::*;

pub(super) async fn dashboard_test_setup(pool: &PgPool, suffix: &str) -> Result<(Uuid, Uuid)> {
    let company = db::companies::create(
        pool,
        &models::NewCompany {
            name: format!("ІТ Dashboard Компанія {suffix}"),
            short_name: None,
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            legal_address: None,
            director_name: None,
            tax_system: None,
            is_vat_payer: false,
        },
    )
    .await?;

    let cp = db::counterparties::create(
        pool,
        company.id,
        &models::NewCounterparty {
            name: format!("ІТ Dashboard Контрагент {suffix}"),
            edrpou: None,
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: None,
        },
    )
    .await?;

    Ok((company.id, cp.id))
}

/// Видаляє всі тестові дані компанії (акти → контрагенти → компанія).
pub(super) async fn dashboard_test_cleanup(pool: &PgPool, company_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM acts WHERE company_id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE company_id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Створює мінімальний акт для поточного місяця.
async fn make_act(
    pool: &PgPool,
    company_id: Uuid,
    cp_id: Uuid,
    suffix: &str,
    tag: &str,
    amount: rust_decimal::Decimal,
    expected_payment_date: Option<chrono::NaiveDate>,
) -> Result<Uuid> {
    let act = db::acts::create(
        pool,
        company_id,
        &models::NewAct {
            number: format!("IT-DASH-{tag}-{suffix}"),
            counterparty_id: cp_id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: None,
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: amount,
            }],
        },
    )
    .await?;
    Ok(act.id)
}

// ─── Dashboard: KPI summary ───────────────────────────────────────────────────

#[tokio::test]
async fn dashboard_kpi_summary_aggregates_acts_correctly() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    // Чернетка — не враховується ні в revenue, ні в unpaid
    make_act(
        &pool,
        company_id,
        cp_id,
        &suffix,
        "DRAFT",
        dec!(500.00),
        None,
    )
    .await?;

    // Виставлений — в unpaid_total
    let issued_id = make_act(
        &pool,
        company_id,
        cp_id,
        &suffix,
        "ISSUED",
        dec!(2000.00),
        None,
    )
    .await?;
    db::acts::change_status(&pool, issued_id, models::ActStatus::Issued).await?;

    // Оплачений — в revenue_this_month
    let paid_id = make_act(
        &pool,
        company_id,
        cp_id,
        &suffix,
        "PAID",
        dec!(3000.00),
        None,
    )
    .await?;
    db::acts::change_status(&pool, paid_id, models::ActStatus::Issued).await?;
    sqlx::query("UPDATE acts SET status = 'paid' WHERE id = $1")
        .bind(paid_id)
        .execute(&pool)
        .await?;

    let kpi = db::dashboard::get_kpi_summary(&pool, company_id).await?;

    assert_eq!(
        kpi.revenue_this_month,
        dec!(3000.00),
        "тільки оплачені акти поточного місяця"
    );
    assert_eq!(kpi.unpaid_total, dec!(2000.00), "виставлені + підписані");
    assert_eq!(kpi.acts_this_month, 3, "всі три акти — поточний місяць");
    assert_eq!(kpi.active_counterparties, 1, "один активний контрагент");

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Dashboard: revenue by month ─────────────────────────────────────────────

#[tokio::test]
async fn dashboard_revenue_by_month_fills_all_slots() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    // Оплачений акт у поточному місяці
    let act_id = make_act(
        &pool,
        company_id,
        cp_id,
        &suffix,
        "REV",
        dec!(7777.00),
        None,
    )
    .await?;
    db::acts::change_status(&pool, act_id, models::ActStatus::Issued).await?;
    sqlx::query("UPDATE acts SET status = 'paid' WHERE id = $1")
        .bind(act_id)
        .execute(&pool)
        .await?;

    // Перевіряємо для N = 1, 3, 6
    for months in [1u32, 3, 6] {
        let result = db::dashboard::revenue_by_month(&pool, company_id, months).await?;

        assert_eq!(
            result.len(),
            months as usize,
            "revenue_by_month({months}) має повертати рівно {months} записів"
        );

        // Знаходимо слот поточного місяця
        let today = Utc::now().date_naive();
        let current_slot = result
            .iter()
            .find(|m| m.month_num == today.month() && m.year == today.year())
            .expect("поточний місяць має бути в результаті");
        assert_eq!(
            current_slot.amount,
            dec!(7777.00),
            "оплачений акт у поточному місяці"
        );

        // Решта слотів — нуль (свіжа компанія без інших актів)
        for slot in result
            .iter()
            .filter(|m| !(m.month_num == today.month() && m.year == today.year()))
        {
            assert_eq!(slot.amount, dec!(0), "порожній місяць має суму 0");
        }

        // Місяці — в монотонному порядку (всі (year, month) зростають або спадають)
        let pairs: Vec<(i32, u32)> = result.iter().map(|m| (m.year, m.month_num)).collect();
        let ascending = pairs
            .windows(2)
            .all(|w| (w[0].0, w[0].1) <= (w[1].0, w[1].1));
        let descending = pairs
            .windows(2)
            .all(|w| (w[0].0, w[0].1) >= (w[1].0, w[1].1));
        assert!(
            ascending || descending,
            "місяці мають бути впорядковані монотонно"
        );
    }

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Dashboard: acts status distribution ─────────────────────────────────────

#[tokio::test]
async fn dashboard_acts_status_distribution_counts_by_status() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    // 1 чернетка, 2 виставлені, 1 підписаний — всі в поточному місяці
    make_act(&pool, company_id, cp_id, &suffix, "D1", dec!(100.00), None).await?;

    let i1 = make_act(&pool, company_id, cp_id, &suffix, "I1", dec!(200.00), None).await?;
    let i2 = make_act(&pool, company_id, cp_id, &suffix, "I2", dec!(300.00), None).await?;
    db::acts::change_status(&pool, i1, models::ActStatus::Issued).await?;
    db::acts::change_status(&pool, i2, models::ActStatus::Issued).await?;

    let s1 = make_act(&pool, company_id, cp_id, &suffix, "S1", dec!(400.00), None).await?;
    db::acts::change_status(&pool, s1, models::ActStatus::Issued).await?;
    db::acts::advance_status(&pool, s1).await?;

    let slices = db::dashboard::acts_status_distribution(&pool, company_id).await?;

    let count_for = |status: &str| -> i64 {
        slices
            .iter()
            .find(|s| s.status == status)
            .map(|s| s.count)
            .unwrap_or(0)
    };

    assert_eq!(count_for("draft"), 1, "одна чернетка");
    assert_eq!(count_for("issued"), 2, "два виставлені");
    assert_eq!(count_for("signed"), 1, "один підписаний");
    assert_eq!(count_for("paid"), 0, "оплачених немає");

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Dashboard: upcoming payments ────────────────────────────────────────────

#[tokio::test]
async fn dashboard_upcoming_payments_overdue_appears_first() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    let yesterday = (Utc::now() - Duration::days(1)).date_naive();
    let next_month = (Utc::now() + Duration::days(30)).date_naive();

    // Прострочений — вчора, статус issued
    let overdue_id = make_act(
        &pool,
        company_id,
        cp_id,
        &suffix,
        "OVD",
        dec!(1500.00),
        Some(yesterday),
    )
    .await?;
    db::acts::change_status(&pool, overdue_id, models::ActStatus::Issued).await?;

    // Майбутній — +30 днів, статус signed
    let future_id = make_act(
        &pool,
        company_id,
        cp_id,
        &suffix,
        "FUT",
        dec!(2500.00),
        Some(next_month),
    )
    .await?;
    db::acts::change_status(&pool, future_id, models::ActStatus::Issued).await?;
    db::acts::advance_status(&pool, future_id).await?;

    let upcoming = db::dashboard::upcoming_payments(&pool, company_id, 10).await?;

    assert_eq!(
        upcoming.len(),
        2,
        "обидва акти з expected_payment_date мають бути в списку"
    );

    // Прострочений йде першим
    assert!(upcoming[0].is_overdue, "перший запис має бути прострочений");
    assert_eq!(upcoming[0].amount, dec!(1500.00));

    // Майбутній — другий, не прострочений
    assert!(
        !upcoming[1].is_overdue,
        "другий запис не має бути прострочений"
    );
    assert_eq!(upcoming[1].amount, dec!(2500.00));

    // Формат дати: "DD Міс" (наприклад "07 Кві")
    assert!(
        upcoming[0].date_label.len() >= 6 && upcoming[0].date_label.contains(' '),
        "date_label має формат 'DD Міс': '{}'",
        upcoming[0].date_label
    );

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Dashboard: recent acts ───────────────────────────────────────────────────

#[tokio::test]
async fn dashboard_get_recent_acts_returns_latest_first() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    make_act(&pool, company_id, cp_id, &suffix, "R1", dec!(1000.00), None).await?;
    make_act(&pool, company_id, cp_id, &suffix, "R2", dec!(2000.00), None).await?;

    let recent = db::dashboard::get_recent_acts(&pool, company_id, 5).await?;

    assert_eq!(recent.len(), 2, "обидва акти мають бути в результаті");

    // Найновіший (R2) — перший завдяки ORDER BY created_at DESC
    assert_eq!(recent[0].num, format!("IT-DASH-R2-{suffix}"));
    assert_eq!(recent[1].num, format!("IT-DASH-R1-{suffix}"));

    // Статус — рядок "draft"
    assert_eq!(recent[0].status, "draft");

    // Формат дати: "ДД.ММ.РРРР" — рівно 10 символів
    assert_eq!(recent[0].date.len(), 10, "date має формат ДД.ММ.РРРР");
    assert_eq!(recent[0].date.chars().nth(2), Some('.'));
    assert_eq!(recent[0].date.chars().nth(5), Some('.'));

    // limit=1 повертає тільки один запис
    let limited = db::dashboard::get_recent_acts(&pool, company_id, 1).await?;
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].num, format!("IT-DASH-R2-{suffix}"));

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn dashboard_company_isolation_applies_to_kpi_and_revenue() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_a, cp_a) = dashboard_test_setup(&pool, &format!("{suffix}-A")).await?;
    let (company_b, cp_b) = dashboard_test_setup(&pool, &format!("{suffix}-B")).await?;

    let act_a = make_act(
        &pool,
        company_a,
        cp_a,
        &format!("{suffix}-A"),
        "ISO-A",
        dec!(1111.00),
        None,
    )
    .await?;
    db::acts::change_status(&pool, act_a, models::ActStatus::Issued).await?;
    sqlx::query("UPDATE acts SET status = 'paid' WHERE id = $1")
        .bind(act_a)
        .execute(&pool)
        .await?;

    let act_b = make_act(
        &pool,
        company_b,
        cp_b,
        &format!("{suffix}-B"),
        "ISO-B",
        dec!(9999.00),
        None,
    )
    .await?;
    db::acts::change_status(&pool, act_b, models::ActStatus::Issued).await?;
    sqlx::query("UPDATE acts SET status = 'paid' WHERE id = $1")
        .bind(act_b)
        .execute(&pool)
        .await?;

    let kpi_a = db::dashboard::get_kpi_summary(&pool, company_a).await?;
    assert_eq!(kpi_a.revenue_this_month, dec!(1111.00));
    assert_eq!(kpi_a.unpaid_total, dec!(0));
    assert_eq!(kpi_a.active_counterparties, 1);

    let revenue_a = db::dashboard::revenue_by_month(&pool, company_a, 3).await?;
    let total_a: Decimal = revenue_a.iter().map(|row| row.amount).sum();
    assert_eq!(
        total_a,
        dec!(1111.00),
        "ряд доходу не повинен бачити іншу компанію"
    );

    dashboard_test_cleanup(&pool, company_a).await?;
    dashboard_test_cleanup(&pool, company_b).await?;
    Ok(())
}

#[tokio::test]
async fn dashboard_upcoming_payments_includes_only_issued_and_signed() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;
    let due_date = (Utc::now() + Duration::days(7)).date_naive();

    let draft_id = make_act(
        &pool,
        company_id,
        cp_id,
        &suffix,
        "UP-DRAFT",
        dec!(100.00),
        Some(due_date),
    )
    .await?;

    let issued_id = make_act(
        &pool,
        company_id,
        cp_id,
        &suffix,
        "UP-ISSUED",
        dec!(200.00),
        Some(due_date),
    )
    .await?;
    db::acts::change_status(&pool, issued_id, models::ActStatus::Issued).await?;

    let signed_id = make_act(
        &pool,
        company_id,
        cp_id,
        &suffix,
        "UP-SIGNED",
        dec!(300.00),
        Some(due_date),
    )
    .await?;
    db::acts::change_status(&pool, signed_id, models::ActStatus::Issued).await?;
    db::acts::advance_status(&pool, signed_id).await?;

    let paid_id = make_act(
        &pool,
        company_id,
        cp_id,
        &suffix,
        "UP-PAID",
        dec!(400.00),
        Some(due_date),
    )
    .await?;
    db::acts::change_status(&pool, paid_id, models::ActStatus::Issued).await?;
    sqlx::query("UPDATE acts SET status = 'paid' WHERE id = $1")
        .bind(paid_id)
        .execute(&pool)
        .await?;

    let upcoming = db::dashboard::upcoming_payments(&pool, company_id, 10).await?;
    let amounts: Vec<Decimal> = upcoming.iter().map(|row| row.amount).collect();

    assert_eq!(
        upcoming.len(),
        2,
        "тільки issued і signed мають потрапити в upcoming"
    );
    assert!(amounts.contains(&dec!(200.00)));
    assert!(amounts.contains(&dec!(300.00)));
    assert!(
        !amounts.contains(&dec!(100.00)),
        "draft не має потрапляти в upcoming"
    );
    assert!(
        !amounts.contains(&dec!(400.00)),
        "paid не має потрапляти в upcoming"
    );

    // suppress unused warnings for ids that intentionally stay only in DB rows
    let _ = draft_id;

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn dashboard_empty_company_returns_zeroed_metrics() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, _cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    let kpi = db::dashboard::get_kpi_summary(&pool, company_id).await?;
    assert_eq!(kpi.revenue_this_month, dec!(0));
    assert_eq!(kpi.unpaid_total, dec!(0));
    assert_eq!(kpi.acts_this_month, 0);
    assert_eq!(
        kpi.active_counterparties, 1,
        "setup створює одного активного контрагента"
    );

    let revenue = db::dashboard::revenue_by_month(&pool, company_id, 4).await?;
    assert_eq!(revenue.len(), 4);
    assert!(revenue.iter().all(|row| row.amount == dec!(0)));

    let status = db::dashboard::acts_status_distribution(&pool, company_id).await?;
    assert!(status.is_empty());

    let upcoming = db::dashboard::upcoming_payments(&pool, company_id, 5).await?;
    assert!(upcoming.is_empty());

    let recent = db::dashboard::get_recent_acts(&pool, company_id, 5).await?;
    assert!(recent.is_empty());

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn dashboard_recent_acts_respects_limit() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    make_act(&pool, company_id, cp_id, &suffix, "L1", dec!(100.00), None).await?;
    make_act(&pool, company_id, cp_id, &suffix, "L2", dec!(200.00), None).await?;
    make_act(&pool, company_id, cp_id, &suffix, "L3", dec!(300.00), None).await?;

    let recent = db::dashboard::get_recent_acts(&pool, company_id, 2).await?;
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].num, format!("IT-DASH-L3-{suffix}"));
    assert_eq!(recent[1].num, format!("IT-DASH-L2-{suffix}"));

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Contracts ────────────────────────────────────────────────────────────────
