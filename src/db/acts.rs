// CRUD операції для актів виконаних робіт
//
// Транзакційна вставка: create() відкриває транзакцію, вставляє акт + всі позиції,
#![allow(dead_code)]
// перераховує total_amount, і лише тоді робить commit.
// Якщо будь-який крок провалився — транзакція автоматично відкатується при drop().

use anyhow::{Result, bail};
use chrono::{Datelike, Duration}; // .year(), Duration для дат
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

/// KPI-агрегати для сторінки списку актів.
pub struct ActKpi {
    /// Кількість актів створених у поточному місяці.
    pub acts_this_month: i64,
    /// Сума оплачених актів поточного місяця.
    pub revenue_this_month: Decimal,
    /// Загальна сума виставлених і підписаних (ще не оплачених) актів.
    pub unpaid_total: Decimal,
    /// Кількість актів у статусі "виставлено"/"підписано" старших за 30 днів.
    pub overdue_count: i64,
}

use crate::models::act::{Act, ActItem, ActListRow, ActStatus, NewAct, NewActItem, UpdateAct};

fn count_index_for_status(status: &ActStatus) -> usize {
    match status {
        ActStatus::Draft => 1,
        ActStatus::Issued => 2,
        ActStatus::Signed => 3,
        ActStatus::Paid => 4,
    }
}

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
    list_filtered(pool, company_id, status_filter, None, None, None, None).await
}

/// Отримати список актів компанії з фільтром за статусом, текстовим пошуком,
/// контрагентом і діапазоном дат.
///
/// Використовує `QueryBuilder` для динамічної побудови WHERE-умов
/// замість 4-гілкового match — дозволяє довільну комбінацію фільтрів.
pub async fn list_filtered(
    pool: &PgPool,
    company_id: Uuid,
    status_filter: Option<ActStatus>,
    search_query: Option<&str>,
    counterparty_id: Option<Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
) -> Result<Vec<ActListRow>> {
    let search_query = search_query.map(str::trim).filter(|q| !q.is_empty());
    let has_search = search_query.is_some();

    let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        r#"SELECT a.id, a.number, a.date,
               c.name AS counterparty_name,
               a.total_amount, a.status
        FROM acts a
        JOIN counterparties c ON c.id = a.counterparty_id
        WHERE a.company_id = "#,
    );
    qb.push_bind(company_id);

    if let Some(status) = status_filter {
        qb.push(" AND a.status = ");
        qb.push_bind(status);
    }
    if let Some(q) = search_query {
        let pattern = format!("%{q}%");
        qb.push(" AND (a.number ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR c.name ILIKE ");
        qb.push_bind(pattern);
        qb.push(")");
    }
    if let Some(cp_id) = counterparty_id {
        qb.push(" AND a.counterparty_id = ");
        qb.push_bind(cp_id);
    }
    if let Some(df) = date_from {
        qb.push(" AND a.date >= ");
        qb.push_bind(df);
    }
    if let Some(dt) = date_to {
        qb.push(" AND a.date <= ");
        qb.push_bind(dt);
    }
    qb.push(" ORDER BY a.date DESC, a.number");
    if has_search {
        qb.push(" LIMIT 100");
    }

    let rows = qb.build_query_as::<ActListRow>().fetch_all(pool).await?;
    Ok(rows)
}

/// Повернути кількість актів за кожним статусом для компанії.
///
/// Результат: `[всього, draft, issued, signed, paid]` (5 елементів).
/// Використовується для відображення лічильників на вкладках списку актів.
pub async fn count_by_status(pool: &PgPool, company_id: Uuid) -> Result<Vec<i32>> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT status, COUNT(*)::int AS cnt FROM acts WHERE company_id = $1 GROUP BY status",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    let mut counts = [0i32; 5];
    for row in &rows {
        let status: ActStatus = row.get("status");
        let cnt: i32 = row.get("cnt");
        let idx = count_index_for_status(&status);
        counts[idx] = cnt;
        counts[0] += cnt; // індекс 0 = "Всі" = сума
    }

    Ok(counts.to_vec())
}

/// Повернути KPI-агрегати для сторінки списку актів.
///
/// Усі 4 метрики обчислюються одним SQL запитом через FILTER:
///   - `acts_this_month`    — COUNT актів з датою >= перше число поточного місяця
///   - `revenue_this_month` — SUM(total_amount) WHERE status='paid' AND date >= перше місяця
///   - `unpaid_total`       — SUM(total_amount) WHERE status IN ('issued','signed')
///   - `overdue_count`      — COUNT WHERE status IN ('issued','signed') AND date < today-30d
pub async fn get_kpi(pool: &PgPool, company_id: Uuid) -> Result<ActKpi> {
    use sqlx::Row;

    let today = chrono::Utc::now().date_naive();
    let first_of_month = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .unwrap_or(today);
    let overdue_threshold = today - Duration::days(30);

    let row = sqlx::query(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE date >= $2)                                                       AS acts_this_month,
            COALESCE(SUM(total_amount) FILTER (WHERE status = 'paid' AND date >= $2), 0::numeric)    AS revenue_this_month,
            COALESCE(SUM(total_amount) FILTER (WHERE status IN ('issued','signed')), 0::numeric)     AS unpaid_total,
            COUNT(*) FILTER (WHERE status IN ('issued','signed') AND date < $3)                      AS overdue_count
        FROM acts
        WHERE company_id = $1
        "#,
    )
    .bind(company_id)
    .bind(first_of_month)
    .bind(overdue_threshold)
    .fetch_one(pool)
    .await?;

    Ok(ActKpi {
        acts_this_month:    row.get::<i64, _>("acts_this_month"),
        revenue_this_month: row.get::<Decimal, _>("revenue_this_month"),
        unpaid_total:       row.get::<Decimal, _>("unpaid_total"),
        overdue_count:      row.get::<i64, _>("overdue_count"),
    })
}

/// Отримати один акт разом з усіма його позиціями.
/// Повертає `None` якщо акт не знайдено.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<(Act, Vec<ActItem>)>> {
    let act = sqlx::query_as::<_, Act>(
        r#"
        SELECT id, number, counterparty_id, contract_id, category_id,
               date, expected_payment_date, total_amount,
               status, notes, bas_id, created_at, updated_at
        FROM acts
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let Some(act) = act else {
        return Ok(None);
    };

    // Окремий запит на позиції — sqlx не підтримує JOIN з масивом у query_as!
    let items = sqlx::query_as::<_, ActItem>(
        r#"
        SELECT id, act_id, description, quantity, unit, unit_price, amount,
               created_at, updated_at
        FROM act_items
        WHERE act_id = $1
        ORDER BY created_at
        "#,
    )
    .bind(id)
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
        r#"INSERT INTO acts (company_id, number, counterparty_id, contract_id, category_id,
                             date, expected_payment_date, notes, bas_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING id, number, counterparty_id, contract_id, category_id,
                     date, expected_payment_date, total_amount,
                     status, notes, bas_id, created_at, updated_at"#
    )
    .bind(company_id)
    .bind(&data.number)
    .bind(data.counterparty_id)
    .bind(data.contract_id)
    .bind(data.category_id)
    .bind(data.date)
    .bind(data.expected_payment_date)
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
           RETURNING id, number, counterparty_id, contract_id, category_id,
                     date, expected_payment_date, total_amount,
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
    let row = sqlx::query_as::<_, Act>(
        r#"
        UPDATE acts
        SET number                = $2,
            counterparty_id       = $3,
            contract_id           = $4,
            category_id           = $5,
            date                  = $6,
            expected_payment_date = $7,
            notes                 = $8,
            updated_at            = NOW()
        WHERE id = $1
        RETURNING id, number, counterparty_id, contract_id, category_id,
                  date, expected_payment_date, total_amount,
                  status, notes, bas_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&data.number)
    .bind(data.counterparty_id)
    .bind(data.contract_id)
    .bind(data.category_id)
    .bind(data.date)
    .bind(data.expected_payment_date)
    .bind(&data.notes)
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

    let act = sqlx::query_as::<_, Act>(
        r#"
        UPDATE acts SET status = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, number, counterparty_id, contract_id, category_id,
                  date, expected_payment_date, total_amount,
                  status, notes, bas_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(new_status)
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
    let act = sqlx::query_as::<_, Act>(
        r#"
        UPDATE acts
        SET number                 = $2,
            counterparty_id        = $3,
            contract_id            = $4,
            category_id            = $5,
            date                   = $6,
            expected_payment_date  = $7,
            notes                  = $8,
            updated_at             = NOW()
        WHERE id = $1
        RETURNING id, number, counterparty_id, contract_id, category_id,
                  date, expected_payment_date, total_amount,
                  status, notes, bas_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&data.number)
    .bind(data.counterparty_id)
    .bind(data.contract_id)
    .bind(data.category_id)
    .bind(data.date)
    .bind(data.expected_payment_date)
    .bind(&data.notes)
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
    let act = sqlx::query_as::<_, Act>(
        r#"
        UPDATE acts SET total_amount = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, number, counterparty_id, contract_id, category_id,
                  date, expected_payment_date, total_amount,
                  status, notes, bas_id, created_at, updated_at
        "#,
    )
    .bind(act.id)
    .bind(total)
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

/// Видалити акт та всі його позиції (ON DELETE CASCADE у БД).
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_index_for_status_matches_tabs_order() {
        assert_eq!(count_index_for_status(&ActStatus::Draft), 1);
        assert_eq!(count_index_for_status(&ActStatus::Issued), 2);
        assert_eq!(count_index_for_status(&ActStatus::Signed), 3);
        assert_eq!(count_index_for_status(&ActStatus::Paid), 4);
    }

    #[test]
    fn db_acts_public_api_is_exposed() {
        // Перевіряємо що всі публічні функції доступні та компілюються
        let _ = generate_next_number;
        let _ = counterparties_for_select;
        let _ = list;
        let _ = list_filtered;
        let _ = get_by_id;
        let _ = create;
        let _ = update;
        let _ = change_status;
        let _ = get_for_edit;
        let _ = update_with_items;
        let _ = advance_status;
    }
}
