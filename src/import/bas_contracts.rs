use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use quick_xml::events::Event;
use quick_xml::Reader;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tokio::fs;
use uuid::Uuid;

use crate::db;
use crate::models::contract::ContractStatus;

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
    pub rows: Vec<ContractImportPlanRow>,
}

pub async fn parse_contracts_xml_file(path: &Path) -> Result<Vec<ImportedContract>> {
    let xml_text = fs::read_to_string(path)
        .await
        .with_context(|| format!("Не вдалося прочитати файл {}", path.display()))?;
    parse_contracts_xml(&xml_text)
}

pub fn parse_contracts_xml(xml: &str) -> Result<Vec<ImportedContract>> {
    let mut reader = Reader::from_str(xml);
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
        let Some(counterparty_id) =
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

        let existing = match row.bas_id.as_deref() {
            Some(bas_id) => db::contracts::find_by_bas_id(pool, bas_id).await?,
            None => None,
        };

        match existing {
            Some(contract) => {
                report.updated += 1;
                report.rows.push(ContractImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    number: row.number.clone(),
                    action: ContractImportAction::Update,
                    note: None,
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
                    note: None,
                });
                if !dry_run {
                    let _ = db::contracts::create_imported(
                        pool,
                        company_id,
                        counterparty_id,
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
) -> Result<Option<Uuid>> {
    let Some(counterparty_bas_id) = counterparty_bas_id else {
        return Ok(None);
    };

    Ok(
        db::counterparties::find_by_bas_id(pool, counterparty_bas_id)
            .await?
            .map(|cp| cp.id),
    )
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
    fn parse_contracts_xml_supports_ukrainian_tags() {
        let xml = r#"
            <root>
                <договір>
                    <код>ctr-002</код>
                    <кодконтрагента>cp-002</кодконтрагента>
                    <номер>ДГ-002</номер>
                    <дата>15.04.2026</дата>
                    <сума>8 499,99</сума>
                </договір>
            </root>
        "#;

        let rows = parse_contracts_xml(xml).expect("українські теги мають парситися");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount, Some(Decimal::new(849999, 2)));
        assert_eq!(
            rows[0].date,
            NaiveDate::from_ymd_opt(2026, 4, 15).expect("валідна дата")
        );
    }

    #[test]
    fn parse_contracts_xml_fails_when_no_records_found() {
        let error = parse_contracts_xml("<root/>").expect_err("порожній XML має давати помилку");
        assert!(error.to_string().contains("не знайдено жодного договору"));
    }
}
