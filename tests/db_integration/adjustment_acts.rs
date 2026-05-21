use rust_decimal_macros::dec;

use super::*;

#[tokio::test]
async fn test_adjustment_act_create_and_get() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-ТОВ {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-КОР-{suffix}"), dec!(10000.00), "issued", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let adj = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id)
        .await.unwrap();

    assert_eq!(adj.original_act_id, act_id);
    assert_eq!(adj.counterparty_id, cp.id);
    assert!(adj.number.starts_with("КОР-"), "number must start with КОР-");
    assert_eq!(
        adj.status,
        acta::models::adjustment_act::AdjustmentActStatus::Draft
    );
    assert_eq!(adj.total_amount, dec!(0));

    let (fetched, items) = acta::db::adjustment_acts::get_full(
        &pool, DEFAULT_COMPANY_ID, adj.id,
    ).await.unwrap().unwrap();

    assert_eq!(fetched.id, adj.id);
    assert!(items.is_empty(), "fresh draft must have no items");
}

#[tokio::test]
async fn test_adjustment_act_numbering() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-NUM {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-NUM-{suffix}"), dec!(5000.00), "draft", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let adj1 = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();
    let adj2 = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    let seq1: u32 = adj1.number.rsplit_once('-').unwrap().1.parse().unwrap();
    let seq2: u32 = adj2.number.rsplit_once('-').unwrap().1.parse().unwrap();
    assert!(seq2 > seq1, "sequential adj acts must have ascending numbers");
}

#[tokio::test]
async fn test_adjustment_act_status_advance_sets_is_adjusted() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-STATUS {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-STATUS-{suffix}"), dec!(3000.00), "signed", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let is_adj_before: bool = sqlx::query_scalar(
        "SELECT is_adjusted FROM acts WHERE id = $1"
    )
    .bind(act_id)
    .fetch_one(&pool)
    .await.unwrap();
    assert!(!is_adj_before, "is_adjusted must start as false");

    let adj = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    // Draft → Issued → Signed → Applied (3 transitions)
    for _ in 0..3 {
        acta::db::adjustment_acts::change_status_scoped(&pool, DEFAULT_COMPANY_ID, adj.id)
            .await.unwrap().expect("transition must succeed");
    }

    let is_adj_after: bool = sqlx::query_scalar(
        "SELECT is_adjusted FROM acts WHERE id = $1"
    )
    .bind(act_id)
    .fetch_one(&pool)
    .await.unwrap();
    assert!(is_adj_after, "is_adjusted must be TRUE after adj reaches Applied");

    let status: String = sqlx::query_scalar(
        "SELECT status::text FROM adjustment_acts WHERE id = $1"
    )
    .bind(adj.id)
    .fetch_one(&pool)
    .await.unwrap();
    assert_eq!(status, "applied");
}

#[tokio::test]
async fn test_adjustment_act_delete_clears_is_adjusted() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-DEL {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-DEL-{suffix}"), dec!(7000.00), "paid", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let adj = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    // Advance to Applied
    for _ in 0..3 {
        acta::db::adjustment_acts::change_status_scoped(&pool, DEFAULT_COMPANY_ID, adj.id)
            .await.unwrap().unwrap();
    }

    let is_adj: bool = sqlx::query_scalar("SELECT is_adjusted FROM acts WHERE id = $1")
        .bind(act_id).fetch_one(&pool).await.unwrap();
    assert!(is_adj);

    let deleted = acta::db::adjustment_acts::delete_scoped(&pool, DEFAULT_COMPANY_ID, adj.id)
        .await.unwrap();
    assert!(deleted);

    let is_adj_after: bool = sqlx::query_scalar("SELECT is_adjusted FROM acts WHERE id = $1")
        .bind(act_id).fetch_one(&pool).await.unwrap();
    assert!(!is_adj_after, "is_adjusted must be FALSE when last applied adj deleted");
}

#[tokio::test]
async fn test_adjustment_acts_list_for_act() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-LIST {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-LIST-{suffix}"), dec!(5000.00), "issued", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let _adj1 = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();
    let _adj2 = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    let rows = acta::db::adjustment_acts::list_for_act(&pool, DEFAULT_COMPANY_ID, act_id)
        .await.unwrap();

    assert_eq!(rows.len(), 2, "must return both adj acts for the act");
    for row in &rows {
        assert_eq!(row.original_act_id, act_id);
        assert_eq!(row.counterparty_id, cp.id);
    }
}

#[tokio::test]
async fn test_issued_signed_adj_does_not_affect_effective_amount() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-EFF {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-EFF-{suffix}"), dec!(10000.00), "signed", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let adj = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    // Simulate a -1000 adjustment
    sqlx::query("UPDATE adjustment_acts SET total_amount = -1000 WHERE id = $1")
        .bind(adj.id)
        .execute(&pool)
        .await.unwrap();

    // Advance to Signed only (not Applied)
    for _ in 0..2 {
        acta::db::adjustment_acts::change_status_scoped(&pool, DEFAULT_COMPANY_ID, adj.id)
            .await.unwrap().unwrap();
    }

    let effective: rust_decimal::Decimal = sqlx::query_scalar(
        r#"SELECT a.total_amount + COALESCE(
               (SELECT SUM(aa.total_amount) FROM adjustment_acts aa
                WHERE aa.original_act_id = a.id AND aa.status = 'applied'),
               0)
           FROM acts a WHERE a.id = $1"#
    )
    .bind(act_id)
    .fetch_one(&pool)
    .await.unwrap();

    assert_eq!(
        effective, dec!(10000.00),
        "signed adj must NOT affect effective_amount — only applied ones do"
    );
}

#[tokio::test]
async fn test_duplicate_adj_number_rejected_by_constraint() {
    let Some(pool) = test_pool().await.expect("pool") else { return };
    let suffix = unique_suffix();

    let cp = create_test_counterparty(
        &pool, &suffix,
        &format!("КОР-UNIQ {suffix}"), None, None,
    ).await.unwrap();

    let act_id = create_test_act(
        &pool, DEFAULT_COMPANY_ID, cp.id,
        &format!("АКТ-UNIQ-{suffix}"), dec!(1000.00), "draft", None,
        chrono::Utc::now().date_naive(),
    ).await.unwrap();

    let adj = acta::db::adjustment_acts::create(&pool, DEFAULT_COMPANY_ID, act_id).await.unwrap();

    // Attempt to bypass generate_next_number and insert duplicate number
    let result = sqlx::query(
        r#"INSERT INTO adjustment_acts
           (company_id, original_act_id, counterparty_id, number, date, direction, total_amount, status)
           SELECT company_id, original_act_id, counterparty_id, $1, date, direction, 0, 'draft'
           FROM adjustment_acts WHERE id = $2"#
    )
    .bind(&adj.number)
    .bind(adj.id)
    .execute(&pool)
    .await;

    assert!(result.is_err(), "UNIQUE(company_id, number) must reject duplicate number");
    let err_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        err_msg.contains("unique") || err_msg.contains("duplicate"),
        "error must mention uniqueness violation"
    );
}
