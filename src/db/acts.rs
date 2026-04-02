// CRUD операції для актів виконаних робіт
//
// Транзакційна вставка: create() відкриває транзакцію, вставляє акт + всі позиції,
#![allow(dead_code)]
// перераховує total_amount, і лише тоді робить commit.
// Якщо будь-який крок провалився — транзакція автоматично відкатується при drop().

use anyhow::{Result, bail};
use chrono::Datelike; // .year() для chrono::DateTime
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::act::{Act, ActItem, ActListRow, ActStatus, NewAct, NewActItem, UpdateAct};

/// Згенерувати наступний номер акту у форматі "АКТ-РРРР-NNN".
///
/// Логіка:
///   1. Шукаємо всі акти поточного року в межах компанії.
///   2. Серед номерів що відповідають шаблону "АКТ-РРРР-NNN" — беремо максимальний суфікс.
///   3. Повертаємо суфікс + 1, відформатований з нулями до трьох цифр.
///
/// Нумерація ізольована по компаніях: АКТ-2026-001 у двох різних компаній — це норма.
///
/// Чому MAX(number) не достатньо:
///   Лексикографічний MAX рядків не гарантує числовий максимум ("АКТ-2026-9" > "АКТ-2026-10").
///   Тому парсимо лише числову частину після останнього дефісу.
pub async fn generate_next_number(pool: &PgPool, company_id: Uuid) -> Result<String> {
    use sqlx::Row;
    let year = chrono::Utc::now().year();

    // Отримуємо всі номери актів поточного року для цієї компанії.
    // Runtime-style query — не потребує cargo sqlx prepare.
    let rows = sqlx::query(
        r#"SELECT number FROM acts WHERE company_id = $1 AND EXTRACT(YEAR FROM date)::int = $2"#
    )
    .bind(company_id)
    .bind(year as i32)
    .fetch_all(pool)
    .await?;

    // Парсимо числову частину кожного номеру і знаходимо максимум.
    //
    // Формат номеру: "АКТ-РРРР-NNN"
    // rsplit_once('-') — розбиває рядок по останньому дефісу:
    //   "АКТ-2026-042" → ("АКТ-2026", "042")
    // parse::<u32>()  — перетворює "042" → 42
    let max_seq = rows
        .iter()
        .filter_map(|r| {
            let num: Option<String> = r.try_get("number").ok();
            num.and_then(|n| n.rsplit_once('-').and_then(|(_, s)| s.parse::<u32>().ok()))
        })
        .max()
        .unwrap_or(0); // якщо немає жодного акту — починаємо з 0

    // Форматуємо: рік + порядковий номер з провідними нулями до 3 цифр
    // format!("{:03}", n) → "001", "042", "100" тощо
    Ok(format!("АКТ-{year}-{:03}", max_seq + 1))
}

/// Отримати список активних контрагентів компанії для ComboBox у формі акту.
///
/// Повертає пари (UUID, назва), відсортовані за назвою.
/// Фільтрує лише по активній компанії — контрагенти інших компаній не видно.
///
/// Чому не використовуємо повну структуру Counterparty:
///   Для ComboBox потрібні лише id та name — зайві поля марно вантажили б мережу.
pub async fn counterparties_for_select(pool: &PgPool, company_id: Uuid) -> Result<Vec<(Uuid, String)>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name FROM counterparties WHERE is_archived = FALSE AND company_id = $1 ORDER BY name"
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    // Перетворюємо результат у Vec<(Uuid, String)>
    let result = rows.into_iter().map(|r| (r.get("id"), r.get("name"))).collect();
    Ok(result)
}

/// Отримати список актів компанії з JOIN на назву контрагента.
///
/// `status_filter = None`  → усі акти.
/// `status_filter = Some(s)` → лише акти з вказаним статусом.
pub async fn list(pool: &PgPool, company_id: Uuid, status_filter: Option<ActStatus>) -> Result<Vec<ActListRow>> {
    list_filtered(pool, company_id, status_filter, None).await
}

/// Отримати список актів компанії з фільтром за статусом і текстовим пошуком.
///
/// Всі 4 гілки фільтрують за `company_id` — ізоляція даних між компаніями.
/// Гілки (None,None) та (Some,None) використовують `query_as!` для compile-time перевірки типів.
/// Гілки з текстовим пошуком використовують runtime-style через динамічний ILIKE.
pub async fn list_filtered(
    pool: &PgPool,
    company_id: Uuid,
    status_filter: Option<ActStatus>,
    search_query: Option<&str>,
) -> Result<Vec<ActListRow>> {
    let search_query = search_query.map(str::trim).filter(|q| !q.is_empty());

    let rows = match (status_filter, search_query) {
        (None, None) => {
            // $1 = company_id — runtime-style щоб не потребувати cargo sqlx prepare
            sqlx::query_as::<_, ActListRow>(
                r#"
                SELECT a.id, a.number, a.date,
                       c.name AS counterparty_name,
                       a.total_amount,
                       a.status
                FROM acts a
                JOIN counterparties c ON c.id = a.counterparty_id
                WHERE a.company_id = $1
                ORDER BY a.date DESC, a.number
                "#,
            )
            .bind(company_id)
            .fetch_all(pool)
            .await?
        }
        (Some(status), None) => {
            // $1 = status, $2 = company_id
            sqlx::query_as::<_, ActListRow>(
                r#"
                SELECT a.id, a.number, a.date,
                       c.name AS counterparty_name,
                       a.total_amount,
                       a.status
                FROM acts a
                JOIN counterparties c ON c.id = a.counterparty_id
                WHERE a.status = $1 AND a.company_id = $2
                ORDER BY a.date DESC, a.number
                "#,
            )
            .bind(status)
            .bind(company_id)
            .fetch_all(pool)
            .await?
        }
        (None, Some(q)) => {
            // $1 = pattern, $2 = company_id
            let pattern = format!("%{q}%");
            sqlx::query_as::<_, ActListRow>(
                r#"
                SELECT a.id, a.number, a.date,
                       c.name AS counterparty_name,
                       a.total_amount,
                       a.status
                FROM acts a
                JOIN counterparties c ON c.id = a.counterparty_id
                WHERE (a.number ILIKE $1 OR c.name ILIKE $1)
                  AND a.company_id = $2
                ORDER BY a.date DESC, a.number
                LIMIT 100
                "#,
            )
            .bind(pattern)
            .bind(company_id)
            .fetch_all(pool)
            .await?
        }
        (Some(status), Some(q)) => {
            // $1 = status, $2 = pattern, $3 = company_id
            let pattern = format!("%{q}%");
            sqlx::query_as::<_, ActListRow>(
                r#"
                SELECT a.id, a.number, a.date,
                       c.name AS counterparty_name,
                       a.total_amount,
                       a.status
                FROM acts a
                JOIN counterparties c ON c.id = a.counterparty_id
                WHERE a.status = $1
                  AND (a.number ILIKE $2 OR c.name ILIKE $2)
                  AND a.company_id = $3
                ORDER BY a.date DESC, a.number
                LIMIT 100
                "#,
            )
            .bind(status)
            .bind(pattern)
            .bind(company_id)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows)
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
///
/// `company_id` — прив'язує акт до конкретної компанії в мульти-компанійній системі.
pub async fn create(pool: &PgPool, company_id: Uuid, data: &NewAct) -> Result<Act> {
    let mut tx = pool.begin().await?;

    // 1. Вставляємо заголовок акту (total_amount = 0, оновимо після позицій)
    // Runtime-style query щоб включити company_id без перегенерації sqlx cache.
    // ActStatus має #[derive(sqlx::Type)] — тому sqlx декодує ENUM автоматично.
    let act = sqlx::query_as::<_, Act>(
        r#"INSERT INTO acts (company_id, number, counterparty_id, contract_id, date, notes, bas_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, number, counterparty_id, contract_id, date, total_amount,
                     status, notes, bas_id, created_at, updated_at"#
    )
    .bind(company_id)
    .bind(&data.number)
    .bind(data.counterparty_id)
    .bind(data.contract_id)
    .bind(data.date)
    .bind(&data.notes)
    .bind(&data.bas_id)
    .fetch_one(&mut *tx)
    .await?;

    // 2. Вставляємо позиції та рахуємо суму
    //
    // `&mut *tx` — розіменовуємо Transaction<Postgres> до &mut PgConnection,
    // бо sqlx execute() приймає тільки PgConnection, не Transaction безпосередньо.
    let mut total = Decimal::ZERO;

    for item in &data.items {
        // Обчислюємо суму тут, а не в SQL — щоб мати контроль над заокругленням.
        let amount = (item.quantity * item.unit_price).round_dp(2);
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

    // 3. Оновлюємо total_amount в акті (runtime-style для узгодженості з INSERT вище)
    let act = sqlx::query_as::<_, Act>(
        r#"UPDATE acts SET total_amount = $2, updated_at = NOW()
           WHERE id = $1
           RETURNING id, number, counterparty_id, contract_id, date, total_amount,
                     status, notes, bas_id, created_at, updated_at"#
    )
    .bind(act.id)
    .bind(total)
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
            row.status
                .next()
                .map(|s: ActStatus| s.to_string())
                .unwrap_or_else(|| "(фінальний статус)".into())
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

/// Завантажити акт з позиціями для форми редагування.
/// Делегує до `get_by_id` — логіка ідентична.
pub async fn get_for_edit(pool: &PgPool, id: Uuid) -> Result<Option<(Act, Vec<ActItem>)>> {
    get_by_id(pool, id).await
}

/// Оновити акт разом з позиціями в одній транзакції.
///
/// Стара логіка "редагування без позицій" (функція `update`) залишається
/// для зворотної сумісності. Ця функція — повна заміна позицій:
///   1. UPDATE заголовку акту
///   2. DELETE усіх старих позицій
///   3. INSERT нових позицій
///   4. UPDATE total_amount = сума нових позицій
pub async fn update_with_items(
    pool: &PgPool,
    id: Uuid,
    data: UpdateAct,
    items: Vec<NewActItem>,
) -> Result<Act> {
    let mut tx = pool.begin().await?;

    // 1. Оновлюємо заголовок акту
    let act = sqlx::query_as!(
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
    .fetch_optional(&mut *tx)
    .await?;

    // Якщо акт не знайдено — повертаємо помилку (None неможливий для update)
    let act = match act {
        Some(a) => a,
        None => anyhow::bail!("Акт з id={} не знайдено", id),
    };

    // 2. Видаляємо всі старі позиції
    sqlx::query!("DELETE FROM act_items WHERE act_id = $1", id)
        .execute(&mut *tx)
        .await?;

    // 3. Вставляємо нові позиції, рахуємо суму
    let mut total = Decimal::ZERO;

    for item in &items {
        let amount = (item.quantity * item.unit_price).round_dp(2);
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

    // 4. Оновлюємо total_amount
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_acts_public_api_is_exposed() {
        // Перевіряємо що всі публічні функції доступні та компілюються
        let _ = generate_next_number as fn(&PgPool, Uuid) -> _;
        let _ = counterparties_for_select as fn(&PgPool, Uuid) -> _;
        let _ = list as fn(&PgPool, Uuid, Option<ActStatus>) -> _;
        let _ = list_filtered as fn(&PgPool, Uuid, Option<ActStatus>, Option<&str>) -> _;
        let _ = get_by_id;
        let _ = create as fn(&PgPool, Uuid, &NewAct) -> _;
        let _ = update;
        let _ = change_status;
        let _ = get_for_edit;
        let _ = update_with_items;
        let _ = advance_status;
    }
}
