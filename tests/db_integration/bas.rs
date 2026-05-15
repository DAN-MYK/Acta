use super::*;

#[tokio::test]
async fn bas_counterparty_preview_updates_existing_by_exact_name() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let name = format!("ІТ Preview Контрагент {suffix}");
    let counterparty = create_test_counterparty(&pool, &suffix, &name, None, None).await?;

    let imported = ImportedCounterparty {
        bas_id: None,
        name: name.clone(),
        edrpou: None,
        ipn: None,
        iban: Some("UA123456789012345678901234567".to_string()),
        address: None,
        phone: None,
        email: None,
    };

    let report =
        apply_imported_counterparties(&pool, DEFAULT_COMPANY_ID, &[imported], true).await?;
    assert_eq!(report.updated, 1);
    assert!(report.rows[0]
        .note
        .as_deref()
        .unwrap_or_default()
        .contains("exact name"));

    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(counterparty.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn bas_counterparty_preview_marks_conflict_for_duplicate_name() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let name = format!("ІТ Conflict Контрагент {suffix}");
    let cp1 = create_test_counterparty(&pool, &suffix, &name, None, Some(format!("cp-a-{suffix}")))
        .await?;
    let cp2 = create_test_counterparty(&pool, &suffix, &name, None, Some(format!("cp-b-{suffix}")))
        .await?;

    let imported = ImportedCounterparty {
        bas_id: None,
        name: name.clone(),
        edrpou: None,
        ipn: None,
        iban: None,
        address: None,
        phone: None,
        email: None,
    };

    let report =
        apply_imported_counterparties(&pool, DEFAULT_COMPANY_ID, &[imported], true).await?;
    assert_eq!(report.conflicts, 1);
    assert!(report.rows[0]
        .note
        .as_deref()
        .unwrap_or_default()
        .contains("conflict"));

    sqlx::query("DELETE FROM counterparties WHERE id = $1 OR id = $2")
        .bind(cp1.id)
        .bind(cp2.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn bas_contract_preview_updates_existing_by_number_fallback() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = create_test_counterparty(
        &pool,
        &suffix,
        &format!("ІТ Contract Preview {suffix}"),
        None,
        Some(format!("cp-contract-preview-{suffix}")),
    )
    .await?;
    let number = format!("ДГ-PREVIEW-{suffix}");
    let contract = create_test_contract(&pool, &suffix, cp.id, &number, None).await?;

    let imported = ImportedContract {
        bas_id: None,
        counterparty_bas_id: cp.bas_id.clone(),
        number: number.clone(),
        subject: Some("оновлений предмет".to_string()),
        date: Utc::now().date_naive(),
        expires_at: None,
        amount: Some(dec!(5000.00)),
        notes: None,
        status: models::contract::ContractStatus::Active,
    };

    let report = apply_imported_contracts(&pool, DEFAULT_COMPANY_ID, &[imported], true).await?;
    assert_eq!(report.updated, 1);
    assert!(report.rows[0]
        .note
        .as_deref()
        .unwrap_or_default()
        .contains("contract number"));

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
async fn bas_contract_preview_marks_conflict_for_duplicate_number() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = create_test_counterparty(
        &pool,
        &suffix,
        &format!("ІТ Contract Conflict {suffix}"),
        None,
        Some(format!("cp-contract-conflict-{suffix}")),
    )
    .await?;
    let number = format!("ДГ-CONFLICT-{suffix}");
    let c1 = create_test_contract(
        &pool,
        &suffix,
        cp.id,
        &number,
        Some(format!("ctr-a-{suffix}")),
    )
    .await?;
    let c2 = create_test_contract(
        &pool,
        &suffix,
        cp.id,
        &number,
        Some(format!("ctr-b-{suffix}")),
    )
    .await?;

    let imported = ImportedContract {
        bas_id: None,
        counterparty_bas_id: cp.bas_id.clone(),
        number: number.clone(),
        subject: None,
        date: Utc::now().date_naive(),
        expires_at: None,
        amount: None,
        notes: None,
        status: models::contract::ContractStatus::Active,
    };

    let report = apply_imported_contracts(&pool, DEFAULT_COMPANY_ID, &[imported], true).await?;
    assert_eq!(report.conflicts, 1);
    assert!(report.rows[0]
        .note
        .as_deref()
        .unwrap_or_default()
        .contains("conflict"));

    sqlx::query("DELETE FROM contracts WHERE id = $1 OR id = $2")
        .bind(c1.id)
        .bind(c2.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn bas_act_preview_updates_existing_by_header_fingerprint() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = create_test_counterparty(
        &pool,
        &suffix,
        &format!("ІТ Act Preview {suffix}"),
        None,
        Some(format!("cp-act-preview-{suffix}")),
    )
    .await?;
    let contract = create_test_contract(
        &pool,
        &suffix,
        cp.id,
        &format!("ACT-CONTRACT-{suffix}"),
        Some(format!("act-contract-preview-{suffix}")),
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("ACT-PREVIEW-{suffix}"),
            counterparty_id: cp.id,
            contract_id: Some(contract.id),
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Issued,
            notes: Some("before preview".to_string()),
            bas_id: None,
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(2.0000),
                unit: "год".to_string(),
                unit_price: dec!(1500.00),
            }],
        },
    )
    .await?;

    let imported = ImportedAct {
        bas_id: None,
        counterparty_bas_id: cp.bas_id.clone(),
        contract_bas_id: contract.bas_id.clone(),
        number: act.number.clone(),
        date: act.date,
        expected_payment_date: None,
        direction: models::DocumentDirection::Outgoing,
        status: models::ActStatus::Signed,
        notes: Some("after preview".to_string()),
        items: vec![ImportedActItem {
            description: "Послуга".to_string(),
            quantity: dec!(2.0000),
            unit: "год".to_string(),
            unit_price: dec!(1500.00),
        }],
    };

    let report = apply_imported_acts(&pool, DEFAULT_COMPANY_ID, &[imported], true).await?;
    assert_eq!(report.updated, 1);
    assert!(report.rows[0]
        .note
        .as_deref()
        .unwrap_or_default()
        .contains("header fingerprint"));

    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
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
async fn bas_payment_preview_marks_duplicate_by_bank_ref() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(2500.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name: Some("Тест Банк".to_string()),
            bank_ref: Some(format!("PAY-PREVIEW-{suffix}")),
            description: Some("Оплата рахунку".to_string()),
        },
    )
    .await?;

    let imported = ParsedBankRow {
        date: payment.date,
        amount: payment.amount,
        direction: payment.direction.clone(),
        description: payment.description.clone().unwrap_or_default(),
        bank_ref: payment.bank_ref.clone(),
        bank_name: payment.bank_name.clone().unwrap_or_default(),
        counterparty_name: None,
        counterparty_iban: None,
        currency: None,
    };

    let report = apply_imported_payments(&pool, DEFAULT_COMPANY_ID, &[imported], true).await?;
    assert_eq!(report.skipped, 1);
    assert!(report.rows[0]
        .note
        .as_deref()
        .unwrap_or_default()
        .contains("bank_ref"));

    sqlx::query("DELETE FROM payments WHERE id = $1")
        .bind(payment.id)
        .execute(&pool)
        .await?;

    Ok(())
}
