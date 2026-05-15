use super::super::*;
use acta::app_ctx::AppCtx;
use acta::tauri_api::documents::document_change_counterparty;

// ─── Допоміжні структури ──────────────────────────────────────────────────────

struct DocFixture {
    cp_original: models::Counterparty,
    cp_new: models::Counterparty,
}

async fn seed_two_counterparties(pool: &PgPool, suffix: &str) -> Result<DocFixture> {
    let cp_original = create_test_counterparty(
        pool,
        suffix,
        &format!("ІТ ChangeCP Original {suffix}"),
        None,
        Some(format!("it-chcp-orig-{suffix}")),
    )
    .await?;
    let cp_new = create_test_counterparty(
        pool,
        suffix,
        &format!("ІТ ChangeCP New {suffix}"),
        None,
        Some(format!("it-chcp-new-{suffix}")),
    )
    .await?;
    Ok(DocFixture {
        cp_original,
        cp_new,
    })
}

// ─── Тест: act: ───────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and seeded DB"]
async fn document_change_counterparty_works_for_act() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let fixture = seed_two_counterparties(&pool, &suffix).await?;
    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);

    // Створити тестовий акт з original-контрагентом
    let act_id = create_test_act(
        &pool,
        DEFAULT_COMPANY_ID,
        fixture.cp_original.id,
        &format!("IT-CHCP-ACT-{suffix}"),
        rust_decimal_macros::dec!(1000.00),
        "draft",
        None,
        chrono::Utc::now().date_naive(),
    )
    .await?;

    let doc_id = format!("act:{act_id}");
    let new_cp_id_str = fixture.cp_new.id.to_string();

    // Виклик
    let result =
        document_change_counterparty(&ctx, doc_id.clone(), new_cp_id_str.clone()).await?;

    // Перевірка відповіді
    assert!(result.ok, "result.ok має бути true");
    assert_eq!(
        result.counterparty_id, new_cp_id_str,
        "counterparty_id у відповіді має збігатись з переданим"
    );
    assert_eq!(
        result.counterparty_name,
        format!("ІТ ChangeCP New {suffix}"),
        "counterparty_name у відповіді має збігатись з іменем нового контрагента"
    );

    // Перевірка в БД: counterparty_id справді оновився
    let stored_cp_id: uuid::Uuid =
        sqlx::query_scalar("SELECT counterparty_id FROM acts WHERE id = $1")
            .bind(act_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(
        stored_cp_id, fixture.cp_new.id,
        "В БД counterparty_id акту має бути оновлений"
    );

    // Прибирання
    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = ANY($1)")
        .bind(vec![fixture.cp_original.id, fixture.cp_new.id])
        .execute(&pool)
        .await?;

    Ok(())
}

// ─── Тест: inv: ───────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and seeded DB"]
async fn document_change_counterparty_works_for_invoice() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let fixture = seed_two_counterparties(&pool, &suffix).await?;
    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);

    // Створити тестовий рахунок з original-контрагентом
    let invoice_id = create_test_invoice(
        &pool,
        DEFAULT_COMPANY_ID,
        fixture.cp_original.id,
        &format!("IT-CHCP-INV-{suffix}"),
        rust_decimal_macros::dec!(2000.00),
        "draft",
        None,
        chrono::Utc::now().date_naive(),
    )
    .await?;

    let doc_id = format!("inv:{invoice_id}");
    let new_cp_id_str = fixture.cp_new.id.to_string();

    // Виклик
    let result =
        document_change_counterparty(&ctx, doc_id.clone(), new_cp_id_str.clone()).await?;

    // Перевірка відповіді
    assert!(result.ok, "result.ok має бути true");
    assert_eq!(
        result.counterparty_id, new_cp_id_str,
        "counterparty_id у відповіді має збігатись з переданим"
    );
    assert_eq!(
        result.counterparty_name,
        format!("ІТ ChangeCP New {suffix}"),
        "counterparty_name у відповіді має збігатись з іменем нового контрагента"
    );

    // Перевірка в БД
    let stored_cp_id: uuid::Uuid =
        sqlx::query_scalar("SELECT counterparty_id FROM invoices WHERE id = $1")
            .bind(invoice_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(
        stored_cp_id, fixture.cp_new.id,
        "В БД counterparty_id рахунку має бути оновлений"
    );

    // Прибирання
    sqlx::query("DELETE FROM invoice_items WHERE invoice_id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = ANY($1)")
        .bind(vec![fixture.cp_original.id, fixture.cp_new.id])
        .execute(&pool)
        .await?;

    Ok(())
}

// ─── Тест: wbl: ───────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and seeded DB"]
async fn document_change_counterparty_works_for_waybill() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let fixture = seed_two_counterparties(&pool, &suffix).await?;
    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);

    // Створити тестову накладну з original-контрагентом
    let waybill = db::waybills::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewWaybill {
            number: format!("IT-CHCP-WBL-{suffix}"),
            counterparty_id: fixture.cp_original.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: chrono::Utc::now().date_naive(),
            notes: None,
            bas_id: None,
            items: vec![],
        },
    )
    .await?;

    let doc_id = format!("wbl:{}", waybill.id);
    let new_cp_id_str = fixture.cp_new.id.to_string();

    // Виклик
    let result =
        document_change_counterparty(&ctx, doc_id.clone(), new_cp_id_str.clone()).await?;

    // Перевірка відповіді
    assert!(result.ok, "result.ok має бути true");
    assert_eq!(
        result.counterparty_id, new_cp_id_str,
        "counterparty_id у відповіді має збігатись з переданим"
    );
    assert_eq!(
        result.counterparty_name,
        format!("ІТ ChangeCP New {suffix}"),
        "counterparty_name у відповіді має збігатись з іменем нового контрагента"
    );

    // Перевірка в БД
    let stored_cp_id: uuid::Uuid =
        sqlx::query_scalar("SELECT counterparty_id FROM waybills WHERE id = $1")
            .bind(waybill.id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(
        stored_cp_id, fixture.cp_new.id,
        "В БД counterparty_id накладної має бути оновлений"
    );

    // Прибирання
    sqlx::query("DELETE FROM waybill_items WHERE waybill_id = $1")
        .bind(waybill.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM waybills WHERE id = $1")
        .bind(waybill.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = ANY($1)")
        .bind(vec![fixture.cp_original.id, fixture.cp_new.id])
        .execute(&pool)
        .await?;

    Ok(())
}

// ─── Тест: некоректний UUID контрагента повертає Err ─────────────────────────

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and seeded DB"]
async fn document_change_counterparty_rejects_invalid_counterparty_uuid() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let cp = create_test_counterparty(
        &pool,
        &suffix,
        &format!("ІТ ChangeCP Invalid {suffix}"),
        None,
        None,
    )
    .await?;
    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);

    let act_id = create_test_act(
        &pool,
        DEFAULT_COMPANY_ID,
        cp.id,
        &format!("IT-CHCP-ERR-{suffix}"),
        rust_decimal_macros::dec!(500.00),
        "draft",
        None,
        chrono::Utc::now().date_naive(),
    )
    .await?;

    let result = document_change_counterparty(
        &ctx,
        format!("act:{act_id}"),
        "not-a-uuid".to_string(),
    )
    .await;

    assert!(
        result.is_err(),
        "Некоректний UUID контрагента має повертати Err"
    );

    // Прибирання
    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}
