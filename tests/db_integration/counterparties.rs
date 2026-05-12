use super::super::*;

#[tokio::test]
async fn counterparties_crud_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let bas_id = format!("it-cp-{suffix}");
    let edrpou = suffix[..8].to_string();

    let new_cp = models::NewCounterparty {
        name: format!("ІТ Контрагент {suffix}"),
        edrpou: Some(edrpou),
        ipn: None,
        iban: Some("UA123456789012345678901234567".to_string()),
        address: Some("Київ".to_string()),
        phone: Some("+380500000000".to_string()),
        email: Some("it@example.com".to_string()),
        notes: Some("integration".to_string()),
        bas_id: Some(bas_id.clone()),
    };

    let created = db::counterparties::create(&pool, DEFAULT_COMPANY_ID, &new_cp).await?;

    let fetched = db::counterparties::get_by_id(&pool, DEFAULT_COMPANY_ID, created.id)
        .await?
        .expect("counterparty exists");
    assert_eq!(fetched.name, new_cp.name);

    let foreign_company_id: Uuid =
        sqlx::query_scalar("INSERT INTO companies (name) VALUES ($1) RETURNING id")
            .bind(format!("ІТ Foreign Company {suffix}"))
            .fetch_one(&pool)
            .await?;
    let foreign_fetch =
        db::counterparties::get_by_id(&pool, foreign_company_id, created.id).await?;
    assert!(
        foreign_fetch.is_none(),
        "контрагент не повинен читатися поза межами своєї компанії"
    );

    let found_by_bas =
        db::counterparties::find_by_bas_id_scoped(&pool, DEFAULT_COMPANY_ID, &bas_id).await?;
    assert!(found_by_bas.is_some());

    let search = db::counterparties::search(&pool, DEFAULT_COMPANY_ID, "ІТ Контрагент").await?;
    assert!(search.iter().any(|cp| cp.id == created.id));

    let archived =
        db::counterparties::archive_scoped(&pool, DEFAULT_COMPANY_ID, created.id).await?;
    assert!(archived);

    let archived_fetched = db::counterparties::get_by_id(&pool, DEFAULT_COMPANY_ID, created.id)
        .await?
        .expect("archived counterparty still exists");
    assert!(archived_fetched.is_archived);

    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(created.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(foreign_company_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn counterparties_scoped_mutations_reject_foreign_company() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let foreign_company_id: Uuid =
        sqlx::query_scalar("INSERT INTO companies (name) VALUES ($1) RETURNING id")
            .bind(format!("IT Foreign Counterparty Company {suffix}"))
            .fetch_one(&pool)
            .await?;

    let created = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("IT Scoped Counterparty {suffix}"),
            edrpou: None,
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-scoped-cp-{suffix}")),
        },
    )
    .await?;

    assert!(db::counterparties::update_scoped(
        &pool,
        foreign_company_id,
        created.id,
        &models::UpdateCounterparty {
            name: "Foreign name".to_string(),
            edrpou: None,
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
        },
    )
    .await?
    .is_none());
    assert!(!db::counterparties::archive_scoped(&pool, foreign_company_id, created.id,).await?);

    let own = db::counterparties::get_by_id(&pool, DEFAULT_COMPANY_ID, created.id)
        .await?
        .expect("own company still sees counterparty");
    assert_eq!(own.name, created.name);
    assert!(!own.is_archived);

    assert!(db::counterparties::archive_scoped(&pool, DEFAULT_COMPANY_ID, created.id,).await?);
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(created.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(foreign_company_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn counterparties_allow_same_bas_id_in_different_companies() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let shared_bas_id = format!("it-shared-bas-{suffix}");
    let foreign_company_id: Uuid =
        sqlx::query_scalar("INSERT INTO companies (name) VALUES ($1) RETURNING id")
            .bind(format!("IT BAS Company {suffix}"))
            .fetch_one(&pool)
            .await?;

    let first = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("IT BAS Counterparty A {suffix}"),
            edrpou: None,
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(shared_bas_id.clone()),
        },
    )
    .await?;
    let second = db::counterparties::create(
        &pool,
        foreign_company_id,
        &models::NewCounterparty {
            name: format!("IT BAS Counterparty B {suffix}"),
            edrpou: None,
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(shared_bas_id.clone()),
        },
    )
    .await?;

    let found_first =
        db::counterparties::find_by_bas_id_scoped(&pool, DEFAULT_COMPANY_ID, &shared_bas_id)
            .await?
            .expect("default company match");
    let found_second =
        db::counterparties::find_by_bas_id_scoped(&pool, foreign_company_id, &shared_bas_id)
            .await?
            .expect("foreign company match");
    assert_eq!(found_first.id, first.id);
    assert_eq!(found_second.id, second.id);

    sqlx::query("DELETE FROM counterparties WHERE id = ANY($1)")
        .bind(vec![first.id, second.id])
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(foreign_company_id)
        .execute(&pool)
        .await?;

    Ok(())
}
