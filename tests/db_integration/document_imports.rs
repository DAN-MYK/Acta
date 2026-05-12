use super::super::*;

#[tokio::test]
async fn bas_invoice_preview_marks_conflict_for_duplicate_fingerprint() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp_name = format!("ІТ Invoice Conflict {suffix}");
    let cp = create_test_counterparty(
        &pool,
        &suffix,
        &cp_name,
        Some(suffix[..8].to_string()),
        None,
    )
    .await?;

    let number = format!("INV-CONFLICT-{suffix}");
    let date = Utc::now().date_naive();
    let create_invoice = |note: &str| models::NewInvoice {
        number: number.clone(),
        counterparty_id: cp.id,
        contract_id: None,
        category_id: None,
        direction: models::DocumentDirection::Outgoing,
        date,
        expected_payment_date: None,
        notes: Some(note.to_string()),
        bas_id: None,
        items: vec![models::NewInvoiceItem {
            position: 1,
            description: "Товар".to_string(),
            unit: Some("шт".to_string()),
            quantity: dec!(1.0000),
            price: dec!(1000.00),
        }],
    };
    let inv1 = db::invoices::create(&pool, DEFAULT_COMPANY_ID, &create_invoice("a")).await?;
    let inv2 = db::invoices::create(&pool, DEFAULT_COMPANY_ID, &create_invoice("b")).await?;

    let imported = ImportedInvoice {
        bas_id: None,
        counterparty_bas_id: None,
        counterparty_name: Some(cp_name),
        counterparty_edrpou: cp.edrpou.clone(),
        contract_bas_id: None,
        contract_number: None,
        number: number.clone(),
        date,
        expected_payment_date: None,
        direction: models::DocumentDirection::Outgoing,
        status: models::invoice::InvoiceStatus::Issued,
        total_amount: dec!(1000.00),
        vat_amount: Decimal::ZERO,
        notes: None,
        items: vec![ImportedInvoiceItem {
            description: "Товар".to_string(),
            unit: Some("шт".to_string()),
            quantity: dec!(1.0000),
            price: dec!(1000.00),
        }],
    };

    let report = apply_imported_invoices(&pool, DEFAULT_COMPANY_ID, &[imported], true).await?;
    assert_eq!(report.conflicts, 1);
    assert!(report.rows[0]
        .note
        .as_deref()
        .unwrap_or_default()
        .contains("conflict"));

    sqlx::query("DELETE FROM invoices WHERE id = $1 OR id = $2")
        .bind(inv1.id)
        .bind(inv2.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn bas_invoice_import_resolves_dependencies_and_creates_items() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp_name = format!("ІТ Invoice Контрагент {suffix}");
    let cp = create_test_counterparty(
        &pool,
        &suffix,
        &cp_name,
        Some(suffix[..8].to_string()),
        None,
    )
    .await?;
    let contract_number = format!("INV-CONTRACT-{suffix}");
    let contract = create_test_contract(&pool, &suffix, cp.id, &contract_number, None).await?;

    let imported = ImportedInvoice {
        bas_id: Some(format!("import-invoice-{suffix}")),
        counterparty_bas_id: None,
        counterparty_name: Some(cp_name.clone()),
        counterparty_edrpou: cp.edrpou.clone(),
        contract_bas_id: None,
        contract_number: Some(contract_number.clone()),
        number: format!("INV-{suffix}"),
        date: Utc::now().date_naive(),
        expected_payment_date: Some(Utc::now().date_naive() + Duration::days(10)),
        direction: models::DocumentDirection::Outgoing,
        status: models::invoice::InvoiceStatus::Issued,
        total_amount: dec!(3000.00),
        vat_amount: dec!(500.00),
        notes: Some("integration import".to_string()),
        items: vec![
            ImportedInvoiceItem {
                description: "Товар 1".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(2.0000),
                price: dec!(1000.00),
            },
            ImportedInvoiceItem {
                description: "Товар 2".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(1000.00),
            },
        ],
    };

    let report = apply_imported_invoices(&pool, DEFAULT_COMPANY_ID, &[imported], false).await?;
    assert_eq!(report.created, 1);
    assert!(report.rows[0]
        .note
        .as_deref()
        .unwrap_or_default()
        .contains("cp: counterparty ЄДРПОУ"));
    assert!(report.rows[0]
        .note
        .as_deref()
        .unwrap_or_default()
        .contains("contract: contract number"));

    let stored = db::invoices::find_by_bas_id_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        &format!("import-invoice-{suffix}"),
    )
    .await?
    .expect("invoice imported");
    let loaded = db::invoices::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, stored.id)
        .await?
        .expect("invoice with items exists");
    assert_eq!(loaded.0.counterparty_id, cp.id);
    assert_eq!(loaded.0.contract_id, Some(contract.id));
    assert_eq!(loaded.0.total_amount, dec!(3000.00));
    assert_eq!(loaded.1.len(), 2);

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(stored.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM contracts WHERE id = $1")
        .bind(contract.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn bas_invoice_import_marks_conflict_for_tolerant_header_match() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = create_test_counterparty(
        &pool,
        &suffix,
        &format!("ІТ Tolerant Контрагент {suffix}"),
        Some(suffix[..8].to_string()),
        Some(format!("tol-cp-{suffix}")),
    )
    .await?;

    let existing = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("TOL-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: Some("before import".to_string()),
            bas_id: None,
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Позиція".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(1000.00),
            }],
        },
    )
    .await?;

    let imported = ImportedInvoice {
        bas_id: None,
        counterparty_bas_id: None,
        counterparty_name: Some(cp.name.clone()),
        counterparty_edrpou: cp.edrpou.clone(),
        contract_bas_id: None,
        contract_number: None,
        number: existing.number.clone(),
        date: existing.date,
        expected_payment_date: None,
        direction: existing.direction,
        status: models::invoice::InvoiceStatus::Paid,
        total_amount: dec!(1000.03),
        vat_amount: Decimal::ZERO,
        notes: Some("after import".to_string()),
        items: vec![ImportedInvoiceItem {
            description: "Оновлена позиція".to_string(),
            unit: Some("шт".to_string()),
            quantity: dec!(1.0000),
            price: dec!(1000.00),
        }],
    };

    let report = apply_imported_invoices(&pool, DEFAULT_COMPANY_ID, &[imported], false).await?;
    assert_eq!(report.conflicts, 1);
    assert!(report.rows[0]
        .note
        .as_deref()
        .unwrap_or_default()
        .contains("tolerant"));

    let loaded = db::invoices::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, existing.id)
        .await?
        .expect("invoice still exists");
    assert_eq!(loaded.0.status, models::invoice::InvoiceStatus::Draft);
    assert_eq!(loaded.1.len(), 1);
    assert_eq!(loaded.1[0].description, "Позиція");

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(existing.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}
