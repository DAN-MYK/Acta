// db/search.rs — повнотекстовий пошук для Command Palette (Ctrl+K)
//
// Повертає плоский список CommandPaletteResult, згрупований за категоріями.
// Статичні елементи (навігація, створити) — завжди у відповідь на порожній запит.
// Динамічні — з БД: акти, рахунки, накладні, контрагенти.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use super::ilike_pattern;

#[derive(Clone)]
pub struct SearchResultItem {
    pub kind: String,   // "header" | "item"
    pub action: String, // "navigate" | "create" | "open_doc" | "open_cp" | ""
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub shortcut: String,
}

impl SearchResultItem {
    fn header(title: &str) -> Self {
        Self {
            kind: "header".into(),
            action: "".into(),
            id: "".into(),
            title: title.into(),
            subtitle: "".into(),
            shortcut: "".into(),
        }
    }
    fn item(action: &str, id: &str, title: &str, subtitle: &str, shortcut: &str) -> Self {
        Self {
            kind: "item".into(),
            action: action.into(),
            id: id.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            shortcut: shortcut.into(),
        }
    }
}

static NAV_ITEMS: &[(&str, &str, &str)] = &[
    ("dashboard", "Головна", ""),
    ("counterparties", "Контрагенти", ""),
    ("acts", "Акти", ""),
    ("invoices", "Рахунки", ""),
    ("waybills", "Накладні", ""),
    ("payments", "Платежі", ""),
    ("tasks", "Задачі", ""),
    ("documents", "Документи", ""),
    ("settings", "Налаштування", ""),
];

static CREATE_ITEMS: &[(&str, &str, &str)] = &[
    ("act", "Новий акт", ""),
    ("invoice", "Новий рахунок", ""),
    ("waybill", "Нова накладна", ""),
    ("counterparty", "Новий контрагент", ""),
];

/// Повний пошук для command palette.
/// Якщо `query` порожній — повертає статичні пункти навігації + "Створити".
/// Якщо `query` непорожній — шукає у БД + фільтрує статику.
pub async fn search(pool: &PgPool, company_id: Uuid, query: &str) -> Result<Vec<SearchResultItem>> {
    let q = query.trim();

    if q.is_empty() {
        return Ok(static_items_all());
    }

    let q_lower = q.to_lowercase();
    let pattern = ilike_pattern(q);

    let mut results: Vec<SearchResultItem> = Vec::new();

    // ── Документи (акти + рахунки + накладні) ────────────────────────────────
    let docs = search_docs(pool, company_id, &pattern).await?;
    if !docs.is_empty() {
        results.push(SearchResultItem::header("ДОКУМЕНТИ"));
        results.extend(docs);
    }

    // ── Контрагенти ──────────────────────────────────────────────────────────
    let cps = search_counterparties(pool, company_id, &pattern).await?;
    if !cps.is_empty() {
        results.push(SearchResultItem::header("КОНТРАГЕНТИ"));
        results.extend(cps);
    }

    // ── Навігація (фільтрована) ───────────────────────────────────────────────
    let nav: Vec<SearchResultItem> = NAV_ITEMS
        .iter()
        .filter(|(_, label, _)| label.to_lowercase().contains(&q_lower))
        .map(|(id, label, hint)| SearchResultItem::item("navigate", id, label, "", hint))
        .collect();
    if !nav.is_empty() {
        results.push(SearchResultItem::header("НАВІГАЦІЯ"));
        results.extend(nav);
    }

    // ── Створити (фільтрована) ────────────────────────────────────────────────
    let create: Vec<SearchResultItem> = CREATE_ITEMS
        .iter()
        .filter(|(_, label, _)| label.to_lowercase().contains(&q_lower))
        .map(|(id, label, hint)| SearchResultItem::item("create", id, label, "", hint))
        .collect();
    if !create.is_empty() {
        results.push(SearchResultItem::header("СТВОРИТИ"));
        results.extend(create);
    }

    Ok(results)
}

fn static_items_all() -> Vec<SearchResultItem> {
    let mut v = Vec::new();
    v.push(SearchResultItem::header("НАВІГАЦІЯ"));
    for (id, label, hint) in NAV_ITEMS {
        v.push(SearchResultItem::item("navigate", id, label, "", hint));
    }
    v.push(SearchResultItem::header("СТВОРИТИ"));
    for (id, label, hint) in CREATE_ITEMS {
        v.push(SearchResultItem::item("create", id, label, "", hint));
    }
    v
}

struct DocRow {
    id: String,
    num: String,
    counterparty_name: String,
    amount: String,
}

async fn search_docs(
    pool: &PgPool,
    company_id: Uuid,
    pattern: &str,
) -> Result<Vec<SearchResultItem>> {
    // Шукаємо у актах
    let acts = sqlx::query_as::<_, (String, String, String, rust_decimal::Decimal)>(
        r#"
        SELECT a.id::text, a.number, cp.name, a.total_amount
        FROM acts a
        JOIN counterparties cp ON cp.id = a.counterparty_id
        WHERE a.company_id = $1
          AND (a.number ILIKE $2 ESCAPE '\' OR cp.name ILIKE $2 ESCAPE '\')
        ORDER BY a.created_at DESC
        LIMIT 5
        "#,
    )
    .bind(company_id)
    .bind(pattern)
    .fetch_all(pool)
    .await?;

    let invoices = sqlx::query_as::<_, (String, String, String, rust_decimal::Decimal)>(
        r#"
        SELECT i.id::text, i.number, cp.name, i.total_amount
        FROM invoices i
        JOIN counterparties cp ON cp.id = i.counterparty_id
        WHERE i.company_id = $1
          AND (i.number ILIKE $2 ESCAPE '\' OR cp.name ILIKE $2 ESCAPE '\')
        ORDER BY i.created_at DESC
        LIMIT 5
        "#,
    )
    .bind(company_id)
    .bind(pattern)
    .fetch_all(pool)
    .await?;

    let waybills = sqlx::query_as::<_, (String, String, String, rust_decimal::Decimal)>(
        r#"
        SELECT w.id::text, w.number, cp.name, w.total_amount
        FROM waybills w
        JOIN counterparties cp ON cp.id = w.counterparty_id
        WHERE w.company_id = $1
          AND (w.number ILIKE $2 ESCAPE '\' OR cp.name ILIKE $2 ESCAPE '\')
        ORDER BY w.created_at DESC
        LIMIT 5
        "#,
    )
    .bind(company_id)
    .bind(pattern)
    .fetch_all(pool)
    .await?;

    let mut rows: Vec<DocRow> = Vec::new();
    for (id, num, cp, amt) in acts {
        rows.push(DocRow {
            id: format!("act:{id}"),
            num,
            counterparty_name: cp,
            amount: format_amount(amt),
        });
    }
    for (id, num, cp, amt) in invoices {
        rows.push(DocRow {
            id: format!("inv:{id}"),
            num,
            counterparty_name: cp,
            amount: format_amount(amt),
        });
    }
    for (id, num, cp, amt) in waybills {
        rows.push(DocRow {
            id: format!("wbl:{id}"),
            num,
            counterparty_name: cp,
            amount: format_amount(amt),
        });
    }

    Ok(rows
        .into_iter()
        .map(|r| {
            let subtitle = format!("{} • {}", r.counterparty_name, r.amount);
            SearchResultItem::item("open_doc", &r.id, &r.num, &subtitle, "")
        })
        .collect())
}

async fn search_counterparties(
    pool: &PgPool,
    company_id: Uuid,
    pattern: &str,
) -> Result<Vec<SearchResultItem>> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>)>(
        r#"
        SELECT id::text, name, edrpou
        FROM counterparties
        WHERE company_id = $1
          AND NOT is_archived
          AND (name ILIKE $2 ESCAPE '\' OR edrpou ILIKE $2 ESCAPE '\')
        ORDER BY name
        LIMIT 5
        "#,
    )
    .bind(company_id)
    .bind(pattern)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, edrpou)| {
            let subtitle = edrpou.map(|e| format!("ЄДРПОУ: {e}")).unwrap_or_default();
            SearchResultItem::item("open_cp", &id, &name, &subtitle, "")
        })
        .collect())
}

fn format_amount(amt: rust_decimal::Decimal) -> String {
    use rust_decimal::prelude::ToPrimitive;
    let val = amt.to_f64().unwrap_or(0.0);
    if val == 0.0 {
        return "0 ₴".into();
    }
    let s = format!("{:.0}", val);
    // Grouping: 1234567 → 1 234 567
    let digits: Vec<char> = s.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut result = String::new();
    let len = digits.len();
    for (i, d) in digits.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('\u{00A0}'); // non-breaking space
        }
        result.push(*d);
    }
    format!("{result} ₴")
}
