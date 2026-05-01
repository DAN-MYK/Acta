// Утиліта імпорту даних з BAS.
//
// Поточний expanded baseline:
// - counterparties: XML + XLSX/XLS
// - contracts: XML + XLSX/XLS
// - acts: XML + XLSX/XLS
// - invoices: XML + XLSX/XLS
// - payments: CSV
// - dry-run підключається до БД і показує aggregated DB-aware preview

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use acta::import::bas_acts::{self, ActImportAction, ActImportReport};
use acta::import::bas_contracts::{self, ContractImportAction, ContractImportReport};
use acta::import::bas_counterparties::{self, CounterpartyImportReport, ImportAction};
use acta::import::bas_invoices::{self, InvoiceImportAction, InvoiceImportReport};
use acta::import::bas_payments::{self, PaymentImportAction, PaymentImportReport};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ImporterKey {
    Counterparties,
    Contracts,
    Acts,
    Invoices,
    Payments,
}

#[derive(Debug, Clone, Copy)]
struct ImporterSpec {
    key: ImporterKey,
    artifact_kinds: &'static [BasArtifactKind],
    totals_label: &'static str,
}

const IMPORTER_SPECS: &[ImporterSpec] = &[
    ImporterSpec {
        key: ImporterKey::Counterparties,
        artifact_kinds: &[BasArtifactKind::Counterparties],
        totals_label: "контрагентів",
    },
    ImporterSpec {
        key: ImporterKey::Contracts,
        artifact_kinds: &[BasArtifactKind::Contracts],
        totals_label: "договорів",
    },
    ImporterSpec {
        key: ImporterKey::Acts,
        artifact_kinds: &[BasArtifactKind::Acts],
        totals_label: "актів",
    },
    ImporterSpec {
        key: ImporterKey::Invoices,
        artifact_kinds: &[BasArtifactKind::Invoices],
        totals_label: "накладних",
    },
    ImporterSpec {
        key: ImporterKey::Payments,
        artifact_kinds: &[BasArtifactKind::Payments, BasArtifactKind::BankCsv],
        totals_label: "платежів",
    },
];

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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ImportTotals {
    files: usize,
    parsed: usize,
    created: usize,
    updated: usize,
    skipped: usize,
    conflicts: usize,
    errors: usize,
    reasons: BTreeMap<String, usize>,
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
        "csv" => {
            if name.contains("payment") || name.contains("bank") || name.contains("statement") {
                BasArtifactKind::Payments
            } else {
                BasArtifactKind::BankCsv
            }
        }
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
    println!("=== BAS MIGRATE ===");
    println!("Вхідна директорія: {}", report.root.display());
    if dry_run {
        println!("Режим: dry-run / DB-aware preview");
        println!("Preview звіряється з поточним станом БД, але змін не записує.");
    } else {
        println!("Режим: імпорт у БД");
    }

    println!(
        "Знайдено підтримуваних файлів: {}",
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
            println!("  - {} -> {}", kind.label(), count);
        }
    }
}

fn is_importable_artifact(kind: BasArtifactKind, path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .unwrap_or_default();

    match kind {
        BasArtifactKind::Counterparties
        | BasArtifactKind::Contracts
        | BasArtifactKind::Invoices => {
            matches!(extension.as_str(), "xml" | "xlsx" | "xls")
        }
        BasArtifactKind::Acts => matches!(extension.as_str(), "xml" | "xlsx" | "xls"),
        BasArtifactKind::Payments | BasArtifactKind::BankCsv => extension == "csv",
        BasArtifactKind::Unknown => false,
    }
}

fn collect_supported_artifacts<'a>(
    report: &'a DiscoveryReport,
    kind: BasArtifactKind,
) -> Vec<&'a BasArtifact> {
    report
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == kind)
        .filter(|artifact| is_importable_artifact(kind, &artifact.path))
        .collect()
}

fn collect_supported_artifacts_multi<'a>(
    report: &'a DiscoveryReport,
    kinds: &[BasArtifactKind],
) -> Vec<&'a BasArtifact> {
    let mut artifacts = Vec::new();
    for kind in kinds {
        artifacts.extend(collect_supported_artifacts(report, *kind));
    }
    artifacts.sort_by(|left, right| left.path.cmp(&right.path));
    artifacts
}

fn build_importer_artifacts<'a>(
    report: &'a DiscoveryReport,
) -> BTreeMap<ImporterKey, Vec<&'a BasArtifact>> {
    IMPORTER_SPECS
        .iter()
        .map(|spec| {
            (
                spec.key,
                collect_supported_artifacts_multi(report, spec.artifact_kinds),
            )
        })
        .collect()
}

fn build_importer_totals() -> BTreeMap<ImporterKey, ImportTotals> {
    IMPORTER_SPECS
        .iter()
        .map(|spec| (spec.key, ImportTotals::default()))
        .collect()
}

fn push_reason(reasons: &mut BTreeMap<String, usize>, label: impl Into<String>) {
    let label = label.into();
    *reasons.entry(label).or_insert(0) += 1;
}

fn merge_reason_counts(
    target: &mut BTreeMap<String, usize>,
    additions: impl IntoIterator<Item = (String, usize)>,
) {
    for (label, count) in additions {
        *target.entry(label).or_insert(0) += count;
    }
}

fn print_reason_summary(reasons: &BTreeMap<String, usize>) {
    if reasons.is_empty() {
        return;
    }

    println!("  Зведення причин:");
    for (label, count) in reasons {
        println!("    - {} -> {}", label, count);
    }
}

fn accumulate_totals(
    totals: &mut ImportTotals,
    parsed: usize,
    created: usize,
    updated: usize,
    skipped: usize,
    conflicts: usize,
    reasons: BTreeMap<String, usize>,
) {
    totals.files += 1;
    totals.parsed += parsed;
    totals.created += created;
    totals.updated += updated;
    totals.skipped += skipped;
    totals.conflicts += conflicts;
    merge_reason_counts(&mut totals.reasons, reasons);
}

fn collect_reason_counts<Row>(
    rows: &[Row],
    label_for: impl Fn(&Row) -> Cow<'_, str>,
) -> BTreeMap<String, usize> {
    let mut reasons = BTreeMap::new();
    for row in rows {
        push_reason(&mut reasons, label_for(row).into_owned());
    }
    reasons
}

fn action_label(action: &str) -> &'static str {
    match action {
        "create" => "СТВОРИТИ",
        "update" => "ОНОВИТИ",
        "skip" => "ПРОПУСТИТИ",
        "conflict" => "КОНФЛІКТ",
        _ => "ДІЯ",
    }
}

fn format_counts(
    parsed: usize,
    created: usize,
    updated: usize,
    skipped: usize,
    conflicts: usize,
) -> String {
    format!(
        "рядків: {}, створити: {}, оновити: {}, пропустити: {}, конфлікти: {}",
        parsed, created, updated, skipped, conflicts
    )
}

fn format_counts_no_update(
    parsed: usize,
    created: usize,
    skipped: usize,
    conflicts: usize,
) -> String {
    format!(
        "рядків: {}, створити: {}, пропустити: {}, конфлікти: {}",
        parsed, created, skipped, conflicts
    )
}

fn print_report_header(path: &Path, dry_run: bool, summary: &str) {
    let mode = if dry_run { "PREVIEW" } else { "ІМПОРТ" };
    println!();
    println!("=== {} :: {} ===", mode, path.display());
    println!("  {}", summary);
}

fn print_report_rows<Row>(rows: &[Row], render: impl Fn(&Row) -> String) {
    for row in rows {
        println!("  • {}", render(row));
    }
}

fn counterparty_reason_counts(report: &CounterpartyImportReport) -> BTreeMap<String, usize> {
    collect_reason_counts(&report.rows, |row| {
        match (&row.action, row.note.as_deref()) {
            (ImportAction::Create, Some(note)) => Cow::Borrowed(note),
            (ImportAction::Create, None) => Cow::Borrowed("create: новий контрагент"),
            (ImportAction::Update, Some(note)) => Cow::Borrowed(note),
            (ImportAction::Update, None) => Cow::Borrowed("update: знайдено existing row у БД"),
            (ImportAction::Conflict, Some(note)) => Cow::Borrowed(note),
            (ImportAction::Conflict, None) => Cow::Borrowed("conflict: неоднозначний match"),
        }
    })
}

fn contract_reason_counts(report: &ContractImportReport) -> BTreeMap<String, usize> {
    collect_reason_counts(&report.rows, |row| {
        match (&row.action, row.note.as_deref()) {
            (ContractImportAction::Create, Some(note)) => Cow::Borrowed(note),
            (ContractImportAction::Create, None) => {
                Cow::Borrowed("create: нового договору в БД ще немає")
            }
            (ContractImportAction::Update, Some(note)) => Cow::Borrowed(note),
            (ContractImportAction::Update, None) => {
                Cow::Borrowed("update: знайдено existing row у БД")
            }
            (ContractImportAction::Conflict, Some(note)) => Cow::Borrowed(note),
            (ContractImportAction::Conflict, None) => {
                Cow::Borrowed("conflict: неоднозначний match")
            }
            (ContractImportAction::Skip, Some(note)) => Cow::Borrowed(note),
            (ContractImportAction::Skip, None) => Cow::Borrowed("skip: без уточненої причини"),
        }
    })
}

fn act_reason_counts(report: &ActImportReport) -> BTreeMap<String, usize> {
    collect_reason_counts(&report.rows, |row| {
        match (&row.action, row.note.as_deref()) {
            (ActImportAction::Create, Some(note)) => Cow::Borrowed(note),
            (ActImportAction::Create, None) => Cow::Borrowed("create: нового акту в БД ще немає"),
            (ActImportAction::Update, Some(note)) => Cow::Borrowed(note),
            (ActImportAction::Update, None) => Cow::Borrowed("update: знайдено existing row у БД"),
            (ActImportAction::Conflict, Some(note)) => Cow::Borrowed(note),
            (ActImportAction::Conflict, None) => Cow::Borrowed("conflict: неоднозначний match"),
            (ActImportAction::Skip, Some(note)) => Cow::Borrowed(note),
            (ActImportAction::Skip, None) => Cow::Borrowed("skip: без уточненої причини"),
        }
    })
}

fn payment_reason_counts(report: &PaymentImportReport) -> BTreeMap<String, usize> {
    collect_reason_counts(&report.rows, |row| {
        match (&row.action, row.note.as_deref()) {
            (PaymentImportAction::Create, Some(note)) => Cow::Borrowed(note),
            (PaymentImportAction::Create, None) => {
                Cow::Borrowed("create: не знайдено дубліката в БД")
            }
            (PaymentImportAction::Skip, Some(note)) => Cow::Borrowed(note),
            (PaymentImportAction::Skip, None) => Cow::Borrowed("skip: без уточненої причини"),
        }
    })
}

fn invoice_reason_counts(report: &InvoiceImportReport) -> BTreeMap<String, usize> {
    collect_reason_counts(&report.rows, |row| {
        match (&row.action, row.note.as_deref()) {
            (InvoiceImportAction::Create, Some(note)) => Cow::Borrowed(note),
            (InvoiceImportAction::Create, None) => {
                Cow::Borrowed("create: нової накладної в БД ще немає")
            }
            (InvoiceImportAction::Update, Some(note)) => Cow::Borrowed(note),
            (InvoiceImportAction::Update, None) => {
                Cow::Borrowed("update: знайдено existing row у БД")
            }
            (InvoiceImportAction::Conflict, Some(note)) => Cow::Borrowed(note),
            (InvoiceImportAction::Conflict, None) => Cow::Borrowed("conflict: неоднозначний match"),
            (InvoiceImportAction::Skip, Some(note)) => Cow::Borrowed(note),
            (InvoiceImportAction::Skip, None) => Cow::Borrowed("skip: без уточненої причини"),
        }
    })
}

fn print_counterparty_report(path: &Path, report: &CounterpartyImportReport, dry_run: bool) {
    print_report_header(
        path,
        dry_run,
        &format_counts(
            report.parsed,
            report.created,
            report.updated,
            report.skipped,
            report.conflicts,
        ),
    );
    print_report_rows(&report.rows, |row| {
        let action = match row.action {
            ImportAction::Create => "create",
            ImportAction::Update => "update",
            ImportAction::Conflict => "conflict",
        };
        let bas_id = row.bas_id.as_deref().unwrap_or("-");
        if let Some(note) = &row.note {
            format!(
                "{} | bas_id={} | {} | {}",
                action_label(action),
                bas_id,
                row.name,
                note
            )
        } else {
            format!(
                "{} | bas_id={} | {}",
                action_label(action),
                bas_id,
                row.name
            )
        }
    });
    print_reason_summary(&counterparty_reason_counts(report));
}

fn print_contract_report(path: &Path, report: &ContractImportReport, dry_run: bool) {
    print_report_header(
        path,
        dry_run,
        &format_counts(
            report.parsed,
            report.created,
            report.updated,
            report.skipped,
            report.conflicts,
        ),
    );
    print_report_rows(&report.rows, |row| {
        let action = match row.action {
            ContractImportAction::Create => "create",
            ContractImportAction::Update => "update",
            ContractImportAction::Skip => "skip",
            ContractImportAction::Conflict => "conflict",
        };
        let bas_id = row.bas_id.as_deref().unwrap_or("-");
        if let Some(note) = &row.note {
            format!(
                "{} | bas_id={} | договір {} | {}",
                action_label(action),
                bas_id,
                row.number,
                note
            )
        } else {
            format!(
                "{} | bas_id={} | договір {}",
                action_label(action),
                bas_id,
                row.number
            )
        }
    });
    print_reason_summary(&contract_reason_counts(report));
}

fn print_act_report(path: &Path, report: &ActImportReport, dry_run: bool) {
    print_report_header(
        path,
        dry_run,
        &format_counts(
            report.parsed,
            report.created,
            report.updated,
            report.skipped,
            report.conflicts,
        ),
    );
    print_report_rows(&report.rows, |row| {
        let action = match row.action {
            ActImportAction::Create => "create",
            ActImportAction::Update => "update",
            ActImportAction::Skip => "skip",
            ActImportAction::Conflict => "conflict",
        };
        let bas_id = row.bas_id.as_deref().unwrap_or("-");
        if let Some(note) = &row.note {
            format!(
                "{} | bas_id={} | акт {} | {}",
                action_label(action),
                bas_id,
                row.number,
                note
            )
        } else {
            format!(
                "{} | bas_id={} | акт {}",
                action_label(action),
                bas_id,
                row.number
            )
        }
    });
    print_reason_summary(&act_reason_counts(report));
}

fn print_payment_report(path: &Path, report: &PaymentImportReport, dry_run: bool) {
    print_report_header(
        path,
        dry_run,
        &format_counts_no_update(
            report.parsed,
            report.created,
            report.skipped,
            report.conflicts,
        ),
    );
    print_report_rows(&report.rows, |row| {
        let action = match row.action {
            PaymentImportAction::Create => "create",
            PaymentImportAction::Skip => "skip",
        };
        let bank_ref = row.bank_ref.as_deref().unwrap_or("-");
        if let Some(note) = &row.note {
            format!(
                "{} | bank_ref={} | {} | {}",
                action_label(action),
                bank_ref,
                row.description,
                note
            )
        } else {
            format!(
                "{} | bank_ref={} | {}",
                action_label(action),
                bank_ref,
                row.description
            )
        }
    });
    print_reason_summary(&payment_reason_counts(report));
}

fn print_invoice_report(path: &Path, report: &InvoiceImportReport, dry_run: bool) {
    print_report_header(
        path,
        dry_run,
        &format_counts(
            report.parsed,
            report.created,
            report.updated,
            report.skipped,
            report.conflicts,
        ),
    );
    print_report_rows(&report.rows, |row| {
        let action = match row.action {
            InvoiceImportAction::Create => "create",
            InvoiceImportAction::Update => "update",
            InvoiceImportAction::Skip => "skip",
            InvoiceImportAction::Conflict => "conflict",
        };
        let bas_id = row.bas_id.as_deref().unwrap_or("-");
        if let Some(note) = &row.note {
            format!(
                "{} | bas_id={} | накладна {} | {}",
                action_label(action),
                bas_id,
                row.number,
                note
            )
        } else {
            format!(
                "{} | bas_id={} | накладна {}",
                action_label(action),
                bas_id,
                row.number
            )
        }
    });
    print_reason_summary(&invoice_reason_counts(report));
}

fn print_totals(label: &str, totals: &ImportTotals) {
    if totals.files == 0 {
        return;
    }

    println!();
    println!("=== ПІДСУМОК: {} ===", label);
    println!("  файлів: {}", totals.files);
    println!("  рядків: {}", totals.parsed);
    println!("  створити: {}", totals.created);
    println!("  оновити: {}", totals.updated);
    println!("  пропустити: {}", totals.skipped);
    println!("  конфлікти: {}", totals.conflicts);
    println!("  помилок: {}", totals.errors);

    print_reason_summary(&totals.reasons);
}

fn has_supported_artifacts(
    artifacts_by_importer: &BTreeMap<ImporterKey, Vec<&BasArtifact>>,
) -> bool {
    artifacts_by_importer
        .values()
        .any(|artifacts| !artifacts.is_empty())
}

fn sum_totals_from_map(totals_by_importer: &BTreeMap<ImporterKey, ImportTotals>) -> ImportTotals {
    let mut overall = ImportTotals::default();

    for spec in IMPORTER_SPECS {
        if let Some(totals) = totals_by_importer.get(&spec.key) {
            overall.files += totals.files;
            overall.parsed += totals.parsed;
            overall.created += totals.created;
            overall.updated += totals.updated;
            overall.skipped += totals.skipped;
            overall.conflicts += totals.conflicts;
            overall.errors += totals.errors;
            merge_reason_counts(&mut overall.reasons, totals.reasons.clone());
        }
    }

    overall
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

async fn process_artifacts<Report, ProcessFn, PrintFn, StatsFn, ReasonsFn>(
    artifacts: &[&BasArtifact],
    pool: &PgPool,
    company_id: Uuid,
    dry_run: bool,
    totals: &mut ImportTotals,
    process: ProcessFn,
    print_report: PrintFn,
    stats: StatsFn,
    reason_counts: ReasonsFn,
) -> bool
where
    ProcessFn: for<'a> Fn(
        &'a Path,
        &'a PgPool,
        Uuid,
        bool,
    ) -> Pin<Box<dyn Future<Output = Result<Report>> + 'a>>,
    PrintFn: Fn(&Path, &Report, bool),
    StatsFn: Fn(&Report) -> (usize, usize, usize, usize, usize),
    ReasonsFn: Fn(&Report) -> BTreeMap<String, usize>,
{
    let mut failed = false;

    for artifact in artifacts {
        match process(&artifact.path, pool, company_id, dry_run).await {
            Ok(import_report) => {
                let (parsed, created, updated, skipped, conflicts) = stats(&import_report);
                accumulate_totals(
                    totals,
                    parsed,
                    created,
                    updated,
                    skipped,
                    conflicts,
                    reason_counts(&import_report),
                );
                print_report(&artifact.path, &import_report, dry_run);
            }
            Err(error) => {
                failed = true;
                totals.errors += 1;
                eprintln!("Помилка імпорту {}: {error}", artifact.path.display());
            }
        }
    }

    failed
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let parsed = match parse_args(&args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
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

    let artifacts_by_importer = build_importer_artifacts(&report);

    if !has_supported_artifacts(&artifacts_by_importer) {
        println!("Не знайдено файлів для реалізованих BAS importer-ів.");
        if report.recognized_count() > 0 {
            println!("Накладні та інші нерозпізнані формати ще не реалізовані.");
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
    let mut totals_by_importer = build_importer_totals();

    failed |= process_artifacts(
        artifacts_by_importer
            .get(&ImporterKey::Counterparties)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        &pool,
        company_id,
        opts.dry_run,
        totals_by_importer
            .get_mut(&ImporterKey::Counterparties)
            .expect("totals for counterparties must exist"),
        |path, pool, company_id, dry_run| {
            Box::pin(bas_counterparties::import_counterparties_from_xml(
                pool, company_id, path, dry_run,
            ))
        },
        print_counterparty_report,
        |report| {
            (
                report.parsed,
                report.created,
                report.updated,
                report.skipped,
                report.conflicts,
            )
        },
        counterparty_reason_counts,
    )
    .await;

    failed |= process_artifacts(
        artifacts_by_importer
            .get(&ImporterKey::Contracts)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        &pool,
        company_id,
        opts.dry_run,
        totals_by_importer
            .get_mut(&ImporterKey::Contracts)
            .expect("totals for contracts must exist"),
        |path, pool, company_id, dry_run| {
            Box::pin(bas_contracts::import_contracts_from_xml(
                pool, company_id, path, dry_run,
            ))
        },
        print_contract_report,
        |report| {
            (
                report.parsed,
                report.created,
                report.updated,
                report.skipped,
                report.conflicts,
            )
        },
        contract_reason_counts,
    )
    .await;

    failed |= process_artifacts(
        artifacts_by_importer
            .get(&ImporterKey::Acts)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        &pool,
        company_id,
        opts.dry_run,
        totals_by_importer
            .get_mut(&ImporterKey::Acts)
            .expect("totals for acts must exist"),
        |path, pool, company_id, dry_run| {
            Box::pin(bas_acts::import_acts_from_file(
                pool, company_id, path, dry_run,
            ))
        },
        print_act_report,
        |report| {
            (
                report.parsed,
                report.created,
                report.updated,
                report.skipped,
                report.conflicts,
            )
        },
        act_reason_counts,
    )
    .await;

    failed |= process_artifacts(
        artifacts_by_importer
            .get(&ImporterKey::Invoices)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        &pool,
        company_id,
        opts.dry_run,
        totals_by_importer
            .get_mut(&ImporterKey::Invoices)
            .expect("totals for invoices must exist"),
        |path, pool, company_id, dry_run| {
            Box::pin(bas_invoices::import_invoices_from_file(
                pool, company_id, path, dry_run,
            ))
        },
        print_invoice_report,
        |report| {
            (
                report.parsed,
                report.created,
                report.updated,
                report.skipped,
                report.conflicts,
            )
        },
        invoice_reason_counts,
    )
    .await;

    failed |= process_artifacts(
        artifacts_by_importer
            .get(&ImporterKey::Payments)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        &pool,
        company_id,
        opts.dry_run,
        totals_by_importer
            .get_mut(&ImporterKey::Payments)
            .expect("totals for payments must exist"),
        |path, pool, company_id, dry_run| {
            Box::pin(bas_payments::import_payments_from_csv(
                pool, company_id, path, dry_run,
            ))
        },
        print_payment_report,
        |report| {
            (
                report.parsed,
                report.created,
                report.updated,
                report.skipped,
                report.conflicts,
            )
        },
        payment_reason_counts,
    )
    .await;

    println!();
    for spec in IMPORTER_SPECS {
        if let Some(totals) = totals_by_importer.get(&spec.key) {
            print_totals(spec.totals_label, totals);
        }
    }

    let overall = sum_totals_from_map(&totals_by_importer);
    print_totals("усього", &overall);

    if failed {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_artifact, collect_supported_artifacts, collect_supported_artifacts_multi,
        discover_artifacts, is_importable_artifact, parse_args, BasArtifact, BasArtifactKind,
        CliOptions, DiscoveryReport, ParseOutcome,
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
            classify_artifact(Path::new("contracts_export.xlsx")),
            Some(BasArtifactKind::Contracts)
        );
        assert_eq!(
            classify_artifact(Path::new("Акти.xml")),
            Some(BasArtifactKind::Acts)
        );
    }

    #[test]
    fn classify_artifact_detects_payments_csv() {
        let kind = classify_artifact(Path::new("payment_statement.csv"));
        assert_eq!(kind, Some(BasArtifactKind::Payments));
    }

    #[test]
    fn importable_extensions_match_kind() {
        assert!(is_importable_artifact(
            BasArtifactKind::Counterparties,
            Path::new("counterparties.xlsx")
        ));
        assert!(is_importable_artifact(
            BasArtifactKind::Contracts,
            Path::new("contracts.xls")
        ));
        assert!(is_importable_artifact(
            BasArtifactKind::Payments,
            Path::new("payments.csv")
        ));
        assert!(is_importable_artifact(
            BasArtifactKind::Invoices,
            Path::new("invoices.xlsx")
        ));
        assert!(is_importable_artifact(
            BasArtifactKind::Acts,
            Path::new("acts.xlsx")
        ));
    }

    #[test]
    fn discover_artifacts_collects_supported_files_recursively() {
        let root = temp_dir("discovery");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("вкладена директорія має створюватися");

        fs::write(root.join("Контрагенти.xlsx"), "stub").expect("тестовий файл має створюватися");
        fs::write(nested.join("contracts_export.xml"), "<xml/>")
            .expect("тестовий файл має створюватися");
        fs::write(nested.join("payment_statement.csv"), "date,amount")
            .expect("тестовий файл має створюватися");
        fs::write(nested.join("notes.txt"), "skip").expect("тестовий файл має створюватися");

        let report = discover_artifacts(&root).expect("discovery має спрацювати");
        assert_eq!(report.recognized_count(), 3);
        assert_eq!(report.count_by_kind(BasArtifactKind::Counterparties), 1);
        assert_eq!(report.count_by_kind(BasArtifactKind::Contracts), 1);
        assert_eq!(report.count_by_kind(BasArtifactKind::Payments), 1);
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
    fn collect_supported_artifacts_keeps_requested_extensions() {
        let report = DiscoveryReport {
            root: PathBuf::from("."),
            artifacts: vec![
                BasArtifact {
                    path: PathBuf::from("contracts.xml"),
                    kind: BasArtifactKind::Contracts,
                },
                BasArtifact {
                    path: PathBuf::from("contracts.xlsx"),
                    kind: BasArtifactKind::Contracts,
                },
                BasArtifact {
                    path: PathBuf::from("acts.xml"),
                    kind: BasArtifactKind::Acts,
                },
                BasArtifact {
                    path: PathBuf::from("payments.csv"),
                    kind: BasArtifactKind::Payments,
                },
                BasArtifact {
                    path: PathBuf::from("bank_statement.csv"),
                    kind: BasArtifactKind::BankCsv,
                },
            ],
            skipped_files: Vec::new(),
        };

        let contracts = collect_supported_artifacts(&report, BasArtifactKind::Contracts);
        let payments = collect_supported_artifacts_multi(
            &report,
            &[BasArtifactKind::Payments, BasArtifactKind::BankCsv],
        );
        assert_eq!(contracts.len(), 2);
        assert_eq!(payments.len(), 2);
        assert_eq!(payments[0].path, PathBuf::from("bank_statement.csv"));
        assert_eq!(payments[1].path, PathBuf::from("payments.csv"));
    }
}
