// Утиліта імпорту даних з BAS.
//
// Поточний baseline:
// - перевіряє вхідну директорію
// - рекурсивно знаходить підтримувані файли експорту
// - класифікує їх за типом сутності
// - у dry-run виводить план імпорту без змін у БД

use std::fs;
use std::path::{Path, PathBuf};

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
        self.artifacts.iter().filter(|item| item.kind == kind).count()
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
            if name.contains("контраг") || name.contains("counterpart") || name.contains("client") {
                BasArtifactKind::Counterparties
            } else if name.contains("догов") || name.contains("contract") {
                BasArtifactKind::Contracts
            } else if name.contains("акт") || name.contains("act") {
                BasArtifactKind::Acts
            } else if name.contains("наклад") || name.contains("invoice") || name.contains("рах") {
                BasArtifactKind::Invoices
            } else if name.contains("плат") || name.contains("payment") || name.contains("bank") {
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
        return Err(format!("Очікується директорія, а не файл: {}", root.display()));
    }

    let mut report = DiscoveryReport {
        root: root.to_path_buf(),
        artifacts: Vec::new(),
        skipped_files: Vec::new(),
    };

    visit_dir(root, &mut report)?;
    report.artifacts.sort_by(|left, right| left.path.cmp(&right.path));
    report.skipped_files.sort();

    Ok(report)
}

fn visit_dir(path: &Path, report: &mut DiscoveryReport) -> Result<(), String> {
    let entries = fs::read_dir(path)
        .map_err(|error| format!("Не вдалося прочитати директорію {}: {error}", path.display()))?;

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

fn print_report(report: &DiscoveryReport, dry_run: bool) {
    println!("Вхідна директорія: {}", report.root.display());
    if dry_run {
        println!("Режим dry-run: зміни до БД не застосовуються");
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

    if report.artifacts.is_empty() {
        println!("Підтримувані файли не знайдено.");
        return;
    }

    println!("План імпорту:");
    for artifact in &report.artifacts {
        println!("  - [{}] {}", artifact.kind.label(), artifact.path.display());
    }

    if !report.skipped_files.is_empty() {
        println!(
            "Пропущено {} файлів з непідтримуваними розширеннями.",
            report.skipped_files.len()
        );
    }
}

fn main() {
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

    print_report(&report, opts.dry_run);

    if !opts.dry_run {
        println!("Імпорт у БД ще не реалізований. Запустіть з --dry-run для перевірки експорту.");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_artifact, discover_artifacts, parse_args, BasArtifactKind, CliOptions,
        ParseOutcome,
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
    fn classify_artifact_detects_bank_csv() {
        let kind = classify_artifact(Path::new("statement.csv"));
        assert_eq!(kind, Some(BasArtifactKind::BankCsv));
    }

    #[test]
    fn discover_artifacts_collects_supported_files_recursively() {
        let root = temp_dir("discovery");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("вкладена директорія має створюватися");

        fs::write(root.join("Контрагенти.xml"), "<xml/>").expect("тестовий файл має створюватися");
        fs::write(nested.join("acts_export.xml"), "<xml/>").expect("тестовий файл має створюватися");
        fs::write(nested.join("notes.txt"), "skip").expect("тестовий файл має створюватися");

        let report = discover_artifacts(&root).expect("discovery має спрацювати");
        assert_eq!(report.recognized_count(), 2);
        assert_eq!(report.count_by_kind(BasArtifactKind::Counterparties), 1);
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
}
