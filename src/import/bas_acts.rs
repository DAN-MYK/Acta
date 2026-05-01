use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use calamine::{open_workbook_auto, Reader as CalamineReader};
use chrono::NaiveDate;
use quick_xml::events::Event;
use quick_xml::Reader;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tokio::fs;
use tokio::task;
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

pub async fn parse_acts_file(path: &Path) -> Result<Vec<ImportedAct>> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "xlsx" | "xls" => parse_acts_excel_file(path).await,
        _ => parse_acts_xml_file(path).await,
    }
}

pub async fn parse_acts_xml_file(path: &Path) -> Result<Vec<ImportedAct>> {
    let xml_text = fs::read_to_string(path).await.with_context(|| {
        format!(
            "РќРµ РІРґР°Р»РѕСЃСЏ РїСЂРѕС‡РёС‚Р°С‚Рё С„Р°Р№Р» {}",
            path.display()
        )
    })?;
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
        return Err(anyhow!(
            "РЈ XML РЅРµ Р·РЅР°Р№РґРµРЅРѕ Р¶РѕРґРЅРѕРіРѕ Р°РєС‚Сѓ"
        ));
    }

    Ok(rows)
}

pub async fn import_acts_from_xml(
    pool: &PgPool,
    company_id: Uuid,
    path: &Path,
    dry_run: bool,
) -> Result<ActImportReport> {
    let rows = parse_acts_file(path).await?;
    apply_imported_acts(pool, company_id, &rows, dry_run).await
}

pub async fn import_acts_from_file(
    pool: &PgPool,
    company_id: Uuid,
    path: &Path,
    dry_run: bool,
) -> Result<ActImportReport> {
    import_acts_from_xml(pool, company_id, path, dry_run).await
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
                note: Some("РќРµ Р·РЅР°Р№РґРµРЅРѕ РєРѕРЅС‚СЂР°РіРµРЅС‚Р° Р·Р° bas_id".to_string()),
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
                note: Some("РќРµ Р·РЅР°Р№РґРµРЅРѕ РґРѕРіРѕРІС–СЂ Р·Р° bas_id".to_string()),
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
                        "conflict: Р·РЅР°Р№РґРµРЅРѕ {} Р°РєС‚С–РІ Р·Р° С‚РёРј СЃР°РјРёРј fingerprint",
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
                    anyhow!("РђРєС‚ Р·РЅР°Р№РґРµРЅРѕ РґР»СЏ preview, Р°Р»Рµ РЅРµ РІРґР°Р»РѕСЃСЏ РїРµСЂРµС‡РёС‚Р°С‚Рё Р№РѕРіРѕ РїРѕР·РёС†С–С—")
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
        return Some("conflict: С–РјРїРѕСЂС‚РѕРІР°РЅРёР№ Р°РєС‚ РїСЂРёРІ'СЏР·СѓС”С‚СЊСЃСЏ РґРѕ С–РЅС€РѕРіРѕ РєРѕРЅС‚СЂР°РіРµРЅС‚Р°".to_string());
    }
    if existing.contract_id != contract_id {
        return Some("conflict: С–РјРїРѕСЂС‚РѕРІР°РЅРёР№ Р°РєС‚ РїСЂРёРІ'СЏР·СѓС”С‚СЊСЃСЏ РґРѕ С–РЅС€РѕРіРѕ РґРѕРіРѕРІРѕСЂСѓ".to_string());
    }
    if existing.direction != imported.direction {
        return Some(
            "conflict: РЅР°РїСЂСЏРј Р°РєС‚Сѓ РЅРµ Р·Р±С–РіР°С”С‚СЊСЃСЏ Р· existing row".to_string(),
        );
    }
    if existing.date != imported.date {
        return Some(
            "conflict: РґР°С‚Р° Р°РєС‚Сѓ РЅРµ Р·Р±С–РіР°С”С‚СЊСЃСЏ Р· existing row".to_string(),
        );
    }
    if normalize_optional_text(Some(&existing.number))
        != normalize_optional_text(Some(&imported.number))
    {
        return Some(
            "conflict: РЅРѕРјРµСЂ Р°РєС‚Сѓ РЅРµ Р·Р±С–РіР°С”С‚СЊСЃСЏ Р· existing row".to_string(),
        );
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
        return Some("conflict: РєС–Р»СЊРєС–СЃС‚СЊ РїРѕР·РёС†С–Р№ Р°РєС‚Сѓ РЅРµ Р·Р±С–РіР°С”С‚СЊСЃСЏ Р· existing row".to_string());
    }
    if imported_items != stored_items {
        return Some(
            "conflict: РїРѕР·РёС†С–С— Р°РєС‚Сѓ РІС–РґСЂС–Р·РЅСЏСЋС‚СЊСЃСЏ РІС–Рґ existing row"
                .to_string(),
        );
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
            .ok_or_else(|| anyhow!("РЈ Р°РєС‚С– {number} РІС–РґСЃСѓС‚РЅСЏ РґР°С‚Р°"))?,
    )?;

    if let Some(item) = build_act_item_from_fields(fields.clone())? {
        items.push(item);
    }

    if items.is_empty() {
        if let Some(total) = optional_decimal(fields.get("amount"))? {
            items.push(ImportedActItem {
                description: clean_optional(fields.get("subject"))
                    .unwrap_or_else(|| format!("Р†РјРїРѕСЂС‚ Р· BAS: {number}")),
                quantity: Decimal::ONE,
                unit: "РїРѕСЃР»СѓРіР°".to_string(),
                unit_price: total,
            });
        } else {
            bail!("РЈ Р°РєС‚С– {number} РЅРµ Р·РЅР°Р№РґРµРЅРѕ РїРѕР·РёС†С–Р№ Р°Р±Рѕ РїС–РґСЃСѓРјРєРѕРІРѕС— СЃСѓРјРё");
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
    let unit = clean_optional(fields.get("unit")).unwrap_or_else(|| "РїРѕСЃР»СѓРіР°".to_string());
    let amount = optional_decimal(fields.get("amount"))?;
    let unit_price = match optional_decimal(fields.get("unit_price"))? {
        Some(unit_price) => unit_price,
        None => {
            let amount = amount.ok_or_else(|| {
                anyhow!("РЈ РїРѕР·РёС†С–С— '{description}' РІС–РґСЃСѓС‚РЅСЏ С†С–РЅР°")
            })?;
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
        Some("incoming") | Some("РІС…С–РґРЅРёР№") | Some("РІС…С–РґРЅР°") => {
            DocumentDirection::Incoming
        }
        _ => DocumentDirection::Outgoing,
    }
}

fn parse_act_status(raw: Option<&str>) -> ActStatus {
    match raw.map(normalize_value).as_deref() {
        Some("draft") | Some("С‡РµСЂРЅРµС‚РєР°") => ActStatus::Draft,
        Some("signed") | Some("РїС–РґРїРёСЃР°РЅРѕ") => ActStatus::Signed,
        Some("paid") | Some("РѕРїР»Р°С‡РµРЅРѕ") => ActStatus::Paid,
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
        "Р°РєС‚"
            | "act"
            | "document"
            | "record"
            | "row"
            | "РґРѕРєСѓРјРµРЅС‚"
            | "Р°РєС‚РІРёРєРѕРЅР°РЅРёС…СЂРѕР±С–С‚"
            | "Р°РєС‚РІС‹РїРѕР»РЅРµРЅРЅС‹С…СЂР°Р±РѕС‚"
    )
}

fn is_act_item_tag(tag: &str) -> bool {
    matches!(
        tag,
        "item"
            | "РїРѕР·РёС†С–СЏ"
            | "РїРѕР·РёС†РёРё"
            | "position"
            | "service"
            | "С‚РѕРІР°СЂ"
            | "РїРѕСЃР»СѓРіР°"
            | "СЃС‚СЂРѕРєР°"
            | "СЂСЏРґРѕРє"
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

    if matches!(tag, "id" | "uid" | "uuid" | "РєРѕРґ") {
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
        "id" | "bas_id" | "basid" | "uid" | "uuid" | "РєРѕРґ" => Some("bas_id"),
        "number" | "РЅРѕРјРµСЂ" | "num" | "docnumber" | "РЅРѕРјРµСЂРґРѕРєСѓРјРµРЅС‚Р°" => {
            Some("number")
        }
        "date" | "РґР°С‚Р°" | "documentdate" | "docdate" | "РґР°С‚Р°РґРѕРєСѓРјРµРЅС‚Р°" => {
            Some("date")
        }
        "expected_payment_date"
        | "payment_date"
        | "expecteddate"
        | "РґР°С‚Р°РїР»Р°С‚РµР¶Сѓ"
        | "РґР°С‚Р°РїРѕРіР°С€РµРЅРЅСЏ" => Some("expected_payment_date"),
        "direction" | "РЅР°РїСЂСЏРј" | "doctype" | "С‚РёРїРґРѕРєСѓРјРµРЅС‚Р°" => {
            Some("direction")
        }
        "status" | "СЃС‚Р°С‚СѓСЃ" => Some("status"),
        "notes" | "comment" | "description" | "РїСЂРёРјС–С‚РєР°" | "РєРѕРјРµРЅС‚Р°СЂ" => {
            Some("notes")
        }
        "subject" | "title" | "name" | "РЅР°Р·РІР°" | "content" | "Р·РјС–СЃС‚" => {
            Some("subject")
        }
        "amount" | "sum" | "total" | "СЃСѓРјР°" | "summa" | "documenttotal" => {
            Some("amount")
        }
        "counterparty_id"
        | "counterparty_bas_id"
        | "client_id"
        | "partner_id"
        | "partner"
        | "counterparty"
        | "РєРѕРЅС‚СЂР°РіРµРЅС‚id"
        | "РєРѕРЅС‚СЂР°РіРµРЅС‚"
        | "РєРѕРґРєРѕРЅС‚СЂР°РіРµРЅС‚Р°"
        | "РєРѕРЅС‚СЂР°РіРµРЅС‚РєРѕРґ" => Some("counterparty_bas_id"),
        "contract_id"
        | "contract_bas_id"
        | "contract"
        | "agreement"
        | "РґРѕРіРѕРІС–СЂ"
        | "РґРѕРіРѕРІРѕСЂ"
        | "РґРѕРіРѕРІС–СЂid"
        | "РєРѕРґРґРѕРіРѕРІРѕСЂСѓ"
        | "contractref" => Some("contract_bas_id"),
        _ => None,
    }
}

fn is_counterparty_container_tag(tag: &str) -> bool {
    matches!(
        tag,
        "counterparty"
            | "partner"
            | "client"
            | "РєРѕРЅС‚СЂР°РіРµРЅС‚"
            | "РїР°СЂС‚РЅРµСЂ"
            | "РєР»РёРµРЅС‚"
    )
}

fn is_contract_container_tag(tag: &str) -> bool {
    matches!(
        tag,
        "contract" | "agreement" | "РґРѕРіРѕРІС–СЂ" | "РґРѕРіРѕРІРѕСЂ"
    )
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
        | "РїРѕСЃР»СѓРіР°"
        | "РѕРїРёСЃ"
        | "РЅРѕРјРµРЅРєР»Р°С‚СѓСЂР°"
        | "РЅР°Р№РјРµРЅСѓРІР°РЅРЅСЏ"
        | "СЃРѕРґРµСЂР¶Р°РЅРёРµ"
        | "service_name" => Some("description"),
        "quantity"
        | "qty"
        | "count"
        | "РєС–Р»СЊРєС–СЃС‚СЊ"
        | "РєРѕР»РёС‡РµСЃС‚РІРѕ"
        | "РѕР±СЃСЏРі" => Some("quantity"),
        "unit" | "uom" | "measure" | "РѕРґРёРЅРёС†СЏ" | "РµРґРёРЅРёС†Р°" | "РѕРґРІРёРј" => {
            Some("unit")
        }
        "price" | "unit_price" | "rate" | "cost" | "tariff" | "С†С–РЅР°" | "С†РµРЅР°" => {
            Some("unit_price")
        }
        "amount" | "sum" | "total" | "СЃСѓРјР°" | "summa" | "linetotal" => Some("amount"),
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

    Err(anyhow!(
        "РќРµ РІРґР°Р»РѕСЃСЏ СЂРѕР·С–Р±СЂР°С‚Рё РґР°С‚Сѓ '{raw}'"
    ))
}

fn parse_decimal(raw: &str) -> Result<Decimal> {
    let normalized = raw.trim().replace(' ', "").replace(',', ".");
    normalized
        .parse::<Decimal>()
        .with_context(|| format!("РќРµ РІРґР°Р»РѕСЃСЏ СЂРѕР·С–Р±СЂР°С‚Рё СЃСѓРјСѓ '{raw}'"))
}

async fn parse_acts_excel_file(path: &Path) -> Result<Vec<ImportedAct>> {
    let path = path.to_path_buf();
    task::spawn_blocking(move || {
        let mut workbook = open_workbook_auto(&path).with_context(|| {
            format!(
                "РќРµ РІРґР°Р»РѕСЃСЏ РІС–РґРєСЂРёС‚Рё Excel С„Р°Р№Р» {}",
                path.display()
            )
        })?;
        let sheet_name = workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("РЈ Excel С„Р°Р№Р»С– РЅРµРјР°С” Р¶РѕРґРЅРѕРіРѕ sheet"))?;
        let range = workbook.worksheet_range(&sheet_name)?;

        let mut rows_iter = range.rows();
        let headers = rows_iter
            .next()
            .ok_or_else(|| anyhow!("Excel С„Р°Р№Р» РЅРµ РјС–СЃС‚РёС‚СЊ Р·Р°РіРѕР»РѕРІРєС–РІ"))?
            .iter()
            .map(|cell| normalize_tag(&cell.to_string()))
            .collect::<Vec<_>>();

        let mut grouped: BTreeMap<String, ImportedAct> = BTreeMap::new();
        for row in rows_iter {
            let mut fields = BTreeMap::new();
            for (idx, cell) in row.iter().enumerate() {
                let value = cell.to_string().trim().to_string();
                if value.is_empty() {
                    continue;
                }
                if let Some(field) = headers
                    .get(idx)
                    .and_then(|header| map_act_field_name(header))
                {
                    fields.entry(field.to_string()).or_insert(value.clone());
                }
                if let Some(field) = headers
                    .get(idx)
                    .and_then(|header| map_act_item_field_name(header))
                {
                    fields.entry(field.to_string()).or_insert(value);
                }
            }

            if let Some(act) = build_act_from_fields(fields, Vec::new())? {
                let key = act_merge_key(&act);
                if let Some(existing) = grouped.get_mut(&key) {
                    merge_act(existing, act)?;
                } else {
                    grouped.insert(key, act);
                }
            }
        }

        let rows = grouped.into_values().collect::<Vec<_>>();
        if rows.is_empty() {
            return Err(anyhow!(
                "РЈ Excel РЅРµ Р·РЅР°Р№РґРµРЅРѕ Р¶РѕРґРЅРѕРіРѕ Р°РєС‚Сѓ"
            ));
        }

        Ok(rows)
    })
    .await
    .context("Excel parser РґР»СЏ Р°РєС‚С–РІ Р·Р°РІРµСЂС€РёРІСЃСЏ РїРѕРјРёР»РєРѕСЋ")?
}

fn merge_act(target: &mut ImportedAct, incoming: ImportedAct) -> Result<()> {
    if target.number != incoming.number
        || target.date != incoming.date
        || target.direction != incoming.direction
    {
        bail!("Excel grouping РґР»СЏ Р°РєС‚С–РІ РѕС‚СЂРёРјР°РІ РЅРµСЃСѓРјС–СЃРЅС– header rows");
    }

    if target.bas_id.is_none() {
        target.bas_id = incoming.bas_id;
    }
    if target.counterparty_bas_id.is_none() {
        target.counterparty_bas_id = incoming.counterparty_bas_id;
    }
    if target.contract_bas_id.is_none() {
        target.contract_bas_id = incoming.contract_bas_id;
    }
    if target.expected_payment_date.is_none() {
        target.expected_payment_date = incoming.expected_payment_date;
    }
    if target.notes.is_none() {
        target.notes = incoming.notes;
    }
    target.status = incoming.status;
    target.items.extend(incoming.items);

    Ok(())
}

fn act_merge_key(row: &ImportedAct) -> String {
    if let Some(bas_id) = row.bas_id.as_deref() {
        return format!("bas:{bas_id}");
    }

    format!(
        "{}|{}|{}|{}",
        row.number,
        row.date,
        row.counterparty_bas_id.as_deref().unwrap_or(""),
        row.contract_bas_id.as_deref().unwrap_or("")
    )
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
                    <number>РђРљРў-001</number>
                    <date>2026-04-10</date>
                    <item>
                        <description>РљРѕРЅСЃСѓР»СЊС‚Р°С†С–С—</description>
                        <quantity>2</quantity>
                        <unit>РіРѕРґ</unit>
                        <price>1500</price>
                    </item>
                </act>
            </acts>
        "#;

        let rows =
            parse_acts_xml(xml).expect("РїР°СЂСЃРёРЅРі Р°РєС‚С–РІ РјР°С” СЃРїСЂР°С†СЋРІР°С‚Рё");
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
                <Р°РєС‚>
                    <РєРѕРґ>act-002</РєРѕРґ>
                    <РєРѕРґРєРѕРЅС‚СЂР°РіРµРЅС‚Р°>cp-002</РєРѕРґРєРѕРЅС‚СЂР°РіРµРЅС‚Р°>
                    <РЅРѕРјРµСЂ>РђРљРў-002</РЅРѕРјРµСЂ>
                    <РґР°С‚Р°>11.04.2026</РґР°С‚Р°>
                    <СЃСѓРјР°>3 200,00</СЃСѓРјР°>
                </Р°РєС‚>
            </root>
        "#;

        let rows = parse_acts_xml(xml)
            .expect("fallback-РїРѕР·РёС†С–СЏ РјР°С” Р±СѓРґСѓРІР°С‚РёСЃСЏ Р· СЃСѓРјРё");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].items.len(), 1);
        assert_eq!(rows[0].items[0].unit_price, Decimal::new(320000, 2));
        assert_eq!(rows[0].status, ActStatus::Issued);
    }

    #[test]
    fn parse_acts_xml_supports_nested_bas_like_fields() {
        let xml = r#"
            <Р”РѕРєСѓРјРµРЅС‚>
                <РљРѕРґ>act-777</РљРѕРґ>
                <РќРѕРјРµСЂР”РѕРєСѓРјРµРЅС‚Р°>РђРљРў-777</РќРѕРјРµСЂР”РѕРєСѓРјРµРЅС‚Р°>
                <Р”Р°С‚Р°Р”РѕРєСѓРјРµРЅС‚Р°>2026-04-10</Р”Р°С‚Р°Р”РѕРєСѓРјРµРЅС‚Р°>
                <РљРѕРЅС‚СЂР°РіРµРЅС‚>
                    <РљРѕРґ>cp-777</РљРѕРґ>
                </РљРѕРЅС‚СЂР°РіРµРЅС‚>
                <Р”РѕРіРѕРІС–СЂ>
                    <РљРѕРґ>ctr-777</РљРѕРґ>
                </Р”РѕРіРѕРІС–СЂ>
                <РЎС‚СЂРѕРєР°>
                    <РќР°Р№РјРµРЅСѓРІР°РЅРЅСЏ>РџРѕСЃР»СѓРіРё РїС–РґС‚СЂРёРјРєРё</РќР°Р№РјРµРЅСѓРІР°РЅРЅСЏ>
                    <РљС–Р»СЊРєС–СЃС‚СЊ>2</РљС–Р»СЊРєС–СЃС‚СЊ>
                    <РћРґР’РёРј>РїРѕСЃР»СѓРіР°</РћРґР’РёРј>
                    <РЎСѓРјР°>3000</РЎСѓРјР°>
                </РЎС‚СЂРѕРєР°>
            </Р”РѕРєСѓРјРµРЅС‚>
        "#;

        let rows =
            parse_acts_xml(xml).expect("РІРєР»Р°РґРµРЅРёР№ BAS XML РјР°С” РїР°СЂСЃРёС‚РёСЃСЏ");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bas_id.as_deref(), Some("act-777"));
        assert_eq!(rows[0].counterparty_bas_id.as_deref(), Some("cp-777"));
        assert_eq!(rows[0].contract_bas_id.as_deref(), Some("ctr-777"));
        assert_eq!(rows[0].items.len(), 1);
        assert_eq!(
            rows[0].items[0].description,
            "РџРѕСЃР»СѓРіРё РїС–РґС‚СЂРёРјРєРё"
        );
        assert_eq!(rows[0].items[0].quantity, Decimal::new(2, 0));
        assert_eq!(rows[0].items[0].unit_price, Decimal::new(1500, 0));
    }

    #[test]
    fn parse_acts_xml_reads_cdata_and_fallback_total() {
        let xml = r#"
            <РђРєС‚Р’РёРєРѕРЅР°РЅРёС…Р РѕР±С–С‚>
                <РљРѕРґ><![CDATA[act-900]]></РљРѕРґ>
                <РќРѕРјРµСЂР”РѕРєСѓРјРµРЅС‚Р°><![CDATA[РђРљРў-900]]></РќРѕРјРµСЂР”РѕРєСѓРјРµРЅС‚Р°>
                <Р”Р°С‚Р°Р”РѕРєСѓРјРµРЅС‚Р°>2026-04-11</Р”Р°С‚Р°Р”РѕРєСѓРјРµРЅС‚Р°>
                <РќР°Р·РІР°><![CDATA[РЎСѓРїСЂРѕРІС–Рґ]]></РќР°Р·РІР°>
                <DocumentTotal>4500</DocumentTotal>
            </РђРєС‚Р’РёРєРѕРЅР°РЅРёС…Р РѕР±С–С‚>
        "#;

        let rows =
            parse_acts_xml(xml).expect("CDATA С– fallback total РјР°СЋС‚СЊ РїР°СЂСЃРёС‚РёСЃСЏ");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bas_id.as_deref(), Some("act-900"));
        assert_eq!(rows[0].items.len(), 1);
        assert_eq!(rows[0].items[0].description, "РЎСѓРїСЂРѕРІС–Рґ");
        assert_eq!(rows[0].items[0].unit_price, Decimal::new(4500, 0));
    }

    #[test]
    fn parse_acts_xml_fails_when_no_records_found() {
        let error = parse_acts_xml("<root/>")
            .expect_err("РїРѕСЂРѕР¶РЅС–Р№ XML РјР°С” РґР°РІР°С‚Рё РїРѕРјРёР»РєСѓ");
        assert!(error
            .to_string()
            .contains("РЅРµ Р·РЅР°Р№РґРµРЅРѕ Р¶РѕРґРЅРѕРіРѕ Р°РєС‚Сѓ"));
    }

    #[test]
    fn build_act_from_fields_builds_single_excel_row_item() {
        let fields = BTreeMap::from([
            ("number".to_string(), "ACT-XL-001".to_string()),
            ("date".to_string(), "2026-04-12".to_string()),
            ("counterparty_bas_id".to_string(), "cp-001".to_string()),
            ("contract_bas_id".to_string(), "ctr-001".to_string()),
            ("description".to_string(), "Консультації".to_string()),
            ("quantity".to_string(), "2".to_string()),
            ("unit".to_string(), "год".to_string()),
            ("unit_price".to_string(), "1500.00".to_string()),
        ]);

        let act = build_act_from_fields(fields, Vec::new())
            .expect("рядок Excel має парситися")
            .expect("акт має бути створений");

        assert_eq!(act.number, "ACT-XL-001");
        assert_eq!(act.items.len(), 1);
        assert_eq!(act.items[0].description, "Консультації");
        assert_eq!(act.items[0].quantity, Decimal::new(2, 0));
        assert_eq!(act.items[0].unit_price, Decimal::new(150000, 2));
    }

    #[test]
    fn merge_act_appends_items_from_same_excel_document() {
        let mut first = ImportedAct {
            bas_id: Some("act-001".to_string()),
            counterparty_bas_id: Some("cp-001".to_string()),
            contract_bas_id: Some("ctr-001".to_string()),
            number: "ACT-001".to_string(),
            date: NaiveDate::from_ymd_opt(2026, 4, 10).expect("валідна дата"),
            expected_payment_date: None,
            direction: DocumentDirection::Outgoing,
            status: ActStatus::Issued,
            notes: None,
            items: vec![ImportedActItem {
                description: "Послуга 1".to_string(),
                quantity: Decimal::ONE,
                unit: "шт".to_string(),
                unit_price: Decimal::new(10000, 2),
            }],
        };
        let second = ImportedAct {
            items: vec![ImportedActItem {
                description: "Послуга 2".to_string(),
                quantity: Decimal::new(3, 0),
                unit: "год".to_string(),
                unit_price: Decimal::new(250000, 2),
            }],
            ..first.clone()
        };

        merge_act(&mut first, second).expect("merge має спрацювати");
        assert_eq!(first.items.len(), 2);
        assert_eq!(act_merge_key(&first), "bas:act-001");
    }
}
