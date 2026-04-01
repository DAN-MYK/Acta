// CRUD операції для актів виконаних робіт
//
// Транзакційна вставка: create() відкриває транзакцію, вставляє акт + всі позиції,
// перераховує total_amount, і лише тоді робить commit.
// Якщо будь-який крок провалився — транзакція автоматично відкатується при drop().

use anyhow::{bail, Result};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::act::{Act, ActItem, ActListRow, ActStatus, NewAct, UpdateAct};


/// Отримати список актів з JOIN на назву контрагента.
///
/// `status_filter = None`  → усі акти.
/// `status_filter = Some(s)` → лише акти з вказаним статусом.
///
/// Два окремих `query_as!` замість динамічного SQL —
/// так зберігається перевірка типів під час компіляції.
pub async fn list(pool: &PgPool, status_filter: Option<ActStatus>) -> Result<Vec<ActListRow>> {
    match status_filter {
        None => {
            let rows = sqlx::query_as!(
                ActListRow,
                r#"
                SELECT a.id, a.number, a.date,
                       c.name AS counterparty_name,
                       a.total_amount,
                       a.status AS "status: ActStatus"
                FROM acts a
                JOIN counterparties c ON c.id = a.counterparty_id
                ORDER BY a.date DESC, a.number
                "#
            )
            .fetch_all(pool)
            .await?;
            Ok(rows)
        }
        Some(status) => {
            let rows = sqlx::query_as!(
                ActListRow,
                r#"
                SELECT a.id, a.number, a.date,
                       c.name AS counterparty_name,
                       a.total_amount,
                       a.status AS "status: ActStatus"
                FROM acts a
                JOIN counterparties c ON c.id = a.counterparty_id
                WHERE a.status = $1
                ORDER BY a.date DESC, a.number
                "#,
                status as ActStatus
            )
            .fetch_all(pool)
            .await?;
            Ok(rows)
        }
    }
}

/// Отримати один акт разом з усіма його позиціями.
/// Повертає `None` якщо акт не знайдено.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<(Act, Vec<ActItem>)>> {
    let act = sqlx::query_as!(
        Act,
        r#"
        SELECT id, number, counterparty_id, contract_id, date, total_amount,
               status AS "status: ActStatus", notes, bas_id, created_at, updated_at
        FROM acts
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    let Some(act) = act else {
        return Ok(None);
    };

    // Окремий запит на позиції — sqlx не підтримує JOIN з масивом у query_as!
    let items = sqlx::query_as!(
        ActItem,
        r#"
        SELECT id, act_id, description, quantity, unit, unit_price, amount,
               created_at, updated_at
        FROM act_items
        WHERE act_id = $1
        ORDER BY created_at
        "#,
        id
    )
    .fetch_all(pool)
    .await?;

    Ok(Some((act, items)))
}

/// Створити новий акт разом з позиціями в одній транзакції.
///
/// Транзакція потрібна щоб акт без позицій або позиції без акту
/// ніколи не потрапляли до БД навіть при збої на середині.
///
/// `pool.begin()` → транзакція, `tx.commit()` → фіксуємо.
/// Якщо `tx` виходить зі scope без commit — автоматичний rollback.
pub async fn create(pool: &PgPool, data: &NewAct) -> Result<Act> {
    let mut tx = pool.begin().await?;

    // 1. Вставляємо заголовок акту (total_amount = 0, оновимо після позицій)
    let act = sqlx::query_as!(
        Act,
        r#"
        INSERT INTO acts (number, counterparty_id, contract_id, date, notes, bas_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, number, counterparty_id, contract_id, date, total_amount,
                  status AS "status: ActStatus", notes, bas_id, created_at, updated_at
        "#,
        data.number,
        data.counterparty_id,
        data.contract_id,
        data.date,
        data.notes,
        data.bas_id,
    )
    .fetch_one(&mut *tx)
    .await?;

    // 2. Вставляємо позиції та рахуємо суму
    //
    // `&mut *tx` — розіменовуємо Transaction<Postgres> до &mut PgConnection,
    // бо sqlx execute() приймає тільки PgConnection, не Transaction безпосередньо.
    let mut total = Decimal::ZERO;

    for item in &data.items {
        // Обчислюємо суму тут, а не в SQL — щоб мати контроль над заокругленням.
        let amount = item.quantity * item.unit_price;
        total += amount;

        sqlx::query!(
            r#"
            INSERT INTO act_items (act_id, description, quantity, unit, unit_price, amount)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            act.id,
            item.description,
            item.quantity,
            item.unit,
            item.unit_price,
            amount,
        )
        .execute(&mut *tx)
        .await?;
    }

    // 3. Оновлюємо total_amount в акті
    let act = sqlx::query_as!(
        Act,
        r#"
        UPDATE acts SET total_amount = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, number, counterparty_id, contract_id, date, total_amount,
                  status AS "status: ActStatus", notes, bas_id, created_at, updated_at
        "#,
        act.id,
        total,
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(act)
}

/// Оновити заголовок акту (без позицій — MVP).
/// Повертає `None` якщо акт не знайдено.
pub async fn update(pool: &PgPool, id: Uuid, data: &UpdateAct) -> Result<Option<Act>> {
    let row = sqlx::query_as!(
        Act,
        r#"
        UPDATE acts
        SET number          = $2,
            counterparty_id = $3,
            contract_id     = $4,
            date            = $5,
            notes           = $6,
            updated_at      = NOW()
        WHERE id = $1
        RETURNING id, number, counterparty_id, contract_id, date, total_amount,
                  status AS "status: ActStatus", notes, bas_id, created_at, updated_at
        "#,
        id,
        data.number,
        data.counterparty_id,
        data.contract_id,
        data.date,
        data.notes,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Змінити статус акту з перевіркою допустимості переходу.
///
/// Дозволені переходи: Draft → Issued → Signed → Paid (лише вперед).
/// Повертає помилку при спробі перескочити статус або піти назад.
pub async fn change_status(pool: &PgPool, id: Uuid, new_status: ActStatus) -> Result<Option<Act>> {
    // Читаємо поточний статус — потрібен для валідації переходу
    let current = sqlx::query!(
        r#"SELECT status AS "status: ActStatus" FROM acts WHERE id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = current else {
        return Ok(None);
    };

    // Перевіряємо що новий статус — наступний дозволений
    if row.status.next().as_ref() != Some(&new_status) {
        bail!(
            "Недопустимий перехід статусу: '{}' → '{}'. Очікувалось: '{}'",
            row.status,
            new_status,
            row.status.next().map(|s: ActStatus| s.to_string()).unwrap_or_else(|| "(фінальний статус)".into())
        );
    }

    let act = sqlx::query_as!(
        Act,
        r#"
        UPDATE acts SET status = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, number, counterparty_id, contract_id, date, total_amount,
                  status AS "status: ActStatus", notes, bas_id, created_at, updated_at
        "#,
        id,
        new_status as ActStatus,
    )
    .fetch_optional(pool)
    .await?;

    Ok(act)
}

/// Перевести акт до наступного статусу (зручна обгортка над `change_status`).
/// Повертає помилку якщо акт вже у фінальному статусі "Оплачено".
pub async fn advance_status(pool: &PgPool, id: Uuid) -> Result<Option<Act>> {
    let current = sqlx::query!(
        r#"SELECT status AS "status: ActStatus" FROM acts WHERE id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = current else {
        return Ok(None);
    };

    let Some(next) = row.status.next() else {
        bail!("Акт вже в фінальному статусі '{}'", row.status);
    };

    change_status(pool, id, next).await
}
