use super::super::*;

#[tokio::test]
async fn companies_create_update_archive_and_summary_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let new_company = models::NewCompany {
        name: format!("ІТ Компанія {suffix}"),
        short_name: Some(format!("IT-{suffix}")),
        edrpou: Some(suffix[..8].to_string()),
        ipn: Some(format!("3{}", &suffix[..9])),
        iban: Some("UA999999999999999999999999999".to_string()),
        legal_address: Some("м. Київ, вул. Інтеграційна, 1".to_string()),
        director_name: Some("Тестовий Директор".to_string()),
        tax_system: Some("simplified".to_string()),
        is_vat_payer: false,
    };

    let created = db::companies::create(&pool, &new_company).await?;
    assert_eq!(created.name, new_company.name);
    assert_eq!(created.ipn, new_company.ipn);
    assert!(!created.is_archived);

    let fetched = db::companies::get_by_id(&pool, created.id)
        .await?
        .expect("company exists");
    assert_eq!(fetched.name, new_company.name);

    let active_companies = db::companies::list(&pool).await?;
    assert!(active_companies.iter().any(|c| c.id == created.id));

    let summaries = db::companies::list_with_summary(&pool).await?;
    let summary = summaries
        .iter()
        .find(|c| c.id == created.id)
        .expect("company summary exists");
    assert_eq!(summary.act_count, 0);
    assert_eq!(summary.total_amount, dec!(0.00));

    let updated = db::companies::update(
        &pool,
        created.id,
        &models::UpdateCompany {
            name: format!("Оновлена ІТ Компанія {suffix}"),
            short_name: Some("ОІТ".to_string()),
            edrpou: Some(suffix[..8].to_string()),
            iban: Some("UA111111111111111111111111111".to_string()),
            legal_address: Some("м. Львів, вул. Оновлена, 2".to_string()),
            director_name: Some("Новий Директор".to_string()),
            accountant_name: Some("Новий Бухгалтер".to_string()),
            tax_system: Some("general".to_string()),
            is_vat_payer: true,
            logo_path: Some("storage/logo/test.png".to_string()),
        },
    )
    .await?
    .expect("company updated");

    assert_eq!(updated.name, format!("Оновлена ІТ Компанія {suffix}"));
    assert_eq!(updated.short_name.as_deref(), Some("ОІТ"));
    assert_eq!(updated.director_name.as_deref(), Some("Новий Директор"));
    assert!(updated.is_vat_payer);

    let archived = db::companies::archive(&pool, created.id).await?;
    assert!(archived);

    let active_after_archive = db::companies::list(&pool).await?;
    assert!(!active_after_archive.iter().any(|c| c.id == created.id));

    let all_companies = db::companies::list_all(&pool).await?;
    let archived_company = all_companies
        .iter()
        .find(|c| c.id == created.id)
        .expect("archived company visible in list_all");
    assert!(archived_company.is_archived);

    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(created.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn companies_summary_counts_real_acts_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let company = db::companies::create(
        &pool,
        &models::NewCompany {
            name: format!("ІТ Summary Компанія {suffix}"),
            short_name: Some(format!("SUM-{suffix}")),
            edrpou: Some(suffix[..8].to_string()),
            ipn: Some(format!("4{}", &suffix[..9])),
            iban: Some("UA888888888888888888888888888".to_string()),
            legal_address: Some("м. Київ, вул. Агрегаційна, 2".to_string()),
            director_name: Some("Тестовий Керівник".to_string()),
            tax_system: Some("general".to_string()),
            is_vat_payer: true,
        },
    )
    .await?;

    let cp = db::counterparties::create(
        &pool,
        company.id,
        &models::NewCounterparty {
            name: format!("ІТ Summary Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-summary-cp-{suffix}")),
        },
    )
    .await?;

    let act_one = db::acts::create(
        &pool,
        company.id,
        &models::NewAct {
            number: format!("SUM-ACT-1-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: Some(format!("it-summary-act-1-{suffix}")),
            items: vec![models::NewActItem {
                description: "Послуга 1".to_string(),
                quantity: dec!(2.0000),
                unit: "год".to_string(),
                unit_price: dec!(1000.00),
            }],
        },
    )
    .await?;

    let act_two = db::acts::create(
        &pool,
        company.id,
        &models::NewAct {
            number: format!("SUM-ACT-2-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Issued,
            notes: None,
            bas_id: Some(format!("it-summary-act-2-{suffix}")),
            items: vec![models::NewActItem {
                description: "Послуга 2".to_string(),
                quantity: dec!(3.0000),
                unit: "год".to_string(),
                unit_price: dec!(500.00),
            }],
        },
    )
    .await?;

    let summaries = db::companies::list_with_summary(&pool).await?;
    let summary = summaries
        .iter()
        .find(|c| c.id == company.id)
        .expect("company summary exists");

    assert_eq!(summary.act_count, 2);
    assert_eq!(summary.total_amount, dec!(3500.00));

    sqlx::query("DELETE FROM acts WHERE id = $1 OR id = $2")
        .bind(act_one.id)
        .bind(act_two.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(company.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn companies_get_by_id_missing_returns_none() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let missing = db::companies::get_by_id(&pool, Uuid::new_v4()).await?;
    assert!(missing.is_none());

    Ok(())
}

#[tokio::test]
async fn companies_archive_missing_returns_false() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let archived = db::companies::archive(&pool, Uuid::new_v4()).await?;
    assert!(!archived);

    Ok(())
}

#[tokio::test]
async fn companies_update_missing_returns_none() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let updated = db::companies::update(
        &pool,
        Uuid::new_v4(),
        &models::UpdateCompany {
            name: "Неіснуюча компанія".to_string(),
            short_name: Some("НК".to_string()),
            edrpou: Some("12345678".to_string()),
            iban: Some("UA123456789012345678901234567".to_string()),
            legal_address: Some("м. Київ".to_string()),
            director_name: Some("Тестовий Директор".to_string()),
            accountant_name: Some("Тестовий Бухгалтер".to_string()),
            tax_system: Some("general".to_string()),
            is_vat_payer: false,
            logo_path: None,
        },
    )
    .await?;

    assert!(updated.is_none());

    Ok(())
}

#[tokio::test]
async fn companies_list_is_sorted_by_name() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let names = ["Яблуко", "Абрикос", "Малина"];
    let mut created_ids = Vec::new();

    for (index, name) in names.iter().enumerate() {
        let company = db::companies::create(
            &pool,
            &models::NewCompany {
                name: format!("{name} {suffix}"),
                short_name: Some(format!("SORT-{index}-{suffix}")),
                edrpou: Some(format!("{:08}", 10_000_000 + index)),
                ipn: Some(format!("5{:09}", index)),
                iban: Some(format!("UA{:027}", index + 1)),
                legal_address: None,
                director_name: None,
                tax_system: Some("general".to_string()),
                is_vat_payer: false,
            },
        )
        .await?;
        created_ids.push(company.id);
    }

    let listed_names: Vec<String> = db::companies::list(&pool)
        .await?
        .into_iter()
        .filter(|c| created_ids.contains(&c.id))
        .map(|c| c.name)
        .collect();

    for id in &created_ids {
        sqlx::query("DELETE FROM companies WHERE id = $1")
            .bind(*id)
            .execute(&pool)
            .await?;
    }

    assert_eq!(
        listed_names,
        vec![
            format!("Абрикос {suffix}"),
            format!("Малина {suffix}"),
            format!("Яблуко {suffix}")
        ]
    );

    Ok(())
}
