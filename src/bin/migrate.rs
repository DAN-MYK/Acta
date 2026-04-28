// Утиліта імпорту даних з BAS.
//
// Поточний baseline:
// - counterparties XML
// - contracts XML
// - acts XML
// - dry-run теж підключається до БД і показує реальний create/update/skip plan

use std::fs;
use std::path::{Path, PathBuf};

use acta::import::bas_acts::{self, ActImportAction, ActImportReport};
use acta::import::bas_contracts::{self, ContractImportAction, ContractImportReport};
use acta::import::bas_counterparties::{self, CounterpartyImportReport, ImportAction};
use anyhow::{anyhow, Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
struct CliOptions {
    input_dir: String,
    dry_run: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum ParseOutcome {
    Run(CliOptions),
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BasArtifactKind {
    Counterparties,
    Contracts,
    Acts,
    Invoices,
    Payments,
    BankCsv,
    Unknown,
}

impl BasArtifactKind {
    fn label(self) -> &'static str {
        match self {
            BasArtifactKind::Counterparties => "Контрагенти",
            BasArtifactKind::Contracts => "Договори",
            BasArtifactKind::Acts => "Акти",
            BasArtifactKind::Invoices => "Накладні/рахунки",
            BasArtifactKind::Payments => "Платежі",
            BasArtifactKind::BankCsv => "Банківські CSV",
            BasArtifactKind::Unknown => "Нерозпізнані",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BasArtifact {
    path: PathBuf,
    kind: BasArtifactKind,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DiscoveryReport {
    root: PathBuf,
    artifacts: Vec<BasArtifact>,
    skipped_files: Vec<PathBuf>,
}

impl DiscoveryReport {
    fn recognized_count(&self) -> usize {
        self.artifacts.len()
    }

    fn count_by_kind(&self, kind: BasArtifactKind) -> usize {
        self.artifacts
            .iter()
            .filter(|item| item.kind == kind)
            .count()
    }
}

fn parse_args(args: &[String]) -> Result<ParseOutcome, String> {
    let mut input_dir: Option<String> = None;
    let mut dry_run = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" | "-i" => {
                i += 1;
                if i < args.len() {
                    input_dir = Some(args[i].clone());
                } else {
                    return Err("Помилка: --input потребує шлях до директорії".to_string());
                }
            }
            "--dry-run" => dry_run = true,
            "--help" | "-h" => return Ok(ParseOutcome::Help),
            other => return Err(format!("Невідомий аргумент: {other}")),
        }
        i += 1;
    }

    let Some(input_dir) = input_dir else {
        return Err("Помилка: --input є обов'язковим аргументом".to_string());
    };

    Ok(ParseOutcome::Run(CliOptions { input_dir, dry_run }))
}

fn classify_artifact(path: &Path) -> Option<BasArtifactKind> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())?;

    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .unwrap_or_default();

    let kind = match extension.as_str() {
        "csv" => BasArtifactKind::BankCsv,
        "xml" | "xlsx" | "xls" => {
            if name.contains("контраг") || name.contains("counterpart") || name.contains("client")
            {
                BasArtifactKind::Counterparties
            } else if name.contains("догов") || name.contains("contract") {
                BasArtifactKind::Contracts
            } else if name.contains("акт") || name.contains("act") {
                BasArtifactKind::Acts
            } else if name.contains("наклад") || name.contains("invoice") || name.contains("рах")
            {
                BasArtifactKind::Invoices
            } else if name.contains("плат") || name.contains("payment") || name.contains("bank")
            {
                BasArtifactKind::Payments
            } else {
                BasArtifactKind::Unknown
            }
        }
        _ => return None,
    };

    Some(kind)
}

fn discover_artifacts(root: &Path) -> Result<DiscoveryReport, String> {
    if !root.exists() {
        return Err(format!("Вхідну директорію не знайдено: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!(
            "Очікується директорія, а не файл: {}",
            root.display()
        ));
    }

    let mut report = DiscoveryReport {
        root: root.to_path_buf(),
        artifacts: Vec::new(),
        skipped_files: Vec::new(),
    };

    visit_dir(root, &mut report)?;
    report
        .artifacts
        .sort_by(|left, right| left.path.cmp(&right.path));
    report.skipped_files.sort();

    Ok(report)
}

fn visit_dir(path: &Path, report: &mut DiscoveryReport) -> Result<(), String> {
    let entries = fs::read_dir(path).map_err(|error| {
        format!(
            "Не вдалося прочитати директорію {}: {error}",
            path.display()
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Не вдалося прочитати елемент директорії {}: {error}",
                path.display()
            )
        })?;
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "Не вдалося визначити тип елемента {}: {error}",
                entry.path().display()
            )
        })?;
        let entry_path = entry.path();

        if file_type.is_dir() {
            visit_dir(&entry_path, report)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        match classify_artifact(&entry_path) {
            Some(kind) => report.artifacts.push(BasArtifact {
                path: entry_path,
                kind,
            }),
            None => report.skipped_files.push(entry_path),
        }
    }

    Ok(())
}

fn print_discovery_report(report: &DiscoveryReport, dry_run: bool) {
    println!("Вхідна директорія: {}", report.root.display());
    if dry_run {
        println!("Режим dry-run: зміни до БД не застосовуються, але preview звіряється з поточним станом БД");
    }

    println!(
        "Знайдено {} підтримуваних файлів експорту",
        report.recognized_count()
    );

    for kind in [
        BasArtifactKind::Counterparties,
        BasArtifactKind::Contracts,
        BasArtifactKind::Acts,
        BasArtifactKind::Invoices,
        BasArtifactKind::Payments,
        BasArtifactKind::BankCsv,
        BasArtifactKind::Unknown,
    ] {
        let count = report.count_by_kind(kind);
        if count > 0 {
            println!("  - {}: {}", kind.label(), count);
        }
    }
}

fn is_xml_artifact(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("xml"))
        .unwrap_or(false)
}

fn collect_supported_artifacts<'a>(
    report: &'a DiscoveryReport,
    kind: BasArtifactKind,
) -> Vec<&'a BasArtifact> {
    report
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == kind)
        .filter(|artifact| is_xml_artifact(&artifact.path))
        .collect()
}

fn print_counterparty_report(path: &Path, report: &CounterpartyImportReport, dry_run: bool) {
    let mode = if dry_run { "dry-run" } else { "import" };
    println!(
        "[{}] {}: parsed={}, create={}, update={}, skipped={}",
        mode,
        path.display(),
        report.parsed,
        report.created,
        report.updated,
        report.skipped
    );

    for row in &report.rows {
        let action = match row.action {
            ImportAction::Create => "create",
            ImportAction::Update => "update",
        };
        let bas_id = row.bas_id.as_deref().unwrap_or("-");
        println!("  - {} | {} | {}", action, bas_id, row.name);
    }
}

fn print_contract_report(path: &Path, report: &ContractImportReport, dry_run: bool) {
    let mode = if dry_run { "dry-run" } else { "import" };
    println!(
        "[{}] {}: parsed={}, create={}, update={}, skipped={}",
        mode,
        path.display(),
        report.parsed,
        report.created,
        report.updated,
        report.skipped
    );

    for row in &report.rows {
        let action = match row.action {
            ContractImportAction::Create => "create",
            ContractImportAction::Update => "update",
            ContractImportAction::Skip => "skip",
        };
        let bas_id = row.bas_id.as_deref().unwrap_or("-");
        if let Some(note) = &row.note {
            println!("  - {} | {} | {} | {}", action, bas_id, row.number, note);
        } else {
            println!("  - {} | {} | {}", action, bas_id, row.number);
        }
    }
}

fn print_act_report(path: &Path, report: &ActImportReport, dry_run: bool) {
    let mode = if dry_run { "dry-run" } else { "import" };
    println!(
        "[{}] {}: parsed={}, create={}, update={}, skipped={}",
        mode,
        path.display(),
        report.parsed,
        report.created,
        report.updated,
        report.skipped
    );

    for row in &report.rows {
        let action = match row.action {
            ActImportAction::Create => "create",
            ActImportAction::Update => "update",
            ActImportAction::Skip => "skip",
        };
        let bas_id = row.bas_id.as_deref().unwrap_or("-");
        if let Some(note) = &row.note {
            println!("  - {} | {} | {} | {}", action, bas_id, row.number, note);
        } else {
            println!("  - {} | {} | {}", action, bas_id, row.number);
        }
    }
}

async fn connect_pool() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL не задано для BAS import preview/import")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    Ok(pool)
}

async fn first_company_id(pool: &PgPool) -> Result<Uuid> {
    let company = acta::db::companies::list(pool)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("У БД немає активної компанії для імпорту"))?;
    Ok(company.id)
}

async fn process_counterparties_artifact(
    path: &Path,
    pool: &PgPool,
    company_id: Uuid,
    dry_run: bool,
) -> Result<CounterpartyImportReport> {
    bas_counterparties::import_counterparties_from_xml(pool, company_id, path, dry_run).await
}

async fn process_contracts_artifact(
    path: &Path,
    pool: &PgPool,
    company_id: Uuid,
    dry_run: bool,
) -> Result<ContractImportReport> {
    bas_contracts::import_contracts_from_xml(pool, company_id, path, dry_run).await
}

async fn process_acts_artifact(
    path: &Path,
    pool: &PgPool,
    company_id: Uuid,
    dry_run: bool,
) -> Result<ActImportReport> {
    bas_acts::import_acts_from_xml(pool, company_id, path, dry_run).await
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let parsed = match parse_args(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            eprintln!("Використання: migrate --input <директорія> [--dry-run]");
            std::process::exit(1);
        }
    };

    if parsed == ParseOutcome::Help {
        println!("Використання: migrate --input <директорія> [--dry-run]");
        return;
    }

    let ParseOutcome::Run(opts) = parsed else {
        return;
    };

    let input_dir = PathBuf::from(&opts.input_dir);
    let report = match discover_artifacts(&input_dir) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("Помилка discovery: {error}");
            std::process::exit(1);
        }
    };

    print_discovery_report(&report, opts.dry_run);

    let counterparty_artifacts =
        collect_supported_artifacts(&report, BasArtifactKind::Counterparties);
    let contract_artifacts = collect_supported_artifacts(&report, BasArtifactKind::Contracts);
    let act_artifacts = collect_supported_artifacts(&report, BasArtifactKind::Acts);

    if counterparty_artifacts.is_empty()
        && contract_artifacts.is_empty()
        && act_artifacts.is_empty()
    {
        println!("Не знайдено XML-файлів для реалізованих BAS importer-ів.");
        if report.recognized_count() > 0 {
            println!("Накладні, платежі та не-XML варіанти ще не реалізовані.");
        }
        return;
    }

    let pool = match connect_pool().await {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("Помилка підключення до БД: {error}");
            std::process::exit(1);
        }
    };
    let company_id = match first_company_id(&pool).await {
        Ok(company_id) => company_id,
        Err(error) => {
            eprintln!("Помилка вибору компанії: {error}");
            std::process::exit(1);
        }
    };

    let mut failed = false;

    for artifact in counterparty_artifacts {
        match process_counterparties_artifact(&artifact.path, &pool, company_id, opts.dry_run).await
        {
            Ok(import_report) => {
                print_counterparty_report(&artifact.path, &import_report, opts.dry_run)
            }
            Err(error) => {
                failed = true;
                eprintln!("Помилка імпорту {}: {error}", artifact.path.display());
            }
        }
    }

    for artifact in contract_artifacts {
        match process_contracts_artifact(&artifact.path, &pool, company_id, opts.dry_run).await {
            Ok(import_report) => {
                print_contract_report(&artifact.path, &import_report, opts.dry_run)
            }
            Err(error) => {
                failed = true;
                eprintln!("Помилка імпорту {}: {error}", artifact.path.display());
            }
        }
    }

    for artifact in act_artifacts {
        match process_acts_artifact(&artifact.path, &pool, company_id, opts.dry_run).await {
            Ok(import_report) => print_act_report(&artifact.path, &import_report, opts.dry_run),
            Err(error) => {
                failed = true;
                eprintln!("Помилка імпорту {}: {error}", artifact.path.display());
            }
        }
    }

    if failed {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_artifact, collect_supported_artifacts, discover_artifacts, is_xml_artifact,
        parse_args, BasArtifactKind, CliOptions, DiscoveryReport, ParseOutcome,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("acta-migrate-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("тимчасова директорія має створюватися");
        path
    }

    #[test]
    fn parse_args_accepts_input_and_dry_run() {
        let parsed = parse_args(&args(&["migrate", "--input", "./bas", "--dry-run"]));
        assert_eq!(
            parsed,
            Ok(ParseOutcome::Run(CliOptions {
                input_dir: "./bas".to_string(),
                dry_run: true
            }))
        );
    }

    #[test]
    fn parse_args_supports_short_input_flag() {
        let parsed = parse_args(&args(&["migrate", "-i", "./bas"]));
        assert_eq!(
            parsed,
            Ok(ParseOutcome::Run(CliOptions {
                input_dir: "./bas".to_string(),
                dry_run: false
            }))
        );
    }

    #[test]
    fn parse_args_returns_help() {
        let parsed = parse_args(&args(&["migrate", "--help"]));
        assert_eq!(parsed, Ok(ParseOutcome::Help));
    }

    #[test]
    fn parse_args_requires_input_value() {
        let parsed = parse_args(&args(&["migrate", "--input"]));
        assert_eq!(
            parsed,
            Err("Помилка: --input потребує шлях до директорії".to_string())
        );
    }

    #[test]
    fn parse_args_fails_on_unknown_arg() {
        let parsed = parse_args(&args(&["migrate", "--wat"]));
        assert_eq!(parsed, Err("Невідомий аргумент: --wat".to_string()));
    }

    #[test]
    fn classify_artifact_detects_counterparties() {
        let kind = classify_artifact(Path::new("Контрагенти_експорт.xml"));
        assert_eq!(kind, Some(BasArtifactKind::Counterparties));
    }

    #[test]
    fn classify_artifact_detects_contracts_and_acts() {
        assert_eq!(
            classify_artifact(Path::new("contracts_export.xml")),
            Some(BasArtifactKind::Contracts)
        );
        assert_eq!(
            classify_artifact(Path::new("Акти.xml")),
            Some(BasArtifactKind::Acts)
        );
    }

    #[test]
    fn classify_artifact_detects_bank_csv() {
        let kind = classify_artifact(Path::new("statement.csv"));
        assert_eq!(kind, Some(BasArtifactKind::BankCsv));
    }

    #[test]
    fn xml_filter_accepts_only_xml_files() {
        assert!(is_xml_artifact(Path::new("contracts.xml")));
        assert!(!is_xml_artifact(Path::new("contracts.xlsx")));
    }

    #[test]
    fn discover_artifacts_collects_supported_files_recursively() {
        let root = temp_dir("discovery");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("вкладена директорія має створюватися");

        fs::write(root.join("Контрагенти.xml"), "<xml/>").expect("тестовий файл має створюватися");
        fs::write(nested.join("contracts_export.xml"), "<xml/>")
            .expect("тестовий файл має створюватися");
        fs::write(nested.join("acts_export.xml"), "<xml/>")
            .expect("тестовий файл має створюватися");
        fs::write(nested.join("notes.txt"), "skip").expect("тестовий файл має створюватися");

        let report = discover_artifacts(&root).expect("discovery має спрацювати");
        assert_eq!(report.recognized_count(), 3);
        assert_eq!(report.count_by_kind(BasArtifactKind::Counterparties), 1);
        assert_eq!(report.count_by_kind(BasArtifactKind::Contracts), 1);
        assert_eq!(report.count_by_kind(BasArtifactKind::Acts), 1);
        assert_eq!(report.skipped_files.len(), 1);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discover_artifacts_fails_for_missing_dir() {
        let root = temp_dir("missing");
        let _ = fs::remove_dir_all(&root);
        let error = discover_artifacts(&root).expect_err("має бути помилка для відсутньої папки");
        assert!(error.contains("Вхідну директорію не знайдено"));
    }

    #[test]
    fn collect_supported_artifacts_keeps_only_xml_of_requested_kind() {
        let report = DiscoveryReport {
            root: PathBuf::from("."),
            artifacts: vec![
                super::BasArtifact {
                    path: PathBuf::from("contracts.xml"),
                    kind: BasArtifactKind::Contracts,
                },
                super::BasArtifact {
                    path: PathBuf::from("contracts.xlsx"),
                    kind: BasArtifactKind::Contracts,
                },
                super::BasArtifact {
                    path: PathBuf::from("acts.xml"),
                    kind: BasArtifactKind::Acts,
                },
            ],
            skipped_files: Vec::new(),
        };

        let contracts = collect_supported_artifacts(&report, BasArtifactKind::Contracts);
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].path, PathBuf::from("contracts.xml"));
    }
}
