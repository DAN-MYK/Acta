use std::env;

use acta::{db, models};
use anyhow::Result;
use chrono::{Datelike, Duration, Utc};
use rust_decimal_macros::dec;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

// UUID дефолтної компанії з міграції 012_companies.sql
const DEFAULT_COMPANY_ID: Uuid = Uuid::from_bytes([0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,1]);

async fn test_pool() -> Result<Option<PgPool>> {
    let url = env::var("TEST_DATABASE_URL")
        .ok()
        .or_else(|| env::var("DATABASE_URL").ok());

    let Some(url) = url else {
        eprintln!("skip db integration test: TEST_DATABASE_URL or DATABASE_URL is not set");
        return Ok(None);
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(Some(pool))
}

fn unique_suffix() -> String {
    Uuid::new_v4().simple().to_string()
}

async fn relation_exists(pool: &PgPool, relation_name: &str) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM pg_class WHERE relname = $1)"
    )
    .bind(relation_name)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

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

    let fetched = db::counterparties::get_by_id(&pool, created.id)
        .await?
        .expect("counterparty exists");
    assert_eq!(fetched.name, new_cp.name);

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

    Ok(())
}

#[tokio::test]
async fn acts_create_and_status_flow_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Акт Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-act-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("IT-ACT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: "outgoing".to_string(),
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: Some("integration test".to_string()),
            bas_id: Some(format!("it-act-{suffix}")),
            items: vec![
                models::NewActItem {
                    description: "Послуга 1".to_string(),
                    quantity: dec!(2.0000),
                    unit: "год".to_string(),
                    unit_price: dec!(1000.00),
                },
                models::NewActItem {
                    description: "Послуга 2".to_string(),
                    quantity: dec!(1.0000),
                    unit: "год".to_string(),
                    unit_price: dec!(500.00),
                },
            ],
        },
    )
    .await?;

    assert_eq!(act.total_amount, dec!(2500.00));

    let loaded = db::acts::get_by_id(&pool, act.id)
        .await?
        .expect("act exists");
    assert_eq!(loaded.0.number, act.number);
    assert_eq!(loaded.1.len(), 2);

    let issued = db::acts::change_status(&pool, act.id, models::ActStatus::Issued)
        .await?
        .expect("status changed");
    assert_eq!(issued.status, models::ActStatus::Issued);

    let invalid = db::acts::change_status(&pool, act.id, models::ActStatus::Draft).await;
    assert!(invalid.is_err());

    let signed = db::acts::advance_status(&pool, act.id)
        .await?
        .expect("advanced");
    assert_eq!(signed.status, models::ActStatus::Signed);

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
async fn tasks_create_update_and_delete_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Task Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-task-cp-{suffix}")),
        },
    )
    .await?;

    let new_task = models::NewTask {
        title: format!("ІТ Задача {suffix}"),
        description: Some("перевірка нагадування".to_string()),
        priority: models::TaskPriority::High,
        due_date: Some(Utc::now() + Duration::days(1)),
        reminder_at: Some(Utc::now()),
        counterparty_id: Some(cp.id),
        act_id: None,
    };

    let created = db::tasks::create(&pool, DEFAULT_COMPANY_ID, &new_task).await?;
    assert_eq!(created.status, models::TaskStatus::Open);

    let open_tasks = db::tasks::list_open(&pool).await?;
    assert!(open_tasks.iter().any(|t| t.id == created.id));

    let due = db::tasks::due_reminders(&pool).await?;
    assert!(due.iter().any(|t| t.id == created.id));

    let done = db::tasks::set_status(&pool, created.id, models::TaskStatus::Done)
        .await?
        .expect("status updated");
    assert_eq!(done.status, models::TaskStatus::Done);

    let deleted = db::tasks::delete(&pool, created.id).await?;
    assert!(deleted);

    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn tasks_update_and_get_by_id_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Task Update Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-task-update-cp-{suffix}")),
        },
    )
    .await?;

    let created = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Оновити задачу {suffix}"),
            description: Some("Початковий опис".to_string()),
            priority: models::TaskPriority::Normal,
            due_date: Some(Utc::now() + Duration::days(2)),
            reminder_at: None,
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?;

    let updated = db::tasks::update(
        &pool,
        created.id,
        &models::NewTask {
            title: format!("Оновлена задача {suffix}"),
            description: Some("Оновлений опис".to_string()),
            priority: models::TaskPriority::Critical,
            due_date: Some(Utc::now() + Duration::days(3)),
            reminder_at: Some(Utc::now() + Duration::hours(1)),
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?
    .expect("task updated");

    assert_eq!(updated.title, format!("Оновлена задача {suffix}"));
    assert_eq!(updated.description.as_deref(), Some("Оновлений опис"));
    assert_eq!(updated.priority, models::TaskPriority::Critical);

    let fetched = db::tasks::get_by_id(&pool, created.id)
        .await?
        .expect("task exists");
    assert_eq!(fetched.title, updated.title);
    assert_eq!(fetched.priority, updated.priority);

    let missing = db::tasks::update(
        &pool,
        Uuid::new_v4(),
        &models::NewTask {
            title: "Missing".to_string(),
            description: None,
            priority: models::TaskPriority::Low,
            due_date: None,
            reminder_at: None,
            counterparty_id: None,
            act_id: None,
        },
    )
    .await?;
    assert!(missing.is_none());

    sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(created.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn tasks_list_by_act_returns_only_related_tasks() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Task Act Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-task-act-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("TASK-ACT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: "outgoing".to_string(),
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: Some(format!("it-task-act-{suffix}")),
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "год".to_string(),
                unit_price: dec!(100.00),
            }],
        },
    )
    .await?;

    let related = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Задача по акту {suffix}"),
            description: None,
            priority: models::TaskPriority::High,
            due_date: None,
            reminder_at: None,
            counterparty_id: Some(cp.id),
            act_id: Some(act.id),
        },
    )
    .await?;

    let unrelated = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Інша задача {suffix}"),
            description: None,
            priority: models::TaskPriority::Low,
            due_date: None,
            reminder_at: None,
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?;

    let tasks = db::tasks::list_by_act(&pool, act.id).await?;
    assert!(tasks.iter().any(|t| t.id == related.id));
    assert!(!tasks.iter().any(|t| t.id == unrelated.id));

    sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(related.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(unrelated.id)
        .execute(&pool)
        .await?;
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
async fn tasks_due_reminders_and_list_open_filter_correctly() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Reminder Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-task-reminder-cp-{suffix}")),
        },
    )
    .await?;

    let urgent = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Термінова задача {suffix}"),
            description: None,
            priority: models::TaskPriority::Critical,
            due_date: Some(Utc::now() + Duration::hours(1)),
            reminder_at: Some(Utc::now()),
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?;

    let later = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Пізніша задача {suffix}"),
            description: None,
            priority: models::TaskPriority::Low,
            due_date: Some(Utc::now() + Duration::days(3)),
            reminder_at: Some(Utc::now() + Duration::days(2)),
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?;

    let done = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Закрита задача {suffix}"),
            description: None,
            priority: models::TaskPriority::High,
            due_date: Some(Utc::now() + Duration::hours(2)),
            reminder_at: Some(Utc::now()),
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?;
    db::tasks::set_status(&pool, done.id, models::TaskStatus::Done)
        .await?
        .expect("done task updated");

    let open_tasks = db::tasks::list_open(&pool).await?;
    let urgent_pos = open_tasks.iter().position(|t| t.id == urgent.id).expect("urgent in open list");
    let later_pos = open_tasks.iter().position(|t| t.id == later.id).expect("later in open list");
    assert!(urgent_pos < later_pos);
    assert!(!open_tasks.iter().any(|t| t.id == done.id));

    let due = db::tasks::due_reminders(&pool).await?;
    assert!(due.iter().any(|t| t.id == urgent.id));
    assert!(!due.iter().any(|t| t.id == later.id));
    assert!(!due.iter().any(|t| t.id == done.id));

    for id in [urgent.id, later.id, done.id] {
        sqlx::query("DELETE FROM tasks WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await?;
    }
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

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
            direction: "outgoing".to_string(),
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

    let loaded = db::invoices::get_by_id(&pool, invoice.id)
        .await?
        .expect("invoice exists");
    assert_eq!(loaded.0.number, invoice.number);
    assert_eq!(loaded.1.len(), 2);

    let editable = db::invoices::get_for_edit(&pool, invoice.id)
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
    )
    .await?;
    assert!(listed.iter().any(|row| row.id == invoice.id));

    let updated = db::invoices::update_with_items(
        &pool,
        invoice.id,
        models::UpdateInvoice {
            number: format!("IT-INV-UPD-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: Some("updated invoice".to_string()),
        },
        vec![
            models::NewInvoiceItem {
                position: 1,
                description: "Оновлений товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(5.0000),
                price: dec!(250.00),
            },
        ],
    )
    .await?;

    assert_eq!(updated.number, format!("IT-INV-UPD-{suffix}"));
    assert_eq!(updated.total_amount, dec!(1250.00));

    let reloaded = db::invoices::get_by_id(&pool, invoice.id)
        .await?
        .expect("invoice still exists");
    assert_eq!(reloaded.1.len(), 1);
    assert_eq!(reloaded.1[0].amount, dec!(1250.00));

    let issued = db::invoices::change_status(&pool, invoice.id, models::InvoiceStatus::Issued)
        .await?
        .expect("invoice issued");
    assert_eq!(issued.status, models::InvoiceStatus::Issued);

    let invalid = db::invoices::change_status(&pool, invoice.id, models::InvoiceStatus::Draft).await;
    assert!(invalid.is_err());

    let signed = db::invoices::advance_status(&pool, invoice.id)
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
            direction: "outgoing".to_string(),
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
            direction: "outgoing".to_string(),
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
            direction: "outgoing".to_string(),
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
            direction: "outgoing".to_string(),
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

    let issued = db::invoices::change_status(&pool, issued_seed.id, models::InvoiceStatus::Issued)
        .await?
        .expect("invoice issued");
    assert_eq!(issued.status, models::InvoiceStatus::Issued);

    let issued_only = db::invoices::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(models::InvoiceStatus::Issued),
        None,
        None,
        None,
        None,
        None,
    )
    .await?;
    assert!(issued_only.iter().any(|row| row.id == issued.id));
    assert!(!issued_only.iter().any(|row| row.id == draft.id));

    let by_search = db::invoices::list_filtered(
        &pool,
        DEFAULT_COMPANY_ID,
        Some(models::InvoiceStatus::Issued),
        None,
        Some("FILTER-ISSUED"),
        None,
        None,
        None,
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
            direction: "outgoing".to_string(),
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

    db::invoices::change_status(&pool, invoice.id, models::InvoiceStatus::Issued)
        .await?
        .expect("issued");
    db::invoices::advance_status(&pool, invoice.id)
        .await?
        .expect("signed");
    db::invoices::advance_status(&pool, invoice.id)
        .await?
        .expect("paid");

    let err = db::invoices::advance_status(&pool, invoice.id)
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

    let err = db::invoices::update_with_items(
        &pool,
        Uuid::new_v4(),
        models::UpdateInvoice {
            number: "MISSING".to_string(),
            counterparty_id: Uuid::new_v4(),
            contract_id: None,
            category_id: None,
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
            company_id:      DEFAULT_COMPANY_ID,
            date:            Utc::now().date_naive(),
            amount:          dec!(1500.00),
            direction:       models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name:       Some("ПриватБанк".to_string()),
            bank_ref:        Some(format!("REF-{suffix}")),
            description:     Some("Тестове надходження".to_string()),
        },
    )
    .await?;

    // get_by_id повертає правильні поля
    let fetched = db::payments::get_by_id(&pool, income.id)
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
            company_id:      DEFAULT_COMPANY_ID,
            date:            Utc::now().date_naive(),
            amount:          dec!(200.00),
            direction:       models::payment::PaymentDirection::Expense,
            counterparty_id: None,
            bank_name:       None,
            bank_ref:        None,
            description:     Some("Тестова витрата".to_string()),
        },
    )
    .await?;

    // list(None) повертає обидва записи
    let all = db::payments::list(&pool, DEFAULT_COMPANY_ID, None).await?;
    assert!(all.iter().any(|p| p.id == income.id));
    assert!(all.iter().any(|p| p.id == expense.id));

    // list(Income) не містить витрату
    let incomes =
        db::payments::list(&pool, DEFAULT_COMPANY_ID, Some(models::payment::PaymentDirection::Income))
            .await?;
    assert!(incomes.iter().any(|p| p.id == income.id));
    assert!(!incomes.iter().any(|p| p.id == expense.id));

    // list(Expense) не містить надходження
    let expenses =
        db::payments::list(&pool, DEFAULT_COMPANY_ID, Some(models::payment::PaymentDirection::Expense))
            .await?;
    assert!(expenses.iter().any(|p| p.id == expense.id));
    assert!(!expenses.iter().any(|p| p.id == income.id));

    // update змінює суму та банк
    let updated = db::payments::update(
        &pool,
        income.id,
        models::payment::UpdatePayment {
            date:            Utc::now().date_naive(),
            amount:          dec!(2000.00),
            direction:       models::payment::PaymentDirection::Income,
            counterparty_id: None,
            bank_name:       Some("Monobank".to_string()),
            bank_ref:        Some(format!("NEW-REF-{suffix}")),
            description:     Some("Оновлено".to_string()),
        },
    )
    .await?
    .expect("оновлення має повернути запис");
    assert_eq!(updated.amount, dec!(2000.00));
    assert_eq!(updated.bank_name.as_deref(), Some("Monobank"));

    // mark_reconciled встановлює is_reconciled = true
    db::payments::mark_reconciled(&pool, income.id).await?;
    let reconciled = db::payments::get_by_id(&pool, income.id)
        .await?
        .expect("платіж існує");
    assert!(reconciled.is_reconciled);

    // delete: після видалення get_by_id повертає None
    db::payments::delete(&pool, income.id).await?;
    db::payments::delete(&pool, expense.id).await?;
    assert!(db::payments::get_by_id(&pool, income.id).await?.is_none());

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
            name:    format!("ІТ Платіж Контрагент {suffix}"),
            edrpou:  Some(suffix[..8].to_string()),
            ipn:     None,
            iban:    None,
            address: None,
            phone:   None,
            email:   None,
            notes:   None,
            bas_id:  Some(format!("it-pay-cp-{suffix}")),
        },
    )
    .await?;

    // Платіж із контрагентом
    let with_cp = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id:      DEFAULT_COMPANY_ID,
            date:            Utc::now().date_naive(),
            amount:          dec!(500.00),
            direction:       models::payment::PaymentDirection::Income,
            counterparty_id: Some(cp.id),
            bank_name:       None,
            bank_ref:        None,
            description:     None,
        },
    )
    .await?;

    // Платіж без контрагента (інший одержувач)
    let without_cp = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id:      DEFAULT_COMPANY_ID,
            date:            Utc::now().date_naive(),
            amount:          dec!(300.00),
            direction:       models::payment::PaymentDirection::Expense,
            counterparty_id: None,
            bank_name:       None,
            bank_ref:        None,
            description:     None,
        },
    )
    .await?;

    let by_cp = db::payments::list_by_counterparty(&pool, DEFAULT_COMPANY_ID, cp.id).await?;
    assert!(by_cp.iter().any(|p| p.id == with_cp.id));
    assert!(!by_cp.iter().any(|p| p.id == without_cp.id));
    assert_eq!(by_cp.iter().find(|p| p.id == with_cp.id).map(|p| p.counterparty_id), Some(Some(cp.id)));

    db::payments::delete(&pool, with_cp.id).await?;
    db::payments::delete(&pool, without_cp.id).await?;
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
            name:    format!("ІТ Link Контрагент {suffix}"),
            edrpou:  Some(suffix[..8].to_string()),
            ipn:     None,
            iban:    None,
            address: None,
            phone:   None,
            email:   None,
            notes:   None,
            bas_id:  Some(format!("it-link-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number:                format!("IT-LINK-ACT-{suffix}"),
            counterparty_id:       cp.id,
            contract_id:           None,
            category_id:           None,
            direction:             "outgoing".to_string(),
            date:                  Utc::now().date_naive(),
            expected_payment_date: None,
            status:                models::ActStatus::Draft,
            notes:                 None,
            bas_id:                None,
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity:    dec!(1.0000),
                unit:        "шт".to_string(),
                unit_price:  dec!(3000.00),
            }],
        },
    )
    .await?;

    let invoice = db::invoices::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewInvoice {
            number:                format!("IT-LINK-INV-{suffix}"),
            counterparty_id:       cp.id,
            contract_id:           None,
            category_id:           None,
            direction:             "outgoing".to_string(),
            date:                  Utc::now().date_naive(),
            expected_payment_date: None,
            notes:                 None,
            bas_id:                None,
            items: vec![models::NewInvoiceItem {
                position:    1,
                description: "Товар".to_string(),
                unit:        Some("шт".to_string()),
                quantity:    dec!(2.0000),
                price:       dec!(1500.00),
            }],
        },
    )
    .await?;

    let payment = db::payments::create(
        &pool,
        models::payment::NewPayment {
            company_id:      DEFAULT_COMPANY_ID,
            date:            Utc::now().date_naive(),
            amount:          dec!(4500.00),
            direction:       models::payment::PaymentDirection::Income,
            counterparty_id: Some(cp.id),
            bank_name:       None,
            bank_ref:        None,
            description:     Some("Повна оплата".to_string()),
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
    db::payments::delete(&pool, payment.id).await?;

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
            company_id:      DEFAULT_COMPANY_ID,
            title:           format!("Оренда офісу {suffix}"),
            amount:          Some(dec!(5000.00)),
            direction:       models::payment::PaymentDirection::Expense,
            scheduled_date:  future_date,
            recurrence:      models::payment::ScheduleRecurrence::None,
            recurrence_end:  None,
            counterparty_id: None,
            notes:           Some("integration test schedule".to_string()),
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
    let upcoming_after = db::payments::list_upcoming_schedule(&pool, DEFAULT_COMPANY_ID, 100).await?;
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
async fn dashboard_test_setup(pool: &PgPool, suffix: &str) -> Result<(Uuid, Uuid)> {
    let company = db::companies::create(
        pool,
        &models::NewCompany {
            name:          format!("ІТ Dashboard Компанія {suffix}"),
            short_name:    None,
            edrpou:        Some(suffix[..8].to_string()),
            ipn:           None,
            iban:          None,
            legal_address: None,
            director_name: None,
            tax_system:    None,
            is_vat_payer:  false,
        },
    )
    .await?;

    let cp = db::counterparties::create(
        pool,
        company.id,
        &models::NewCounterparty {
            name:    format!("ІТ Dashboard Контрагент {suffix}"),
            edrpou:  None,
            ipn:     None,
            iban:    None,
            address: None,
            phone:   None,
            email:   None,
            notes:   None,
            bas_id:  None,
        },
    )
    .await?;

    Ok((company.id, cp.id))
}

/// Видаляє всі тестові дані компанії (акти → контрагенти → компанія).
async fn dashboard_test_cleanup(pool: &PgPool, company_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM acts WHERE company_id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE company_id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Створює мінімальний акт для поточного місяця.
async fn make_act(
    pool: &PgPool,
    company_id: Uuid,
    cp_id: Uuid,
    suffix: &str,
    tag: &str,
    amount: rust_decimal::Decimal,
    expected_payment_date: Option<chrono::NaiveDate>,
) -> Result<Uuid> {
    let act = db::acts::create(
        pool,
        company_id,
        &models::NewAct {
            number:                format!("IT-DASH-{tag}-{suffix}"),
            counterparty_id:       cp_id,
            contract_id:           None,
            category_id:           None,
            direction:             "outgoing".to_string(),
            date:                  Utc::now().date_naive(),
            expected_payment_date,
            status:                models::ActStatus::Draft,
            notes:                 None,
            bas_id:                None,
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity:    dec!(1.0000),
                unit:        "шт".to_string(),
                unit_price:  amount,
            }],
        },
    )
    .await?;
    Ok(act.id)
}

// ─── Dashboard: KPI summary ───────────────────────────────────────────────────

#[tokio::test]
async fn dashboard_kpi_summary_aggregates_acts_correctly() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    // Чернетка — не враховується ні в revenue, ні в unpaid
    make_act(&pool, company_id, cp_id, &suffix, "DRAFT", dec!(500.00), None).await?;

    // Виставлений — в unpaid_total
    let issued_id = make_act(&pool, company_id, cp_id, &suffix, "ISSUED", dec!(2000.00), None).await?;
    db::acts::change_status(&pool, issued_id, models::ActStatus::Issued).await?;

    // Оплачений — в revenue_this_month
    let paid_id = make_act(&pool, company_id, cp_id, &suffix, "PAID", dec!(3000.00), None).await?;
    db::acts::change_status(&pool, paid_id, models::ActStatus::Issued).await?;
    sqlx::query("UPDATE acts SET status = 'paid' WHERE id = $1")
        .bind(paid_id)
        .execute(&pool)
        .await?;

    let kpi = db::dashboard::get_kpi_summary(&pool, company_id).await?;

    assert_eq!(kpi.revenue_this_month, dec!(3000.00), "тільки оплачені акти поточного місяця");
    assert_eq!(kpi.unpaid_total, dec!(2000.00), "виставлені + підписані");
    assert_eq!(kpi.acts_this_month, 3, "всі три акти — поточний місяць");
    assert_eq!(kpi.active_counterparties, 1, "один активний контрагент");

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Dashboard: revenue by month ─────────────────────────────────────────────

#[tokio::test]
async fn dashboard_revenue_by_month_fills_all_slots() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    // Оплачений акт у поточному місяці
    let act_id = make_act(&pool, company_id, cp_id, &suffix, "REV", dec!(7777.00), None).await?;
    db::acts::change_status(&pool, act_id, models::ActStatus::Issued).await?;
    sqlx::query("UPDATE acts SET status = 'paid' WHERE id = $1")
        .bind(act_id)
        .execute(&pool)
        .await?;

    // Перевіряємо для N = 1, 3, 6
    for months in [1u32, 3, 6] {
        let result = db::dashboard::revenue_by_month(&pool, company_id, months).await?;

        assert_eq!(
            result.len(), months as usize,
            "revenue_by_month({months}) має повертати рівно {months} записів"
        );

        // Знаходимо слот поточного місяця
        let today = Utc::now().date_naive();
        let current_slot = result
            .iter()
            .find(|m| m.month_num == today.month() && m.year == today.year())
            .expect("поточний місяць має бути в результаті");
        assert_eq!(current_slot.amount, dec!(7777.00), "оплачений акт у поточному місяці");

        // Решта слотів — нуль (свіжа компанія без інших актів)
        for slot in result.iter().filter(|m| !(m.month_num == today.month() && m.year == today.year())) {
            assert_eq!(slot.amount, dec!(0), "порожній місяць має суму 0");
        }

        // Місяці — в монотонному порядку (всі (year, month) зростають або спадають)
        let pairs: Vec<(i32, u32)> = result.iter().map(|m| (m.year, m.month_num)).collect();
        let ascending  = pairs.windows(2).all(|w| (w[0].0, w[0].1) <= (w[1].0, w[1].1));
        let descending = pairs.windows(2).all(|w| (w[0].0, w[0].1) >= (w[1].0, w[1].1));
        assert!(ascending || descending, "місяці мають бути впорядковані монотонно");
    }

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Dashboard: acts status distribution ─────────────────────────────────────

#[tokio::test]
async fn dashboard_acts_status_distribution_counts_by_status() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    // 1 чернетка, 2 виставлені, 1 підписаний — всі в поточному місяці
    make_act(&pool, company_id, cp_id, &suffix, "D1", dec!(100.00), None).await?;

    let i1 = make_act(&pool, company_id, cp_id, &suffix, "I1", dec!(200.00), None).await?;
    let i2 = make_act(&pool, company_id, cp_id, &suffix, "I2", dec!(300.00), None).await?;
    db::acts::change_status(&pool, i1, models::ActStatus::Issued).await?;
    db::acts::change_status(&pool, i2, models::ActStatus::Issued).await?;

    let s1 = make_act(&pool, company_id, cp_id, &suffix, "S1", dec!(400.00), None).await?;
    db::acts::change_status(&pool, s1, models::ActStatus::Issued).await?;
    db::acts::advance_status(&pool, s1).await?;

    let slices = db::dashboard::acts_status_distribution(&pool, company_id).await?;

    let count_for = |status: &str| -> i64 {
        slices.iter().find(|s| s.status == status).map(|s| s.count).unwrap_or(0)
    };

    assert_eq!(count_for("draft"),  1, "одна чернетка");
    assert_eq!(count_for("issued"), 2, "два виставлені");
    assert_eq!(count_for("signed"), 1, "один підписаний");
    assert_eq!(count_for("paid"),   0, "оплачених немає");

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Dashboard: upcoming payments ────────────────────────────────────────────

#[tokio::test]
async fn dashboard_upcoming_payments_overdue_appears_first() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    let yesterday  = (Utc::now() - Duration::days(1)).date_naive();
    let next_month = (Utc::now() + Duration::days(30)).date_naive();

    // Прострочений — вчора, статус issued
    let overdue_id = make_act(&pool, company_id, cp_id, &suffix, "OVD", dec!(1500.00), Some(yesterday)).await?;
    db::acts::change_status(&pool, overdue_id, models::ActStatus::Issued).await?;

    // Майбутній — +30 днів, статус signed
    let future_id = make_act(&pool, company_id, cp_id, &suffix, "FUT", dec!(2500.00), Some(next_month)).await?;
    db::acts::change_status(&pool, future_id, models::ActStatus::Issued).await?;
    db::acts::advance_status(&pool, future_id).await?;

    let upcoming = db::dashboard::upcoming_payments(&pool, company_id, 10).await?;

    assert_eq!(upcoming.len(), 2, "обидва акти з expected_payment_date мають бути в списку");

    // Прострочений йде першим
    assert!(upcoming[0].is_overdue, "перший запис має бути прострочений");
    assert_eq!(upcoming[0].amount, dec!(1500.00));

    // Майбутній — другий, не прострочений
    assert!(!upcoming[1].is_overdue, "другий запис не має бути прострочений");
    assert_eq!(upcoming[1].amount, dec!(2500.00));

    // Формат дати: "DD Міс" (наприклад "07 Кві")
    assert!(
        upcoming[0].date_label.len() >= 6 && upcoming[0].date_label.contains(' '),
        "date_label має формат 'DD Міс': '{}'", upcoming[0].date_label
    );

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Dashboard: recent acts ───────────────────────────────────────────────────

#[tokio::test]
async fn dashboard_get_recent_acts_returns_latest_first() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    make_act(&pool, company_id, cp_id, &suffix, "R1", dec!(1000.00), None).await?;
    make_act(&pool, company_id, cp_id, &suffix, "R2", dec!(2000.00), None).await?;

    let recent = db::dashboard::get_recent_acts(&pool, company_id, 5).await?;

    assert_eq!(recent.len(), 2, "обидва акти мають бути в результаті");

    // Найновіший (R2) — перший завдяки ORDER BY created_at DESC
    assert_eq!(recent[0].num, format!("IT-DASH-R2-{suffix}"));
    assert_eq!(recent[1].num, format!("IT-DASH-R1-{suffix}"));

    // Статус — рядок "draft"
    assert_eq!(recent[0].status, "draft");

    // Формат дати: "ДД.ММ.РРРР" — рівно 10 символів
    assert_eq!(recent[0].date.len(), 10, "date має формат ДД.ММ.РРРР");
    assert_eq!(recent[0].date.chars().nth(2), Some('.'));
    assert_eq!(recent[0].date.chars().nth(5), Some('.'));

    // limit=1 повертає тільки один запис
    let limited = db::dashboard::get_recent_acts(&pool, company_id, 1).await?;
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].num, format!("IT-DASH-R2-{suffix}"));

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Contracts ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn contracts_crud_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    let contract = db::contracts::create(
        &pool,
        models::NewContract {
            company_id,
            counterparty_id: cp_id,
            number:          format!("ДГ-{suffix}"),
            subject:         Some("Розробка ПЗ".to_string()),
            date:            Utc::now().date_naive(),
            expires_at:      Some((Utc::now() + Duration::days(365)).date_naive()),
            amount:          Some(dec!(50000.00)),
        },
    )
    .await?;

    // get_by_id: всі поля
    let fetched = db::contracts::get_by_id(&pool, contract.id).await?;
    assert_eq!(fetched.number, format!("ДГ-{suffix}"));
    assert_eq!(fetched.subject.as_deref(), Some("Розробка ПЗ"));
    assert_eq!(fetched.amount, Some(dec!(50000.00)));
    assert_eq!(fetched.status, models::ContractStatus::Active, "default status = active");
    assert_eq!(fetched.company_id, company_id);
    assert_eq!(fetched.counterparty_id, cp_id);

    // list: договір присутній, дата у форматі "ДД.ММ.РРРР"
    let listed = db::contracts::list(&pool, company_id).await?;
    let row = listed.iter().find(|r| r.id == contract.id).expect("договір у списку");
    assert_eq!(row.date.len(), 10);
    assert_eq!(row.date.chars().nth(2), Some('.'));
    assert_eq!(row.counterparty_name, format!("ІТ Dashboard Контрагент {suffix}"));

    // update: змінюємо номер, статус, примітки
    let updated = db::contracts::update(
        &pool,
        contract.id,
        models::UpdateContract {
            number:     format!("ДГ-UPD-{suffix}"),
            subject:    Some("Розробка ПЗ (оновлено)".to_string()),
            date:       contract.date,
            expires_at: contract.expires_at,
            amount:     Some(dec!(55000.00)),
            status:     models::ContractStatus::Expired,
            notes:      Some("термін закінчився".to_string()),
        },
    )
    .await?;

    assert_eq!(updated.number, format!("ДГ-UPD-{suffix}"));
    assert_eq!(updated.status, models::ContractStatus::Expired);
    assert_eq!(updated.notes.as_deref(), Some("термін закінчився"));
    assert_eq!(updated.amount, Some(dec!(55000.00)));

    // delete: після видалення відсутній у списку
    db::contracts::delete(&pool, contract.id).await?;
    let after_delete = db::contracts::list(&pool, company_id).await?;
    assert!(!after_delete.iter().any(|r| r.id == contract.id));

    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn contracts_list_by_counterparty_isolates_by_cp() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp1_id) = dashboard_test_setup(&pool, &suffix).await?;

    let cp2 = db::counterparties::create(
        &pool,
        company_id,
        &models::NewCounterparty {
            name:    format!("ІТ Contracts CP2 {suffix}"),
            edrpou:  None,
            ipn:     None,
            iban:    None,
            address: None,
            phone:   None,
            email:   None,
            notes:   None,
            bas_id:  None,
        },
    )
    .await?;

    let new_contract = |cp: Uuid, tag: &str| models::NewContract {
        company_id,
        counterparty_id: cp,
        number:          format!("ДГ-{tag}-{suffix}"),
        subject:         None,
        date:            Utc::now().date_naive(),
        expires_at:      None,
        amount:          None,
    };

    let c1 = db::contracts::create(&pool, new_contract(cp1_id, "A")).await?;
    let c2 = db::contracts::create(&pool, new_contract(cp1_id, "B")).await?;
    let c3 = db::contracts::create(&pool, new_contract(cp2.id, "C")).await?;

    let for_cp1 = db::contracts::list_by_counterparty(&pool, company_id, cp1_id).await?;
    let for_cp2 = db::contracts::list_by_counterparty(&pool, company_id, cp2.id).await?;

    assert_eq!(for_cp1.len(), 2);
    assert!(for_cp1.iter().all(|r| r.counterparty_id == cp1_id));
    assert_eq!(for_cp2.len(), 1);
    assert_eq!(for_cp2[0].id, c3.id);

    let all = db::contracts::list(&pool, company_id).await?;
    assert!(all.iter().any(|r| r.id == c1.id));
    assert!(all.iter().any(|r| r.id == c2.id));
    assert!(all.iter().any(|r| r.id == c3.id));

    for id in [c1.id, c2.id, c3.id] {
        db::contracts::delete(&pool, id).await?;
    }
    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn contracts_list_for_select_returns_only_active() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let (company_id, cp_id) = dashboard_test_setup(&pool, &suffix).await?;

    let make = |tag: &str| models::NewContract {
        company_id,
        counterparty_id: cp_id,
        number:          format!("ДГ-SEL-{tag}-{suffix}"),
        subject:         None,
        date:            Utc::now().date_naive(),
        expires_at:      None,
        amount:          None,
    };

    let active    = db::contracts::create(&pool, make("ACT")).await?;
    let to_expire = db::contracts::create(&pool, make("EXP")).await?;
    let to_term   = db::contracts::create(&pool, make("TRM")).await?;

    db::contracts::update(&pool, to_expire.id, models::UpdateContract {
        number:     to_expire.number.clone(),
        subject:    None,
        date:       to_expire.date,
        expires_at: None,
        amount:     None,
        status:     models::ContractStatus::Expired,
        notes:      None,
    }).await?;

    db::contracts::update(&pool, to_term.id, models::UpdateContract {
        number:     to_term.number.clone(),
        subject:    None,
        date:       to_term.date,
        expires_at: None,
        amount:     None,
        status:     models::ContractStatus::Terminated,
        notes:      None,
    }).await?;

    let selectable = db::contracts::list_for_select(&pool, company_id, cp_id).await?;
    assert_eq!(selectable.len(), 1);
    assert_eq!(selectable[0].id, active.id);

    for id in [active.id, to_expire.id, to_term.id] {
        db::contracts::delete(&pool, id).await?;
    }
    dashboard_test_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Categories ───────────────────────────────────────────────────────────────

async fn category_company_cleanup(pool: &PgPool, company_id: Uuid) -> Result<()> {
    // categories мають ON DELETE CASCADE від company
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn make_category_company(pool: &PgPool, suffix: &str, tag: &str) -> Result<Uuid> {
    let company = db::companies::create(
        pool,
        &models::NewCompany {
            name:          format!("ІТ Cat{tag} Компанія {suffix}"),
            short_name:    None,
            edrpou:        Some(suffix[..8].to_string()),
            ipn:           None,
            iban:          None,
            legal_address: None,
            director_name: None,
            tax_system:    None,
            is_vat_payer:  false,
        },
    )
    .await?;
    Ok(company.id)
}

#[tokio::test]
async fn categories_crud_and_archive_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let company_id = make_category_company(&pool, &suffix, "CRUD").await?;

    let cat = db::categories::create(
        &pool,
        models::NewCategory {
            name:       "Консалтинг".to_string(),
            kind:       "income".to_string(),
            parent_id:  None,
            company_id,
        },
    )
    .await?;

    assert_eq!(cat.name, "Консалтинг");
    assert_eq!(cat.kind, "income");
    assert!(!cat.is_archived);

    // list: присутня
    let all = db::categories::list(&pool, company_id).await?;
    assert!(all.iter().any(|c| c.id == cat.id));

    // update: перейменування
    let renamed = db::categories::update(
        &pool,
        cat.id,
        models::UpdateCategory { name: "ІТ Консалтинг".to_string(), parent_id: None },
    )
    .await?;
    assert_eq!(renamed.name, "ІТ Консалтинг");

    // archive
    db::categories::archive(&pool, cat.id).await?;

    // list включає, але is_archived = true
    let after = db::categories::list(&pool, company_id).await?;
    let row = after.iter().find(|c| c.id == cat.id).expect("архівована категорія у list");
    assert!(row.is_archived);

    // list_for_select / list_all_for_select виключають архівовані
    let sel = db::categories::list_for_select(&pool, company_id, "income").await?;
    assert!(!sel.iter().any(|c| c.id == cat.id));

    let all_sel = db::categories::list_all_for_select(&pool, company_id).await?;
    assert!(!all_sel.iter().any(|c| c.id == cat.id));

    category_company_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn categories_hierarchy_and_select_depth() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let company_id = make_category_company(&pool, &suffix, "HIER").await?;

    let parent = db::categories::create(
        &pool,
        models::NewCategory {
            name: "Розробка".to_string(), kind: "income".to_string(),
            parent_id: None, company_id,
        },
    )
    .await?;

    let child = db::categories::create(
        &pool,
        models::NewCategory {
            name: "Мобільна розробка".to_string(), kind: "income".to_string(),
            parent_id: Some(parent.id), company_id,
        },
    )
    .await?;

    let expense = db::categories::create(
        &pool,
        models::NewCategory {
            name: "Оренда".to_string(), kind: "expense".to_string(),
            parent_id: None, company_id,
        },
    )
    .await?;

    // list_for_select("income"): тільки income, depth коректний
    let income = db::categories::list_for_select(&pool, company_id, "income").await?;
    assert_eq!(income.len(), 2);

    let p = income.iter().find(|c| c.id == parent.id).expect("батько");
    let ch = income.iter().find(|c| c.id == child.id).expect("дочірня");
    assert_eq!(p.depth,  0, "батько depth=0");
    assert_eq!(ch.depth, 1, "дочірня depth=1");

    // expense не потрапляє в income
    assert!(!income.iter().any(|c| c.id == expense.id));

    // list_all_for_select: всі три
    let all = db::categories::list_all_for_select(&pool, company_id).await?;
    assert_eq!(all.len(), 3);

    // parent_id NULLS FIRST → батько перед дочірньою
    let income_order: Vec<Uuid> = income.iter().map(|c| c.id).collect();
    assert_eq!(income_order[0], parent.id);

    category_company_cleanup(&pool, company_id).await?;
    Ok(())
}

#[tokio::test]
async fn categories_seed_defaults_creates_standard_entries() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let company_id = make_category_company(&pool, &suffix, "SEED").await?;

    db::categories::seed_defaults(&pool, company_id).await?;

    let income  = db::categories::list_for_select(&pool, company_id, "income").await?;
    let expense = db::categories::list_for_select(&pool, company_id, "expense").await?;
    let all     = db::categories::list_all_for_select(&pool, company_id).await?;

    assert_eq!(income.len(),  4, "4 income: Розробка ПЗ, Консалтинг, Тех. підтримка, Навчання");
    assert_eq!(expense.len(), 5, "5 expense: Зарплата, Оренда, Маркетинг, Податки, Комунальні");
    assert_eq!(all.len(),     9);

    // Ідемпотентність: ON CONFLICT DO NOTHING
    db::categories::seed_defaults(&pool, company_id).await?;
    assert_eq!(db::categories::list_all_for_select(&pool, company_id).await?.len(), 9);

    // Конкретні назви income
    let names: Vec<&str> = income.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Розробка ПЗ"));
    assert!(names.contains(&"Консалтинг"));

    category_company_cleanup(&pool, company_id).await?;
    Ok(())
}

// ─── Acts: generate_next_number ──────────────────────────────────────────────

#[tokio::test]
async fn acts_generate_next_number_uses_numeric_max() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let year = Utc::now().year();

    // Ізольована компанія — без жодного акту
    let (company_id,): (Uuid,) =
        sqlx::query_as("INSERT INTO companies (name) VALUES ($1) RETURNING id")
            .bind(format!("ІТ NextNum Компанія {suffix}"))
            .fetch_one(&pool)
            .await?;

    // Порожня компанія → перший номер завжди "АКТ-РРРР-001"
    let first = db::acts::generate_next_number(&pool, company_id).await?;
    assert_eq!(first, format!("АКТ-{year}-001"));

    let cp = db::counterparties::create(
        &pool,
        company_id,
        &models::NewCounterparty {
            name:    format!("ІТ NextNum Контрагент {suffix}"),
            edrpou:  Some(suffix[..8].to_string()),
            ipn:     None,
            iban:    None,
            address: None,
            phone:   None,
            email:   None,
            notes:   None,
            bas_id:  Some(format!("it-nn-cp-{suffix}")),
        },
    )
    .await?;

    // Вставляємо непадовані номери "АКТ-РРРР-9" та "АКТ-РРРР-10":
    // лексикографічно "9" > "10", але числово max = 10 → очікуємо "011"
    for num_suffix in ["9", "10"] {
        db::acts::create(
            &pool,
            company_id,
            &models::NewAct {
                number:                format!("АКТ-{year}-{num_suffix}"),
                counterparty_id:       cp.id,
                contract_id:           None,
                category_id:           None,
                direction:             "outgoing".to_string(),
                date:                  Utc::now().date_naive(),
                expected_payment_date: None,
                status:                models::ActStatus::Draft,
                notes:                 None,
                bas_id:                None,
                items:                 vec![models::NewActItem {
                    description: "Тест".to_string(),
                    quantity:    dec!(1.0000),
                    unit:        "шт".to_string(),
                    unit_price:  dec!(1.00),
                }],
            },
        )
        .await?;
    }

    let next = db::acts::generate_next_number(&pool, company_id).await?;
    assert_eq!(
        next,
        format!("АКТ-{year}-011"),
        "числовий MAX(10) + 1 = 11, а не лексикографічний ('9' > '10')"
    );

    sqlx::query("DELETE FROM acts WHERE company_id = $1")
        .bind(company_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(company_id)
        .execute(&pool)
        .await?;

    Ok(())
}

// ─── Acts: get_kpi ───────────────────────────────────────────────────────────

#[tokio::test]
async fn acts_get_kpi_aggregates_this_month_and_overdue() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let today = Utc::now().date_naive();
    // Дата старіша 30 днів — гарантовано потрапляє у overdue
    let overdue_date = today - Duration::days(45);

    let (company_id,): (Uuid,) =
        sqlx::query_as("INSERT INTO companies (name) VALUES ($1) RETURNING id")
            .bind(format!("ІТ KPI Компанія {suffix}"))
            .fetch_one(&pool)
            .await?;

    let cp = db::counterparties::create(
        &pool,
        company_id,
        &models::NewCounterparty {
            name:    format!("ІТ KPI Контрагент {suffix}"),
            edrpou:  Some(suffix[..8].to_string()),
            ipn:     None,
            iban:    None,
            address: None,
            phone:   None,
            email:   None,
            notes:   None,
            bas_id:  Some(format!("it-kpi-cp-{suffix}")),
        },
    )
    .await?;

    let make_item = |price: u32| models::NewActItem {
        description: "Послуга".to_string(),
        quantity:    dec!(1.0000),
        unit:        "шт".to_string(),
        unit_price:  rust_decimal::Decimal::from(price),
    };

    // Акт 1: today, статус Draft → acts_this_month += 1
    db::acts::create(
        &pool,
        company_id,
        &models::NewAct {
            number:                format!("KPI-DRAFT-{suffix}"),
            counterparty_id:       cp.id,
            contract_id:           None,
            category_id:           None,
            direction:             "outgoing".to_string(),
            date:                  today,
            expected_payment_date: None,
            status:                models::ActStatus::Draft,
            notes:                 None,
            bas_id:                None,
            items:                 vec![make_item(1000)],
        },
    )
    .await?;

    // Акт 2: today, статус Paid → acts_this_month += 1, revenue_this_month += 2000
    let act_paid = db::acts::create(
        &pool,
        company_id,
        &models::NewAct {
            number:                format!("KPI-PAID-{suffix}"),
            counterparty_id:       cp.id,
            contract_id:           None,
            category_id:           None,
            direction:             "outgoing".to_string(),
            date:                  today,
            expected_payment_date: None,
            status:                models::ActStatus::Draft,
            notes:                 None,
            bas_id:                None,
            items:                 vec![make_item(2000)],
        },
    )
    .await?;
    db::acts::change_status(&pool, act_paid.id, models::ActStatus::Issued).await?;
    db::acts::change_status(&pool, act_paid.id, models::ActStatus::Signed).await?;
    db::acts::change_status(&pool, act_paid.id, models::ActStatus::Paid).await?;

    // Акт 3: today, статус Issued → acts_this_month += 1, unpaid_total += 3000
    let act_issued_new = db::acts::create(
        &pool,
        company_id,
        &models::NewAct {
            number:                format!("KPI-ISSUED-NEW-{suffix}"),
            counterparty_id:       cp.id,
            contract_id:           None,
            category_id:           None,
            direction:             "outgoing".to_string(),
            date:                  today,
            expected_payment_date: None,
            status:                models::ActStatus::Draft,
            notes:                 None,
            bas_id:                None,
            items:                 vec![make_item(3000)],
        },
    )
    .await?;
    db::acts::change_status(&pool, act_issued_new.id, models::ActStatus::Issued).await?;

    // Акт 4: дата 45 днів тому, статус Issued
    // → unpaid_total += 4000, overdue_count += 1 (issued + date < today-30d)
    // → НЕ входить до acts_this_month (45 днів тому ≠ поточний місяць)
    let act_overdue = db::acts::create(
        &pool,
        company_id,
        &models::NewAct {
            number:                format!("KPI-OVERDUE-{suffix}"),
            counterparty_id:       cp.id,
            contract_id:           None,
            category_id:           None,
            direction:             "outgoing".to_string(),
            date:                  overdue_date,
            expected_payment_date: None,
            status:                models::ActStatus::Draft,
            notes:                 None,
            bas_id:                None,
            items:                 vec![make_item(4000)],
        },
    )
    .await?;
    db::acts::change_status(&pool, act_overdue.id, models::ActStatus::Issued).await?;

    let kpi = db::acts::get_kpi(&pool, company_id).await?;

    // acts 1, 2, 3 мають date = today (поточний місяць), act 4 — 45 днів тому
    assert_eq!(kpi.acts_this_month, 3);

    // Тільки act 2 (paid + цей місяць) = 2000
    assert_eq!(kpi.revenue_this_month, dec!(2000.00));

    // acts 3 (3000) + 4 (4000) = 7000 — статус issued, незалежно від дати
    assert_eq!(kpi.unpaid_total, dec!(7000.00));

    // Тільки act 4: issued І date < today-30d
    assert_eq!(kpi.overdue_count, 1);

    sqlx::query("DELETE FROM acts WHERE company_id = $1")
        .bind(company_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(company_id)
        .execute(&pool)
        .await?;

    Ok(())
}

// ─── Acts: update_with_items ─────────────────────────────────────────────────

#[tokio::test]
async fn acts_update_with_items_replaces_positions_and_recalculates_total() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name:    format!("ІТ UpdateItems Контрагент {suffix}"),
            edrpou:  Some(suffix[..8].to_string()),
            ipn:     None,
            iban:    None,
            address: None,
            phone:   None,
            email:   None,
            notes:   None,
            bas_id:  Some(format!("it-uwi-cp-{suffix}")),
        },
    )
    .await?;

    // Створюємо акт з двома позиціями: 500 + 300 = 800
    let original = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number:                format!("UWI-{suffix}"),
            counterparty_id:       cp.id,
            contract_id:           None,
            category_id:           None,
            direction:             "outgoing".to_string(),
            date:                  Utc::now().date_naive(),
            expected_payment_date: None,
            status:                models::ActStatus::Draft,
            notes:                 Some("оригінал".to_string()),
            bas_id:                None,
            items:                 vec![
                models::NewActItem {
                    description: "Стара послуга 1".to_string(),
                    quantity:    dec!(1.0000),
                    unit:        "шт".to_string(),
                    unit_price:  dec!(500.00),
                },
                models::NewActItem {
                    description: "Стара послуга 2".to_string(),
                    quantity:    dec!(1.0000),
                    unit:        "шт".to_string(),
                    unit_price:  dec!(300.00),
                },
            ],
        },
    )
    .await?;
    assert_eq!(original.total_amount, dec!(800.00));

    let (_, items_before) = db::acts::get_by_id(&pool, original.id)
        .await?
        .expect("act exists");
    assert_eq!(items_before.len(), 2);

    // Оновлюємо: 1 нова позиція qty=3, price=400 → total = 1200
    let updated = db::acts::update_with_items(
        &pool,
        original.id,
        models::UpdateAct {
            number:                format!("UWI-UPDATED-{suffix}"),
            counterparty_id:       cp.id,
            contract_id:           None,
            category_id:           None,
            date:                  Utc::now().date_naive(),
            expected_payment_date: None,
            notes:                 Some("оновлено".to_string()),
        },
        vec![models::NewActItem {
            description: "Нова послуга".to_string(),
            quantity:    dec!(3.0000),
            unit:        "год".to_string(),
            unit_price:  dec!(400.00),
        }],
    )
    .await?;

    // Перевіряємо заголовок
    assert_eq!(updated.number, format!("UWI-UPDATED-{suffix}"));
    assert_eq!(updated.total_amount, dec!(1200.00));
    assert_eq!(updated.notes.as_deref(), Some("оновлено"));

    // Перевіряємо позиції: старі замінились на нову
    let (_, items_after) = db::acts::get_by_id(&pool, original.id)
        .await?
        .expect("act exists after update");
    assert_eq!(items_after.len(), 1);
    assert_eq!(items_after[0].description, "Нова послуга");
    assert_eq!(items_after[0].amount, dec!(1200.00));

    // Оновлення неіснуючого акту — anyhow помилка з текстом "не знайдено"
    let missing_err = db::acts::update_with_items(
        &pool,
        Uuid::new_v4(),
        models::UpdateAct {
            number:                "MISSING".to_string(),
            counterparty_id:       cp.id,
            contract_id:           None,
            category_id:           None,
            date:                  Utc::now().date_naive(),
            expected_payment_date: None,
            notes:                 None,
        },
        vec![],
    )
    .await
    .expect_err("неіснуючий акт має повертати помилку");
    assert!(
        missing_err.to_string().contains("не знайдено"),
        "повідомлення помилки: {missing_err}"
    );

    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(original.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}
