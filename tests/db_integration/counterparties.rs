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

    let before_archived = db::counterparties::count_archived(&pool).await?;
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

    let found_by_bas = db::counterparties::find_by_bas_id(&pool, &bas_id).await?;
    assert!(found_by_bas.is_some());

    let search = db::counterparties::search(&pool, DEFAULT_COMPANY_ID, "ІТ Контрагент").await?;
    assert!(search.iter().any(|cp| cp.id == created.id));

    let archived = db::counterparties::archive(&pool, created.id).await?;
    assert!(archived);

    let after_archived = db::counterparties::count_archived(&pool).await?;
    assert!(after_archived >= before_archived + 1);

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
