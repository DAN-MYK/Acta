use super::*;
use crate::dashboard::{dashboard_test_cleanup, dashboard_test_setup};

#[tokio::test]
async fn contracts_crud_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    let contract = db::contracts::create(
        &pool,
        models::NewContract {
            company_id,
            counterparty_id: cp_id,
            number: format!("ДГ-{suffix}"),
            subject: Some("Розробка ПЗ".to_string()),
            date: Utc::now().date_naive(),
            expires_at: Some((Utc::now() + Duration::days(365)).date_naive()),
            amount: Some(dec!(50000.00)),
        },
    )
    .await?;

    // get_by_id: всі поля
    let fetched = db::contracts::get_by_id(&pool, contract.id).await?;
    assert_eq!(fetched.number, format!("ДГ-{suffix}"));
    assert_eq!(fetched.subject.as_deref(), Some("Розробка ПЗ"));
    assert_eq!(fetched.amount, Some(dec!(50000.00)));
    assert_eq!(
        fetched.status,
        models::ContractStatus::Active,
        "default status = active"
    );
    assert_eq!(fetched.company_id, company_id);
    assert_eq!(fetched.counterparty_id, cp_id);

    // list: договір присутній, дата у форматі "ДД.ММ.РРРР"
    let listed = db::contracts::list(&pool, company_id).await?;
    let row = listed
        .iter()
        .find(|r| r.id == contract.id)
        .expect("договір у списку");
    assert_eq!(row.date.len(), 10);
    assert_eq!(row.date.chars().nth(2), Some('.'));
    assert_eq!(
        row.counterparty_name,
        format!("ІТ Dashboard Контрагент {suffix}")
    );

    // update: змінюємо номер, статус, примітки
    let updated = db::contracts::update(
        &pool,
        contract.id,
        models::UpdateContract {
            number: format!("ДГ-UPD-{suffix}"),
            subject: Some("Розробка ПЗ (оновлено)".to_string()),
            date: contract.date,
            expires_at: contract.expires_at,
            amount: Some(dec!(55000.00)),
            status: models::ContractStatus::Expired,
            notes: Some("термін закінчився".to_string()),
        },
    )
    .await?;

    assert_eq!(updated.number, format!("ДГ-UPD-{suffix}"));
    assert_eq!(updated.status, models::ContractStatus::Expired);
    assert_eq!(updated.notes.as_deref(), Some("термін закінчився"));
    assert_eq!(updated.amount, Some(dec!(55000.00)));

    // delete: після видалення відсутній у списку
    db::contracts::delete(&pool, contract.id).await?;
    let after_delete = db::contracts::list(&pool, company_id).await?;
    assert!(!after_delete.iter().any(|r| r.id == contract.id));

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn contracts_list_by_counterparty_isolates_by_cp() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp1_id) = dashboard_test_setup(&pool, &suffix).await?;

    let cp2 = db::counterparties::create(
        &pool,
        company_id,
        &models::NewCounterparty {
            name: format!("ІТ Contracts CP2 {suffix}"),
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

    let new_contract = |cp: Uuid, tag: &str| models::NewContract {
        company_id,
        counterparty_id: cp,
        number: format!("ДГ-{tag}-{suffix}"),
        subject: None,
        date: Utc::now().date_naive(),
        expires_at: None,
        amount: None,
    };

    let c1 = db::contracts::create(&pool, new_contract(cp1_id, "A")).await?;
    let c2 = db::contracts::create(&pool, new_contract(cp1_id, "B")).await?;
    let c3 = db::contracts::create(&pool, new_contract(cp2.id, "C")).await?;

    let for_cp1 = db::contracts::list_by_counterparty(&pool, company_id, cp1_id).await?;
    let for_cp2 = db::contracts::list_by_counterparty(&pool, company_id, cp2.id).await?;

    assert_eq!(for_cp1.len(), 2);
    assert!(for_cp1.iter().all(|r| r.counterparty_id == cp1_id));
    assert_eq!(for_cp2.len(), 1);
    assert_eq!(for_cp2[0].id, c3.id);

    let all = db::contracts::list(&pool, company_id).await?;
    assert!(all.iter().any(|r| r.id == c1.id));
    assert!(all.iter().any(|r| r.id == c2.id));
    assert!(all.iter().any(|r| r.id == c3.id));

    for id in [c1.id, c2.id, c3.id] {
        db::contracts::delete(&pool, id).await?;
    }
    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn contracts_list_for_select_returns_only_active() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    let make = |tag: &str| models::NewContract {
        company_id,
        counterparty_id: cp_id,
        number: format!("ДГ-SEL-{tag}-{suffix}"),
        subject: None,
        date: Utc::now().date_naive(),
        expires_at: None,
        amount: None,
    };

    let active = db::contracts::create(&pool, make("ACT")).await?;
    let to_expire = db::contracts::create(&pool, make("EXP")).await?;
    let to_term = db::contracts::create(&pool, make("TRM")).await?;

    db::contracts::update(
        &pool,
        to_expire.id,
        models::UpdateContract {
            number: to_expire.number.clone(),
            subject: None,
            date: to_expire.date,
            expires_at: None,
            amount: None,
            status: models::ContractStatus::Expired,
            notes: None,
        },
    )
    .await?;

    db::contracts::update(
        &pool,
        to_term.id,
        models::UpdateContract {
            number: to_term.number.clone(),
            subject: None,
            date: to_term.date,
            expires_at: None,
            amount: None,
            status: models::ContractStatus::Terminated,
            notes: None,
        },
    )
    .await?;

    let selectable = db::contracts::list_for_select(&pool, company_id, cp_id).await?;
    assert_eq!(selectable.len(), 1);
    assert_eq!(selectable[0].id, active.id);

    for id in [active.id, to_expire.id, to_term.id] {
        db::contracts::delete(&pool, id).await?;
    }
    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Categories ───────────────────────────────────────────────────────────────

async fn category_company_cleanup(pool: &PgPool, company_id: Uuid) -> Result<()> {
    // categories мають ON DELETE CASCADE від company
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn make_category_company(pool: &PgPool, suffix: &str, tag: &str) -> Result<Uuid> {
    let company = db::companies::create(
        pool,
        &models::NewCompany {
            name: format!("ІТ Cat{tag} Компанія {suffix}"),
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
    Ok(company.id)
}

#[tokio::test]
async fn categories_crud_and_archive_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let company_id = make_category_company(&pool, &suffix, "CRUD").await?;

    let cat = db::categories::create(
        &pool,
        models::NewCategory {
            name: "Консалтинг".to_string(),
            kind: models::CategoryKind::Income,
            parent_id: None,
            company_id,
        },
    )
    .await?;

    assert_eq!(cat.name, "Консалтинг");
    assert_eq!(cat.kind, models::CategoryKind::Income);
    assert!(!cat.is_archived);

    // list: присутня
    let all = db::categories::list(&pool, company_id).await?;
    assert!(all.iter().any(|c| c.id == cat.id));

    // update: перейменування
    let renamed = db::categories::update(
        &pool,
        cat.id,
        models::UpdateCategory {
            name: "ІТ Консалтинг".to_string(),
            parent_id: None,
        },
    )
    .await?;
    assert_eq!(renamed.name, "ІТ Консалтинг");

    // archive
    db::categories::archive(&pool, cat.id).await?;

    // list включає, але is_archived = true
    let after = db::categories::list(&pool, company_id).await?;
    let row = after
        .iter()
        .find(|c| c.id == cat.id)
        .expect("архівована категорія у list");
    assert!(row.is_archived);

    // list_for_select / list_all_for_select виключають архівовані
    let sel =
        db::categories::list_for_select(&pool, company_id, models::CategoryKind::Income).await?;
    assert!(!sel.iter().any(|c| c.id == cat.id));

    let all_sel = db::categories::list_all_for_select(&pool, company_id).await?;
    assert!(!all_sel.iter().any(|c| c.id == cat.id));

    category_company_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn categories_hierarchy_and_select_depth() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let company_id = make_category_company(&pool, &suffix, "HIER").await?;

    let parent = db::categories::create(
        &pool,
        models::NewCategory {
            name: "Розробка".to_string(),
            kind: models::CategoryKind::Income,
            parent_id: None,
            company_id,
        },
    )
    .await?;

    let child = db::categories::create(
        &pool,
        models::NewCategory {
            name: "Мобільна розробка".to_string(),
            kind: models::CategoryKind::Income,
            parent_id: Some(parent.id),
            company_id,
        },
    )
    .await?;

    let expense = db::categories::create(
        &pool,
        models::NewCategory {
            name: "Оренда".to_string(),
            kind: models::CategoryKind::Expense,
            parent_id: None,
            company_id,
        },
    )
    .await?;

    // list_for_select(Income): тільки income, depth коректний
    let income =
        db::categories::list_for_select(&pool, company_id, models::CategoryKind::Income).await?;
    assert_eq!(income.len(), 2);

    let p = income.iter().find(|c| c.id == parent.id).expect("батько");
    let ch = income.iter().find(|c| c.id == child.id).expect("дочірня");
    assert_eq!(p.depth, 0, "батько depth=0");
    assert_eq!(ch.depth, 1, "дочірня depth=1");

    // expense не потрапляє в income
    assert!(!income.iter().any(|c| c.id == expense.id));

    // list_all_for_select: всі три
    let all = db::categories::list_all_for_select(&pool, company_id).await?;
    assert_eq!(all.len(), 3);

    // parent_id NULLS FIRST → батько перед дочірньою
    let income_order: Vec<Uuid> = income.iter().map(|c| c.id).collect();
    assert_eq!(income_order[0], parent.id);

    category_company_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn categories_seed_defaults_creates_standard_entries() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let company_id = make_category_company(&pool, &suffix, "SEED").await?;

    db::categories::seed_defaults(&pool, company_id).await?;

    let income =
        db::categories::list_for_select(&pool, company_id, models::CategoryKind::Income).await?;
    let expense =
        db::categories::list_for_select(&pool, company_id, models::CategoryKind::Expense).await?;
    let all = db::categories::list_all_for_select(&pool, company_id).await?;

    assert_eq!(
        income.len(),
        4,
        "4 income: Розробка ПЗ, Консалтинг, Тех. підтримка, Навчання"
    );
    assert_eq!(
        expense.len(),
        5,
        "5 expense: Зарплата, Оренда, Маркетинг, Податки, Комунальні"
    );
    assert_eq!(all.len(), 9);

    // Ідемпотентність: ON CONFLICT DO NOTHING
    db::categories::seed_defaults(&pool, company_id).await?;
    assert_eq!(
        db::categories::list_all_for_select(&pool, company_id)
            .await?
            .len(),
        9
    );

    // Конкретні назви income
    let names: Vec<&str> = income.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Розробка ПЗ"));
    assert!(names.contains(&"Консалтинг"));

    category_company_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Acts: generate_next_number ──────────────────────────────────────────────

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
