use super::super::*;

#[tokio::test]
async fn acts_create_and_status_flow_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Акт Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-act-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-ACT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: Some("integration test".to_string()),
            bas_id: Some(format!("it-act-{suffix}")),
            items: vec![
                models::NewActItem {
                    description: "Послуга 1".to_string(),
                    quantity: dec!(2.0000),
                    unit: "год".to_string(),
                    unit_price: dec!(1000.00),
                },
                models::NewActItem {
                    description: "Послуга 2".to_string(),
                    quantity: dec!(1.0000),
                    unit: "год".to_string(),
                    unit_price: dec!(500.00),
                },
            ],
        },
    )
    .await?;

    assert_eq!(act.total_amount, dec!(2500.00));

    let loaded = db::acts::get_by_id(&pool, act.id)
        .await?
        .expect("act exists");
    assert_eq!(loaded.0.number, act.number);
    assert_eq!(loaded.1.len(), 2);

    let issued = db::acts::change_status(&pool, act.id, models::ActStatus::Issued)
        .await?
        .expect("status changed");
    assert_eq!(issued.status, models::ActStatus::Issued);

    let invalid = db::acts::change_status(&pool, act.id, models::ActStatus::Draft).await;
    assert!(invalid.is_err());

    let signed = db::acts::advance_status(&pool, act.id)
        .await?
        .expect("advanced");
    assert_eq!(signed.status, models::ActStatus::Signed);

    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn acts_change_status_rejects_skipping_forward_transition() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Акт Skip Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-act-skip-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-ACT-SKIP-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: Some(format!("it-act-skip-{suffix}")),
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "год".to_string(),
                unit_price: dec!(1000.00),
            }],
        },
    )
    .await?;

    let result = db::acts::change_status(&pool, act.id, models::ActStatus::Paid).await;
    assert!(result.is_err());

    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn acts_change_status_rejects_same_status_transition() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Акт Same Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-act-same-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-ACT-SAME-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: Some(format!("it-act-same-{suffix}")),
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "год".to_string(),
                unit_price: dec!(800.00),
            }],
        },
    )
    .await?;

    let result = db::acts::change_status(&pool, act.id, models::ActStatus::Draft).await;
    assert!(result.is_err());

    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn acts_change_status_rejects_transition_from_paid() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Акт Paid Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-act-paid-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-ACT-PAID-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: Some(format!("it-act-paid-{suffix}")),
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "год".to_string(),
                unit_price: dec!(1200.00),
            }],
        },
    )
    .await?;

    db::acts::change_status(&pool, act.id, models::ActStatus::Issued).await?;
    db::acts::change_status(&pool, act.id, models::ActStatus::Signed).await?;
    db::acts::change_status(&pool, act.id, models::ActStatus::Paid).await?;

    let result = db::acts::change_status(&pool, act.id, models::ActStatus::Issued).await;
    assert!(result.is_err());

    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn acts_advance_status_on_paid_returns_error() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Акт Advance Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-act-advance-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-ACT-ADV-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: Some(format!("it-act-advance-{suffix}")),
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "год".to_string(),
                unit_price: dec!(1400.00),
            }],
        },
    )
    .await?;

    db::acts::change_status(&pool, act.id, models::ActStatus::Issued).await?;
    db::acts::change_status(&pool, act.id, models::ActStatus::Signed).await?;
    db::acts::change_status(&pool, act.id, models::ActStatus::Paid).await?;

    let result = db::acts::advance_status(&pool, act.id).await;
    assert!(result.is_err());

    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn acts_status_functions_return_none_for_missing_id() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let missing_id = Uuid::new_v4();

    let change_result =
        db::acts::change_status(&pool, missing_id, models::ActStatus::Issued).await?;
    assert!(matches!(change_result, None));

    let advance_result = db::acts::advance_status(&pool, missing_id).await?;
    assert!(matches!(advance_result, None));

    Ok(())
}

#[tokio::test]
async fn acts_generate_next_number_uses_numeric_max() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let year = Utc::now().year();

    // Ізольована компанія — без жодного акту
    let (company_id,): (Uuid,) =
        sqlx::query_as("INSERT INTO companies (name) VALUES ($1) RETURNING id")
            .bind(format!("ІТ NextNum Компанія {suffix}"))
            .fetch_one(&pool)
            .await?;

    // Порожня компанія → перший номер завжди "АКТ-РРРР-001"
    let first = db::acts::generate_next_number(&pool, company_id).await?;
    assert_eq!(first, format!("АКТ-{year}-001"));

    let cp = db::counterparties::create(
        &pool,
        company_id,
        &models::NewCounterparty {
            name: format!("ІТ NextNum Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-nn-cp-{suffix}")),
        },
    )
    .await?;

    // Вставляємо непадовані номери "АКТ-РРРР-9" та "АКТ-РРРР-10":
    // лексикографічно "9" > "10", але числово max = 10 → очікуємо "011"
    for num_suffix in ["9", "10"] {
        db::acts::create(
            &pool,
            company_id,
            &models::NewAct {
                number: format!("АКТ-{year}-{num_suffix}"),
                counterparty_id: cp.id,
                contract_id: None,
                category_id: None,
                direction: models::DocumentDirection::Outgoing,
                date: Utc::now().date_naive(),
                expected_payment_date: None,
                status: models::ActStatus::Draft,
                notes: None,
                bas_id: None,
                items: vec![models::NewActItem {
                    description: "Тест".to_string(),
                    quantity: dec!(1.0000),
                    unit: "шт".to_string(),
                    unit_price: dec!(1.00),
                }],
            },
        )
        .await?;
    }

    let next = db::acts::generate_next_number(&pool, company_id).await?;
    assert_eq!(
        next,
        format!("АКТ-{year}-011"),
        "числовий MAX(10) + 1 = 11, а не лексикографічний ('9' > '10')"
    );

    sqlx::query("DELETE FROM acts WHERE company_id = $1")
        .bind(company_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(company_id)
        .execute(&pool)
        .await?;

    Ok(())
}

// ─── Acts: get_kpi ───────────────────────────────────────────────────────────

#[tokio::test]
async fn acts_get_kpi_aggregates_this_month_and_overdue() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let today = Utc::now().date_naive();
    // Дата старіша 30 днів — гарантовано потрапляє у overdue
    let overdue_date = today - Duration::days(45);

    let (company_id,): (Uuid,) =
        sqlx::query_as("INSERT INTO companies (name) VALUES ($1) RETURNING id")
            .bind(format!("ІТ KPI Компанія {suffix}"))
            .fetch_one(&pool)
            .await?;

    let cp = db::counterparties::create(
        &pool,
        company_id,
        &models::NewCounterparty {
            name: format!("ІТ KPI Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-kpi-cp-{suffix}")),
        },
    )
    .await?;

    let make_item = |price: u32| models::NewActItem {
        description: "Послуга".to_string(),
        quantity: dec!(1.0000),
        unit: "шт".to_string(),
        unit_price: rust_decimal::Decimal::from(price),
    };

    // Акт 1: today, статус Draft → acts_this_month += 1
    db::acts::create(
        &pool,
        company_id,
        &models::NewAct {
            number: format!("KPI-DRAFT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: today,
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: None,
            items: vec![make_item(1000)],
        },
    )
    .await?;

    // Акт 2: today, статус Paid → acts_this_month += 1, revenue_this_month += 2000
    let act_paid = db::acts::create(
        &pool,
        company_id,
        &models::NewAct {
            number: format!("KPI-PAID-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: today,
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: None,
            items: vec![make_item(2000)],
        },
    )
    .await?;
    db::acts::change_status(&pool, act_paid.id, models::ActStatus::Issued).await?;
    db::acts::change_status(&pool, act_paid.id, models::ActStatus::Signed).await?;
    db::acts::change_status(&pool, act_paid.id, models::ActStatus::Paid).await?;

    // Акт 3: today, статус Issued → acts_this_month += 1, unpaid_total += 3000
    let act_issued_new = db::acts::create(
        &pool,
        company_id,
        &models::NewAct {
            number: format!("KPI-ISSUED-NEW-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: today,
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: None,
            items: vec![make_item(3000)],
        },
    )
    .await?;
    db::acts::change_status(&pool, act_issued_new.id, models::ActStatus::Issued).await?;

    // Акт 4: дата 45 днів тому, статус Issued
    // → unpaid_total += 4000, overdue_count += 1 (issued + date < today-30d)
    // → НЕ входить до acts_this_month (45 днів тому ≠ поточний місяць)
    let act_overdue = db::acts::create(
        &pool,
        company_id,
        &models::NewAct {
            number: format!("KPI-OVERDUE-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: overdue_date,
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: None,
            items: vec![make_item(4000)],
        },
    )
    .await?;
    db::acts::change_status(&pool, act_overdue.id, models::ActStatus::Issued).await?;

    let kpi = db::acts::get_kpi(&pool, company_id).await?;

    // acts 1, 2, 3 мають date = today (поточний місяць), act 4 — 45 днів тому
    assert_eq!(kpi.acts_this_month, 3);

    // Тільки act 2 (paid + цей місяць) = 2000
    assert_eq!(kpi.revenue_this_month, dec!(2000.00));

    // acts 3 (3000) + 4 (4000) = 7000 — статус issued, незалежно від дати
    assert_eq!(kpi.unpaid_total, dec!(7000.00));

    // Тільки act 4: issued І date < today-30d
    assert_eq!(kpi.overdue_count, 1);

    sqlx::query("DELETE FROM acts WHERE company_id = $1")
        .bind(company_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(company_id)
        .execute(&pool)
        .await?;

    Ok(())
}

// ─── Acts: update_with_items ─────────────────────────────────────────────────

#[tokio::test]
async fn acts_update_with_items_replaces_positions_and_recalculates_total() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ UpdateItems Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-uwi-cp-{suffix}")),
        },
    )
    .await?;

    // Створюємо акт з двома позиціями: 500 + 300 = 800
    let original = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("UWI-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: Some("оригінал".to_string()),
            bas_id: None,
            items: vec![
                models::NewActItem {
                    description: "Стара послуга 1".to_string(),
                    quantity: dec!(1.0000),
                    unit: "шт".to_string(),
                    unit_price: dec!(500.00),
                },
                models::NewActItem {
                    description: "Стара послуга 2".to_string(),
                    quantity: dec!(1.0000),
                    unit: "шт".to_string(),
                    unit_price: dec!(300.00),
                },
            ],
        },
    )
    .await?;
    assert_eq!(original.total_amount, dec!(800.00));

    let (_, items_before) = db::acts::get_by_id(&pool, original.id)
        .await?
        .expect("act exists");
    assert_eq!(items_before.len(), 2);

    // Оновлюємо: 1 нова позиція qty=3, price=400 → total = 1200
    let updated = db::acts::update_with_items(
        &pool,
        original.id,
        models::UpdateAct {
            number: format!("UWI-UPDATED-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: Some("оновлено".to_string()),
        },
        vec![models::NewActItem {
            description: "Нова послуга".to_string(),
            quantity: dec!(3.0000),
            unit: "год".to_string(),
            unit_price: dec!(400.00),
        }],
    )
    .await?;

    // Перевіряємо заголовок
    assert_eq!(updated.number, format!("UWI-UPDATED-{suffix}"));
    assert_eq!(updated.total_amount, dec!(1200.00));
    assert_eq!(updated.notes.as_deref(), Some("оновлено"));

    // Перевіряємо позиції: старі замінились на нову
    let (_, items_after) = db::acts::get_by_id(&pool, original.id)
        .await?
        .expect("act exists after update");
    assert_eq!(items_after.len(), 1);
    assert_eq!(items_after[0].description, "Нова послуга");
    assert_eq!(items_after[0].amount, dec!(1200.00));

    // Оновлення неіснуючого акту — anyhow помилка з текстом "не знайдено"
    let missing_err = db::acts::update_with_items(
        &pool,
        Uuid::new_v4(),
        models::UpdateAct {
            number: "MISSING".to_string(),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
        },
        vec![],
    )
    .await
    .expect_err("неіснуючий акт має повертати помилку");
    assert!(
        missing_err.to_string().contains("не знайдено"),
        "повідомлення помилки: {missing_err}"
    );

    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(original.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn seed_counterparty(
    pool: &sqlx::PgPool,
    prefix: &str,
) -> Result<models::Counterparty> {
    let suffix = unique_suffix();
    db::counterparties::create(
        pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("{prefix}-{suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("seed-cp-{prefix}-{suffix}")),
        },
    )
    .await
}

async fn seed_act(
    pool: &sqlx::PgPool,
    cp: &models::Counterparty,
    number: &str,
    amount: Decimal,
) -> Result<models::Act> {
    db::acts::create(
        pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: number.to_string(),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: chrono::Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: None,
            items: vec![models::NewActItem {
                description: "x".into(),
                quantity: dec!(1.0000),
                unit: "шт".into(),
                unit_price: amount,
            }],
        },
    )
    .await
}

async fn seed_act_with_due(
    pool: &sqlx::PgPool,
    cp: &models::Counterparty,
    number: &str,
    amount: Decimal,
    due: Option<chrono::NaiveDate>,
) -> Result<models::Act> {
    db::acts::create(
        pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: number.to_string(),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: chrono::Utc::now().date_naive(),
            expected_payment_date: due,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: None,
            items: vec![models::NewActItem {
                description: "x".into(),
                quantity: dec!(1.0000),
                unit: "шт".into(),
                unit_price: amount,
            }],
        },
    )
    .await
}

// ─── Acts: list_filtered — amount range (Task 2) ─────────────────────────────

#[tokio::test]
async fn list_filtered_amount_range() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

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

    sqlx::query("DELETE FROM acts WHERE counterparty_id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    Ok(())
}

// ─── Acts: list_filtered — multi-status (Task 3) ─────────────────────────────

#[tokio::test]
async fn list_filtered_multi_status() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let cp = seed_counterparty(&pool, "MS-CP").await?;
    seed_act(&pool, &cp, "MS-DRAFT", dec!(100)).await?;
    let issued = seed_act(&pool, &cp, "MS-ISSUED", dec!(100)).await?;
    db::acts::change_status(&pool, issued.id, models::ActStatus::Issued)
        .await?
        .expect("issued");
    let paid_act = seed_act(&pool, &cp, "MS-PAID", dec!(100)).await?;
    db::acts::change_status(&pool, paid_act.id, models::ActStatus::Issued).await?;
    db::acts::change_status(&pool, paid_act.id, models::ActStatus::Signed).await?;
    db::acts::change_status(&pool, paid_act.id, models::ActStatus::Paid).await?;

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

    sqlx::query("DELETE FROM acts WHERE counterparty_id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    Ok(())
}

// ─── Acts: list_filtered — overdue_only (Task 4) ─────────────────────────────

#[tokio::test]
async fn list_filtered_overdue_only() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let today = chrono::NaiveDate::from_ymd_opt(2026, 5, 8).unwrap();
    let past = chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
    let future = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

    let cp = seed_counterparty(&pool, "OVD-CP").await?;

    let paid = seed_act_with_due(&pool, &cp, "OVD-PAID", dec!(100), Some(past)).await?;
    db::acts::change_status(&pool, paid.id, models::ActStatus::Issued).await?;
    db::acts::change_status(&pool, paid.id, models::ActStatus::Signed).await?;
    db::acts::change_status(&pool, paid.id, models::ActStatus::Paid).await?;

    let overdue = seed_act_with_due(&pool, &cp, "OVD-ISSUED", dec!(100), Some(past)).await?;
    db::acts::change_status(&pool, overdue.id, models::ActStatus::Issued).await?;

    let future_due = seed_act_with_due(&pool, &cp, "OVD-FUTURE", dec!(100), Some(future)).await?;
    db::acts::change_status(&pool, future_due.id, models::ActStatus::Issued).await?;

    seed_act_with_due(&pool, &cp, "OVD-DRAFT", dec!(100), Some(past)).await?;

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
    assert_eq!(
        numbers.len(),
        1,
        "Expected only OVD-ISSUED, got: {:?}",
        numbers
    );
    assert_eq!(numbers[0], "OVD-ISSUED");

    sqlx::query("DELETE FROM acts WHERE counterparty_id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    Ok(())
}
