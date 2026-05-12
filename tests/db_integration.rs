use std::env;

use acta::import::bank_common::ParsedBankRow;
use acta::import::bas_acts::{ImportedAct, ImportedActItem, apply_imported_acts};
use acta::import::bas_contracts::{ImportedContract, apply_imported_contracts};
use acta::import::bas_counterparties::{ImportedCounterparty, apply_imported_counterparties};
use acta::import::bas_invoices::{ImportedInvoice, ImportedInvoiceItem, apply_imported_invoices};
use acta::import::bas_payments::apply_imported_payments;
use acta::{db, models};
use anyhow::Result;
use chrono::{Datelike, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

// UUID дефолтної компанії з міграції 012_companies.sql
const DEFAULT_COMPANY_ID: Uuid = Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);

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
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM pg_class WHERE relname = $1)")
            .bind(relation_name)
            .fetch_one(pool)
            .await?;

    Ok(exists)
}

async fn create_test_counterparty(
    pool: &PgPool,
    suffix: &str,
    name: &str,
    edrpou: Option<String>,
    bas_id: Option<String>,
) -> Result<models::Counterparty> {
    db::counterparties::create(
        pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: name.to_string(),
            edrpou,
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: bas_id.or_else(|| Some(format!("it-cp-{suffix}"))),
        },
    )
    .await
}

async fn create_test_contract(
    pool: &PgPool,
    suffix: &str,
    counterparty_id: Uuid,
    number: &str,
    bas_id: Option<String>,
) -> Result<models::contract::Contract> {
    let fallback_bas_id = format!("it-contract-{suffix}");
    let bas_id_ref = bas_id.as_deref().unwrap_or(&fallback_bas_id);
    db::contracts::create_imported(
        pool,
        DEFAULT_COMPANY_ID,
        counterparty_id,
        number,
        Some("integration contract"),
        Utc::now().date_naive(),
        None,
        None,
        None,
        Some(bas_id_ref),
        models::contract::ContractStatus::Active,
    )
    .await
}

async fn create_test_category(
    pool: &PgPool,
    company_id: Uuid,
    name: &str,
    kind: &str,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO categories (id, company_id, name, kind)
           VALUES ($1, $2, $3, $4)"#,
    )
    .bind(id)
    .bind(company_id)
    .bind(name)
    .bind(kind)
    .execute(pool)
    .await?;
    Ok(id)
}

async fn create_test_act(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    number: &str,
    amount: Decimal,
    status: &str,
    category_id: Option<Uuid>,
    date: chrono::NaiveDate,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO acts
           (id, company_id, counterparty_id, number, date, total_amount, status, category_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7::act_status, $8)"#,
    )
    .bind(id)
    .bind(company_id)
    .bind(counterparty_id)
    .bind(number)
    .bind(date)
    .bind(amount)
    .bind(status)
    .bind(category_id)
    .execute(pool)
    .await?;
    Ok(id)
}

async fn create_test_payment(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Option<Uuid>,
    amount: Decimal,
    direction: &str,
    date: chrono::NaiveDate,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO payments
           (id, company_id, counterparty_id, amount, direction, date)
           VALUES ($1, $2, $3, $4, $5::payment_direction, $6)"#,
    )
    .bind(id)
    .bind(company_id)
    .bind(counterparty_id)
    .bind(amount)
    .bind(direction)
    .bind(date)
    .execute(pool)
    .await?;
    Ok(id)
}

async fn create_test_invoice(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    number: &str,
    amount: Decimal,
    status: &str,
    expected_payment_date: Option<chrono::NaiveDate>,
    date: chrono::NaiveDate,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO invoices
           (id, company_id, counterparty_id, number, date, total_amount, status, expected_payment_date)
           VALUES ($1, $2, $3, $4, $5, $6, $7::invoice_status, $8)"#,
    )
    .bind(id)
    .bind(company_id)
    .bind(counterparty_id)
    .bind(number)
    .bind(date)
    .bind(amount)
    .bind(status)
    .bind(expected_payment_date)
    .execute(pool)
    .await?;
    Ok(id)
}

async fn create_test_payment_schedule(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Option<Uuid>,
    title: &str,
    amount: Decimal,
    scheduled_date: chrono::NaiveDate,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO payment_schedule
           (id, company_id, counterparty_id, title, amount, direction, scheduled_date, is_completed, recurrence)
           VALUES ($1, $2, $3, $4, $5, 'expense'::payment_direction, $6, FALSE, 'none')"#,
    )
    .bind(id)
    .bind(company_id)
    .bind(counterparty_id)
    .bind(title)
    .bind(amount)
    .bind(scheduled_date)
    .execute(pool)
    .await?;
    Ok(id)
}

#[path = "db_integration/bas.rs"]
mod bas;
#[path = "db_integration/catalog_and_numbering.rs"]
mod catalog_and_numbering;
#[path = "db_integration/dashboard.rs"]
mod dashboard;
#[path = "db_integration/documents_tasks_companies.rs"]
mod documents_tasks_companies;
#[path = "db_integration/payments.rs"]
mod payments;
#[path = "db_integration/pdf_paths.rs"]
mod pdf_paths;
#[path = "db_integration/waybills.rs"]
mod waybills;
