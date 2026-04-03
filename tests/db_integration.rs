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
