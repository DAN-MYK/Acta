use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use tokio::fs;

use crate::app_ctx::AppCtx;
use crate::import::bas_acts::{import_acts_from_xml, parse_acts_xml_file};
use crate::import::bas_contracts::{import_contracts_from_xml, parse_contracts_xml_file};
use crate::import::bas_counterparties::{import_counterparties_from_xml, parse_counterparties_xml_file};
use crate::import::bas_invoices::{import_invoices_from_file, parse_invoices_file};
use crate::import::bas_payments::{apply_imported_payments, import_payments_from_csv, parse_payments_csv_file};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportEntityPlanDto {
    pub entity_type: String,
    pub file_name: String,
    pub parsed: usize,
    pub will_create: usize,
    pub will_skip: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlanDto {
    pub entities: Vec<ImportEntityPlanDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportEntityResultDto {
    pub entity_type: String,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultDto {
    pub entities: Vec<ImportEntityResultDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileType {
    Counterparties,
    Contracts,
    Acts,
    Invoices,
    Payments,
}

fn route_file(path: &Path) -> Option<FileType> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    let name = path.file_stem()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "csv" => Some(FileType::Payments),
        "xlsx" | "xls" => {
            if name.contains("counterpart") || name.contains("контрагент") {
                Some(FileType::Counterparties)
            } else if name.contains("invoice")
                || name.contains("рахунок")
                || name.contains("накладна")
            {
                Some(FileType::Invoices)
            } else {
                None
            }
        }
        "xml" => {
            if name.contains("counterpart") || name.contains("контрагент") {
                Some(FileType::Counterparties)
            } else if name.contains("contract")
                || name.contains("договор")
                || name.contains("договір")
            {
                Some(FileType::Contracts)
            } else if (name.contains("act") || name.contains("акт"))
                && !name.contains("contract")
                && !name.contains("договор")
                && !name.contains("договір")
            {
                Some(FileType::Acts)
            } else if name.contains("invoice")
                || name.contains("рахунок")
                || name.contains("накладна")
            {
                Some(FileType::Invoices)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn bas_import_dir() -> PathBuf {
    PathBuf::from("storage/import/bas")
}

async fn collect_sorted_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let mut entries = fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

pub async fn import_bas_plan(ctx: &AppCtx) -> Result<ImportPlanDto> {
    let dir = bas_import_dir();
    fs::create_dir_all(&dir).await?;
    let files = collect_sorted_files(&dir).await?;

    const ENTITY_TYPES: &[(&str, FileType)] = &[
        ("counterparties", FileType::Counterparties),
        ("contracts", FileType::Contracts),
        ("acts", FileType::Acts),
        ("invoices", FileType::Invoices),
        ("payments", FileType::Payments),
    ];

    let mut entities = Vec::new();
    for &(entity_type, file_type) in ENTITY_TYPES {
        let matched = files.iter().find(|p| route_file(p) == Some(file_type));
        let dto = match matched {
            None => ImportEntityPlanDto {
                entity_type: entity_type.to_string(),
                file_name: String::new(),
                parsed: 0,
                will_create: 0,
                will_skip: 0,
                error: None,
            },
            Some(path) => {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                if file_type == FileType::Payments {
                    match parse_payments_csv_file(path).await {
                        Err(e) => ImportEntityPlanDto {
                            entity_type: entity_type.to_string(),
                            file_name,
                            parsed: 0,
                            will_create: 0,
                            will_skip: 0,
                            error: Some(e.to_string()),
                        },
                        Ok(rows) => {
                            match apply_imported_payments(ctx.pool(), ctx.company_id(), &rows, true)
                                .await
                            {
                                Ok(report) => ImportEntityPlanDto {
                                    entity_type: entity_type.to_string(),
                                    file_name,
                                    parsed: report.parsed,
                                    will_create: report.created,
                                    will_skip: report.skipped,
                                    error: None,
                                },
                                Err(e) => ImportEntityPlanDto {
                                    entity_type: entity_type.to_string(),
                                    file_name,
                                    parsed: rows.len(),
                                    will_create: 0,
                                    will_skip: 0,
                                    error: Some(e.to_string()),
                                },
                            }
                        }
                    }
                } else {
                    let count_result: Result<usize> = match file_type {
                        FileType::Counterparties => {
                            parse_counterparties_xml_file(path).await.map(|r| r.len())
                        }
                        FileType::Contracts => {
                            parse_contracts_xml_file(path).await.map(|r| r.len())
                        }
                        FileType::Acts => parse_acts_xml_file(path).await.map(|r| r.len()),
                        FileType::Invoices => parse_invoices_file(path).await.map(|r| r.len()),
                        FileType::Payments => unreachable!(),
                    };
                    match count_result {
                        Ok(parsed) => ImportEntityPlanDto {
                            entity_type: entity_type.to_string(),
                            file_name,
                            parsed,
                            will_create: 0,
                            will_skip: 0,
                            error: None,
                        },
                        Err(e) => ImportEntityPlanDto {
                            entity_type: entity_type.to_string(),
                            file_name,
                            parsed: 0,
                            will_create: 0,
                            will_skip: 0,
                            error: Some(e.to_string()),
                        },
                    }
                }
            }
        };
        entities.push(dto);
    }

    Ok(ImportPlanDto { entities })
}

pub async fn import_bas_execute(ctx: &AppCtx) -> Result<ImportResultDto> {
    let dir = bas_import_dir();
    fs::create_dir_all(&dir).await?;
    let files = collect_sorted_files(&dir).await?;

    const ENTITY_TYPES: &[(&str, FileType)] = &[
        ("counterparties", FileType::Counterparties),
        ("contracts", FileType::Contracts),
        ("acts", FileType::Acts),
        ("invoices", FileType::Invoices),
        ("payments", FileType::Payments),
    ];

    let mut entities = Vec::new();
    for &(entity_type, file_type) in ENTITY_TYPES {
        let matched = files.iter().find(|p| route_file(p) == Some(file_type));
        let dto = match matched {
            None => ImportEntityResultDto {
                entity_type: entity_type.to_string(),
                created: 0,
                updated: 0,
                skipped: 0,
                conflicts: 0,
                error: None,
            },
            Some(path) => {
                let pool = ctx.pool();
                let company_id = ctx.company_id();
                let result = match file_type {
                    FileType::Counterparties => {
                        import_counterparties_from_xml(pool, company_id, path, false)
                            .await
                            .map(|r| (r.created, r.updated, r.skipped, r.conflicts))
                    }
                    FileType::Contracts => {
                        import_contracts_from_xml(pool, company_id, path, false)
                            .await
                            .map(|r| (r.created, r.updated, r.skipped, r.conflicts))
                    }
                    FileType::Acts => {
                        import_acts_from_xml(pool, company_id, path, false)
                            .await
                            .map(|r| (r.created, r.updated, r.skipped, r.conflicts))
                    }
                    FileType::Invoices => {
                        import_invoices_from_file(pool, company_id, path, false)
                            .await
                            .map(|r| (r.created, r.updated, r.skipped, r.conflicts))
                    }
                    FileType::Payments => {
                        import_payments_from_csv(pool, company_id, path, false)
                            .await
                            .map(|r| (r.created, r.updated, r.skipped, r.conflicts))
                    }
                };
                match result {
                    Ok((created, updated, skipped, conflicts)) => ImportEntityResultDto {
                        entity_type: entity_type.to_string(),
                        created,
                        updated,
                        skipped,
                        conflicts,
                        error: None,
                    },
                    Err(e) => ImportEntityResultDto {
                        entity_type: entity_type.to_string(),
                        created: 0,
                        updated: 0,
                        skipped: 0,
                        conflicts: 0,
                        error: Some(e.to_string()),
                    },
                }
            }
        };
        entities.push(dto);
    }

    Ok(ImportResultDto { entities })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_csv_is_payments() {
        assert_eq!(route_file(Path::new("bank_export.csv")), Some(FileType::Payments));
        assert_eq!(route_file(Path::new("payments.csv")), Some(FileType::Payments));
    }

    #[test]
    fn route_xml_by_filename_keyword() {
        assert_eq!(
            route_file(Path::new("counterparties.xml")),
            Some(FileType::Counterparties)
        );
        assert_eq!(
            route_file(Path::new("counterpart_2024.xlsx")),
            Some(FileType::Counterparties)
        );
        assert_eq!(
            route_file(Path::new("contracts_2024.xml")),
            Some(FileType::Contracts)
        );
        assert_eq!(route_file(Path::new("acts.xml")), Some(FileType::Acts));
        assert_eq!(route_file(Path::new("invoices.xlsx")), Some(FileType::Invoices));
    }

    #[test]
    fn route_unrecognized_returns_none() {
        assert_eq!(route_file(Path::new("data.txt")), None);
        assert_eq!(route_file(Path::new("report.xml")), None);
    }

    #[test]
    fn xlsx_acts_not_routed() {
        let path = Path::new("acts_2024.xlsx");
        assert_eq!(route_file(path), None);
    }

    #[test]
    fn xlsx_contracts_not_routed() {
        let path = Path::new("contracts_2024.xlsx");
        assert_eq!(route_file(path), None);
    }
}
