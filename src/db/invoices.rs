// CRUD операції для видаткових накладних
//
// Всі запити — runtime-style (sqlx::query_as::<_, T>()) без макросів,
// щоб не потребувати cargo sqlx prepare при зміні схеми.
//
// Транзакційна вставка: create() та update_with_items() відкривають транзакцію,
// вставляють заголовок + позиції, перераховують total_amount, потім commit.
#![allow(dead_code)]

use anyhow::{Result, bail};
use chrono::Datelike;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::invoice::{
    Invoice, InvoiceItem, InvoiceListRow, InvoiceStatus, NewInvoice, NewInvoiceItem, UpdateInvoice,
};

/// Згенерувати наступний номер накладної у форматі "НАК-РРРР-NNN".
///
/// Нумерація ізольована по компаніях і по роках.
/// Парсимо числову частину після останнього дефісу — щоб уникнути
/// лексикографічного MAX ("НАК-2026-9" > "НАК-2026-10" — хибний результат).
pub async fn generate_next_number(pool: &PgPool, company_id: Uuid) -> Result<String> {
    use sqlx::Row;
    let year = chrono::Utc::now().year();

    let rows = sqlx::query(
        "SELECT number FROM invoices WHERE company_id = $1 AND EXTRACT(YEAR FROM date)::int = $2",
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
    status_filter: Option<InvoiceStatus>,
) -> Result<Vec<InvoiceListRow>> {
    list_filtered(pool, company_id, status_filter, None, None, None, None).await
}

/// Список накладних з фільтром за статусом, текстовим пошуком,
/// контрагентом і діапазоном дат.
///
/// Використовує `QueryBuilder` для динамічної побудови WHERE-умов
/// аналогічно acts::list_filtered.
pub async fn list_filtered(
    pool: &PgPool,
    company_id: Uuid,
    status_filter: Option<InvoiceStatus>,
    search_query: Option<&str>,
    counterparty_id: Option<Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
) -> Result<Vec<InvoiceListRow>> {
    let search_query = search_query.map(str::trim).filter(|q| !q.is_empty());
    let has_search = search_query.is_some();

    let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        r#"SELECT i.id, i.number, i.date,
               c.name AS counterparty_name,
               i.total_amount, i.status
        FROM invoices i
        JOIN counterparties c ON c.id = i.counterparty_id
        WHERE i.company_id = "#,
    );
    qb.push_bind(company_id);

    if let Some(status) = status_filter {
        qb.push(" AND i.status = ");
        qb.push_bind(status);
    }
    if let Some(q) = search_query {
        let pattern = format!("%{q}%");
        qb.push(" AND (i.number ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR c.name ILIKE ");
        qb.push_bind(pattern);
        qb.push(")");
    }
    if let Some(cp_id) = counterparty_id {
        qb.push(" AND i.counterparty_id = ");
        qb.push_bind(cp_id);
    }
    if let Some(df) = date_from {
        qb.push(" AND i.date >= ");
        qb.push_bind(df);
    }
    if let Some(dt) = date_to {
        qb.push(" AND i.date <= ");
        qb.push_bind(dt);
    }
    qb.push(" ORDER BY i.date DESC, i.number");
    if has_search {
        qb.push(" LIMIT 100");
    }

    let rows = qb.build_query_as::<InvoiceListRow>().fetch_all(pool).await?;
    Ok(rows)
}

/// Отримати одну накладну разом з усіма позиціями.
/// Повертає `None` якщо накладну не знайдено.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<(Invoice, Vec<InvoiceItem>)>> {
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        SELECT id, company_id, number, counterparty_id, contract_id, date,
               total_amount, vat_amount, status, notes, pdf_path, bas_id,
               created_at, updated_at
        FROM invoices WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let Some(invoice) = invoice else {
        return Ok(None);
    };

    let items = sqlx::query_as::<_, InvoiceItem>(
        r#"
        SELECT id, invoice_id, position, description, unit, quantity, price, amount,
               created_at, updated_at
        FROM invoice_items
        WHERE invoice_id = $1
        ORDER BY position, created_at
        "#,
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(Some((invoice, items)))
}

/// Завантажити накладну з позиціями для форми редагування.
pub async fn get_for_edit(pool: &PgPool, id: Uuid) -> Result<Option<(Invoice, Vec<InvoiceItem>)>> {
    get_by_id(pool, id).await
}

/// Створити нову накладну разом з позиціями в одній транзакції.
///
/// `vat_amount` = 0 для ФОП без ПДВ (можна розширити логікою у майбутньому).
/// `total_amount` обчислюється як сума (quantity × price) всіх позицій.
pub async fn create(pool: &PgPool, company_id: Uuid, data: &NewInvoice) -> Result<Invoice> {
    let mut tx = pool.begin().await?;

    // 1. Вставляємо заголовок (total_amount = 0, оновимо після позицій)
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        INSERT INTO invoices (company_id, number, counterparty_id, contract_id, category_id,
                              date, expected_payment_date, notes, bas_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id,
                  date, expected_payment_date, total_amount, vat_amount,
                  status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
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
    let mut total = Decimal::ZERO;

    for item in &data.items {
        let amount = (item.quantity * item.price).round_dp(2);
        total += amount;

        sqlx::query(
            r#"
            INSERT INTO invoice_items (invoice_id, position, description, unit, quantity, price, amount)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(invoice.id)
        .bind(item.position)
        .bind(&item.description)
        .bind(&item.unit)
        .bind(item.quantity)
        .bind(item.price)
        .bind(amount)
        .execute(&mut *tx)
        .await?;
    }

    // 3. Оновлюємо total_amount
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices SET total_amount = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id,
                  date, expected_payment_date, total_amount, vat_amount,
                  status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(invoice.id)
    .bind(total)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(invoice)
}

/// Оновити накладну разом з позиціями в одній транзакції.
///
/// Паттерн "replace all": DELETE старих позицій → INSERT нових.
/// Простіше ніж diff, достатньо для документів управлінського обліку.
pub async fn update_with_items(
    pool: &PgPool,
    id: Uuid,
    data: UpdateInvoice,
    items: Vec<NewInvoiceItem>,
) -> Result<Invoice> {
    let mut tx = pool.begin().await?;

    // 1. Оновлюємо заголовок
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices
        SET number                = $2,
            counterparty_id       = $3,
            contract_id           = $4,
            category_id           = $5,
            date                  = $6,
            expected_payment_date = $7,
            notes                 = $8,
            updated_at            = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id,
                  date, expected_payment_date, total_amount, vat_amount,
                  status, notes, pdf_path, bas_id, created_at, updated_at
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

    let invoice = match invoice {
        Some(i) => i,
        None => bail!("Накладна з id={} не знайдена", id),
    };

    // 2. Видаляємо всі старі позиції
    sqlx::query("DELETE FROM invoice_items WHERE invoice_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // 3. Вставляємо нові позиції
    let mut total = Decimal::ZERO;

    for item in &items {
        let amount = (item.quantity * item.price).round_dp(2);
        total += amount;

        sqlx::query(
            r#"
            INSERT INTO invoice_items (invoice_id, position, description, unit, quantity, price, amount)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(invoice.id)
        .bind(item.position)
        .bind(&item.description)
        .bind(&item.unit)
        .bind(item.quantity)
        .bind(item.price)
        .bind(amount)
        .execute(&mut *tx)
        .await?;
    }

    // 4. Оновлюємо total_amount
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices SET total_amount = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id,
                  date, expected_payment_date, total_amount, vat_amount,
                  status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(invoice.id)
    .bind(total)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(invoice)
}

/// Змінити статус накладної з перевіркою допустимості переходу.
///
/// Дозволені переходи: Draft → Issued → Signed → Paid (лише вперед).
pub async fn change_status(
    pool: &PgPool,
    id: Uuid,
    new_status: InvoiceStatus,
) -> Result<Option<Invoice>> {
    use sqlx::Row;

    let row = sqlx::query("SELECT status FROM invoices WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    // Декодуємо статус вручну — runtime query не підтримує `AS "field: Type"` синтаксис
    let current_str: String = row.try_get("status")?;
    let current = parse_status(&current_str)?;

    if current.next().as_ref() != Some(&new_status) {
        bail!(
            "Недопустимий перехід статусу: '{}' → '{}'. Очікувалось: '{}'",
            current,
            new_status,
            current
                .next()
                .map(|s: InvoiceStatus| s.to_string())
                .unwrap_or_else(|| "(фінальний статус)".into())
        );
    }

    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices SET status = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id,
                  date, expected_payment_date, total_amount, vat_amount,
                  status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(new_status)
    .fetch_optional(pool)
    .await?;

    Ok(invoice)
}

/// Перевести накладну до наступного статусу (зручна обгортка над `change_status`).
pub async fn advance_status(pool: &PgPool, id: Uuid) -> Result<Option<Invoice>> {
    use sqlx::Row;

    let row = sqlx::query("SELECT status FROM invoices WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let current_str: String = row.try_get("status")?;
    let current = parse_status(&current_str)?;

    let Some(next) = current.next() else {
        bail!("Накладна вже в фінальному статусі '{}'", current);
    };

    change_status(pool, id, next).await
}

/// Допоміжна функція: парсинг рядка статусу з БД в InvoiceStatus.
fn parse_status(s: &str) -> Result<InvoiceStatus> {
    match s {
        "draft" => Ok(InvoiceStatus::Draft),
        "issued" => Ok(InvoiceStatus::Issued),
        "signed" => Ok(InvoiceStatus::Signed),
        "paid" => Ok(InvoiceStatus::Paid),
        other => bail!("Невідомий статус накладної: '{}'", other),
    }
}

/// Видалити накладну та всі її позиції (ON DELETE CASCADE у БД).
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM invoices WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_invoices_public_api_is_exposed() {
        // Перевіряємо лише що символи існують і компілюються
        let _ = generate_next_number;
        let _ = counterparties_for_select;
        let _ = list;
        let _ = list_filtered;
        let _ = get_by_id;
        let _ = create;
        let _ = update_with_items;
        let _ = change_status;
        let _ = get_for_edit;
        let _ = advance_status;
    }

    #[test]
    fn parse_status_roundtrip() {
        assert!(matches!(parse_status("draft"), Ok(InvoiceStatus::Draft)));
        assert!(matches!(parse_status("issued"), Ok(InvoiceStatus::Issued)));
        assert!(matches!(parse_status("signed"), Ok(InvoiceStatus::Signed)));
        assert!(matches!(parse_status("paid"), Ok(InvoiceStatus::Paid)));
        assert!(parse_status("unknown").is_err());
    }
}
