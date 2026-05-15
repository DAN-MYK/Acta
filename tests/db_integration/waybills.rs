use super::*;

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn seed_waybill_counterparty(
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
            bas_id: Some(format!("seed-wbl-cp-{prefix}-{suffix}")),
        },
    )
    .await
}

async fn seed_waybill(
    pool: &sqlx::PgPool,
    cp: &models::Counterparty,
    number: &str,
    amount: rust_decimal::Decimal,
) -> Result<models::Waybill> {
    db::waybills::create(
        pool,
        DEFAULT_COMPANY_ID,
        &models::NewWaybill {
            number: number.to_string(),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: chrono::Utc::now().date_naive(),
            notes: None,
            bas_id: None,
            items: vec![models::NewWaybillItem {
                position: 1,
                description: "x".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: amount,
            }],
        },
    )
    .await
}

// ─── Waybills: list_filtered — amount range ───────────────────────────────────

#[tokio::test]
async fn list_filtered_amount_range() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let cp = seed_waybill_counterparty(&pool, "WBL-AMT-CP").await?;
    seed_waybill(&pool, &cp, "WBL-AMT-500", dec!(500.00)).await?;
    seed_waybill(&pool, &cp, "WBL-AMT-5000", dec!(5000.00)).await?;
    seed_waybill(&pool, &cp, "WBL-AMT-50000", dec!(50000.00)).await?;

    let mid = db::waybills::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        None,
        None,
        Some("WBL-AMT-"),
        Some(cp.id),
        None,
        None,
        Some(dec!(1000)),
        Some(dec!(10000)),
    )
    .await?;

    assert_eq!(
        mid.len(),
        1,
        "Expected only WBL-AMT-5000, got: {:?}",
        mid.iter().map(|r| &r.number).collect::<Vec<_>>()
    );
    assert_eq!(mid[0].number, "WBL-AMT-5000");

    sqlx::query("DELETE FROM waybills WHERE counterparty_id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    Ok(())
}

// ─── Waybills: list_filtered — multi-status ───────────────────────────────────

#[tokio::test]
async fn list_filtered_multi_status() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let cp = seed_waybill_counterparty(&pool, "WBL-MS-CP").await?;

    // draft — залишається у статусі Draft
    seed_waybill(&pool, &cp, "WBL-MS-DRAFT", dec!(100)).await?;

    // delivered — переходить Draft → Issued → Signed → Delivered
    let wbl_issued = seed_waybill(&pool, &cp, "WBL-MS-DELIVERED", dec!(100)).await?;
    db::waybills::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        wbl_issued.id,
        models::WaybillStatus::Issued,
    )
    .await?;
    db::waybills::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        wbl_issued.id,
        models::WaybillStatus::Signed,
    )
    .await?;
    db::waybills::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        wbl_issued.id,
        models::WaybillStatus::Delivered,
    )
    .await?;

    // signed — не має бути у результаті (не входить у фільтр)
    let wbl_signed = seed_waybill(&pool, &cp, "WBL-MS-SIGNED", dec!(100)).await?;
    db::waybills::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        wbl_signed.id,
        models::WaybillStatus::Issued,
    )
    .await?;
    db::waybills::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        wbl_signed.id,
        models::WaybillStatus::Signed,
    )
    .await?;

    let filtered = db::waybills::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(&["draft".to_string(), "delivered".to_string()]),
        None,
        Some("WBL-MS-"),
        Some(cp.id),
        None,
        None,
        None,
        None,
    )
    .await?;

    let numbers: Vec<&str> = filtered.iter().map(|r| r.number.as_str()).collect();
    assert_eq!(
        numbers.len(),
        2,
        "Expected WBL-MS-DRAFT and WBL-MS-DELIVERED, got: {:?}",
        numbers
    );
    assert!(numbers.contains(&"WBL-MS-DRAFT"));
    assert!(numbers.contains(&"WBL-MS-DELIVERED"));
    assert!(!numbers.contains(&"WBL-MS-SIGNED"));

    sqlx::query("DELETE FROM waybills WHERE counterparty_id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    Ok(())
}
