use std::env;

use acta::{db, models};
use anyhow::Result;
use chrono::{Duration, Utc};
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
            date: Utc::now().date_naive(),
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

    let created = db::tasks::create(&pool, &new_task).await?;
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
