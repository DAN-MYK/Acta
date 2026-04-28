use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use sqlx::PgPool;
use tokio::fs;
use uuid::Uuid;

use crate::db;
use crate::models::counterparty::{Counterparty, NewCounterparty, UpdateCounterparty};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedCounterparty {
    pub bas_id: Option<String>,
    pub name: String,
    pub edrpou: Option<String>,
    pub ipn: Option<String>,
    pub iban: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportAction {
    Create,
    Update,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterpartyImportPlanRow {
    pub bas_id: Option<String>,
    pub name: String,
    pub action: ImportAction,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CounterpartyImportReport {
    pub parsed: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub rows: Vec<CounterpartyImportPlanRow>,
}

pub async fn parse_counterparties_xml_file(path: &Path) -> Result<Vec<ImportedCounterparty>> {
    let xml_text = fs::read_to_string(path)
        .await
        .with_context(|| format!("Не вдалося прочитати файл {}", path.display()))?;
    parse_counterparties_xml(&xml_text)
}

pub fn parse_counterparties_xml(xml: &str) -> Result<Vec<ImportedCounterparty>> {
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
                let is_record = is_record_tag(&tag);
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
                            if let Some(field) = map_field_name(tag) {
                                fields.entry(field.to_string()).or_insert(value);
                            }
                        }
                    }
                }
            }
            Event::End(event) => {
                let tag = normalize_tag(&String::from_utf8_lossy(event.name().as_ref()));
                if is_record_tag(&tag) {
                    if let Some(fields) = current_fields.take() {
                        if let Some(row) = build_counterparty_from_fields(fields)? {
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
        return Err(anyhow!("У XML не знайдено жодного контрагента"));
    }

    Ok(rows)
}

pub async fn import_counterparties_from_xml(
    pool: &PgPool,
    company_id: Uuid,
    path: &Path,
    dry_run: bool,
) -> Result<CounterpartyImportReport> {
    let rows = parse_counterparties_xml_file(path).await?;
    apply_imported_counterparties(pool, company_id, &rows, dry_run).await
}

pub async fn apply_imported_counterparties(
    pool: &PgPool,
    company_id: Uuid,
    rows: &[ImportedCounterparty],
    dry_run: bool,
) -> Result<CounterpartyImportReport> {
    let mut report = CounterpartyImportReport {
        parsed: rows.len(),
        ..CounterpartyImportReport::default()
    };

    for row in rows {
        let existing = if let Some(bas_id) = row.bas_id.as_deref() {
            db::counterparties::find_by_bas_id(pool, bas_id).await?
        } else if let Some(edrpou) = row.edrpou.as_deref() {
            db::counterparties::find_by_edrpou(pool, company_id, edrpou).await?
        } else {
            None
        };

        match existing {
            Some(counterparty) => {
                report.updated += 1;
                report.rows.push(CounterpartyImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    name: row.name.clone(),
                    action: ImportAction::Update,
                });
                if !dry_run {
                    let payload = merge_update_payload(&counterparty, row);
                    let _ = db::counterparties::update(pool, counterparty.id, &payload).await?;
                }
            }
            None => {
                report.created += 1;
                report.rows.push(CounterpartyImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    name: row.name.clone(),
                    action: ImportAction::Create,
                });
                if !dry_run {
                    let payload = NewCounterparty {
                        name: row.name.clone(),
                        edrpou: row.edrpou.clone(),
                        ipn: row.ipn.clone(),
                        iban: row.iban.clone(),
                        address: row.address.clone(),
                        phone: row.phone.clone(),
                        email: row.email.clone(),
                        notes: None,
                        bas_id: row.bas_id.clone(),
                    };
                    let _ = db::counterparties::create(pool, company_id, &payload).await?;
                }
            }
        }
    }

    Ok(report)
}

fn merge_update_payload(
    existing: &Counterparty,
    imported: &ImportedCounterparty,
) -> UpdateCounterparty {
    UpdateCounterparty {
        name: imported.name.clone(),
        edrpou: imported.edrpou.clone().or_else(|| existing.edrpou.clone()),
        ipn: imported.ipn.clone().or_else(|| existing.ipn.clone()),
        iban: imported.iban.clone().or_else(|| existing.iban.clone()),
        address: imported
            .address
            .clone()
            .or_else(|| existing.address.clone()),
        phone: imported.phone.clone().or_else(|| existing.phone.clone()),
        email: imported.email.clone().or_else(|| existing.email.clone()),
        notes: existing.notes.clone(),
    }
}

fn build_counterparty_from_fields(
    fields: BTreeMap<String, String>,
) -> Result<Option<ImportedCounterparty>> {
    let name = fields
        .get("name")
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string();

    if name.is_empty() {
        return Ok(None);
    }

    Ok(Some(ImportedCounterparty {
        bas_id: fields.get("bas_id").cloned(),
        name,
        edrpou: fields.get("edrpou").cloned(),
        ipn: fields.get("ipn").cloned(),
        iban: fields.get("iban").cloned(),
        address: fields.get("address").cloned(),
        phone: fields.get("phone").cloned(),
        email: fields.get("email").cloned(),
    }))
}

fn normalize_tag(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('{')
        .split(['}', ':'])
        .next_back()
        .unwrap_or(raw)
        .to_lowercase()
}

fn is_record_tag(tag: &str) -> bool {
    matches!(
        tag,
        "контрагент" | "counterparty" | "record" | "item" | "row"
    )
}

fn map_field_name(tag: &str) -> Option<&'static str> {
    match tag {
        "id" | "bas_id" | "basid" | "uid" | "uuid" | "ссылка" | "код" => Some("bas_id"),
        "name" | "назва" | "наименование" | "найменування" | "fullname" => {
            Some("name")
        }
        "edrpou" | "єдрпоу" | "едрпоу" | "кодєдрпоу" | "кодедрпоу" => {
            Some("edrpou")
        }
        "ipn" | "іпн" | "инн" | "рнокпп" => Some("ipn"),
        "iban" | "рахунок" | "счет" | "account" => Some("iban"),
        "address" | "адреса" | "legaladdress" | "юр_адреса" => Some("address"),
        "phone" | "телефон" => Some("phone"),
        "email" | "e-mail" | "mail" => Some("email"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_counterparties_xml_reads_basic_rows() {
        let xml = r#"
            <counterparties>
                <counterparty>
                    <id>bas-001</id>
                    <name>ТОВ Альфа</name>
                    <edrpou>12345678</edrpou>
                    <iban>UA123456789012345678901234567</iban>
                    <address>м. Київ</address>
                </counterparty>
                <counterparty>
                    <id>bas-002</id>
                    <name>ФОП Петренко</name>
                    <ipn>1234567890</ipn>
                </counterparty>
            </counterparties>
        "#;

        let rows = parse_counterparties_xml(xml).expect("XML має парситися");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "ТОВ Альфа");
        assert_eq!(rows[0].edrpou.as_deref(), Some("12345678"));
        assert_eq!(rows[1].ipn.as_deref(), Some("1234567890"));
    }

    #[test]
    fn parse_counterparties_xml_supports_ukrainian_tags() {
        let xml = r#"
            <Контрагенти>
                <Контрагент>
                    <Код>bas-010</Код>
                    <Найменування>ТОВ Ромашка</Найменування>
                    <ЄДРПОУ>87654321</ЄДРПОУ>
                    <Адреса>Львів</Адреса>
                </Контрагент>
            </Контрагенти>
        "#;

        let rows = parse_counterparties_xml(xml).expect("XML має парситися");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bas_id.as_deref(), Some("bas-010"));
        assert_eq!(rows[0].name, "ТОВ Ромашка");
        assert_eq!(rows[0].edrpou.as_deref(), Some("87654321"));
    }

    #[test]
    fn parse_counterparties_xml_skips_rows_without_name() {
        let xml = r#"
            <counterparties>
                <counterparty>
                    <id>bas-003</id>
                </counterparty>
                <counterparty>
                    <id>bas-004</id>
                    <name>ТОВ Бета</name>
                </counterparty>
            </counterparties>
        "#;

        let rows = parse_counterparties_xml(xml).expect("XML має парситися");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "ТОВ Бета");
    }
}
