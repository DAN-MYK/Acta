use super::super::*;

#[tokio::test]
async fn invoices_create_update_and_status_flow_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Invoice Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-invoice-cp-{suffix}")),
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("IT-INV-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: Some("integration invoice".to_string()),
            bas_id: Some(format!("it-invoice-{suffix}")),
            items: vec![
                models::NewInvoiceItem {
                    position: 1,
                    description: "Товар 1".to_string(),
                    unit: Some("шт".to_string()),
                    quantity: dec!(2.0000),
                    price: dec!(150.00),
                },
                models::NewInvoiceItem {
                    position: 2,
                    description: "Товар 2".to_string(),
                    unit: Some("шт".to_string()),
                    quantity: dec!(3.0000),
                    price: dec!(200.00),
                },
            ],
        },
    )
    .await?;

    assert_eq!(invoice.total_amount, dec!(900.00));
    assert_eq!(invoice.status, models::InvoiceStatus::Draft);

    let loaded = db::invoices::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, invoice.id)
        .await?
        .expect("invoice exists");
    assert_eq!(loaded.0.number, invoice.number);
    assert_eq!(loaded.1.len(), 2);

    let editable = db::invoices::get_for_edit_scoped(&pool, DEFAULT_COMPANY_ID, invoice.id)
        .await?
        .expect("invoice editable");
    assert_eq!(editable.0.id, invoice.id);

    let listed = db::invoices::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        None,
        None,
        Some("IT-INV-"),
        None,
        None,
        None,
        None,
        None,
        false,
        chrono::Utc::now().date_naive(),
    )
    .await?;
    assert!(listed.iter().any(|row| row.id == invoice.id));

    let updated = db::invoices::update_with_items_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::UpdateInvoice {
            number: format!("IT-INV-UPD-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: Some("updated invoice".to_string()),
        },
        vec![models::NewInvoiceItem {
            position: 1,
            description: "Оновлений товар".to_string(),
            unit: Some("шт".to_string()),
            quantity: dec!(5.0000),
            price: dec!(250.00),
        }],
    )
    .await?
    .expect("накладна має оновитися");

    assert_eq!(updated.number, format!("IT-INV-UPD-{suffix}"));
    assert_eq!(updated.total_amount, dec!(1250.00));

    let reloaded = db::invoices::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, invoice.id)
        .await?
        .expect("invoice still exists");
    assert_eq!(reloaded.1.len(), 1);
    assert_eq!(reloaded.1[0].amount, dec!(1250.00));

    let issued = db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Issued,
    )
    .await?
    .expect("invoice issued");
    assert_eq!(issued.status, models::InvoiceStatus::Issued);

    let invalid = db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Draft,
    )
    .await;
    assert!(invalid.is_err());

    let signed = db::invoices::advance_status_scoped(&pool, DEFAULT_COMPANY_ID, invoice.id)
        .await?
        .expect("invoice signed");
    assert_eq!(signed.status, models::InvoiceStatus::Signed);

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn invoices_change_status_rejects_skipping_forward_transition() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Invoice Skip Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-invoice-skip-cp-{suffix}")),
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("IT-INV-SKIP-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("it-invoice-skip-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(1000.00),
            }],
        },
    )
    .await?;

    let result = db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Paid,
    )
    .await;
    assert!(result.is_err());

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn invoices_change_status_rejects_same_status_transition() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Invoice Same Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-invoice-same-cp-{suffix}")),
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("IT-INV-SAME-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("it-invoice-same-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(800.00),
            }],
        },
    )
    .await?;

    let result = db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Draft,
    )
    .await;
    assert!(result.is_err());

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn invoices_change_status_rejects_transition_from_paid() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Invoice Paid Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-invoice-paid-cp-{suffix}")),
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("IT-INV-PAID-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("it-invoice-paid-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(1200.00),
            }],
        },
    )
    .await?;

    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Issued,
    )
    .await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Signed,
    )
    .await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Paid,
    )
    .await?;

    let result = db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Issued,
    )
    .await;
    assert!(result.is_err());

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn invoices_advance_status_on_paid_returns_error() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Invoice Advance Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-invoice-advance-cp-{suffix}")),
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("IT-INV-ADV-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("it-invoice-advance-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(1400.00),
            }],
        },
    )
    .await?;

    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Issued,
    )
    .await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Signed,
    )
    .await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Paid,
    )
    .await?;

    let result = db::invoices::advance_status_scoped(&pool, DEFAULT_COMPANY_ID, invoice.id).await;
    assert!(result.is_err());

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn invoices_status_functions_return_none_for_missing_id() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let missing_id = Uuid::new_v4();

    let change_result = db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        missing_id,
        models::InvoiceStatus::Issued,
    )
    .await?;
    assert!(matches!(change_result, None));

    let advance_result =
        db::invoices::advance_status_scoped(&pool, DEFAULT_COMPANY_ID, missing_id).await?;
    assert!(matches!(advance_result, None));

    Ok(())
}

#[tokio::test]
async fn invoices_create_keeps_vat_amount_zero_by_current_contract() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Invoice VAT Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-invoice-vat-cp-{suffix}")),
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("IT-INV-VAT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("it-invoice-vat-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар з ПДВ-кейсом".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(3.0000),
                price: dec!(1000.00),
            }],
        },
    )
    .await?;

    assert_eq!(invoice.total_amount, dec!(3000.00));
    assert_eq!(invoice.vat_amount, dec!(0.00));

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn invoices_generate_next_number_uses_numeric_suffix() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Invoice Seq Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-invoice-seq-cp-{suffix}")),
        },
    )
    .await?;

    let year = Utc::now().year();
    let invoice1 = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("НАК-{year}-009"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("it-invoice-seq-1-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(100.00),
            }],
        },
    )
    .await?;

    let invoice2 = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("НАК-{year}-010"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("it-invoice-seq-2-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(100.00),
            }],
        },
    )
    .await?;

    let next_number = db::invoices::generate_next_number(&pool, DEFAULT_COMPANY_ID).await?;
    assert_eq!(next_number, format!("НАК-{year}-011"));

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice1.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice2.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn invoices_list_filtered_respects_status_and_search() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Filter Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-invoice-filter-cp-{suffix}")),
        },
    )
    .await?;

    let draft = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("FILTER-DRAFT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("it-filter-draft-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Чернетка".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(10.00),
            }],
        },
    )
    .await?;

    let issued_seed = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("FILTER-ISSUED-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("it-filter-issued-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Виставлено".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(20.00),
            }],
        },
    )
    .await?;

    let issued = db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        issued_seed.id,
        models::InvoiceStatus::Issued,
    )
    .await?
    .expect("invoice issued");
    assert_eq!(issued.status, models::InvoiceStatus::Issued);

    let issued_only = db::invoices::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(&["issued".to_string()]),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        chrono::Utc::now().date_naive(),
    )
    .await?;
    assert!(issued_only.iter().any(|row| row.id == issued.id));
    assert!(!issued_only.iter().any(|row| row.id == draft.id));

    let by_search = db::invoices::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(&["issued".to_string()]),
        None,
        Some("FILTER-ISSUED"),
        None,
        None,
        None,
        None,
        None,
        false,
        chrono::Utc::now().date_naive(),
    )
    .await?;
    assert_eq!(by_search.len(), 1);
    assert_eq!(by_search[0].id, issued.id);

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(draft.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(issued.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn invoices_advance_status_fails_for_final_status() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Paid Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-invoice-paid-cp-{suffix}")),
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("FINAL-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("it-final-{suffix}")),
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Фінальний".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(10.00),
            }],
        },
    )
    .await?;

    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        invoice.id,
        models::InvoiceStatus::Issued,
    )
    .await?
    .expect("issued");
    db::invoices::advance_status_scoped(&pool, DEFAULT_COMPANY_ID, invoice.id)
        .await?
        .expect("signed");
    db::invoices::advance_status_scoped(&pool, DEFAULT_COMPANY_ID, invoice.id)
        .await?
        .expect("paid");

    let err = db::invoices::advance_status_scoped(&pool, DEFAULT_COMPANY_ID, invoice.id)
        .await
        .expect_err("paid invoice should be final");
    assert!(err.to_string().contains("фінальному статусі"));

    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(invoice.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn invoices_update_with_items_fails_for_missing_invoice() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let err = db::invoices::update_with_items_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        Uuid::new_v4(),
        models::UpdateInvoice {
            number: "MISSING".to_string(),
            counterparty_id: Uuid::new_v4(),
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
        },
        vec![models::NewInvoiceItem {
            position: 1,
            description: "Missing".to_string(),
            unit: Some("шт".to_string()),
            quantity: dec!(1.0000),
            price: dec!(1.00),
        }],
    )
    .await
    .expect_err("missing invoice should fail");

    assert!(err.to_string().contains("не знайдена"));
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn seed_inv_counterparty(pool: &sqlx::PgPool, prefix: &str) -> Result<models::Counterparty> {
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

async fn seed_invoice(
    pool: &sqlx::PgPool,
    cp: &models::Counterparty,
    number: &str,
    amount: rust_decimal::Decimal,
) -> Result<models::Invoice> {
    db::invoices::create(
        pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: number.to_string(),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: chrono::Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: None,
            items: vec![models::NewInvoiceItem {
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

async fn seed_invoice_with_due(
    pool: &sqlx::PgPool,
    cp: &models::Counterparty,
    number: &str,
    amount: rust_decimal::Decimal,
    due: Option<chrono::NaiveDate>,
) -> Result<models::Invoice> {
    db::invoices::create(
        pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: number.to_string(),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: chrono::Utc::now().date_naive(),
            expected_payment_date: due,
            notes: None,
            bas_id: None,
            items: vec![models::NewInvoiceItem {
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

// ─── Invoices: list_filtered — amount range ───────────────────────────────────

#[tokio::test]
async fn list_filtered_amount_range() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let cp = seed_inv_counterparty(&pool, "INV-AMT-CP").await?;
    seed_invoice(&pool, &cp, "INV-AMT-500", dec!(500.00)).await?;
    seed_invoice(&pool, &cp, "INV-AMT-5000", dec!(5000.00)).await?;
    seed_invoice(&pool, &cp, "INV-AMT-50000", dec!(50000.00)).await?;

    let mid = db::invoices::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        None,
        None,
        Some("INV-AMT-"),
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
    assert_eq!(mid[0].number, "INV-AMT-5000");

    sqlx::query("DELETE FROM invoices WHERE counterparty_id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    Ok(())
}

// ─── Invoices: list_filtered — multi-status ───────────────────────────────────

#[tokio::test]
async fn list_filtered_multi_status() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let cp = seed_inv_counterparty(&pool, "INV-MS-CP").await?;
    seed_invoice(&pool, &cp, "INV-MS-DRAFT", dec!(100)).await?;
    let issued = seed_invoice(&pool, &cp, "INV-MS-ISSUED", dec!(100)).await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        issued.id,
        models::InvoiceStatus::Issued,
    )
    .await?
    .expect("issued");
    let paid_inv = seed_invoice(&pool, &cp, "INV-MS-PAID", dec!(100)).await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        paid_inv.id,
        models::InvoiceStatus::Issued,
    )
    .await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        paid_inv.id,
        models::InvoiceStatus::Signed,
    )
    .await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        paid_inv.id,
        models::InvoiceStatus::Paid,
    )
    .await?;

    let filtered = db::invoices::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(&["draft".to_string(), "paid".to_string()]),
        None,
        Some("INV-MS-"),
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
    assert!(numbers.contains(&"INV-MS-DRAFT"));
    assert!(numbers.contains(&"INV-MS-PAID"));
    assert!(!numbers.contains(&"INV-MS-ISSUED"));

    sqlx::query("DELETE FROM invoices WHERE counterparty_id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    Ok(())
}

// ─── Invoices: list_filtered — overdue_only ───────────────────────────────────

#[tokio::test]
async fn list_filtered_overdue_only() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let today = chrono::NaiveDate::from_ymd_opt(2026, 5, 8).unwrap();
    let past = chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
    let future = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

    let cp = seed_inv_counterparty(&pool, "INV-OVD-CP").await?;

    // paid + past due → excluded (paid is not in overdue set)
    let paid = seed_invoice_with_due(&pool, &cp, "INV-OVD-PAID", dec!(100), Some(past)).await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        paid.id,
        models::InvoiceStatus::Issued,
    )
    .await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        paid.id,
        models::InvoiceStatus::Signed,
    )
    .await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        paid.id,
        models::InvoiceStatus::Paid,
    )
    .await?;

    // issued + past due → MATCH
    let overdue =
        seed_invoice_with_due(&pool, &cp, "INV-OVD-ISSUED", dec!(100), Some(past)).await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        overdue.id,
        models::InvoiceStatus::Issued,
    )
    .await?;

    // issued + future due → excluded
    let future_due =
        seed_invoice_with_due(&pool, &cp, "INV-OVD-FUTURE", dec!(100), Some(future)).await?;
    db::invoices::change_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        future_due.id,
        models::InvoiceStatus::Issued,
    )
    .await?;

    // draft + past due → excluded (draft not in overdue set)
    seed_invoice_with_due(&pool, &cp, "INV-OVD-DRAFT", dec!(100), Some(past)).await?;

    let result = db::invoices::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        None,
        None,
        Some("INV-OVD-"),
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
        "Expected only INV-OVD-ISSUED, got: {:?}",
        numbers
    );
    assert_eq!(numbers[0], "INV-OVD-ISSUED");

    sqlx::query("DELETE FROM invoices WHERE counterparty_id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    Ok(())
}
