use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use chrono::NaiveDate;
use quick_xml::events::Event;
use quick_xml::Reader;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tokio::fs;
use uuid::Uuid;

use crate::db;
use crate::models::act::{Act, ActItem, ActStatus, NewAct, NewActItem, UpdateAct};
use crate::models::DocumentDirection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedActItem {
    pub description: String,
    pub quantity: Decimal,
    pub unit: String,
    pub unit_price: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedAct {
    pub bas_id: Option<String>,
    pub counterparty_bas_id: Option<String>,
    pub contract_bas_id: Option<String>,
    pub number: String,
    pub date: NaiveDate,
    pub expected_payment_date: Option<NaiveDate>,
    pub direction: DocumentDirection,
    pub status: ActStatus,
    pub notes: Option<String>,
    pub items: Vec<ImportedActItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActImportAction {
    Create,
    Update,
    Skip,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActImportPlanRow {
    pub bas_id: Option<String>,
    pub number: String,
    pub action: ActImportAction,
    pub note: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ActImportReport {
    pub parsed: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub rows: Vec<ActImportPlanRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Resolution<T> {
    value: T,
    source: &'static str,
}

pub async fn parse_acts_xml_file(path: &Path) -> Result<Vec<ImportedAct>> {
    let xml_text = fs::read_to_string(path)
        .await
        .with_context(|| format!("Не вдалося прочитати файл {}", path.display()))?;
    parse_acts_xml(&xml_text)
}

pub fn parse_acts_xml(xml: &str) -> Result<Vec<ImportedAct>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut current_fields: Option<BTreeMap<String, String>> = None;
    let mut current_items: Vec<ImportedActItem> = Vec::new();
    let mut current_item_fields: Option<BTreeMap<String, String>> = None;
    let mut rows = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) => {
                let tag = normalize_tag(&String::from_utf8_lossy(event.name().as_ref()));
                if is_act_record_tag(&tag) && current_fields.is_none() {
                    current_fields = Some(BTreeMap::new());
                    current_items.clear();
                } else if current_fields.is_some()
                    && is_act_item_tag(&tag)
                    && current_item_fields.is_none()
                {
                    current_item_fields = Some(BTreeMap::new());
                }
                stack.push(tag);
            }
            Event::Text(event) => {
                let value = String::from_utf8_lossy(event.as_ref()).trim().to_string();
                capture_text_value(&stack, &mut current_fields, &mut current_item_fields, value);
            }
            Event::CData(event) => {
                let value = String::from_utf8_lossy(event.as_ref()).trim().to_string();
                capture_text_value(&stack, &mut current_fields, &mut current_item_fields, value);
            }
            Event::End(event) => {
                let tag = normalize_tag(&String::from_utf8_lossy(event.name().as_ref()));
                if is_act_item_tag(&tag) {
                    if let Some(fields) = current_item_fields.take() {
                        if let Some(item) = build_act_item_from_fields(fields)? {
                            current_items.push(item);
                        }
                    }
                } else if is_act_record_tag(&tag) {
                    if let Some(fields) = current_fields.take() {
                        if let Some(row) =
                            build_act_from_fields(fields, std::mem::take(&mut current_items))?
                        {
                            rows.push(row);
                        }
                    }
                }
                let _ = stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    if rows.is_empty() {
        return Err(anyhow!("У XML не знайдено жодного акту"));
    }

    Ok(rows)
}

pub async fn import_acts_from_xml(
    pool: &PgPool,
    company_id: Uuid,
    path: &Path,
    dry_run: bool,
) -> Result<ActImportReport> {
    let rows = parse_acts_xml_file(path).await?;
    apply_imported_acts(pool, company_id, &rows, dry_run).await
}

pub async fn apply_imported_acts(
    pool: &PgPool,
    company_id: Uuid,
    rows: &[ImportedAct],
    dry_run: bool,
) -> Result<ActImportReport> {
    let mut report = ActImportReport {
        parsed: rows.len(),
        ..ActImportReport::default()
    };

    for row in rows {
        let Some(counterparty) =
            resolve_counterparty_id(pool, row.counterparty_bas_id.as_deref()).await?
        else {
            report.skipped += 1;
            report.rows.push(ActImportPlanRow {
                bas_id: row.bas_id.clone(),
                number: row.number.clone(),
                action: ActImportAction::Skip,
                note: Some("Не знайдено контрагента за bas_id".to_string()),
            });
            continue;
        };

        let contract_id =
            resolve_contract_id(pool, company_id, row.contract_bas_id.as_deref()).await?;
        if row.contract_bas_id.is_some() && contract_id.is_none() {
            report.skipped += 1;
            report.rows.push(ActImportPlanRow {
                bas_id: row.bas_id.clone(),
                number: row.number.clone(),
                action: ActImportAction::Skip,
                note: Some("Не знайдено договір за bas_id".to_string()),
            });
            continue;
        }

        let payload = to_new_act(
            row,
            counterparty.value,
            contract_id.as_ref().map(|item| item.value),
        );

        if row.bas_id.is_none() {
            let matches = db::acts::list_import_candidates(
                pool,
                company_id,
                counterparty.value,
                contract_id.as_ref().map(|item| item.value),
                &row.number,
                row.direction.clone(),
                row.date,
                total_amount(&payload.items),
            )
            .await?;
            if matches.len() > 1 {
                report.conflicts += 1;
                report.skipped += 1;
                report.rows.push(ActImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: ActImportAction::Conflict,
                    note: Some(format!(
                        "conflict: знайдено {} актів за тим самим fingerprint",
                        matches.len()
                    )),
                });
                continue;
            }
        }

        let (existing, match_source): (_, Option<&'static str>) = match row.bas_id.as_deref() {
            Some(bas_id) => (
                db::acts::find_by_bas_id(pool, bas_id).await?,
                Some("bas_id"),
            ),
            None => (
                db::acts::find_import_candidate(
                    pool,
                    company_id,
                    counterparty.value,
                    contract_id.as_ref().map(|item| item.value),
                    &row.number,
                    row.direction.clone(),
                    row.date,
                    total_amount(&payload.items),
                )
                .await?,
                Some("header fingerprint"),
            ),
        };
        let note = Some(build_act_note(
            &counterparty,
            contract_id.as_ref(),
            match_source,
        ));

        match existing {
            Some(act) => {
                let loaded = db::acts::get_by_id(pool, act.id).await?.ok_or_else(|| {
                    anyhow!("Акт знайдено для preview, але не вдалося перечитати його позиції")
                })?;
                if let Some(conflict_note) = detect_act_conflict(
                    &loaded.0,
                    &loaded.1,
                    counterparty.value,
                    contract_id.as_ref().map(|item| item.value),
                    row,
                ) {
                    report.conflicts += 1;
                    report.skipped += 1;
                    report.rows.push(ActImportPlanRow {
                        bas_id: row.bas_id.clone(),
                        number: row.number.clone(),
                        action: ActImportAction::Conflict,
                        note: Some(conflict_note),
                    });
                    continue;
                }

                report.updated += 1;
                report.rows.push(ActImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: ActImportAction::Update,
                    note,
                });
                if !dry_run {
                    let (header, items) = split_new_act_for_update(payload);
                    let _ = db::acts::update_with_items(pool, act.id, header, items).await?;
                }
            }
            None => {
                report.created += 1;
                report.rows.push(ActImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: ActImportAction::Create,
                    note,
                });
                if !dry_run {
                    let _ = db::acts::create(pool, company_id, &payload).await?;
                }
            }
        }
    }

    Ok(report)
}

async fn resolve_counterparty_id(
    pool: &PgPool,
    counterparty_bas_id: Option<&str>,
) -> Result<Option<Resolution<Uuid>>> {
    let Some(counterparty_bas_id) = counterparty_bas_id else {
        return Ok(None);
    };

    Ok(
        db::counterparties::find_by_bas_id(pool, counterparty_bas_id)
            .await?
            .map(|cp| Resolution {
                value: cp.id,
                source: "counterparty bas_id",
            }),
    )
}

async fn resolve_contract_id(
    pool: &PgPool,
    company_id: Uuid,
    contract_bas_id: Option<&str>,
) -> Result<Option<Resolution<Uuid>>> {
    let Some(contract_bas_id) = contract_bas_id else {
        return Ok(None);
    };

    Ok(db::contracts::find_by_bas_id(pool, contract_bas_id)
        .await?
        .filter(|contract| contract.company_id == company_id)
        .map(|contract| Resolution {
            value: contract.id,
            source: "contract bas_id",
        }))
}

fn to_new_act(row: &ImportedAct, counterparty_id: Uuid, contract_id: Option<Uuid>) -> NewAct {
    NewAct {
        number: row.number.clone(),
        counterparty_id,
        contract_id,
        category_id: None,
        direction: row.direction.clone(),
        date: row.date,
        expected_payment_date: row.expected_payment_date,
        status: row.status.clone(),
        notes: row.notes.clone(),
        bas_id: row.bas_id.clone(),
        items: row
            .items
            .iter()
            .map(|item| NewActItem {
                description: item.description.clone(),
                quantity: item.quantity,
                unit: item.unit.clone(),
                unit_price: item.unit_price,
            })
            .collect(),
    }
}

fn total_amount(items: &[NewActItem]) -> Decimal {
    items
        .iter()
        .fold(Decimal::ZERO, |acc, item| {
            acc + (item.quantity * item.unit_price).round_dp(2)
        })
        .round_dp(2)
}

fn build_act_note(
    counterparty: &Resolution<Uuid>,
    contract: Option<&Resolution<Uuid>>,
    match_source: Option<&'static str>,
) -> String {
    let mut parts = vec![format!("cp: {}", counterparty.source)];
    if let Some(contract) = contract {
        parts.push(format!("contract: {}", contract.source));
    }
    if let Some(match_source) = match_source {
        parts.push(format!("match: {}", match_source));
    }
    parts.join("; ")
}

fn detect_act_conflict(
    existing: &Act,
    existing_items: &[ActItem],
    counterparty_id: Uuid,
    contract_id: Option<Uuid>,
    imported: &ImportedAct,
) -> Option<String> {
    if existing.counterparty_id != counterparty_id {
        return Some("conflict: імпортований акт прив'язується до іншого контрагента".to_string());
    }
    if existing.contract_id != contract_id {
        return Some("conflict: імпортований акт прив'язується до іншого договору".to_string());
    }
    if existing.direction != imported.direction {
        return Some("conflict: напрям акту не збігається з existing row".to_string());
    }
    if existing.date != imported.date {
        return Some("conflict: дата акту не збігається з existing row".to_string());
    }
    if normalize_optional_text(Some(&existing.number))
        != normalize_optional_text(Some(&imported.number))
    {
        return Some("conflict: номер акту не збігається з existing row".to_string());
    }

    let imported_items = imported
        .items
        .iter()
        .map(|item| {
            (
                normalize_text(&item.description),
                item.quantity,
                normalize_text(&item.unit),
                item.unit_price,
            )
        })
        .collect::<Vec<_>>();
    let stored_items = existing_items
        .iter()
        .map(|item| {
            (
                normalize_text(&item.description),
                item.quantity,
                normalize_text(&item.unit),
                item.unit_price,
            )
        })
        .collect::<Vec<_>>();
    if imported_items.len() != stored_items.len() {
        return Some("conflict: кількість позицій акту не збігається з existing row".to_string());
    }
    if imported_items != stored_items {
        return Some("conflict: позиції акту відрізняються від existing row".to_string());
    }

    None
}

fn normalize_text(value: &str) -> String {
    value.trim().to_lowercase()
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase())
}

fn split_new_act_for_update(data: NewAct) -> (UpdateAct, Vec<NewActItem>) {
    let items = data.items;
    let header = UpdateAct {
        number: data.number,
        counterparty_id: data.counterparty_id,
        contract_id: data.contract_id,
        category_id: data.category_id,
        date: data.date,
        expected_payment_date: data.expected_payment_date,
        notes: data.notes,
    };
    (header, items)
}

fn build_act_from_fields(
    fields: BTreeMap<String, String>,
    mut items: Vec<ImportedActItem>,
) -> Result<Option<ImportedAct>> {
    let number = required_text(&fields, "number");
    let Some(number) = number else {
        return Ok(None);
    };

    let date = parse_date(
        fields
            .get("date")
            .map(String::as_str)
            .ok_or_else(|| anyhow!("У акті {number} відсутня дата"))?,
    )?;

    if items.is_empty() {
        if let Some(total) = optional_decimal(fields.get("amount"))? {
            items.push(ImportedActItem {
                description: clean_optional(fields.get("subject"))
                    .unwrap_or_else(|| format!("Імпорт з BAS: {number}")),
                quantity: Decimal::ONE,
                unit: "послуга".to_string(),
                unit_price: total,
            });
        } else {
            bail!("У акті {number} не знайдено позицій або підсумкової суми");
        }
    }

    Ok(Some(ImportedAct {
        bas_id: clean_optional(fields.get("bas_id")),
        counterparty_bas_id: clean_optional(fields.get("counterparty_bas_id")),
        contract_bas_id: clean_optional(fields.get("contract_bas_id")),
        number,
        date,
        expected_payment_date: optional_date(fields.get("expected_payment_date"))?,
        direction: parse_direction(fields.get("direction").map(String::as_str)),
        status: parse_act_status(fields.get("status").map(String::as_str)),
        notes: clean_optional(fields.get("notes")),
        items,
    }))
}

fn build_act_item_from_fields(fields: BTreeMap<String, String>) -> Result<Option<ImportedActItem>> {
    let description = required_text(&fields, "description");
    let Some(description) = description else {
        return Ok(None);
    };

    let quantity = optional_decimal(fields.get("quantity"))?.unwrap_or(Decimal::ONE);
    let unit = clean_optional(fields.get("unit")).unwrap_or_else(|| "послуга".to_string());
    let amount = optional_decimal(fields.get("amount"))?;
    let unit_price = match optional_decimal(fields.get("unit_price"))? {
        Some(unit_price) => unit_price,
        None => {
            let amount =
                amount.ok_or_else(|| anyhow!("У позиції '{description}' відсутня ціна"))?;
            if quantity.is_zero() {
                amount
            } else {
                (amount / quantity).round_dp(2)
            }
        }
    };

    Ok(Some(ImportedActItem {
        description,
        quantity,
        unit,
        unit_price,
    }))
}

fn parse_direction(raw: Option<&str>) -> DocumentDirection {
    match raw.map(normalize_value).as_deref() {
        Some("incoming") | Some("вхідний") | Some("вхідна") => {
            DocumentDirection::Incoming
        }
        _ => DocumentDirection::Outgoing,
    }
}

fn parse_act_status(raw: Option<&str>) -> ActStatus {
    match raw.map(normalize_value).as_deref() {
        Some("draft") | Some("чернетка") => ActStatus::Draft,
        Some("signed") | Some("підписано") => ActStatus::Signed,
        Some("paid") | Some("оплачено") => ActStatus::Paid,
        _ => ActStatus::Issued,
    }
}

fn normalize_tag(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('{')
        .split(['}', ':'])
        .next_back()
        .unwrap_or(raw)
        .to_lowercase()
}

fn normalize_value(raw: &str) -> String {
    raw.trim().to_lowercase()
}

fn capture_text_value(
    stack: &[String],
    current_fields: &mut Option<BTreeMap<String, String>>,
    current_item_fields: &mut Option<BTreeMap<String, String>>,
    value: String,
) {
    if value.is_empty() {
        return;
    }

    if let Some(fields) = current_item_fields.as_mut() {
        if let Some(field) = map_act_item_field_name_for_stack(stack) {
            fields.entry(field.to_string()).or_insert(value);
        }
    } else if let Some(fields) = current_fields.as_mut() {
        if let Some(field) = map_act_field_name_for_stack(stack) {
            fields.entry(field.to_string()).or_insert(value);
        }
    }
}

fn is_act_record_tag(tag: &str) -> bool {
    matches!(
        tag,
        "акт"
            | "act"
            | "document"
            | "record"
            | "row"
            | "документ"
            | "актвиконанихробіт"
            | "актвыполненныхработ"
    )
}

fn is_act_item_tag(tag: &str) -> bool {
    matches!(
        tag,
        "item"
            | "позиція"
            | "позиции"
            | "position"
            | "service"
            | "товар"
            | "послуга"
            | "строка"
            | "рядок"
            | "line"
            | "serviceline"
    )
}

fn map_act_field_name_for_stack(stack: &[String]) -> Option<&'static str> {
    let tag = stack.last()?.as_str();
    let parent = stack
        .iter()
        .rev()
        .nth(1)
        .map(String::as_str)
        .unwrap_or_default();

    if matches!(tag, "id" | "uid" | "uuid" | "код") {
        if is_counterparty_container_tag(parent) {
            return Some("counterparty_bas_id");
        }
        if is_contract_container_tag(parent) {
            return Some("contract_bas_id");
        }
    }

    map_act_field_name(tag)
}

fn map_act_field_name(tag: &str) -> Option<&'static str> {
    match tag {
        "id" | "bas_id" | "basid" | "uid" | "uuid" | "код" => Some("bas_id"),
        "number" | "номер" | "num" | "docnumber" | "номердокумента" => {
            Some("number")
        }
        "date" | "дата" | "documentdate" | "docdate" | "датадокумента" => {
            Some("date")
        }
        "expected_payment_date"
        | "payment_date"
        | "expecteddate"
        | "датаплатежу"
        | "датапогашення" => Some("expected_payment_date"),
        "direction" | "напрям" | "doctype" | "типдокумента" => Some("direction"),
        "status" | "статус" => Some("status"),
        "notes" | "comment" | "description" | "примітка" | "коментар" => {
            Some("notes")
        }
        "subject" | "title" | "name" | "назва" | "content" | "зміст" => Some("subject"),
        "amount" | "sum" | "total" | "сума" | "summa" | "documenttotal" => Some("amount"),
        "counterparty_id"
        | "counterparty_bas_id"
        | "client_id"
        | "partner_id"
        | "partner"
        | "counterparty"
        | "контрагентid"
        | "контрагент"
        | "кодконтрагента"
        | "контрагенткод" => Some("counterparty_bas_id"),
        "contract_id"
        | "contract_bas_id"
        | "contract"
        | "agreement"
        | "договір"
        | "договор"
        | "договірid"
        | "коддоговору"
        | "contractref" => Some("contract_bas_id"),
        _ => None,
    }
}

fn is_counterparty_container_tag(tag: &str) -> bool {
    matches!(
        tag,
        "counterparty" | "partner" | "client" | "контрагент" | "партнер" | "клиент"
    )
}

fn is_contract_container_tag(tag: &str) -> bool {
    matches!(tag, "contract" | "agreement" | "договір" | "договор")
}

fn map_act_item_field_name_for_stack(stack: &[String]) -> Option<&'static str> {
    let tag = stack.last()?.as_str();
    map_act_item_field_name(tag)
}

fn map_act_item_field_name(tag: &str) -> Option<&'static str> {
    match tag {
        "description"
        | "name"
        | "title"
        | "послуга"
        | "опис"
        | "номенклатура"
        | "найменування"
        | "содержание"
        | "service_name" => Some("description"),
        "quantity" | "qty" | "count" | "кількість" | "количество" | "обсяг" => {
            Some("quantity")
        }
        "unit" | "uom" | "measure" | "одиниця" | "единица" | "одвим" => {
            Some("unit")
        }
        "price" | "unit_price" | "rate" | "cost" | "tariff" | "ціна" | "цена" => {
            Some("unit_price")
        }
        "amount" | "sum" | "total" | "сума" | "summa" | "linetotal" => Some("amount"),
        _ => None,
    }
}

fn required_text(fields: &BTreeMap<String, String>, key: &str) -> Option<String> {
    clean_optional(fields.get(key))
}

fn clean_optional(value: Option<&String>) -> Option<String> {
    value
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn optional_date(value: Option<&String>) -> Result<Option<NaiveDate>> {
    match clean_optional(value) {
        Some(value) => Ok(Some(parse_date(&value)?)),
        None => Ok(None),
    }
}

fn optional_decimal(value: Option<&String>) -> Result<Option<Decimal>> {
    match clean_optional(value) {
        Some(value) => Ok(Some(parse_decimal(&value)?)),
        None => Ok(None),
    }
}

fn parse_date(raw: &str) -> Result<NaiveDate> {
    for format in ["%Y-%m-%d", "%d.%m.%Y", "%d/%m/%Y", "%Y%m%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(raw.trim(), format) {
            return Ok(date);
        }
    }

    Err(anyhow!("Не вдалося розібрати дату '{raw}'"))
}

fn parse_decimal(raw: &str) -> Result<Decimal> {
    let normalized = raw.trim().replace(' ', "").replace(',', ".");
    normalized
        .parse::<Decimal>()
        .with_context(|| format!("Не вдалося розібрати суму '{raw}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_acts_xml_reads_rows_with_items() {
        let xml = r#"
            <acts>
                <act>
                    <id>act-001</id>
                    <counterparty_id>cp-001</counterparty_id>
                    <contract_id>ctr-001</contract_id>
                    <number>АКТ-001</number>
                    <date>2026-04-10</date>
                    <item>
                        <description>Консультації</description>
                        <quantity>2</quantity>
                        <unit>год</unit>
                        <price>1500</price>
                    </item>
                </act>
            </acts>
        "#;

        let rows = parse_acts_xml(xml).expect("парсинг актів має спрацювати");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bas_id.as_deref(), Some("act-001"));
        assert_eq!(rows[0].counterparty_bas_id.as_deref(), Some("cp-001"));
        assert_eq!(rows[0].contract_bas_id.as_deref(), Some("ctr-001"));
        assert_eq!(rows[0].items.len(), 1);
        assert_eq!(rows[0].items[0].quantity, Decimal::new(2, 0));
        assert_eq!(rows[0].items[0].unit_price, Decimal::new(1500, 0));
    }

    #[test]
    fn parse_acts_xml_builds_fallback_item_from_total() {
        let xml = r#"
            <root>
                <акт>
                    <код>act-002</код>
                    <кодконтрагента>cp-002</кодконтрагента>
                    <номер>АКТ-002</номер>
                    <дата>11.04.2026</дата>
                    <сума>3 200,00</сума>
                </акт>
            </root>
        "#;

        let rows = parse_acts_xml(xml).expect("fallback-позиція має будуватися з суми");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].items.len(), 1);
        assert_eq!(rows[0].items[0].unit_price, Decimal::new(320000, 2));
        assert_eq!(rows[0].status, ActStatus::Issued);
    }

    #[test]
    fn parse_acts_xml_supports_nested_bas_like_fields() {
        let xml = r#"
            <Документ>
                <Код>act-777</Код>
                <НомерДокумента>АКТ-777</НомерДокумента>
                <ДатаДокумента>2026-04-10</ДатаДокумента>
                <Контрагент>
                    <Код>cp-777</Код>
                </Контрагент>
                <Договір>
                    <Код>ctr-777</Код>
                </Договір>
                <Строка>
                    <Найменування>Послуги підтримки</Найменування>
                    <Кількість>2</Кількість>
                    <ОдВим>послуга</ОдВим>
                    <Сума>3000</Сума>
                </Строка>
            </Документ>
        "#;

        let rows = parse_acts_xml(xml).expect("вкладений BAS XML має парситися");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bas_id.as_deref(), Some("act-777"));
        assert_eq!(rows[0].counterparty_bas_id.as_deref(), Some("cp-777"));
        assert_eq!(rows[0].contract_bas_id.as_deref(), Some("ctr-777"));
        assert_eq!(rows[0].items.len(), 1);
        assert_eq!(rows[0].items[0].description, "Послуги підтримки");
        assert_eq!(rows[0].items[0].quantity, Decimal::new(2, 0));
        assert_eq!(rows[0].items[0].unit_price, Decimal::new(1500, 0));
    }

    #[test]
    fn parse_acts_xml_reads_cdata_and_fallback_total() {
        let xml = r#"
            <АктВиконанихРобіт>
                <Код><![CDATA[act-900]]></Код>
                <НомерДокумента><![CDATA[АКТ-900]]></НомерДокумента>
                <ДатаДокумента>2026-04-11</ДатаДокумента>
                <Назва><![CDATA[Супровід]]></Назва>
                <DocumentTotal>4500</DocumentTotal>
            </АктВиконанихРобіт>
        "#;

        let rows = parse_acts_xml(xml).expect("CDATA і fallback total мають парситися");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bas_id.as_deref(), Some("act-900"));
        assert_eq!(rows[0].items.len(), 1);
        assert_eq!(rows[0].items[0].description, "Супровід");
        assert_eq!(rows[0].items[0].unit_price, Decimal::new(4500, 0));
    }

    #[test]
    fn parse_acts_xml_fails_when_no_records_found() {
        let error = parse_acts_xml("<root/>").expect_err("порожній XML має давати помилку");
        assert!(error.to_string().contains("не знайдено жодного акту"));
    }
}
