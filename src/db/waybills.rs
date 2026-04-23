// CRUD операції для видаткових накладних (товарних накладних)
//
// Всі запити — runtime-style (sqlx::query_as::<_, T>()) без макросів.
// Транзакційна вставка: create() та update_with_items() відкривають транзакцію,
// вставляють заголовок + позиції, перераховують total_amount, потім commit.

use anyhow::{Result, bail};
use chrono::Datelike;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::DocumentDirection;
use crate::models::waybill::{
    Waybill, WaybillItem, WaybillListRow, WaybillStatus, NewWaybill, NewWaybillItem, UpdateWaybill,
};

/// Згенерувати наступний номер накладної у форматі "НАК-РРРР-NNN".
///
/// Нумерація ізольована по компаніях і по роках.
pub async fn generate_next_number(pool: &PgPool, company_id: Uuid) -> Result<String> {
    use sqlx::Row;
    let year = chrono::Utc::now().year();

    let rows = sqlx::query(
        "SELECT number FROM waybills WHERE company_id = $1 AND EXTRACT(YEAR FROM date)::int = $2",
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

    Ok(format!("НАК-{year}-{:03}", max_seq + 1))
}

/// Отримати активних контрагентів компанії для ComboBox у формі накладної.
pub async fn counterparties_for_select(pool: &PgPool, company_id: Uuid) -> Result<Vec<(Uuid, String)>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name FROM counterparties WHERE is_archived = FALSE AND company_id = $1 ORDER BY name",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.get("id"), r.get("name"))).collect())
}

/// Отримати список накладних компанії. `status_filter = None` → всі.
pub async fn list(
    pool: &PgPool,
    company_id: Uuid,
    status_filter: Option<WaybillStatus>,
) -> Result<Vec<WaybillListRow>> {
    list_filtered(pool, company_id, status_filter, None, None, None, None, None).await
}

/// Список накладних з фільтром за статусом, текстовим пошуком,
/// контрагентом і діапазоном дат.
pub async fn list_filtered(
    pool: &PgPool,
    company_id: Uuid,
    status_filter: Option<WaybillStatus>,
    direction: Option<DocumentDirection>,
    search_query: Option<&str>,
    counterparty_id: Option<Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
) -> Result<Vec<WaybillListRow>> {
    let search_query = search_query.map(str::trim).filter(|q| !q.is_empty());
    let has_search = search_query.is_some();

    let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        r#"SELECT w.id, w.number, w.direction, w.date,
               c.name AS counterparty_name,
               w.total_amount, w.status
        FROM waybills w
        JOIN counterparties c ON c.id = w.counterparty_id
        WHERE w.company_id = "#,
    );
    qb.push_bind(company_id);

    if let Some(status) = status_filter {
        qb.push(" AND w.status = ");
        qb.push_bind(status);
    }
    if let Some(direction) = direction {
        qb.push(" AND w.direction = ");
        qb.push_bind(direction);
    }
    if let Some(q) = search_query {
        let pattern = super::ilike_pattern(&q);
        qb.push(" AND (w.number ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(r" ESCAPE '\' OR c.name ILIKE ");
        qb.push_bind(pattern);
        qb.push(r" ESCAPE '\')");
    }
    if let Some(cp_id) = counterparty_id {
        qb.push(" AND w.counterparty_id = ");
        qb.push_bind(cp_id);
    }
    if let Some(df) = date_from {
        qb.push(" AND w.date >= ");
        qb.push_bind(df);
    }
    if let Some(dt) = date_to {
        qb.push(" AND w.date <= ");
        qb.push_bind(dt);
    }
    qb.push(" ORDER BY w.date DESC, w.number");
    if has_search {
        qb.push(" LIMIT 100");
    }

    let rows = qb.build_query_as::<WaybillListRow>().fetch_all(pool).await?;
    Ok(rows)
}

/// Отримати одну накладну разом з усіма позиціями.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<(Waybill, Vec<WaybillItem>)>> {
    let waybill = sqlx::query_as::<_, Waybill>(
        r#"
        SELECT id, company_id, number, counterparty_id, contract_id, category_id, direction,
               date, total_amount, status, notes, pdf_path, bas_id, created_at, updated_at
        FROM waybills WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let Some(waybill) = waybill else {
        return Ok(None);
    };

    let items = sqlx::query_as::<_, WaybillItem>(
        r#"
        SELECT id, waybill_id, position, description, unit, quantity, price, amount,
               created_at, updated_at
        FROM waybill_items
        WHERE waybill_id = $1
        ORDER BY position, created_at
        "#,
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(Some((waybill, items)))
}

/// Завантажити накладну з позиціями для форми редагування.
pub async fn get_for_edit(pool: &PgPool, id: Uuid) -> Result<Option<(Waybill, Vec<WaybillItem>)>> {
    get_by_id(pool, id).await
}

/// Створити нову накладну разом з позиціями в одній транзакції.
pub async fn create(pool: &PgPool, company_id: Uuid, data: &NewWaybill) -> Result<Waybill> {
    let mut tx = pool.begin().await?;

    let waybill = sqlx::query_as::<_, Waybill>(
        r#"
        INSERT INTO waybills (company_id, number, counterparty_id, contract_id, category_id,
                              direction, date, notes, bas_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
                  date, total_amount, status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(company_id)
    .bind(&data.number)
    .bind(data.counterparty_id)
    .bind(data.contract_id)
    .bind(data.category_id)
    .bind(&data.direction)
    .bind(data.date)
    .bind(&data.notes)
    .bind(&data.bas_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut total = Decimal::ZERO;

    for item in &data.items {
        let amount = (item.quantity * item.price).round_dp(2);
        total += amount;

        sqlx::query(
            r#"
            INSERT INTO waybill_items (waybill_id, position, description, unit, quantity, price, amount)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(waybill.id)
        .bind(item.position)
        .bind(&item.description)
        .bind(&item.unit)
        .bind(item.quantity)
        .bind(item.price)
        .bind(amount)
        .execute(&mut *tx)
        .await?;
    }

    let waybill = sqlx::query_as::<_, Waybill>(
        r#"
        UPDATE waybills SET total_amount = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
                  date, total_amount, status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(waybill.id)
    .bind(total)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(waybill)
}

/// Оновити накладну разом з позиціями в одній транзакції.
///
/// Паттерн "replace all": DELETE старих позицій → INSERT нових.
pub async fn update_with_items(
    pool: &PgPool,
    id: Uuid,
    data: UpdateWaybill,
    items: Vec<NewWaybillItem>,
) -> Result<Waybill> {
    let mut tx = pool.begin().await?;

    let waybill = sqlx::query_as::<_, Waybill>(
        r#"
        UPDATE waybills
        SET number          = $2,
            counterparty_id = $3,
            contract_id     = $4,
            category_id     = $5,
            date            = $6,
            notes           = $7,
            updated_at      = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
                  date, total_amount, status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&data.number)
    .bind(data.counterparty_id)
    .bind(data.contract_id)
    .bind(data.category_id)
    .bind(data.date)
    .bind(&data.notes)
    .fetch_optional(&mut *tx)
    .await?;

    let waybill = match waybill {
        Some(w) => w,
        None => bail!("Накладну з id={} не знайдено", id),
    };

    sqlx::query("DELETE FROM waybill_items WHERE waybill_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let mut total = Decimal::ZERO;

    for item in &items {
        let amount = (item.quantity * item.price).round_dp(2);
        total += amount;

        sqlx::query(
            r#"
            INSERT INTO waybill_items (waybill_id, position, description, unit, quantity, price, amount)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(waybill.id)
        .bind(item.position)
        .bind(&item.description)
        .bind(&item.unit)
        .bind(item.quantity)
        .bind(item.price)
        .bind(amount)
        .execute(&mut *tx)
        .await?;
    }

    let waybill = sqlx::query_as::<_, Waybill>(
        r#"
        UPDATE waybills SET total_amount = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
                  date, total_amount, status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(waybill.id)
    .bind(total)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(waybill)
}

/// Змінити статус накладної з перевіркою допустимості переходу.
pub async fn change_status(
    pool: &PgPool,
    id: Uuid,
    new_status: WaybillStatus,
) -> Result<Option<Waybill>> {
    let current = sqlx::query_scalar::<_, WaybillStatus>(
        "SELECT status FROM waybills WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let Some(current) = current else {
        return Ok(None);
    };

    if !current.can_transition_to(&new_status) {
        bail!(
            "Недопустимий перехід статусу: '{}' → '{}'. Очікувалось: '{}'",
            current,
            new_status,
            current
                .next()
                .map(|s: WaybillStatus| s.to_string())
                .unwrap_or_else(|| "(фінальний статус)".into())
        );
    }

    let waybill = sqlx::query_as::<_, Waybill>(
        r#"
        UPDATE waybills SET status = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
                  date, total_amount, status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(new_status)
    .fetch_optional(pool)
    .await?;

    Ok(waybill)
}

/// Видалити накладну та всі її позиції (ON DELETE CASCADE у БД).
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM waybills WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Перевести накладну до наступного статусу.
pub async fn advance_status(pool: &PgPool, id: Uuid) -> Result<Option<Waybill>> {
    let current = sqlx::query_scalar::<_, WaybillStatus>(
        "SELECT status FROM waybills WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let Some(current) = current else {
        return Ok(None);
    };

    let Some(next) = current.next() else {
        bail!("Накладна вже в фінальному статусі '{}'", current);
    };

    change_status(pool, id, next).await
}

/// KPI-агрегати для сторінки списку накладних.
pub struct WaybillKpi {
    pub waybills_this_month: i64,
    pub delivered_this_month: Decimal,
    pub unsigned_total: Decimal,
    pub overdue_count: i64,
}

/// Повернути кількість накладних за кожним статусом для компанії.
/// Результат: `[всього, draft, issued, signed, delivered]` (5 елементів).
pub async fn count_by_status(pool: &PgPool, company_id: Uuid) -> Result<Vec<i32>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT status, COUNT(*)::int AS cnt FROM waybills WHERE company_id = $1 GROUP BY status",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    let mut counts = [0i32; 5];
    for row in &rows {
        let status: WaybillStatus = row.get("status");
        let cnt: i32 = row.get("cnt");
        let idx = match status {
            WaybillStatus::Draft     => 1,
            WaybillStatus::Issued    => 2,
            WaybillStatus::Signed    => 3,
            WaybillStatus::Delivered => 4,
        };
        counts[idx] = cnt;
        counts[0] += cnt;
    }
    Ok(counts.to_vec())
}

/// Повернути KPI-агрегати для сторінки списку накладних.
pub async fn get_kpi(pool: &PgPool, company_id: Uuid) -> Result<WaybillKpi> {
    use sqlx::Row;
    use chrono::Datelike;
    let today = chrono::Utc::now().date_naive();
    let first_of_month = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .unwrap_or(today);
    let overdue_threshold = today - chrono::Duration::days(30);

    let row = sqlx::query(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE date >= $2)                                                          AS waybills_this_month,
            COALESCE(SUM(total_amount) FILTER (WHERE status = 'delivered' AND date >= $2), 0::numeric)  AS delivered_this_month,
            COALESCE(SUM(total_amount) FILTER (WHERE status IN ('issued','signed')), 0::numeric)        AS unsigned_total,
            COUNT(*) FILTER (WHERE status IN ('issued','signed') AND date < $3)                         AS overdue_count
        FROM waybills
        WHERE company_id = $1
        "#,
    )
    .bind(company_id)
    .bind(first_of_month)
    .bind(overdue_threshold)
    .fetch_one(pool)
    .await?;

    Ok(WaybillKpi {
        waybills_this_month: row.get::<i64, _>("waybills_this_month"),
        delivered_this_month: row.get::<Decimal, _>("delivered_this_month"),
        unsigned_total:       row.get::<Decimal, _>("unsigned_total"),
        overdue_count:        row.get::<i64, _>("overdue_count"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_waybills_public_api_is_exposed() {
        let _ = generate_next_number;
        let _ = counterparties_for_select;
        let _ = list;
        let _ = list_filtered;
        let _ = get_by_id;
        let _ = create;
        let _ = update_with_items;
        let _ = change_status;
        let _ = get_for_edit;
        let _ = delete;
        let _ = advance_status;
        let _ = count_by_status;
        let _ = get_kpi;
    }
}
