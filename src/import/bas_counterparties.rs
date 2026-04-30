use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use calamine::{open_workbook_auto, Reader as CalamineReader};
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;
use sqlx::PgPool;
use tokio::fs;
use tokio::task;
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
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterpartyImportPlanRow {
    pub bas_id: Option<String>,
    pub name: String,
    pub action: ImportAction,
    pub note: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CounterpartyImportReport {
    pub parsed: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub rows: Vec<CounterpartyImportPlanRow>,
}

pub async fn parse_counterparties_xml_file(path: &Path) -> Result<Vec<ImportedCounterparty>> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "xlsx" | "xls" => parse_counterparties_excel_file(path).await,
        _ => {
            let xml_text = fs::read_to_string(path)
                .await
                .with_context(|| format!("Не вдалося прочитати файл {}", path.display()))?;
            parse_counterparties_xml(&xml_text)
        }
    }
}

pub fn parse_counterparties_xml(xml: &str) -> Result<Vec<ImportedCounterparty>> {
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
        if row.bas_id.is_none() && row.edrpou.is_none() {
            let matches =
                db::counterparties::list_by_name_exact(pool, company_id, &row.name).await?;
            if matches.len() > 1 {
                report.conflicts += 1;
                report.skipped += 1;
                report.rows.push(CounterpartyImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    name: row.name.clone(),
                    action: ImportAction::Conflict,
                    note: Some(format!(
                        "conflict: Р·РЅР°Р№РґРµРЅРѕ {} РєРѕРЅС‚СЂР°РіРµРЅС‚С–РІ Р·Р° С‚РѕС‡РЅРѕСЋ РЅР°Р·РІРѕСЋ",
                        matches.len()
                    )),
                });
                continue;
            }
        }

        let (existing, match_source) = if let Some(bas_id) = row.bas_id.as_deref() {
            (
                db::counterparties::find_by_bas_id(pool, bas_id).await?,
                Some("bas_id"),
            )
        } else if let Some(edrpou) = row.edrpou.as_deref() {
            (
                db::counterparties::find_by_edrpou(pool, company_id, edrpou).await?,
                Some("ЄДРПОУ"),
            )
        } else if let Some(counterparty) =
            db::counterparties::find_by_name(pool, company_id, &row.name).await?
        {
            (Some(counterparty), Some("exact name"))
        } else {
            (None, None)
        };

        match existing {
            Some(counterparty) => {
                report.updated += 1;
                report.rows.push(CounterpartyImportPlanRow {
                    bas_id: row.bas_id.clone(),
                    name: row.name.clone(),
                    action: ImportAction::Update,
                    note: Some(format!("match: {}", match_source.unwrap_or("existing row"))),
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
                    note: Some(build_create_note(row)),
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

fn build_create_note(row: &ImportedCounterparty) -> String {
    if row.bas_id.is_some() {
        "create: bas_id не знайдено у БД".to_string()
    } else if row.edrpou.is_some() {
        "create: ЄДРПОУ не знайдено у БД".to_string()
    } else {
        "create: не знайдено match за назвою".to_string()
    }
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

async fn parse_counterparties_excel_file(path: &Path) -> Result<Vec<ImportedCounterparty>> {
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
                if let Some(field) = headers.get(idx).and_then(|header| map_field_name(header)) {
                    fields.entry(field.to_string()).or_insert(value);
                }
            }

            if let Some(counterparty) = build_counterparty_from_fields(fields)? {
                rows.push(counterparty);
            }
        }

        if rows.is_empty() {
            return Err(anyhow!("У Excel не знайдено жодного контрагента"));
        }

        Ok(rows)
    })
    .await
    .context("Excel parser для контрагентів завершився помилкою")?
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
        edrpou: clean_optional(fields.get("edrpou")),
        ipn: clean_optional(fields.get("ipn")),
        iban: clean_optional(fields.get("iban")),
        address: clean_optional(fields.get("address")),
        phone: clean_optional(fields.get("phone")),
        email: clean_optional(fields.get("email")),
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

fn clean_optional(value: Option<&String>) -> Option<String> {
    value
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
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
                    <name>ТОВ Тест</name>
                    <edrpou>12345678</edrpou>
                </counterparty>
            </counterparties>
        "#;

        let rows = parse_counterparties_xml(xml).expect("парсинг контрагентів має спрацювати");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bas_id.as_deref(), Some("bas-001"));
        assert_eq!(rows[0].name, "ТОВ Тест");
        assert_eq!(rows[0].edrpou.as_deref(), Some("12345678"));
    }

    #[test]
    fn build_counterparty_from_fields_reads_excel_like_headers() {
        let mut fields = BTreeMap::new();
        fields.insert("bas_id".to_string(), "cp-001".to_string());
        fields.insert("name".to_string(), "ФОП Приклад".to_string());
        fields.insert("edrpou".to_string(), "12345678".to_string());

        let row = build_counterparty_from_fields(fields)
            .expect("побудова має спрацювати")
            .expect("рядок має бути валідним");
        assert_eq!(row.bas_id.as_deref(), Some("cp-001"));
        assert_eq!(row.name, "ФОП Приклад");
    }

    #[test]
    fn build_create_note_mentions_matching_strategy() {
        let with_bas = ImportedCounterparty {
            bas_id: Some("cp-001".to_string()),
            name: "ТОВ Тест".to_string(),
            edrpou: None,
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
        };
        let with_edrpou = ImportedCounterparty {
            bas_id: None,
            name: "ТОВ Тест".to_string(),
            edrpou: Some("12345678".to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
        };
        let name_only = ImportedCounterparty {
            bas_id: None,
            name: "ТОВ Тест".to_string(),
            edrpou: None,
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
        };

        assert!(build_create_note(&with_bas).contains("bas_id"));
        assert!(build_create_note(&with_edrpou).contains("ЄДРПОУ"));
        assert!(build_create_note(&name_only).contains("назвою"));
    }
}
