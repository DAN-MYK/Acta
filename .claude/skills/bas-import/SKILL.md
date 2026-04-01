# Skill: BAS Import

Використовується при роботі з імпортом з BAS/BAF.

## Контекст
- BAS/BAF — українські конфігурації на платформі 1С:Підприємство 8.3
- Одноразова міграція всього архіву + моніторинг папки нових файлів
- Кожен документ зберігається з `bas_id` для запобігання дублікатів

## Фаза 1: Одноразова міграція

```rust
// cargo run --bin migrate -- --input ./bas-export/
struct MigrationRunner { db: PgPool }

impl MigrationRunner {
    async fn run(&self, input_dir: &Path) -> Result<()> {
        self.import_counterparties(input_dir).await?; // спочатку!
        self.import_contracts(input_dir).await?;
        self.import_acts(input_dir).await?;
        self.import_invoices(input_dir).await?;
        self.import_payments(input_dir).await?;
        Ok(())
    }
}
```

### Ідемпотентний INSERT (важливо!)
```rust
sqlx::query!(
    "INSERT INTO acts (bas_id, number, date, ...)
     VALUES ($1, $2, $3, ...)
     ON CONFLICT (bas_id) DO NOTHING"
).execute(&db).await?;
```

## Фаза 2: Моніторинг папки нових файлів

```rust
use notify::{Watcher, RecursiveMode};

pub async fn start_watcher(watch_path: PathBuf, db: PgPool) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(&watch_path, RecursiveMode::NonRecursive)?;

    for event in rx {
        if let Ok(e) = event {
            if e.kind.is_create() {
                for path in e.paths {
                    tokio::spawn(process_file(path, db.clone()));
                }
            }
        }
    }
    Ok(())
}

async fn process_file(path: PathBuf, db: PgPool) -> Result<()> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("xml")  => import_bas_xml(&path, &db).await?,
        Some("xlsx") => import_bas_excel(&path, &db).await?,
        Some("csv")  => import_bank_statement(&path, &db).await?,
        _ => {}
    }
    // Переміщаємо оброблений файл
    let done = path.parent().unwrap().join("processed").join(path.file_name().unwrap());
    std::fs::rename(&path, &done)?;
    Ok(())
}
```

## XML формат BAS (акти — 1С82АВР)
```rust
use quick_xml::de::from_str;

#[derive(serde::Deserialize)]
struct BasAct {
    #[serde(rename = "Номер")]
    number: String,
    #[serde(rename = "Дата")]
    date: String,
    // ...
}
```

## Excel формат BAS
```rust
use calamine::{Reader, open_workbook, Xlsx};

let mut wb: Xlsx<_> = open_workbook(&path)?;
if let Some(Ok(sheet)) = wb.worksheet_range("Лист1") {
    for row in sheet.rows().skip(1) { // пропускаємо заголовок
        // row[0], row[1], ...
    }
}
```
