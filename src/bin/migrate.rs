// Утиліта імпорту даних з BAS
//
// Використання:
//   cargo run --bin migrate -- --input ./bas-export/
//   cargo run --bin migrate -- --input ./bas-export/ --dry-run

fn main() {
    let args: Vec<String> = std::env::args().collect();

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
                    eprintln!("Помилка: --input потребує шлях до директорії");
                    std::process::exit(1);
                }
            }
            "--dry-run" => dry_run = true,
            "--help" | "-h" => {
                println!("Використання: migrate --input <директорія> [--dry-run]");
                return;
            }
            other => {
                eprintln!("Невідомий аргумент: {other}");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let Some(input) = input_dir else {
        eprintln!("Помилка: --input є обов'язковим аргументом");
        eprintln!("Використання: migrate --input <директорія> [--dry-run]");
        std::process::exit(1);
    };

    println!("Вхідна директорія: {input}");
    if dry_run {
        println!("Режим dry-run: зміни до БД не застосовуються");
    }

    // TODO: логіка імпорту BAS файлів
}
