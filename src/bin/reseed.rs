use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

const CLEANUP_SQL: &str = r#"
WITH demo_companies AS (
    SELECT id
    FROM companies
    WHERE name LIKE 'РўРћР’ "РўРµСЃС‚РѕРІР° РєРѕРјРїР°РЅС–СЏ %'
)
DELETE FROM tasks
WHERE company_id IN (SELECT id FROM demo_companies);
"#;

const CLEANUP_PAYMENTS_SQL: &str = r#"
WITH demo_companies AS (
    SELECT id
    FROM companies
    WHERE name LIKE 'РўРћР’ "РўРµСЃС‚РѕРІР° РєРѕРјРїР°РЅС–СЏ %'
)
DELETE FROM payments
WHERE company_id IN (SELECT id FROM demo_companies);
"#;

const CLEANUP_SCHEDULE_SQL: &str = r#"
WITH demo_companies AS (
    SELECT id
    FROM companies
    WHERE name LIKE 'РўРћР’ "РўРµСЃС‚РѕРІР° РєРѕРјРїР°РЅС–СЏ %'
)
DELETE FROM payment_schedule
WHERE company_id IN (SELECT id FROM demo_companies);
"#;

const CLEANUP_ACTS_SQL: &str = r#"
WITH demo_companies AS (
    SELECT id
    FROM companies
    WHERE name LIKE 'РўРћР’ "РўРµСЃС‚РѕРІР° РєРѕРјРїР°РЅС–СЏ %'
)
DELETE FROM acts
WHERE company_id IN (SELECT id FROM demo_companies);
"#;

const CLEANUP_INVOICES_SQL: &str = r#"
WITH demo_companies AS (
    SELECT id
    FROM companies
    WHERE name LIKE 'РўРћР’ "РўРµСЃС‚РѕРІР° РєРѕРјРїР°РЅС–СЏ %'
)
DELETE FROM invoices
WHERE company_id IN (SELECT id FROM demo_companies);
"#;

const CLEANUP_COUNTERPARTIES_SQL: &str = r#"
WITH demo_companies AS (
    SELECT id
    FROM companies
    WHERE name LIKE 'РўРћР’ "РўРµСЃС‚РѕРІР° РєРѕРјРїР°РЅС–СЏ %'
)
DELETE FROM counterparties
WHERE company_id IN (SELECT id FROM demo_companies);
"#;

const CLEANUP_COMPANIES_SQL: &str = r#"
DELETE FROM companies
WHERE name LIKE 'РўРћР’ "РўРµСЃС‚РѕРІР° РєРѕРјРїР°РЅС–СЏ %';
"#;

const SEED_SQL: &str = include_str!("../../migrations/022_seed_demo_data.sql");

fn cleanup_statements() -> [&'static str; 7] {
    [
        CLEANUP_SQL,
        CLEANUP_PAYMENTS_SQL,
        CLEANUP_SCHEDULE_SQL,
        CLEANUP_ACTS_SQL,
        CLEANUP_INVOICES_SQL,
        CLEANUP_COUNTERPARTIES_SQL,
        CLEANUP_COMPANIES_SQL,
    ]
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL РЅРµ Р·Р°РґР°РЅРѕ. РџРµСЂРµРІС–СЂ .env С„Р°Р№Р».");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    println!("РњС–РіСЂР°С†С–С— Р·Р°СЃС‚РѕСЃРѕРІР°РЅРѕ.");

    let mut tx = pool.begin().await?;

    for statement in cleanup_statements() {
        sqlx::query(statement).execute(&mut *tx).await?;
    }

    sqlx::query(SEED_SQL).execute(&mut *tx).await?;
    tx.commit().await?;

    println!("Р”РµРјРѕ-РґР°РЅС– СѓСЃРїС–С€РЅРѕ РїРµСЂРµРІРёСЃС–СЏРЅС–.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{cleanup_statements, SEED_SQL};

    #[test]
    fn cleanup_statements_cover_all_demo_entities() {
        let statements = cleanup_statements();
        assert_eq!(statements.len(), 7);
        assert!(statements.iter().any(|sql| sql.contains("DELETE FROM tasks")));
        assert!(statements.iter().any(|sql| sql.contains("DELETE FROM payments")));
        assert!(statements.iter().any(|sql| sql.contains("DELETE FROM payment_schedule")));
        assert!(statements.iter().any(|sql| sql.contains("DELETE FROM acts")));
        assert!(statements.iter().any(|sql| sql.contains("DELETE FROM invoices")));
        assert!(statements.iter().any(|sql| sql.contains("DELETE FROM counterparties")));
        assert!(statements.iter().any(|sql| sql.contains("DELETE FROM companies")));
    }

    #[test]
    fn seed_sql_contains_demo_data_for_new_financial_modules() {
        assert!(SEED_SQL.contains("INSERT INTO payments"));
        assert!(SEED_SQL.contains("INSERT INTO payment_acts"));
        assert!(SEED_SQL.contains("INSERT INTO payment_invoices"));
        assert!(SEED_SQL.contains("INSERT INTO payment_schedule"));
        assert!(SEED_SQL.contains("INSERT INTO tasks"));
        assert!(SEED_SQL.contains("INSERT INTO invoices"));
    }
}
