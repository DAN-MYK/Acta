use super::*;

#[tokio::test]
async fn relative_pdf_paths_migration_converts_absolute_invoice_and_waybill_paths() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = create_test_counterparty(
        &pool,
        &suffix,
        &format!("РІдносні PDF шляхи {suffix}"),
        Some(suffix[..8].to_string()),
        Some(format!("pdf-paths-cp-{suffix}")),
    )
    .await?;

    let invoice_number = format!("REL-PDF-INV-{suffix}");
    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: invoice_number.clone(),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("pdf-paths-invoice-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "PDF invoice item".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(100.00),
            }],
        },
    )
    .await?;

    let waybill_number = format!("REL-PDF-WBL-{suffix}");
    let waybill = db::waybills::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewWaybill {
            number: waybill_number.clone(),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            notes: None,
            bas_id: Some(format!("pdf-paths-waybill-{suffix}")),
            items: vec![models::NewWaybillItem {
                position: 1,
                description: "PDF waybill item".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(120.00),
            }],
        },
    )
    .await?;

    let invoice_relative = format!(
        "existing_pdf/invoice/{}_REL-PDF-INV-{suffix}/working.pdf",
        invoice.id
    );
    let waybill_relative = format!(
        "existing_pdf/waybill/{}_REL-PDF-WBL-{suffix}/working.pdf",
        waybill.id
    );
    let storage_root = std::env::temp_dir().join(format!("acta_migration_storage_{suffix}"));
    let invoice_absolute = storage_root.join(&invoice_relative).display().to_string();
    let waybill_absolute = storage_root
        .join(&waybill_relative)
        .display()
        .to_string()
        .replace('\\', "/");

    sqlx::query("UPDATE invoices SET pdf_path = $2 WHERE id = $1")
        .bind(invoice.id)
        .bind(invoice_absolute)
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE waybills SET pdf_path = $2 WHERE id = $1")
        .bind(waybill.id)
        .bind(waybill_absolute)
        .execute(&pool)
        .await?;

    let migration_sql = std::fs::read_to_string("migrations/028_relative_pdf_paths.sql")?;
    for statement in migration_sql.split(';') {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        sqlx::query(statement).execute(&pool).await?;
    }

    let stored_invoice_path =
        sqlx::query_scalar::<_, Option<String>>("SELECT pdf_path FROM invoices WHERE id = $1")
            .bind(invoice.id)
            .fetch_one(&pool)
            .await?;
    let stored_waybill_path =
        sqlx::query_scalar::<_, Option<String>>("SELECT pdf_path FROM waybills WHERE id = $1")
            .bind(waybill.id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(
        stored_invoice_path.as_deref(),
        Some(invoice_relative.as_str())
    );
    assert_eq!(
        stored_waybill_path.as_deref(),
        Some(waybill_relative.as_str())
    );

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM waybills WHERE id = $1")
        .bind(waybill.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}
