use anyhow::Result;
use chrono::Datelike;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::adjustment_act::{
    AdjustmentAct, AdjustmentActItem, AdjustmentActListRow, AdjustmentActStatus,
    NewAdjustmentActItem, UpdateAdjustmentAct,
};
use crate::models::DocumentDirection;

/// Генерує наступний номер у форматі "КОР-РРРР-NNN".
/// Той самий rsplit_once('-') паттерн що і в acts::generate_next_number.
pub async fn generate_next_number(pool: &PgPool, company_id: Uuid) -> Result<String> {
    use sqlx::Row;
    let year = chrono::Utc::now().year();

    let rows = sqlx::query(
        "SELECT number FROM adjustment_acts WHERE company_id = $1 AND EXTRACT(YEAR FROM date)::int = $2"
    )
    .bind(company_id)
    .bind(year as i32)
    .fetch_all(pool)
    .await?;

    let max_seq = rows
        .iter()
        .filter_map(|r| {
            let num: Option<String> = r.try_get("number").ok();
            num.and_then(|n| n.rsplit_once('-').and_then(|(_, s)| s.parse::<u32>().ok()))
        })
        .max()
        .unwrap_or(0);

    Ok(format!("КОР-{year}-{:03}", max_seq + 1))
}

/// Створює акт коригування — чернетку з нульовою сумою.
/// Верифікує що `original_act_id` належить до `company_id`.
/// Копіює `counterparty_id` і `direction` з оригінального акту — не довіряє клієнту.
pub async fn create(pool: &PgPool, company_id: Uuid, original_act_id: Uuid) -> Result<AdjustmentAct> {
    use sqlx::Row;

    let original = sqlx::query(
        "SELECT counterparty_id, direction FROM acts WHERE id = $1 AND company_id = $2"
    )
    .bind(original_act_id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("Оригінальний акт не знайдено в межах компанії"))?;

    let counterparty_id: Uuid = original.get("counterparty_id");
    let direction: DocumentDirection = original.get("direction");
    let number = generate_next_number(pool, company_id).await?;
    let date = chrono::Utc::now().date_naive();

    let mut tx = pool.begin().await?;

    let row = sqlx::query(
        r#"INSERT INTO adjustment_acts
           (company_id, original_act_id, counterparty_id, number, date, direction, total_amount, status)
           VALUES ($1, $2, $3, $4, $5, $6, 0, 'draft')
           RETURNING id, company_id, original_act_id, counterparty_id, number, date,
                     direction, total_amount, status, notes, bas_id, created_at, updated_at"#
    )
    .bind(company_id)
    .bind(original_act_id)
    .bind(counterparty_id)
    .bind(&number)
    .bind(date)
    .bind(direction.as_str())
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(AdjustmentAct {
        id: row.get("id"),
        company_id: row.get("company_id"),
        original_act_id: row.get("original_act_id"),
        counterparty_id: row.get("counterparty_id"),
        number: row.get("number"),
        date: row.get("date"),
        direction: row.get("direction"),
        total_amount: row.get("total_amount"),
        status: row.get("status"),
        notes: row.get("notes"),
        bas_id: row.get("bas_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub async fn get_full(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
) -> Result<Option<(AdjustmentAct, Vec<AdjustmentActItem>)>> {
    let adj = sqlx::query_as::<_, AdjustmentAct>(
        r#"SELECT id, company_id, original_act_id, counterparty_id, number, date,
                  direction, total_amount, status, notes, bas_id, created_at, updated_at
           FROM adjustment_acts WHERE id = $1 AND company_id = $2"#
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let Some(adj) = adj else { return Ok(None) };

    let items = sqlx::query_as::<_, AdjustmentActItem>(
        r#"SELECT id, adjustment_act_id, description, quantity, unit_price, total_price,
                  created_at, updated_at
           FROM adjustment_act_items WHERE adjustment_act_id = $1 ORDER BY created_at"#
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(Some((adj, items)))
}

/// Оновлює заголовок + позиції (DELETE+INSERT в транзакції).
/// total_amount перераховується з items.
pub async fn update_with_items_scoped(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
    update: UpdateAdjustmentAct,
    items: Vec<NewAdjustmentActItem>,
) -> Result<Option<()>> {
    let total_amount: Decimal = items.iter().map(|i| i.quantity * i.unit_price).sum();

    let mut tx = pool.begin().await?;

    let result = sqlx::query(
        r#"UPDATE adjustment_acts
           SET number = $1, date = $2, total_amount = $3, notes = $4, updated_at = NOW()
           WHERE id = $5 AND company_id = $6"#
    )
    .bind(&update.number)
    .bind(update.date)
    .bind(total_amount)
    .bind(&update.notes)
    .bind(id)
    .bind(company_id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }

    sqlx::query("DELETE FROM adjustment_act_items WHERE adjustment_act_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    for item in &items {
        let total_price = item.quantity * item.unit_price;
        sqlx::query(
            r#"INSERT INTO adjustment_act_items
               (adjustment_act_id, description, quantity, unit_price, total_price)
               VALUES ($1, $2, $3, $4, $5)"#
        )
        .bind(id)
        .bind(&item.description)
        .bind(item.quantity)
        .bind(item.unit_price)
        .bind(total_price)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(Some(()))
}

/// Переводить акт коригування на наступний статус.
/// При переході у Applied: виставляє is_adjusted = TRUE на оригінальному акті.
pub async fn change_status_scoped(
    pool: &PgPool,
    company_id: Uuid,
    id: Uuid,
) -> Result<Option<()>> {
    let adj = sqlx::query_as::<_, AdjustmentAct>(
        r#"SELECT id, company_id, original_act_id, counterparty_id, number, date,
                  direction, total_amount, status, notes, bas_id, created_at, updated_at
           FROM adjustment_acts WHERE id = $1 AND company_id = $2"#
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let Some(adj) = adj else { return Ok(None) };

    let next = adj
        .status
        .next()
        .ok_or_else(|| anyhow::anyhow!("Акт коригування вже у фінальному статусі Applied"))?;

    let mut tx = pool.begin().await?;

    sqlx::query(
        "UPDATE adjustment_acts SET status = $1::adjustment_act_status, updated_at = NOW() WHERE id = $2"
    )
    .bind(next.as_str())
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if matches!(next, AdjustmentActStatus::Applied) {
        sqlx::query(
            "UPDATE acts SET is_adjusted = TRUE, updated_at = NOW() WHERE id = $1"
        )
        .bind(adj.original_act_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(Some(()))
}

/// Видаляє акт коригування.
/// Якщо він був Applied і це останній applied — знімає is_adjusted на оригінальному акті.
pub async fn delete_scoped(pool: &PgPool, company_id: Uuid, id: Uuid) -> Result<bool> {
    let adj = sqlx::query_as::<_, AdjustmentAct>(
        r#"SELECT id, company_id, original_act_id, counterparty_id, number, date,
                  direction, total_amount, status, notes, bas_id, created_at, updated_at
           FROM adjustment_acts WHERE id = $1 AND company_id = $2"#
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let Some(adj) = adj else { return Ok(false) };
    let was_applied = matches!(adj.status, AdjustmentActStatus::Applied);
    let original_act_id = adj.original_act_id;

    let mut tx = pool.begin().await?;

    let result = sqlx::query(
        "DELETE FROM adjustment_acts WHERE id = $1 AND company_id = $2"
    )
    .bind(id)
    .bind(company_id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(false);
    }

    if was_applied {
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM adjustment_acts WHERE original_act_id = $1 AND status = 'applied'"
        )
        .bind(original_act_id)
        .fetch_one(&mut *tx)
        .await?;

        if remaining == 0 {
            sqlx::query(
                "UPDATE acts SET is_adjusted = FALSE, updated_at = NOW() WHERE id = $1"
            )
            .bind(original_act_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(true)
}

pub async fn list_for_act(
    pool: &PgPool,
    company_id: Uuid,
    original_act_id: Uuid,
) -> Result<Vec<AdjustmentActListRow>> {
    sqlx::query_as::<_, AdjustmentActListRow>(
        r#"SELECT aa.id, aa.company_id, aa.original_act_id,
                  a.number AS original_act_number,
                  aa.counterparty_id,
                  c.name AS counterparty_name,
                  aa.number, aa.date, aa.total_amount, aa.direction, aa.status
           FROM adjustment_acts aa
           JOIN acts a ON a.id = aa.original_act_id
           JOIN counterparties c ON c.id = aa.counterparty_id
           WHERE aa.company_id = $1 AND aa.original_act_id = $2
           ORDER BY aa.date DESC, aa.number"#
    )
    .bind(company_id)
    .bind(original_act_id)
    .fetch_all(pool)
    .await
    .map_err(anyhow::Error::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn list_filtered(
    pool: &PgPool,
    company_id: Uuid,
    statuses: Option<&[String]>,
    direction: Option<DocumentDirection>,
    search_query: Option<&str>,
    counterparty_id: Option<Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
    amount_min: Option<Decimal>,
    amount_max: Option<Decimal>,
) -> Result<Vec<AdjustmentActListRow>> {
    let search_query = search_query.map(str::trim).filter(|q| !q.is_empty());
    let has_search = search_query.is_some();

    let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        r#"SELECT aa.id, aa.company_id, aa.original_act_id,
                  a.number AS original_act_number,
                  aa.counterparty_id,
                  c.name AS counterparty_name,
                  aa.number, aa.date, aa.total_amount, aa.direction, aa.status
           FROM adjustment_acts aa
           JOIN acts a ON a.id = aa.original_act_id
           JOIN counterparties c ON c.id = aa.counterparty_id
           WHERE aa.company_id = "#,
    );
    qb.push_bind(company_id);

    if let Some(statuses) = statuses.filter(|s| !s.is_empty()) {
        let owned: Vec<String> = statuses.to_vec();
        qb.push(" AND aa.status::text = ANY(")
            .push_bind(owned)
            .push("::text[])");
    }
    if let Some(dir) = direction {
        qb.push(" AND aa.direction = ").push_bind(dir.as_str());
    }
    if let Some(q) = search_query {
        let pattern = super::ilike_pattern(q);
        qb.push(" AND (aa.number ILIKE ")
            .push_bind(pattern.clone())
            .push(r" ESCAPE '\' OR c.name ILIKE ")
            .push_bind(pattern)
            .push(r" ESCAPE '\')");
    }
    if let Some(cp_id) = counterparty_id {
        qb.push(" AND aa.counterparty_id = ").push_bind(cp_id);
    }
    if let Some(df) = date_from {
        qb.push(" AND aa.date >= ").push_bind(df);
    }
    if let Some(dt) = date_to {
        qb.push(" AND aa.date <= ").push_bind(dt);
    }
    if let Some(min) = amount_min {
        qb.push(" AND aa.total_amount >= ").push_bind(min);
    }
    if let Some(max) = amount_max {
        qb.push(" AND aa.total_amount <= ").push_bind(max);
    }
    qb.push(" ORDER BY aa.date DESC, aa.number");
    if has_search {
        qb.push(" LIMIT 100");
    }

    qb.build_query_as::<AdjustmentActListRow>()
        .fetch_all(pool)
        .await
        .map_err(anyhow::Error::from)
}
