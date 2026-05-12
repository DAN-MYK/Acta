use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use calamine::{open_workbook_auto, Reader as CalamineReader};
use chrono::NaiveDate;
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tokio::fs;
use tokio::task;
use uuid::Uuid;

use crate::db;
use crate::models::invoice::{Invoice, InvoiceItem, InvoiceStatus, NewInvoiceItem};
use crate::models::DocumentDirection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedInvoiceItem {
    pub description: String,
    pub unit: Option<String>,
    pub quantity: Decimal,
    pub price: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedInvoice {
    pub bas_id: Option<String>,
    pub counterparty_bas_id: Option<String>,
    pub counterparty_name: Option<String>,
    pub counterparty_edrpou: Option<String>,
    pub contract_bas_id: Option<String>,
    pub contract_number: Option<String>,
    pub number: String,
    pub date: NaiveDate,
    pub expected_payment_date: Option<NaiveDate>,
    pub direction: DocumentDirection,
    pub status: InvoiceStatus,
    pub total_amount: Decimal,
    pub vat_amount: Decimal,
    pub notes: Option<String>,
    pub items: Vec<ImportedInvoiceItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvoiceImportAction {
    Create,
    Update,
    Skip,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceImportPlanRow {
    pub bas_id: Option<String>,
    pub number: String,
    pub action: InvoiceImportAction,
    pub note: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InvoiceImportReport {
    pub parsed: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub rows: Vec<InvoiceImportPlanRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Resolution<T> {
    value: T,
    source: &'static str,
}

fn import_duplicate_tolerance() -> Decimal {
    Decimal::new(5, 2)
}

pub async fn parse_invoices_file(path: &Path) -> Result<Vec<ImportedInvoice>> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "xlsx" | "xls" => parse_invoices_excel_file(path).await,
        _ => {
            let xml_text = fs::read_to_string(path)
                .await
                .with_context(|| format!("Не вдалося прочитати файл {}", path.display()))?;
            parse_invoices_xml(&xml_text)
        }
    }
}

pub fn parse_invoices_xml(xml: &str) -> Result<Vec<ImportedInvoice>> {
    let mut reader = XmlReader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut current_fields: Option<BTreeMap<String, String>> = None;
    let mut current_items: Vec<ImportedInvoiceItem> = Vec::new();
    let mut current_item_fields: Option<BTreeMap<String, String>> = None;
    let mut rows = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) => {
                let tag = normalize_tag(&String::from_utf8_lossy(event.name().as_ref()));
                if is_invoice_record_tag(&tag) && current_fields.is_none() {
                    current_fields = Some(BTreeMap::new());
                    current_items.clear();
                } else if current_fields.is_some()
                    && is_invoice_item_tag(&tag)
                    && current_item_fields.is_none()
                {
                    current_item_fields = Some(BTreeMap::new());
                }
                stack.push(tag);
            }
            Event::Text(event) => {
                let value = String::from_utf8_lossy(event.as_ref()).trim().to_string();
                if value.is_empty() {
                    buf.clear();
                    continue;
                }

                if let Some(fields) = current_item_fields.as_mut() {
                    if let Some(tag) = stack.last() {
                        if let Some(field) = map_invoice_item_field_name(tag) {
                            fields.entry(field.to_string()).or_insert(value);
                        }
                    }
                } else if let Some(fields) = current_fields.as_mut() {
                    if let Some(tag) = stack.last() {
                        if let Some(field) = map_invoice_field_name(tag) {
                            fields.entry(field.to_string()).or_insert(value);
                        }
                    }
                }
            }
            Event::End(event) => {
                let tag = normalize_tag(&String::from_utf8_lossy(event.name().as_ref()));
                if is_invoice_item_tag(&tag) {
                    if let Some(fields) = current_item_fields.take() {
                        if let Some(item) = build_invoice_item_from_fields(fields)? {
                            current_items.push(item);
                        }
                    }
                } else if is_invoice_record_tag(&tag) {
                    if let Some(fields) = current_fields.take() {
                        if let Some(row) =
                            build_invoice_from_fields(fields, std::mem::take(&mut current_items))?
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
        return Err(anyhow!("У XML не знайдено жодної накладної"));
    }

    Ok(rows)
}

pub async fn import_invoices_from_file(
    pool: &PgPool,
    company_id: Uuid,
    path: &Path,
    dry_run: bool,
) -> Result<InvoiceImportReport> {
    let rows = parse_invoices_file(path).await?;
    apply_imported_invoices(pool, company_id, &rows, dry_run).await
}

pub async fn apply_imported_invoices(
    pool: &PgPool,
    company_id: Uuid,
    rows: &[ImportedInvoice],
    dry_run: bool,
) -> Result<InvoiceImportReport> {
    let mut report = InvoiceImportReport {
        parsed: rows.len(),
        ..InvoiceImportReport::default()
    };

    for row in rows {
        if row.counterparty_bas_id.is_none()
            && row.counterparty_edrpou.is_none()
            && row.counterparty_name.is_some()
        {
            let matches = db::counterparties::list_by_name_exact(
                pool,
                company_id,
                row.counterparty_name.as_deref().unwrap_or_default(),
            )
            .await?;
            if matches.len() > 1 {
                report.conflicts += 1;
                report.skipped += 1;
                report.rows.push(InvoiceImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: InvoiceImportAction::Conflict,
                    note: Some(format!(
                        "conflict: знайдено {} контрагентів за точною назвою",
                        matches.len()
                    )),
                });
                continue;
            }
        }

        let Some(counterparty) = resolve_counterparty_id(pool, company_id, row).await? else {
            report.skipped += 1;
            report.rows.push(InvoiceImportPlanRow {
                bas_id: row.bas_id.clone(),
                number: row.number.clone(),
                action: InvoiceImportAction::Skip,
                note: Some("Не знайдено контрагента за bas_id / ЄДРПОУ / назвою".to_string()),
            });
            continue;
        };

        if row.contract_bas_id.is_none() && row.contract_number.is_some() {
            let matches = db::contracts::list_by_number_exact(
                pool,
                company_id,
                counterparty.value,
                row.contract_number.as_deref().unwrap_or_default(),
            )
            .await?;
            if matches.len() > 1 {
                report.conflicts += 1;
                report.skipped += 1;
                report.rows.push(InvoiceImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: InvoiceImportAction::Conflict,
                    note: Some(format!(
                        "conflict: знайдено {} договорів за тим самим номером",
                        matches.len()
                    )),
                });
                continue;
            }
        }

        let contract = resolve_contract_id(pool, company_id, counterparty.value, row).await?;
        if (row.contract_bas_id.is_some() || row.contract_number.is_some()) && contract.is_none() {
            report.skipped += 1;
            report.rows.push(InvoiceImportPlanRow {
                bas_id: row.bas_id.clone(),
                number: row.number.clone(),
                action: InvoiceImportAction::Skip,
                note: Some("Не знайдено договір за bas_id / номером".to_string()),
            });
            continue;
        }

        let contract_id = contract.as_ref().map(|item| item.value);
        if row.bas_id.is_none() {
            let exact_matches = db::invoices::list_import_candidates(
                pool,
                company_id,
                counterparty.value,
                contract_id,
                &row.number,
                row.direction,
                row.date,
                row.total_amount,
            )
            .await?;
            if exact_matches.len() > 1 {
                report.conflicts += 1;
                report.skipped += 1;
                report.rows.push(InvoiceImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: InvoiceImportAction::Conflict,
                    note: Some(format!(
                        "conflict: знайдено {} накладних за точним fingerprint",
                        exact_matches.len()
                    )),
                });
                continue;
            }

            let loose_matches = db::invoices::list_import_candidates_loose(
                pool,
                company_id,
                counterparty.value,
                contract_id,
                &row.number,
                row.direction,
                row.date,
                row.total_amount,
                import_duplicate_tolerance(),
            )
            .await?;
            if exact_matches.is_empty() && loose_matches.len() > 1 {
                report.conflicts += 1;
                report.skipped += 1;
                report.rows.push(InvoiceImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: InvoiceImportAction::Conflict,
                    note: Some(format!(
                        "conflict: знайдено {} накладних у tolerant preview",
                        loose_matches.len()
                    )),
                });
                continue;
            }
        }

        let (existing, duplicate_source): (_, Option<&'static str>) = match row.bas_id.as_deref() {
            Some(bas_id) => (
                db::invoices::find_by_bas_id_scoped(pool, company_id, bas_id).await?,
                Some("bas_id"),
            ),
            None => {
                if let Some(found) = db::invoices::find_import_candidate(
                    pool,
                    company_id,
                    counterparty.value,
                    contract_id,
                    &row.number,
                    row.direction,
                    row.date,
                    row.total_amount,
                )
                .await?
                {
                    (Some(found), Some("header fingerprint"))
                } else {
                    (
                        db::invoices::find_import_candidate_loose(
                            pool,
                            company_id,
                            counterparty.value,
                            contract_id,
                            &row.number,
                            row.direction,
                            row.date,
                            row.total_amount,
                            import_duplicate_tolerance(),
                        )
                        .await?,
                        Some("header fingerprint + tolerant total"),
                    )
                }
            }
        };

        let imported_items = to_new_invoice_items(&row.items);
        let note = build_import_note(&counterparty, contract.as_ref(), duplicate_source);

        match existing {
            Some(invoice) => {
                let loaded = db::invoices::get_by_id_scoped(pool, company_id, invoice.id)
                    .await?
                    .ok_or_else(|| {
                        anyhow!(
                            "Накладну знайдено для preview, але не вдалося перечитати її позиції"
                        )
                    })?;
                if let Some(conflict_note) = detect_invoice_conflict(
                    &loaded.0,
                    &loaded.1,
                    counterparty.value,
                    contract_id,
                    row,
                    duplicate_source,
                ) {
                    report.conflicts += 1;
                    report.skipped += 1;
                    report.rows.push(InvoiceImportPlanRow {
                        bas_id: row.bas_id.clone(),
                        number: row.number.clone(),
                        action: InvoiceImportAction::Conflict,
                        note: Some(conflict_note),
                    });
                    continue;
                }

                report.updated += 1;
                report.rows.push(InvoiceImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: InvoiceImportAction::Update,
                    note,
                });
                if !dry_run {
                    let _ = db::invoices::update_imported_with_items(
                        pool,
                        invoice.id,
                        counterparty.value,
                        contract_id,
                        &row.number,
                        row.direction,
                        row.date,
                        row.expected_payment_date,
                        row.vat_amount,
                        row.status.clone(),
                        row.notes.as_deref(),
                        &imported_items,
                    )
                    .await?;
                }
            }
            None => {
                report.created += 1;
                report.rows.push(InvoiceImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: InvoiceImportAction::Create,
                    note,
                });
                if !dry_run {
                    let _ = db::invoices::create_imported_with_items(
                        pool,
                        company_id,
                        counterparty.value,
                        contract_id,
                        &row.number,
                        row.direction,
                        row.date,
                        row.expected_payment_date,
                        row.vat_amount,
                        row.status.clone(),
                        row.notes.as_deref(),
                        row.bas_id.as_deref(),
                        &imported_items,
                    )
                    .await?;
                }
            }
        }
    }

    Ok(report)
}

async fn resolve_counterparty_id(
    pool: &PgPool,
    company_id: Uuid,
    row: &ImportedInvoice,
) -> Result<Option<Resolution<Uuid>>> {
    if let Some(counterparty_bas_id) = row.counterparty_bas_id.as_deref() {
        if let Some(counterparty) =
            db::counterparties::find_by_bas_id_scoped(pool, company_id, counterparty_bas_id).await?
        {
            return Ok(Some(Resolution {
                value: counterparty.id,
                source: "counterparty bas_id",
            }));
        }
    }

    if let Some(edrpou) = row.counterparty_edrpou.as_deref() {
        if let Some(counterparty) =
            db::counterparties::find_by_edrpou(pool, company_id, edrpou).await?
        {
            return Ok(Some(Resolution {
                value: counterparty.id,
                source: "counterparty ЄДРПОУ",
            }));
        }
    }

    if let Some(name) = row.counterparty_name.as_deref() {
        if let Some(counterparty) = db::counterparties::find_by_name(pool, company_id, name).await?
        {
            return Ok(Some(Resolution {
                value: counterparty.id,
                source: "counterparty exact name",
            }));
        }
    }

    Ok(None)
}

async fn resolve_contract_id(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    row: &ImportedInvoice,
) -> Result<Option<Resolution<Uuid>>> {
    if let Some(contract_bas_id) = row.contract_bas_id.as_deref() {
        if let Some(contract) =
            db::contracts::find_by_bas_id_scoped(pool, company_id, contract_bas_id)
                .await?
                .filter(|contract| contract.counterparty_id == counterparty_id)
        {
            return Ok(Some(Resolution {
                value: contract.id,
                source: "contract bas_id",
            }));
        }
    }

    if let Some(contract_number) = row.contract_number.as_deref() {
        if let Some(contract) =
            db::contracts::find_by_number(pool, company_id, counterparty_id, contract_number)
                .await?
        {
            return Ok(Some(Resolution {
                value: contract.id,
                source: "contract number",
            }));
        }
    }

    Ok(None)
}

fn build_import_note(
    counterparty: &Resolution<Uuid>,
    contract: Option<&Resolution<Uuid>>,
    duplicate_source: Option<&'static str>,
) -> Option<String> {
    let mut parts = vec![format!("cp: {}", counterparty.source)];
    if let Some(contract) = contract {
        parts.push(format!("contract: {}", contract.source));
    }
    if let Some(duplicate_source) = duplicate_source {
        parts.push(format!("match: {}", duplicate_source));
    }
    Some(parts.join("; "))
}

fn detect_invoice_conflict(
    existing: &Invoice,
    existing_items: &[InvoiceItem],
    counterparty_id: Uuid,
    contract_id: Option<Uuid>,
    imported: &ImportedInvoice,
    duplicate_source: Option<&'static str>,
) -> Option<String> {
    if existing.counterparty_id != counterparty_id {
        return Some(
            "conflict: імпортована накладна прив'язується до іншого контрагента".to_string(),
        );
    }
    if existing.contract_id != contract_id {
        return Some("conflict: імпортована накладна прив'язується до іншого договору".to_string());
    }
    if existing.direction != imported.direction {
        return Some("conflict: напрям накладної не збігається з existing row".to_string());
    }
    if existing.date != imported.date {
        return Some("conflict: дата накладної не збігається з existing row".to_string());
    }
    if normalize_optional_text(Some(&existing.number))
        != normalize_optional_text(Some(&imported.number))
    {
        return Some("conflict: номер накладної не збігається з existing row".to_string());
    }
    if matches!(
        duplicate_source,
        Some("header fingerprint + tolerant total")
    ) && existing.total_amount != imported.total_amount
    {
        return Some(
            "conflict: tolerant match знайшов схожу накладну, але сума відрізняється".to_string(),
        );
    }

    let imported_items = imported
        .items
        .iter()
        .map(|item| {
            (
                normalize_text(&item.description),
                normalize_optional_text(item.unit.as_deref()),
                item.quantity,
                item.price,
            )
        })
        .collect::<Vec<_>>();
    let stored_items = existing_items
        .iter()
        .map(|item| {
            (
                normalize_text(&item.description),
                normalize_optional_text(item.unit.as_deref()),
                item.quantity,
                item.price,
            )
        })
        .collect::<Vec<_>>();
    if imported_items.len() != stored_items.len() {
        return Some(
            "conflict: кількість позицій накладної не збігається з existing row".to_string(),
        );
    }
    if imported_items != stored_items && row_identity_is_weak(imported) {
        return Some(
            "conflict: позиції накладної відрізняються, а match був не по bas_id".to_string(),
        );
    }

    None
}

fn row_identity_is_weak(imported: &ImportedInvoice) -> bool {
    imported.bas_id.is_none()
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

async fn parse_invoices_excel_file(path: &Path) -> Result<Vec<ImportedInvoice>> {
    let path = path.to_path_buf();
    task::spawn_blocking(move || {
        let mut workbook = open_workbook_auto(&path)
            .with_context(|| format!("Не вдалося відкрити Excel файл {}", path.display()))?;
        let sheet_name = workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("У Excel файлі немає жодного sheet"))?;
        let range = workbook.worksheet_range(&sheet_name)?;

        let mut rows_iter = range.rows();
        let headers = rows_iter
            .next()
            .ok_or_else(|| anyhow!("Excel файл не містить заголовків"))?
            .iter()
            .map(|cell| normalize_tag(&cell.to_string()))
            .collect::<Vec<_>>();

        let mut grouped: BTreeMap<String, ImportedInvoice> = BTreeMap::new();
        for row in rows_iter {
            let mut fields = BTreeMap::new();
            for (idx, cell) in row.iter().enumerate() {
                let value = cell.to_string().trim().to_string();
                if value.is_empty() {
                    continue;
                }
                if let Some(field) = headers
                    .get(idx)
                    .and_then(|header| map_invoice_field_name(header))
                {
                    fields.entry(field.to_string()).or_insert(value.clone());
                }
                if let Some(field) = headers
                    .get(idx)
                    .and_then(|header| map_invoice_item_field_name(header))
                {
                    fields.entry(field.to_string()).or_insert(value);
                }
            }

            if let Some(invoice) = build_invoice_from_fields(fields, Vec::new())? {
                let key = invoice_merge_key(&invoice);
                if let Some(existing) = grouped.get_mut(&key) {
                    merge_invoice(existing, invoice)?;
                } else {
                    grouped.insert(key, invoice);
                }
            }
        }

        let rows = grouped.into_values().collect::<Vec<_>>();
        if rows.is_empty() {
            return Err(anyhow!("У Excel не знайдено жодної накладної"));
        }

        Ok(rows)
    })
    .await
    .context("Excel parser для накладних завершився помилкою")?
}

fn build_invoice_from_fields(
    fields: BTreeMap<String, String>,
    mut items: Vec<ImportedInvoiceItem>,
) -> Result<Option<ImportedInvoice>> {
    let number = required_text(&fields, "number");
    let Some(number) = number else {
        return Ok(None);
    };

    let date = parse_date(
        fields
            .get("date")
            .map(String::as_str)
            .ok_or_else(|| anyhow!("У накладній {number} відсутня дата"))?,
    )?;

    if let Some(item) = build_invoice_item_from_fields(fields.clone())? {
        items.push(item);
    }

    let total_amount =
        optional_decimal(fields.get("total_amount"))?.or(optional_decimal(fields.get("amount"))?);

    if items.is_empty() {
        let total_amount = total_amount
            .ok_or_else(|| anyhow!("У накладній {number} відсутні позиції або підсумкова сума"))?;
        items.push(ImportedInvoiceItem {
            description: clean_optional(fields.get("subject"))
                .unwrap_or_else(|| format!("Імпорт з BAS: {number}")),
            unit: clean_optional(fields.get("unit")).or(Some("шт".to_string())),
            quantity: Decimal::ONE,
            price: total_amount,
        });
    }

    let computed_total = items
        .iter()
        .fold(Decimal::ZERO, |acc, item| {
            acc + (item.quantity * item.price).round_dp(2)
        })
        .round_dp(2);
    let total_amount = total_amount.unwrap_or(computed_total).round_dp(2);

    Ok(Some(ImportedInvoice {
        bas_id: clean_optional(fields.get("bas_id")),
        counterparty_bas_id: clean_optional(fields.get("counterparty_bas_id")),
        counterparty_name: clean_optional(fields.get("counterparty_name")),
        counterparty_edrpou: clean_optional(fields.get("counterparty_edrpou")),
        contract_bas_id: clean_optional(fields.get("contract_bas_id")),
        contract_number: clean_optional(fields.get("contract_number")),
        number,
        date,
        expected_payment_date: optional_date(fields.get("expected_payment_date"))?,
        direction: parse_direction(fields.get("direction").map(String::as_str)),
        status: parse_invoice_status(fields.get("status").map(String::as_str)),
        total_amount,
        vat_amount: optional_decimal(fields.get("vat_amount"))?.unwrap_or(Decimal::ZERO),
        notes: clean_optional(fields.get("notes")),
        items,
    }))
}

fn build_invoice_item_from_fields(
    fields: BTreeMap<String, String>,
) -> Result<Option<ImportedInvoiceItem>> {
    let description = required_text(&fields, "item_description");
    let Some(description) = description else {
        return Ok(None);
    };

    let quantity = optional_decimal(fields.get("item_quantity"))?.unwrap_or(Decimal::ONE);
    let amount = optional_decimal(fields.get("item_amount"))?;
    let price = match optional_decimal(fields.get("item_price"))? {
        Some(price) => price,
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

    Ok(Some(ImportedInvoiceItem {
        description,
        unit: clean_optional(fields.get("item_unit")),
        quantity,
        price,
    }))
}

fn merge_invoice(target: &mut ImportedInvoice, incoming: ImportedInvoice) -> Result<()> {
    if target.number != incoming.number
        || target.date != incoming.date
        || target.direction != incoming.direction
    {
        bail!("Excel grouping для накладних отримав несумісні header rows");
    }

    if target.bas_id.is_none() {
        target.bas_id = incoming.bas_id;
    }
    if target.counterparty_bas_id.is_none() {
        target.counterparty_bas_id = incoming.counterparty_bas_id;
    }
    if target.counterparty_name.is_none() {
        target.counterparty_name = incoming.counterparty_name;
    }
    if target.counterparty_edrpou.is_none() {
        target.counterparty_edrpou = incoming.counterparty_edrpou;
    }
    if target.contract_bas_id.is_none() {
        target.contract_bas_id = incoming.contract_bas_id;
    }
    if target.contract_number.is_none() {
        target.contract_number = incoming.contract_number;
    }
    if target.expected_payment_date.is_none() {
        target.expected_payment_date = incoming.expected_payment_date;
    }
    if target.notes.is_none() {
        target.notes = incoming.notes;
    }
    target.vat_amount = target.vat_amount.max(incoming.vat_amount);
    target.items.extend(incoming.items);
    target.total_amount = target
        .items
        .iter()
        .fold(Decimal::ZERO, |acc, item| {
            acc + (item.quantity * item.price).round_dp(2)
        })
        .round_dp(2);

    Ok(())
}

fn invoice_merge_key(row: &ImportedInvoice) -> String {
    if let Some(bas_id) = row.bas_id.as_deref() {
        return format!("bas:{bas_id}");
    }

    format!(
        "{}|{}|{}|{}|{}",
        row.number,
        row.date,
        row.counterparty_bas_id.as_deref().unwrap_or(""),
        row.counterparty_name.as_deref().unwrap_or(""),
        row.contract_bas_id.as_deref().unwrap_or("")
    )
}

fn to_new_invoice_items(items: &[ImportedInvoiceItem]) -> Vec<NewInvoiceItem> {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| NewInvoiceItem {
            position: (index + 1) as i16,
            description: item.description.clone(),
            unit: item.unit.clone(),
            quantity: item.quantity,
            price: item.price,
        })
        .collect()
}

fn parse_direction(raw: Option<&str>) -> DocumentDirection {
    match raw.map(normalize_value).as_deref() {
        Some("incoming") | Some("вхідний") | Some("вхідна") => {
            DocumentDirection::Incoming
        }
        _ => DocumentDirection::Outgoing,
    }
}

fn parse_invoice_status(raw: Option<&str>) -> InvoiceStatus {
    match raw.map(normalize_value).as_deref() {
        Some("draft") | Some("чернетка") => InvoiceStatus::Draft,
        Some("signed") | Some("підписано") => InvoiceStatus::Signed,
        Some("paid") | Some("оплачено") => InvoiceStatus::Paid,
        _ => InvoiceStatus::Issued,
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

fn is_invoice_record_tag(tag: &str) -> bool {
    matches!(
        tag,
        "invoice" | "document" | "record" | "row" | "накладна" | "рахунок"
    )
}

fn is_invoice_item_tag(tag: &str) -> bool {
    matches!(tag, "item" | "позиція" | "position" | "товар" | "service")
}

fn map_invoice_field_name(tag: &str) -> Option<&'static str> {
    match tag {
        "id" | "bas_id" | "basid" | "uid" | "uuid" | "код" => Some("bas_id"),
        "number" | "номер" | "num" => Some("number"),
        "date" | "дата" | "documentdate" => Some("date"),
        "expected_payment_date" | "payment_date" | "expecteddate" | "датаплатежу" => {
            Some("expected_payment_date")
        }
        "direction" | "напрям" => Some("direction"),
        "status" | "статус" => Some("status"),
        "notes" | "comment" | "примітка" | "коментар" => Some("notes"),
        "subject" | "title" | "name" | "назва" => Some("subject"),
        "amount" | "sum" | "total" | "сума" => Some("amount"),
        "total_amount" | "documenttotal" | "totalsum" | "підсумоксума" | "сумадокумента" => {
            Some("total_amount")
        }
        "vat_amount" | "vat" | "пдв" | "сумпдв" | "суммапдв" => Some("vat_amount"),
        "counterparty_id"
        | "counterparty_bas_id"
        | "client_id"
        | "partner_id"
        | "контрагентid"
        | "кодконтрагента"
        | "контрагенткод" => Some("counterparty_bas_id"),
        "counterparty"
        | "counterparty_name"
        | "client"
        | "partner"
        | "контрагент"
        | "найменуванняконтрагента"
        | "назваконтрагента" => Some("counterparty_name"),
        "counterparty_edrpou" | "edrpou" | "єдрпоу" | "едрпоу" => {
            Some("counterparty_edrpou")
        }
        "contract_id" | "contract_bas_id" | "договірid" | "коддоговору" | "contractref" => {
            Some("contract_bas_id")
        }
        "contract_number" | "contract" | "договір" | "номердоговору" => {
            Some("contract_number")
        }
        _ => None,
    }
}

fn map_invoice_item_field_name(tag: &str) -> Option<&'static str> {
    match tag {
        "description" | "item_description" | "name" | "title" | "опис" | "номенклатура" => {
            Some("item_description")
        }
        "quantity" | "qty" | "кількість" | "количество" => Some("item_quantity"),
        "unit" | "одиниця" | "единица" => Some("item_unit"),
        "price" | "item_price" | "ціна" | "цена" => Some("item_price"),
        "amount" | "item_amount" | "sum" | "total" | "сума" => Some("item_amount"),
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
    fn parse_invoices_xml_reads_header_rows_with_items() {
        let xml = r#"
            <invoices>
                <invoice>
                    <id>inv-001</id>
                    <counterparty_id>cp-001</counterparty_id>
                    <contract_id>ctr-001</contract_id>
                    <number>ВН-001</number>
                    <date>2026-04-10</date>
                    <item>
                        <description>Ноутбук</description>
                        <quantity>2</quantity>
                        <unit>шт</unit>
                        <price>12500,50</price>
                    </item>
                    <vat_amount>2500,10</vat_amount>
                    <status>paid</status>
                </invoice>
            </invoices>
        "#;

        let rows = parse_invoices_xml(xml).expect("парсинг накладних має спрацювати");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bas_id.as_deref(), Some("inv-001"));
        assert_eq!(rows[0].counterparty_bas_id.as_deref(), Some("cp-001"));
        assert_eq!(rows[0].contract_bas_id.as_deref(), Some("ctr-001"));
        assert_eq!(rows[0].items.len(), 1);
        assert_eq!(rows[0].items[0].quantity, Decimal::new(2, 0));
        assert_eq!(rows[0].items[0].price, Decimal::new(1250050, 2));
        assert_eq!(rows[0].vat_amount, Decimal::new(250010, 2));
        assert_eq!(rows[0].status, InvoiceStatus::Paid);
        assert_eq!(rows[0].total_amount, Decimal::new(2500100, 2));
    }

    #[test]
    fn parse_invoices_xml_builds_fallback_item_from_total() {
        let xml = r#"
            <root>
                <накладна>
                    <код>inv-002</код>
                    <кодконтрагента>cp-002</кодконтрагента>
                    <назваконтрагента>ТОВ Тест</назваконтрагента>
                    <номер>ВН-002</номер>
                    <дата>11.04.2026</дата>
                    <сума>3200</сума>
                </накладна>
            </root>
        "#;

        let rows = parse_invoices_xml(xml).expect("fallback значення мають працювати");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].direction, DocumentDirection::Outgoing);
        assert_eq!(rows[0].status, InvoiceStatus::Issued);
        assert_eq!(rows[0].vat_amount, Decimal::ZERO);
        assert_eq!(rows[0].items.len(), 1);
        assert_eq!(rows[0].total_amount, Decimal::new(3200, 0));
    }

    #[test]
    fn parse_invoices_xml_fails_when_no_records_found() {
        let error = parse_invoices_xml("<root/>").expect_err("порожній XML має давати помилку");
        assert!(error.to_string().contains("не знайдено жодної накладної"));
    }
}
