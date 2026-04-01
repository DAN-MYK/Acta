// Утиліта імпорту даних з BAS
//
// Використання:
//   cargo run --bin migrate -- --input ./bas-export/
//   cargo run --bin migrate -- --input ./bas-export/ --dry-run

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

    println!("Вхідна директорія: {}", opts.input_dir);
    if opts.dry_run {
        println!("Режим dry-run: зміни до БД не застосовуються");
    }

    // TODO: логіка імпорту BAS файлів
}

#[cfg(test)]
mod tests {
    use super::{CliOptions, ParseOutcome, parse_args};

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
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
}
