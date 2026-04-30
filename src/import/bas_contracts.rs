use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
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
use crate::models::contract::{Contract, ContractStatus};

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedContract {
    pub bas_id: Option<String>,
    pub counterparty_bas_id: Option<String>,
    pub number: String,
    pub subject: Option<String>,
    pub date: NaiveDate,
    pub expires_at: Option<NaiveDate>,
    pub amount: Option<Decimal>,
    pub notes: Option<String>,
    pub status: ContractStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractImportAction {
    Create,
    Update,
    Skip,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractImportPlanRow {
    pub bas_id: Option<String>,
    pub number: String,
    pub action: ContractImportAction,
    pub note: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContractImportReport {
    pub parsed: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub rows: Vec<ContractImportPlanRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Resolution<T> {
    value: T,
    source: &'static str,
}

pub async fn parse_contracts_xml_file(path: &Path) -> Result<Vec<ImportedContract>> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "xlsx" | "xls" => parse_contracts_excel_file(path).await,
        _ => {
            let xml_text = fs::read_to_string(path)
                .await
                .with_context(|| format!("Не вдалося прочитати файл {}", path.display()))?;
            parse_contracts_xml(&xml_text)
        }
    }
}

pub fn parse_contracts_xml(xml: &str) -> Result<Vec<ImportedContract>> {
    let mut reader = XmlReader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut current_fields: Option<BTreeMap<String, String>> = None;
    let mut rows = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) => {
                let tag = normalize_tag(&String::from_utf8_lossy(event.name().as_ref()));
                let is_record = is_contract_record_tag(&tag);
                stack.push(tag.clone());
                if is_record && current_fields.is_none() {
                    current_fields = Some(BTreeMap::new());
                }
            }
            Event::Text(event) => {
                if let Some(fields) = current_fields.as_mut() {
                    let value = String::from_utf8_lossy(event.as_ref()).trim().to_string();
                    if !value.is_empty() {
                        if let Some(tag) = stack.last() {
                            if let Some(field) = map_contract_field_name(tag) {
                                fields.entry(field.to_string()).or_insert(value);
                            }
                        }
                    }
                }
            }
            Event::End(event) => {
                let tag = normalize_tag(&String::from_utf8_lossy(event.name().as_ref()));
                if is_contract_record_tag(&tag) {
                    if let Some(fields) = current_fields.take() {
                        if let Some(row) = build_contract_from_fields(fields)? {
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
        return Err(anyhow!("У XML не знайдено жодного договору"));
    }

    Ok(rows)
}

pub async fn import_contracts_from_xml(
    pool: &PgPool,
    company_id: Uuid,
    path: &Path,
    dry_run: bool,
) -> Result<ContractImportReport> {
    let rows = parse_contracts_xml_file(path).await?;
    apply_imported_contracts(pool, company_id, &rows, dry_run).await
}

pub async fn apply_imported_contracts(
    pool: &PgPool,
    company_id: Uuid,
    rows: &[ImportedContract],
    dry_run: bool,
) -> Result<ContractImportReport> {
    let mut report = ContractImportReport {
        parsed: rows.len(),
        ..ContractImportReport::default()
    };

    for row in rows {
        let Some(counterparty) =
            resolve_counterparty_id(pool, row.counterparty_bas_id.as_deref()).await?
        else {
            report.skipped += 1;
            report.rows.push(ContractImportPlanRow {
                bas_id: row.bas_id.clone(),
                number: row.number.clone(),
                action: ContractImportAction::Skip,
                note: Some("Не знайдено контрагента за bas_id".to_string()),
            });
            continue;
        };

        if row.bas_id.is_none() {
            let matches = db::contracts::list_by_number_exact(
                pool,
                company_id,
                counterparty.value,
                &row.number,
            )
            .await?;
            if matches.len() > 1 {
                report.conflicts += 1;
                report.skipped += 1;
                report.rows.push(ContractImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: ContractImportAction::Conflict,
                    note: Some(format!(
                        "conflict: знайдено {} договорів за тим самим номером",
                        matches.len()
                    )),
                });
                continue;
            }
        }

        let (existing, match_source): (_, Option<&'static str>) = match row.bas_id.as_deref() {
            Some(bas_id) => (
                db::contracts::find_by_bas_id(pool, bas_id).await?,
                Some("bas_id"),
            ),
            None => (
                db::contracts::find_by_number(pool, company_id, counterparty.value, &row.number)
                    .await?,
                Some("contract number"),
            ),
        };
        let note = Some(build_contract_note(&counterparty, match_source));

        match existing {
            Some(contract) => {
                if let Some(conflict_note) =
                    detect_contract_conflict(&contract, company_id, counterparty.value, row)
                {
                    report.conflicts += 1;
                    report.skipped += 1;
                    report.rows.push(ContractImportPlanRow {
                        bas_id: row.bas_id.clone(),
                        number: row.number.clone(),
                        action: ContractImportAction::Conflict,
                        note: Some(conflict_note),
                    });
                    continue;
                }

                report.updated += 1;
                report.rows.push(ContractImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: ContractImportAction::Update,
                    note,
                });
                if !dry_run {
                    let _ = db::contracts::update_imported(
                        pool,
                        contract.id,
                        &row.number,
                        row.subject.as_deref(),
                        row.date,
                        row.expires_at,
                        row.amount,
                        row.status.clone(),
                        row.notes.as_deref(),
                    )
                    .await?;
                }
            }
            None => {
                report.created += 1;
                report.rows.push(ContractImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: ContractImportAction::Create,
                    note,
                });
                if !dry_run {
                    let _ = db::contracts::create_imported(
                        pool,
                        company_id,
                        counterparty.value,
                        &row.number,
                        row.subject.as_deref(),
                        row.date,
                        row.expires_at,
                        row.amount,
                        row.notes.as_deref(),
                        row.bas_id.as_deref(),
                        row.status.clone(),
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

fn build_contract_note(
    counterparty: &Resolution<Uuid>,
    match_source: Option<&'static str>,
) -> String {
    let mut parts = vec![format!("cp: {}", counterparty.source)];
    if let Some(match_source) = match_source {
        parts.push(format!("match: {}", match_source));
    }
    parts.join("; ")
}

fn detect_contract_conflict(
    existing: &Contract,
    company_id: Uuid,
    counterparty_id: Uuid,
    imported: &ImportedContract,
) -> Option<String> {
    if existing.company_id != company_id {
        return Some("conflict: bas_id вказує на договір іншої компанії".to_string());
    }
    if existing.counterparty_id != counterparty_id {
        return Some(
            "conflict: імпортований договір прив'язується до іншого контрагента".to_string(),
        );
    }
    if normalize_optional_text(Some(&existing.number))
        != normalize_optional_text(Some(&imported.number))
    {
        return Some("conflict: номер договору не збігається з existing row".to_string());
    }
    None
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase())
}

async fn parse_contracts_excel_file(path: &Path) -> Result<Vec<ImportedContract>> {
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

        let mut rows = Vec::new();
        for row in rows_iter {
            let mut fields = BTreeMap::new();
            for (idx, cell) in row.iter().enumerate() {
                let value = cell.to_string().trim().to_string();
                if value.is_empty() {
                    continue;
                }
                if let Some(field) = headers
                    .get(idx)
                    .and_then(|header| map_contract_field_name(header))
                {
                    fields.entry(field.to_string()).or_insert(value);
                }
            }

            if let Some(contract) = build_contract_from_fields(fields)? {
                rows.push(contract);
            }
        }

        if rows.is_empty() {
            return Err(anyhow!("У Excel не знайдено жодного договору"));
        }

        Ok(rows)
    })
    .await
    .context("Excel parser для договорів завершився помилкою")?
}

fn build_contract_from_fields(
    fields: BTreeMap<String, String>,
) -> Result<Option<ImportedContract>> {
    let number = required_text(&fields, "number");
    let Some(number) = number else {
        return Ok(None);
    };

    let date = parse_date(
        fields
            .get("date")
            .map(String::as_str)
            .ok_or_else(|| anyhow!("У договорі {number} відсутня дата"))?,
    )?;

    Ok(Some(ImportedContract {
        bas_id: clean_optional(fields.get("bas_id")),
        counterparty_bas_id: clean_optional(fields.get("counterparty_bas_id")),
        number,
        subject: clean_optional(fields.get("subject")),
        date,
        expires_at: optional_date(fields.get("expires_at"))?,
        amount: optional_decimal(fields.get("amount"))?,
        notes: clean_optional(fields.get("notes")),
        status: parse_contract_status(fields.get("status").map(String::as_str)),
    }))
}

fn parse_contract_status(raw: Option<&str>) -> ContractStatus {
    match raw.map(normalize_value).as_deref() {
        Some("expired") | Some("прострочений") | Some("завершений") => {
            ContractStatus::Expired
        }
        Some("terminated") | Some("розірваний") => ContractStatus::Terminated,
        _ => ContractStatus::Active,
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

fn is_contract_record_tag(tag: &str) -> bool {
    matches!(
        tag,
        "договір" | "договор" | "contract" | "record" | "item" | "row"
    )
}

fn map_contract_field_name(tag: &str) -> Option<&'static str> {
    match tag {
        "id" | "bas_id" | "basid" | "uid" | "uuid" | "код" => Some("bas_id"),
        "number" | "номер" | "num" => Some("number"),
        "subject" | "title" | "name" | "предмет" | "назва" => Some("subject"),
        "date" | "signed_at" | "дата" | "датадоговору" => Some("date"),
        "expires_at" | "valid_to" | "date_to" | "терміндії" | "кінцевадата" => {
            Some("expires_at")
        }
        "amount" | "sum" | "total" | "сума" => Some("amount"),
        "status" | "статус" => Some("status"),
        "notes" | "comment" | "description" | "примітка" | "коментар" => {
            Some("notes")
        }
        "counterparty_id"
        | "counterparty_bas_id"
        | "client_id"
        | "partner_id"
        | "контрагентid"
        | "кодконтрагента"
        | "контрагенткод" => Some("counterparty_bas_id"),
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
    fn parse_contracts_xml_reads_basic_rows() {
        let xml = r#"
            <contracts>
                <contract>
                    <id>ctr-001</id>
                    <counterparty_id>cp-001</counterparty_id>
                    <number>ДГ-001</number>
                    <date>2026-04-01</date>
                    <expires_at>2026-12-31</expires_at>
                    <amount>12500.50</amount>
                    <subject>Абонентське обслуговування</subject>
                </contract>
            </contracts>
        "#;

        let rows = parse_contracts_xml(xml).expect("парсинг договорів має спрацювати");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bas_id.as_deref(), Some("ctr-001"));
        assert_eq!(rows[0].counterparty_bas_id.as_deref(), Some("cp-001"));
        assert_eq!(rows[0].number, "ДГ-001");
        assert_eq!(rows[0].amount, Some(Decimal::new(1250050, 2)));
    }

    #[test]
    fn build_contract_from_fields_reads_excel_like_headers() {
        let mut fields = BTreeMap::new();
        fields.insert("bas_id".to_string(), "ctr-002".to_string());
        fields.insert("counterparty_bas_id".to_string(), "cp-002".to_string());
        fields.insert("number".to_string(), "ДГ-002".to_string());
        fields.insert("date".to_string(), "2026-04-15".to_string());

        let row = build_contract_from_fields(fields)
            .expect("побудова має спрацювати")
            .expect("рядок має бути валідним");
        assert_eq!(row.bas_id.as_deref(), Some("ctr-002"));
        assert_eq!(row.number, "ДГ-002");
    }
}
