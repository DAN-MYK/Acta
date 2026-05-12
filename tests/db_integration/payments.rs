use super::*;

#[tokio::test]
async fn payments_schema_is_applied_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    assert!(relation_exists(&pool, "payments").await?);
    assert!(relation_exists(&pool, "payment_acts").await?);
    assert!(relation_exists(&pool, "payment_invoices").await?);
    assert!(relation_exists(&pool, "payment_schedule").await?);

    let expected_payment_date_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_name = 'acts'
              AND column_name = 'expected_payment_date'
        )
        "#,
    )
    .fetch_one(&pool)
    .await?;
    assert!(expected_payment_date_exists);

    let invoice_expected_payment_date_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_name = 'invoices'
              AND column_name = 'expected_payment_date'
        )
        "#,
    )
    .fetch_one(&pool)
    .await?;
    assert!(invoice_expected_payment_date_exists);

    Ok(())
}

// ─── Payments: повний CRUD + фільтр по напрямку ──────────────────────────────

#[tokio::test]
async fn payments_crud_and_direction_filter_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();

    // Створюємо надходження з банківськими реквізитами
    let income = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(1500.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name: Some("ПриватБанк".to_string()),
            bank_ref: Some(format!("REF-{suffix}")),
            description: Some("Тестове надходження".to_string()),
        },
    )
    .await?;

    // get_by_id повертає правильні поля
    let fetched = db::payments::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, income.id)
        .await?
        .expect("платіж має існувати");
    assert_eq!(fetched.amount, dec!(1500.00));
    assert_eq!(fetched.direction, models::payment::PaymentDirection::Income);
    assert_eq!(fetched.bank_name.as_deref(), Some("ПриватБанк"));
    assert!(!fetched.is_reconciled);

    // Витрата без банківських реквізитів
    let expense = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(200.00),
            direction: models::payment::PaymentDirection::Expense,
            counterparty_id: None,
            bank_name: None,
            bank_ref: None,
            description: Some("Тестова витрата".to_string()),
        },
    )
    .await?;

    // list(None) повертає обидва записи
    let all = db::payments::list(&pool, DEFAULT_COMPANY_ID, None).await?;
    assert!(all.iter().any(|p| p.id == income.id));
    assert!(all.iter().any(|p| p.id == expense.id));

    // list(Income) не містить витрату
    let incomes = db::payments::list(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(models::payment::PaymentDirection::Income),
    )
    .await?;
    assert!(incomes.iter().any(|p| p.id == income.id));
    assert!(!incomes.iter().any(|p| p.id == expense.id));

    // list(Expense) не містить надходження
    let expenses = db::payments::list(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(models::payment::PaymentDirection::Expense),
    )
    .await?;
    assert!(expenses.iter().any(|p| p.id == expense.id));
    assert!(!expenses.iter().any(|p| p.id == income.id));

    // update змінює суму та банк
    let updated = db::payments::update_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        income.id,
        models::payment::UpdatePayment {
            date: Utc::now().date_naive(),
            amount: dec!(2000.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name: Some("Monobank".to_string()),
            bank_ref: Some(format!("NEW-REF-{suffix}")),
            description: Some("Оновлено".to_string()),
        },
    )
    .await?
    .expect("оновлення має повернути запис");
    assert_eq!(updated.amount, dec!(2000.00));
    assert_eq!(updated.bank_name.as_deref(), Some("Monobank"));

    // delete: після видалення get_by_id повертає None
    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, income.id).await?;
    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, expense.id).await?;
    assert!(
        db::payments::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, income.id)
            .await?
            .is_none()
    );

    Ok(())
}

// ─── Payments: фільтр по контрагенту ─────────────────────────────────────────

#[tokio::test]
async fn payments_list_by_counterparty_filters_correctly() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();

    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Платіж Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-pay-cp-{suffix}")),
        },
    )
    .await?;

    // Платіж із контрагентом
    let with_cp = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(500.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: Some(cp.id),
            bank_name: None,
            bank_ref: None,
            description: None,
        },
    )
    .await?;

    // Платіж без контрагента (інший одержувач)
    let without_cp = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(300.00),
            direction: models::payment::PaymentDirection::Expense,
            counterparty_id: None,
            bank_name: None,
            bank_ref: None,
            description: None,
        },
    )
    .await?;

    let by_cp = db::payments::list_by_counterparty(&pool, DEFAULT_COMPANY_ID, cp.id).await?;
    assert!(by_cp.iter().any(|p| p.id == with_cp.id));
    assert!(!by_cp.iter().any(|p| p.id == without_cp.id));
    assert_eq!(
        by_cp
            .iter()
            .find(|p| p.id == with_cp.id)
            .map(|p| p.counterparty_id),
        Some(Some(cp.id))
    );

    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, with_cp.id).await?;
    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, without_cp.id).await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

// ─── Payments: link_act і link_invoice (включно з upsert) ────────────────────

#[tokio::test]
async fn payments_link_act_and_link_invoice_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();

    // Контрагент для акту і накладної
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Link Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-link-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-LINK-ACT-{suffix}"),
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
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(3000.00),
            }],
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("IT-LINK-INV-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: None,
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(2.0000),
                price: dec!(1500.00),
            }],
        },
    )
    .await?;

    let payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(4500.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: Some(cp.id),
            bank_name: None,
            bank_ref: None,
            description: Some("Повна оплата".to_string()),
        },
    )
    .await?;

    // link_act: прив'язуємо платіж до акту з сумою 3000.00
    db::payments::link_act(&pool, payment.id, act.id, dec!(3000.00)).await?;

    let act_link_amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_acts WHERE payment_id = $1 AND act_id = $2",
    )
    .bind(payment.id)
    .bind(act.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(act_link_amount, dec!(3000.00));

    // upsert: повторна прив'язка оновлює суму
    db::payments::link_act(&pool, payment.id, act.id, dec!(2800.00)).await?;
    let act_link_updated = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_acts WHERE payment_id = $1 AND act_id = $2",
    )
    .bind(payment.id)
    .bind(act.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(act_link_updated, dec!(2800.00));

    // link_invoice: прив'язуємо платіж до накладної з сумою 1500.00
    db::payments::link_invoice(&pool, payment.id, invoice.id, dec!(1500.00)).await?;

    let inv_link_amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_invoices WHERE payment_id = $1 AND invoice_id = $2",
    )
    .bind(payment.id)
    .bind(invoice.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(inv_link_amount, dec!(1500.00));

    // upsert: повторна прив'язка до накладної оновлює суму
    db::payments::link_invoice(&pool, payment.id, invoice.id, dec!(1700.00)).await?;
    let inv_link_updated = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_invoices WHERE payment_id = $1 AND invoice_id = $2",
    )
    .bind(payment.id)
    .bind(invoice.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(inv_link_updated, dec!(1700.00));

    // Видалення платежу каскадно прибирає payment_acts і payment_invoices
    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, payment.id).await?;

    let act_link_gone = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM payment_acts WHERE payment_id = $1)",
    )
    .bind(payment.id)
    .fetch_one(&pool)
    .await?;
    assert!(!act_link_gone, "payment_acts має бути видалено каскадно");

    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
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

// ─── Payments: schedule create / complete / list_upcoming ────────────────────

#[tokio::test]
async fn payments_reconcile_persists_links_and_derived_state_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Reconcile Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-reconcile-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-RECONCILE-ACT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(3000.00),
            }],
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("IT-RECONCILE-INV-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: None,
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(1500.00),
            }],
        },
    )
    .await?;

    let payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(4500.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: Some(cp.id),
            bank_name: Some("Тест Банк".to_string()),
            bank_ref: Some(format!("RECONCILE-{suffix}")),
            description: Some("Оплата документів".to_string()),
        },
    )
    .await?;

    assert!(!payment.is_reconciled);

    db::payments::reconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "act",
        act.id,
        dec!(3000.00),
    )
    .await?;

    let act_link_amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_acts WHERE payment_id = $1 AND act_id = $2",
    )
    .bind(payment.id)
    .bind(act.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(act_link_amount, dec!(3000.00));

    db::payments::reconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "act",
        act.id,
        dec!(2800.00),
    )
    .await?;

    let act_link_amount_after_repeat = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_acts WHERE payment_id = $1 AND act_id = $2",
    )
    .bind(payment.id)
    .bind(act.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        act_link_amount_after_repeat,
        dec!(2800.00),
        "повторний reconcile має безпечно оновлювати amount через upsert"
    );

    let after_act = db::payments::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, payment.id)
        .await?
        .expect("payment exists after act reconcile");
    assert!(
        after_act.is_reconciled,
        "is_reconciled має обчислюватися з наявності links"
    );

    db::payments::reconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "invoice",
        invoice.id,
        dec!(1500.00),
    )
    .await?;

    let invoice_link_amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_invoices WHERE payment_id = $1 AND invoice_id = $2",
    )
    .bind(payment.id)
    .bind(invoice.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(invoice_link_amount, dec!(1500.00));

    db::payments::unreconcile_document_scoped(&pool, DEFAULT_COMPANY_ID, payment.id, "act", act.id)
        .await?;

    let act_link_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM payment_acts WHERE payment_id = $1 AND act_id = $2)",
    )
    .bind(payment.id)
    .bind(act.id)
    .fetch_one(&pool)
    .await?;
    assert!(!act_link_exists, "unreconcile має видаляти act link");

    let after_act_unreconcile =
        db::payments::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, payment.id)
            .await?
            .expect("payment exists after act unreconcile");
    assert!(
        after_act_unreconcile.is_reconciled,
        "поки лишається invoice link, derived state має бути true"
    );

    db::payments::unreconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "invoice",
        invoice.id,
    )
    .await?;

    db::payments::unreconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "invoice",
        invoice.id,
    )
    .await?;

    let invoice_link_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM payment_invoices WHERE payment_id = $1 AND invoice_id = $2)",
    )
    .bind(payment.id)
    .bind(invoice.id)
    .fetch_one(&pool)
    .await?;
    assert!(
        !invoice_link_exists,
        "unreconcile має видаляти invoice link"
    );

    let after_invoice_unreconcile =
        db::payments::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, payment.id)
            .await?
            .expect("payment exists after invoice unreconcile");
    assert!(
        !after_invoice_unreconcile.is_reconciled,
        "без links derived state має скидатися в false"
    );

    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, payment.id).await?;
    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
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
async fn payments_reconcile_rejects_cross_company_documents() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let foreign_company = db::companies::create(
        &pool,
        &models::NewCompany {
            name: format!("ІТ Foreign Reconcile Company {suffix}"),
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

    let default_cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Reconcile Default CP {suffix}"),
            edrpou: Some(format!("9{}", &suffix[..7])),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-reconcile-default-cp-{suffix}")),
        },
    )
    .await?;

    let foreign_cp = db::counterparties::create(
        &pool,
        foreign_company.id,
        &models::NewCounterparty {
            name: format!("ІТ Reconcile Foreign CP {suffix}"),
            edrpou: Some(format!("8{}", &suffix[..7])),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-reconcile-foreign-cp-{suffix}")),
        },
    )
    .await?;

    let foreign_act = db::acts::create(
        &pool,
        foreign_company.id,
        &models::NewAct {
            number: format!("IT-FOREIGN-ACT-{suffix}"),
            counterparty_id: foreign_cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(1000.00),
            }],
        },
    )
    .await?;

    let foreign_invoice = db::invoices::create(
        &pool,
        foreign_company.id,
        &models::NewInvoice {
            number: format!("IT-FOREIGN-INV-{suffix}"),
            counterparty_id: foreign_cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: None,
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(500.00),
            }],
        },
    )
    .await?;

    let payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(1500.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: Some(default_cp.id),
            bank_name: Some("Тест Банк".to_string()),
            bank_ref: Some(format!("FOREIGN-RECONCILE-{suffix}")),
            description: Some("Перевірка міжкомпанійного link".to_string()),
        },
    )
    .await?;

    let act_err = db::payments::reconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "act",
        foreign_act.id,
        dec!(1000.00),
    )
    .await
    .expect_err("reconcile не має дозволяти link на foreign act");
    assert!(
        act_err.to_string().contains("Документ не знайдено"),
        "помилка має явно вказувати на відсутність документа в межах компанії"
    );

    let invoice_err = db::payments::reconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "invoice",
        foreign_invoice.id,
        dec!(500.00),
    )
    .await
    .expect_err("reconcile не має дозволяти link на foreign invoice");
    assert!(
        invoice_err.to_string().contains("Документ не знайдено"),
        "помилка має явно вказувати на відсутність документа в межах компанії"
    );

    let act_unreconcile_err = db::payments::unreconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "act",
        foreign_act.id,
    )
    .await
    .expect_err("unreconcile не має дозволяти foreign act");
    assert!(act_unreconcile_err
        .to_string()
        .contains("Документ не знайдено"));

    let invoice_unreconcile_err = db::payments::unreconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "invoice",
        foreign_invoice.id,
    )
    .await
    .expect_err("unreconcile не має дозволяти foreign invoice");
    assert!(invoice_unreconcile_err
        .to_string()
        .contains("Документ не знайдено"));

    let foreign_links_exist = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(SELECT 1 FROM payment_acts WHERE payment_id = $1 AND act_id = $2)
            OR EXISTS(SELECT 1 FROM payment_invoices WHERE payment_id = $1 AND invoice_id = $3)
        "#,
    )
    .bind(payment.id)
    .bind(foreign_act.id)
    .bind(foreign_invoice.id)
    .fetch_one(&pool)
    .await?;
    assert!(
        !foreign_links_exist,
        "foreign документи не мають створювати links навіть при прямому виклику DB helper"
    );

    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, payment.id).await?;
    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(foreign_act.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(foreign_invoice.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1 OR id = $2")
        .bind(default_cp.id)
        .bind(foreign_cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(foreign_company.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn payments_reconcile_supports_split_and_rejects_overallocation() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Split Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-split-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-SPLIT-ACT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(1500.00),
            }],
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("IT-SPLIT-INV-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: None,
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(2000.00),
            }],
        },
    )
    .await?;

    let payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(3000.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: Some(cp.id),
            bank_name: Some("Тест Банк".to_string()),
            bank_ref: Some(format!("SPLIT-{suffix}")),
            description: Some("Розподіл платежу".to_string()),
        },
    )
    .await?;

    db::payments::reconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "act",
        act.id,
        dec!(1500.00),
    )
    .await?;

    db::payments::reconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "invoice",
        invoice.id,
        dec!(1500.00),
    )
    .await?;

    let payment_after_split = db::payments::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, payment.id)
        .await?
        .expect("payment exists after split reconcile");
    assert!(payment_after_split.is_reconciled);

    let over_payment_err = db::payments::reconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "invoice",
        invoice.id,
        dec!(2000.00),
    )
    .await
    .expect_err("reconcile must reject allocation above remaining payment amount");
    assert!(over_payment_err.to_string().contains("Сума звірки"));

    let over_document_err = db::payments::reconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "act",
        act.id,
        dec!(1600.00),
    )
    .await
    .expect_err("reconcile must reject allocation above document open amount");
    assert!(over_document_err.to_string().contains("документа"));

    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, payment.id).await?;
    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
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
async fn payments_reconcile_split_is_atomic_on_failure() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Atomic Split Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-atomic-split-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-ATM-SPLT-ACT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(1500.00),
            }],
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number: format!("IT-ATM-SPLT-INV-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: None,
            items: vec![models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(2000.00),
            }],
        },
    )
    .await?;

    let payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(3000.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: Some(cp.id),
            bank_name: Some("Тест Банк".to_string()),
            bank_ref: Some(format!("ATOMIC-SPLIT-{suffix}")),
            description: Some("Перевірка атомарного розподілу".to_string()),
        },
    )
    .await?;

    db::payments::reconcile_document_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        "act",
        act.id,
        dec!(1500.00),
    )
    .await?;

    let err = db::payments::reconcile_split_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        payment.id,
        &[
            db::payments::PaymentReconcileAllocation {
                document_kind: "invoice".to_string(),
                document_id: invoice.id,
                amount: dec!(1500.00),
            },
            db::payments::PaymentReconcileAllocation {
                document_kind: "act".to_string(),
                document_id: act.id,
                amount: dec!(1600.00),
            },
        ],
    )
    .await
    .expect_err("split reconcile має відкотити всю транзакцію, якщо один allocation невалідний");
    assert!(err.to_string().contains("документа"));

    let act_amount_after_error = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_acts WHERE payment_id = $1 AND act_id = $2",
    )
    .bind(payment.id)
    .bind(act.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        act_amount_after_error,
        dec!(1500.00),
        "попередній act link має лишитися після rollback"
    );

    let invoice_link_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM payment_invoices WHERE payment_id = $1 AND invoice_id = $2)",
    )
    .bind(payment.id)
    .bind(invoice.id)
    .fetch_one(&pool)
    .await?;
    assert!(
        !invoice_link_exists,
        "новий invoice link не має частково записатися після rollback"
    );

    let payment_after_error = db::payments::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, payment.id)
        .await?
        .expect("payment exists after failed atomic split");
    assert!(payment_after_error.is_reconciled);

    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, payment.id).await?;
    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
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
async fn payments_reconcile_split_waits_for_locked_document_row() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Locked Split Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-locked-split-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-LKD-SPLT-ACT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(2000.00),
            }],
        },
    )
    .await?;

    let payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(500.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: Some(cp.id),
            bank_name: Some("Тест Банк".to_string()),
            bank_ref: Some(format!("LOCKED-SPLIT-{suffix}")),
            description: Some("Перевірка блокування документа".to_string()),
        },
    )
    .await?;

    let mut lock_tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM acts WHERE id = $1 AND company_id = $2 FOR UPDATE")
        .bind(act.id)
        .bind(DEFAULT_COMPANY_ID)
        .execute(&mut *lock_tx)
        .await?;

    let reconcile_pool = pool.clone();
    let mut reconcile_task = tokio::spawn(async move {
        db::payments::reconcile_split_scoped(
            &reconcile_pool,
            DEFAULT_COMPANY_ID,
            payment.id,
            &[db::payments::PaymentReconcileAllocation {
                document_kind: "act".to_string(),
                document_id: act.id,
                amount: dec!(500.00),
            }],
        )
        .await
    });

    let blocked =
        tokio::time::timeout(std::time::Duration::from_millis(200), &mut reconcile_task).await;
    assert!(
        blocked.is_err(),
        "split reconcile має чекати, поки інша транзакція тримає FOR UPDATE lock на документі"
    );

    lock_tx.rollback().await?;

    reconcile_task.await??;

    let payment_after_lock = db::payments::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, payment.id)
        .await?
        .expect("payment exists after locked split reconcile");
    assert!(payment_after_lock.is_reconciled);

    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, payment.id).await?;
    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn payments_reconcile_document_waits_for_locked_document_row() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Locked Single Документ {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-locked-single-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-LKD-SNGL-ACT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(2000.00),
            }],
        },
    )
    .await?;

    let payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id: DEFAULT_COMPANY_ID,
            date: Utc::now().date_naive(),
            amount: dec!(500.00),
            direction: models::payment::PaymentDirection::Income,
            counterparty_id: Some(cp.id),
            bank_name: Some("Тест Банк".to_string()),
            bank_ref: Some(format!("LOCKED-SINGLE-{suffix}")),
            description: Some("Перевірка блокування для single reconcile".to_string()),
        },
    )
    .await?;

    let mut lock_tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM acts WHERE id = $1 AND company_id = $2 FOR UPDATE")
        .bind(act.id)
        .bind(DEFAULT_COMPANY_ID)
        .execute(&mut *lock_tx)
        .await?;

    let reconcile_pool = pool.clone();
    let mut reconcile_task = tokio::spawn(async move {
        db::payments::reconcile_document_scoped(
            &reconcile_pool,
            DEFAULT_COMPANY_ID,
            payment.id,
            "act",
            act.id,
            dec!(500.00),
        )
        .await
    });

    let blocked =
        tokio::time::timeout(std::time::Duration::from_millis(200), &mut reconcile_task).await;
    assert!(
        blocked.is_err(),
        "single reconcile має чекати, поки інша транзакція тримає FOR UPDATE lock на документі"
    );

    lock_tx.rollback().await?;
    reconcile_task.await??;

    let payment_after_lock = db::payments::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, payment.id)
        .await?
        .expect("payment exists after locked single reconcile");
    assert!(payment_after_lock.is_reconciled);

    db::payments::delete_scoped(&pool, DEFAULT_COMPANY_ID, payment.id).await?;
    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn payments_schedule_create_complete_and_list_upcoming_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let future_date = (Utc::now() + Duration::days(30)).date_naive();

    // Одноразовий запланований платіж у майбутньому
    let schedule = db::payments::create_schedule(
        &pool,
        models::payment::NewPaymentSchedule {
            company_id: DEFAULT_COMPANY_ID,
            title: format!("Оренда офісу {suffix}"),
            amount: Some(dec!(5000.00)),
            direction: models::payment::PaymentDirection::Expense,
            scheduled_date: future_date,
            recurrence: models::payment::ScheduleRecurrence::None,
            recurrence_end: None,
            counterparty_id: None,
            notes: Some("integration test schedule".to_string()),
        },
    )
    .await?;

    assert_eq!(schedule.title, format!("Оренда офісу {suffix}"));
    assert_eq!(schedule.amount, Some(dec!(5000.00)));
    assert!(!schedule.is_completed);

    // list_upcoming_schedule включає новий запис
    let upcoming = db::payments::list_upcoming_schedule(&pool, DEFAULT_COMPANY_ID, 100).await?;
    assert!(
        upcoming.iter().any(|s| s.id == schedule.id),
        "новий schedule має бути в upcoming"
    );

    // complete_schedule позначає як виконаний
    db::payments::complete_schedule(&pool, schedule.id).await?;

    // list_upcoming_schedule більше не повертає виконаний запис
    let upcoming_after =
        db::payments::list_upcoming_schedule(&pool, DEFAULT_COMPANY_ID, 100).await?;
    assert!(
        !upcoming_after.iter().any(|s| s.id == schedule.id),
        "виконаний schedule не має бути в upcoming"
    );

    sqlx::query("DELETE FROM payment_schedule WHERE id = $1")
        .bind(schedule.id)
        .execute(&pool)
        .await?;

    Ok(())
}

// ─── Dashboard helpers ────────────────────────────────────────────────────────

/// Створює компанію + контрагента для одного dashboard тесту.
/// Повертає (company_id, counterparty_id).
#[tokio::test]
async fn payments_upcoming_schedule_excludes_past_entries() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let past = db::payments::create_schedule(
        &pool,
        models::payment::NewPaymentSchedule {
            company_id: DEFAULT_COMPANY_ID,
            title: format!("Past schedule {suffix}"),
            amount: Some(dec!(500.00)),
            direction: models::payment::PaymentDirection::Expense,
            scheduled_date: (Utc::now() - Duration::days(1)).date_naive(),
            recurrence: models::payment::ScheduleRecurrence::None,
            recurrence_end: None,
            counterparty_id: None,
            notes: Some("past".to_string()),
        },
    )
    .await?;

    let future = db::payments::create_schedule(
        &pool,
        models::payment::NewPaymentSchedule {
            company_id: DEFAULT_COMPANY_ID,
            title: format!("Future schedule {suffix}"),
            amount: Some(dec!(700.00)),
            direction: models::payment::PaymentDirection::Expense,
            scheduled_date: (Utc::now() + Duration::days(3)).date_naive(),
            recurrence: models::payment::ScheduleRecurrence::None,
            recurrence_end: None,
            counterparty_id: None,
            notes: Some("future".to_string()),
        },
    )
    .await?;

    let upcoming = db::payments::list_upcoming_schedule(&pool, DEFAULT_COMPANY_ID, 100).await?;

    assert!(!upcoming.iter().any(|row| row.id == past.id));
    assert!(upcoming.iter().any(|row| row.id == future.id));

    sqlx::query("DELETE FROM payment_schedule WHERE id = $1 OR id = $2")
        .bind(past.id)
        .bind(future.id)
        .execute(&pool)
        .await?;

    Ok(())
}
