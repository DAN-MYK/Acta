// CRUD операції для видаткових накладних
//
// Всі запити — runtime-style (sqlx::query_as::<_, T>()) без макросів,
// щоб не потребувати cargo sqlx prepare при зміні схеми.
//
// Транзакційна вставка: create() та update_with_items() відкривають транзакцію,
// вставляють заголовок + позиції, перераховують total_amount, потім commit.

use anyhow::{bail, Result};
use chrono::Datelike;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    invoice::{
        Invoice, InvoiceItem, InvoiceListRow, InvoiceStatus, NewInvoice, NewInvoiceItem,
        UpdateInvoice,
    },
    DocumentDirection,
};

/// Згенерувати наступний номер рахунку у форматі "РАХ-РРРР-NNN".
///
/// Нумерація ізольована по компаніях і по роках.
/// Парсимо числову частину після останнього дефісу — щоб уникнути
/// лексикографічного MAX ("РАХ-2026-9" > "РАХ-2026-10" — хибний результат).
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

    Ok(format!("РАХ-{year}-{:03}", max_seq + 1))
}

/// Отримати активних контрагентів компанії для ComboBox у формі накладної.
pub async fn counterparties_for_select(
    pool: &PgPool,
    company_id: Uuid,
) -> Result<Vec<(Uuid, String)>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name FROM counterparties WHERE is_archived = FALSE AND company_id = $1 ORDER BY name",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| (r.get("id"), r.get("name")))
        .collect())
}

/// Отримати список накладних компанії. `status_filter = None` → всі.
pub async fn list(
    pool: &PgPool,
    company_id: Uuid,
    status_filter: Option<InvoiceStatus>,
) -> Result<Vec<InvoiceListRow>> {
    list_filtered(
        pool,
        company_id,
        status_filter,
        None,
        None,
        None,
        None,
        None,
    )
    .await
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
    direction: Option<DocumentDirection>,
    search_query: Option<&str>,
    counterparty_id: Option<Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
) -> Result<Vec<InvoiceListRow>> {
    let search_query = search_query.map(str::trim).filter(|q| !q.is_empty());
    let has_search = search_query.is_some();

    let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        r#"SELECT i.id, i.number, i.direction, i.date,
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
    if let Some(direction) = direction {
        qb.push(" AND i.direction = ");
        qb.push_bind(direction);
    }
    if let Some(q) = search_query {
        let pattern = super::ilike_pattern(&q);
        qb.push(" AND (i.number ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(r" ESCAPE '\' OR c.name ILIKE ");
        qb.push_bind(pattern);
        qb.push(r" ESCAPE '\')");
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

    let rows = qb
        .build_query_as::<InvoiceListRow>()
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Отримати одну накладну разом з усіма позиціями.
/// Повертає `None` якщо накладну не знайдено.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<(Invoice, Vec<InvoiceItem>)>> {
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        SELECT id, company_id, number, counterparty_id, contract_id, category_id, direction,
               date, expected_payment_date, total_amount, vat_amount, status, notes, pdf_path, bas_id,
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

/// Знайти накладну за bas_id для ідемпотентного імпорту з BAS.
pub async fn find_by_bas_id(pool: &PgPool, bas_id: &str) -> Result<Option<Invoice>> {
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        SELECT id, company_id, number, counterparty_id, contract_id, category_id, direction,
               date, expected_payment_date, total_amount, vat_amount,
               status, notes, pdf_path, bas_id, created_at, updated_at
        FROM invoices
        WHERE bas_id = $1
        "#,
    )
    .bind(bas_id)
    .fetch_optional(pool)
    .await?;

    Ok(invoice)
}

/// Створити header-level накладну з BAS без позицій.
///
/// Це partial importer: зберігаємо заголовок документа і суми, а line items
/// буде імпортовано окремим наступним кроком.
#[allow(clippy::too_many_arguments)]
pub async fn create_imported_header(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    contract_id: Option<Uuid>,
    number: &str,
    direction: DocumentDirection,
    date: chrono::NaiveDate,
    expected_payment_date: Option<chrono::NaiveDate>,
    total_amount: Decimal,
    vat_amount: Decimal,
    status: InvoiceStatus,
    notes: Option<&str>,
    bas_id: Option<&str>,
) -> Result<Invoice> {
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        INSERT INTO invoices (
            company_id, counterparty_id, contract_id, number, direction,
            date, expected_payment_date, total_amount, vat_amount,
            status, notes, bas_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
                  date, expected_payment_date, total_amount, vat_amount,
                  status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .bind(contract_id)
    .bind(number)
    .bind(direction)
    .bind(date)
    .bind(expected_payment_date)
    .bind(total_amount)
    .bind(vat_amount)
    .bind(status)
    .bind(notes)
    .bind(bas_id)
    .fetch_one(pool)
    .await?;

    Ok(invoice)
}

/// Оновити header-level накладну з BAS без заміни позицій.
#[allow(clippy::too_many_arguments)]
pub async fn update_imported_header(
    pool: &PgPool,
    id: Uuid,
    counterparty_id: Uuid,
    contract_id: Option<Uuid>,
    number: &str,
    direction: DocumentDirection,
    date: chrono::NaiveDate,
    expected_payment_date: Option<chrono::NaiveDate>,
    total_amount: Decimal,
    vat_amount: Decimal,
    status: InvoiceStatus,
    notes: Option<&str>,
) -> Result<Invoice> {
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices
        SET counterparty_id       = $2,
            contract_id           = $3,
            number                = $4,
            direction             = $5,
            date                  = $6,
            expected_payment_date = $7,
            total_amount          = $8,
            vat_amount            = $9,
            status                = $10,
            notes                 = $11,
            updated_at            = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
                  date, expected_payment_date, total_amount, vat_amount,
                  status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(counterparty_id)
    .bind(contract_id)
    .bind(number)
    .bind(direction)
    .bind(date)
    .bind(expected_payment_date)
    .bind(total_amount)
    .bind(vat_amount)
    .bind(status)
    .bind(notes)
    .fetch_one(pool)
    .await?;

    Ok(invoice)
}

/// Знайти кандидат на дубль імпортованої накладної за стабільним header fingerprint.
pub async fn find_import_candidate(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    contract_id: Option<Uuid>,
    number: &str,
    direction: DocumentDirection,
    date: chrono::NaiveDate,
    total_amount: Decimal,
) -> Result<Option<Invoice>> {
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        SELECT id, company_id, number, counterparty_id, contract_id, category_id, direction,
               date, expected_payment_date, total_amount, vat_amount,
               status, notes, pdf_path, bas_id, created_at, updated_at
        FROM invoices
        WHERE company_id = $1
          AND counterparty_id = $2
          AND contract_id IS NOT DISTINCT FROM $3
          AND lower(trim(number)) = lower(trim($4))
          AND direction = $5
          AND date = $6
          AND total_amount = $7
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .bind(contract_id)
    .bind(number)
    .bind(direction)
    .bind(date)
    .bind(total_amount)
    .fetch_optional(pool)
    .await?;

    Ok(invoice)
}

/// Знайти всіх кандидатів на дубль імпортованої накладної за точним header fingerprint.
#[allow(clippy::too_many_arguments)]
pub async fn list_import_candidates(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    contract_id: Option<Uuid>,
    number: &str,
    direction: DocumentDirection,
    date: chrono::NaiveDate,
    total_amount: Decimal,
) -> Result<Vec<Invoice>> {
    let invoices = sqlx::query_as::<_, Invoice>(
        r#"
        SELECT id, company_id, number, counterparty_id, contract_id, category_id, direction,
               date, expected_payment_date, total_amount, vat_amount,
               status, notes, pdf_path, bas_id, created_at, updated_at
        FROM invoices
        WHERE company_id = $1
          AND counterparty_id = $2
          AND contract_id IS NOT DISTINCT FROM $3
          AND lower(trim(number)) = lower(trim($4))
          AND direction = $5
          AND date = $6
          AND total_amount = $7
        ORDER BY updated_at DESC, created_at DESC
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .bind(contract_id)
    .bind(number)
    .bind(direction)
    .bind(date)
    .bind(total_amount)
    .fetch_all(pool)
    .await?;

    Ok(invoices)
}

/// Знайти кандидат на дубль за header fingerprint з допустимим відхиленням суми.
#[allow(clippy::too_many_arguments)]
pub async fn find_import_candidate_loose(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    contract_id: Option<Uuid>,
    number: &str,
    direction: DocumentDirection,
    date: chrono::NaiveDate,
    total_amount: Decimal,
    tolerance: Decimal,
) -> Result<Option<Invoice>> {
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        SELECT id, company_id, number, counterparty_id, contract_id, category_id, direction,
               date, expected_payment_date, total_amount, vat_amount,
               status, notes, pdf_path, bas_id, created_at, updated_at
        FROM invoices
        WHERE company_id = $1
          AND counterparty_id = $2
          AND contract_id IS NOT DISTINCT FROM $3
          AND lower(trim(number)) = lower(trim($4))
          AND direction = $5
          AND date = $6
          AND abs(total_amount - $7) <= $8
        ORDER BY abs(total_amount - $7) ASC, updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .bind(contract_id)
    .bind(number)
    .bind(direction)
    .bind(date)
    .bind(total_amount)
    .bind(tolerance)
    .fetch_optional(pool)
    .await?;

    Ok(invoice)
}

/// Знайти всіх кандидатів на дубль за tolerant header fingerprint.
#[allow(clippy::too_many_arguments)]
pub async fn list_import_candidates_loose(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    contract_id: Option<Uuid>,
    number: &str,
    direction: DocumentDirection,
    date: chrono::NaiveDate,
    total_amount: Decimal,
    tolerance: Decimal,
) -> Result<Vec<Invoice>> {
    let invoices = sqlx::query_as::<_, Invoice>(
        r#"
        SELECT id, company_id, number, counterparty_id, contract_id, category_id, direction,
               date, expected_payment_date, total_amount, vat_amount,
               status, notes, pdf_path, bas_id, created_at, updated_at
        FROM invoices
        WHERE company_id = $1
          AND counterparty_id = $2
          AND contract_id IS NOT DISTINCT FROM $3
          AND lower(trim(number)) = lower(trim($4))
          AND direction = $5
          AND date = $6
          AND abs(total_amount - $7) <= $8
        ORDER BY abs(total_amount - $7) ASC, updated_at DESC, created_at DESC
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .bind(contract_id)
    .bind(number)
    .bind(direction)
    .bind(date)
    .bind(total_amount)
    .bind(tolerance)
    .fetch_all(pool)
    .await?;

    Ok(invoices)
}

/// Створити імпортовану накладну разом з позиціями, зберігши BAS-статус та напрям.
#[allow(clippy::too_many_arguments)]
pub async fn create_imported_with_items(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    contract_id: Option<Uuid>,
    number: &str,
    direction: DocumentDirection,
    date: chrono::NaiveDate,
    expected_payment_date: Option<chrono::NaiveDate>,
    vat_amount: Decimal,
    status: InvoiceStatus,
    notes: Option<&str>,
    bas_id: Option<&str>,
    items: &[NewInvoiceItem],
) -> Result<Invoice> {
    let mut tx = pool.begin().await?;

    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        INSERT INTO invoices (
            company_id, counterparty_id, contract_id, number, direction,
            date, expected_payment_date, total_amount, vat_amount,
            status, notes, bas_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, $9, $10, $11)
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
                  date, expected_payment_date, total_amount, vat_amount,
                  status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(company_id)
    .bind(counterparty_id)
    .bind(contract_id)
    .bind(number)
    .bind(direction)
    .bind(date)
    .bind(expected_payment_date)
    .bind(vat_amount)
    .bind(status)
    .bind(notes)
    .bind(bas_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut total = Decimal::ZERO;
    for item in items {
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

    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices
        SET total_amount = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
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

/// Оновити імпортовану накладну разом з повною заміною позицій.
#[allow(clippy::too_many_arguments)]
pub async fn update_imported_with_items(
    pool: &PgPool,
    id: Uuid,
    counterparty_id: Uuid,
    contract_id: Option<Uuid>,
    number: &str,
    direction: DocumentDirection,
    date: chrono::NaiveDate,
    expected_payment_date: Option<chrono::NaiveDate>,
    vat_amount: Decimal,
    status: InvoiceStatus,
    notes: Option<&str>,
    items: &[NewInvoiceItem],
) -> Result<Invoice> {
    let mut tx = pool.begin().await?;

    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices
        SET counterparty_id       = $2,
            contract_id           = $3,
            number                = $4,
            direction             = $5,
            date                  = $6,
            expected_payment_date = $7,
            vat_amount            = $8,
            status                = $9,
            notes                 = $10,
            updated_at            = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
                  date, expected_payment_date, total_amount, vat_amount,
                  status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(counterparty_id)
    .bind(contract_id)
    .bind(number)
    .bind(direction)
    .bind(date)
    .bind(expected_payment_date)
    .bind(vat_amount)
    .bind(status)
    .bind(notes)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM invoice_items WHERE invoice_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let mut total = Decimal::ZERO;
    for item in items {
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

    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices
        SET total_amount = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
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
                              direction, date, expected_payment_date, notes, bas_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
                  date, expected_payment_date, total_amount, vat_amount,
                  status, notes, pdf_path, bas_id, created_at, updated_at
        "#,
    )
    .bind(company_id)
    .bind(&data.number)
    .bind(data.counterparty_id)
    .bind(data.contract_id)
    .bind(data.category_id)
    .bind(&data.direction)
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
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
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
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
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
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
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
    let current =
        sqlx::query_scalar::<_, InvoiceStatus>("SELECT status FROM invoices WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

    let Some(current) = current else {
        return Ok(None);
    };

    // Декодуємо статус вручну — runtime query не підтримує `AS "field: Type"` синтаксис

    if !current.can_transition_to(&new_status) {
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
        RETURNING id, company_id, number, counterparty_id, contract_id, category_id, direction,
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
    let current =
        sqlx::query_scalar::<_, InvoiceStatus>("SELECT status FROM invoices WHERE id = $1")
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

/// Допоміжна функція: парсинг рядка статусу з БД в InvoiceStatus.
#[cfg(test)]
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

/// KPI-агрегати для сторінки списку рахунків.
pub struct InvoiceKpi {
    pub invoices_this_month: i64,
    pub revenue_this_month: Decimal,
    pub unpaid_total: Decimal,
    pub overdue_count: i64,
}

/// Повернути кількість рахунків за кожним статусом для компанії.
/// Результат: `[всього, draft, issued, signed, paid]` (5 елементів).
pub async fn count_by_status(pool: &PgPool, company_id: Uuid) -> Result<Vec<i32>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT status, COUNT(*)::int AS cnt FROM invoices WHERE company_id = $1 GROUP BY status",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    let mut counts = [0i32; 5];
    for row in &rows {
        let status: InvoiceStatus = row.get("status");
        let cnt: i32 = row.get("cnt");
        let idx = match status {
            InvoiceStatus::Draft => 1,
            InvoiceStatus::Issued => 2,
            InvoiceStatus::Signed => 3,
            InvoiceStatus::Paid => 4,
        };
        counts[idx] = cnt;
        counts[0] += cnt;
    }
    Ok(counts.to_vec())
}

/// Повернути KPI-агрегати для сторінки списку рахунків.
pub async fn get_kpi(pool: &PgPool, company_id: Uuid) -> Result<InvoiceKpi> {
    use chrono::Datelike;
    use sqlx::Row;
    let today = chrono::Utc::now().date_naive();
    let first_of_month =
        chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let overdue_threshold = today - chrono::Duration::days(30);

    let row = sqlx::query(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE date >= $2)                                                       AS invoices_this_month,
            COALESCE(SUM(total_amount) FILTER (WHERE status = 'paid' AND date >= $2), 0::numeric)    AS revenue_this_month,
            COALESCE(SUM(total_amount) FILTER (WHERE status IN ('issued','signed')), 0::numeric)     AS unpaid_total,
            COUNT(*) FILTER (WHERE status IN ('issued','signed') AND date < $3)                      AS overdue_count
        FROM invoices
        WHERE company_id = $1
        "#,
    )
    .bind(company_id)
    .bind(first_of_month)
    .bind(overdue_threshold)
    .fetch_one(pool)
    .await?;

    Ok(InvoiceKpi {
        invoices_this_month: row.get::<i64, _>("invoices_this_month"),
        revenue_this_month: row.get::<Decimal, _>("revenue_this_month"),
        unpaid_total: row.get::<Decimal, _>("unpaid_total"),
        overdue_count: row.get::<i64, _>("overdue_count"),
    })
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
        let _ = find_by_bas_id;
        let _ = find_import_candidate;
        let _ = list_import_candidates;
        let _ = find_import_candidate_loose;
        let _ = list_import_candidates_loose;
        let _ = create;
        let _ = create_imported_header;
        let _ = create_imported_with_items;
        let _ = update_with_items;
        let _ = update_imported_header;
        let _ = update_imported_with_items;
        let _ = change_status;
        let _ = get_for_edit;
        let _ = advance_status;
        let _ = count_by_status;
        let _ = get_kpi;
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
