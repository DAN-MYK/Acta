# Acta — Claude Code Instructions

## Проект
Десктопна програма управлінського обліку для українського бізнесу.
Акти виконаних робіт, видаткові накладні, контрагенти, звіти.
Мова інтерфейсу та коментарів: **українська**.

## Документація (Obsidian Vault)
Підключено через MCP (obsidian-claude-code-mcp).
Перед задачею читати відповідний файл vault:

- UI компонент → `Technologies/Slint UI.md`
- PDF → `Technologies/PDF Generation.md`
- BAS імпорт → `Integrations/BAS Integration.md`
- Банки → `Integrations/Bank Integrations.md`
- Схема БД → `Database/DB Schema.md`
- Функціонал → `Features/Feature List.md`

## Стек
- Мова: Rust (навчальний проект — пояснювати концепції при написанні коду)
- UI: Slint (.slint файли, БЕЗ веб технологій)
- БД: PostgreSQL + sqlx (async, compile-time перевірка SQL)
- PDF: Typst CLI або як Rust бібліотека + lopdf для читання
- Файловий моніторинг: notify crate
- XML: quick-xml | Excel: calamine

## Налаштування (перший запуск)
```bash
cargo install sqlx-cli --no-default-features --features native-tls,postgres
# .env файл:
# DATABASE_URL=postgres://postgres:password@localhost:5432/acta
sqlx database create && sqlx migrate run
cargo build
```
> `.env` НЕ комітити. Є `.env.example` без паролю.

## Правила коду

### Rust — ОБОВ'ЯЗКОВО
- `rust_decimal::Decimal` для ВСІХ фінансових сум — ніколи f32/f64
- `Result<T>` з anyhow для error handling — ніколи `.unwrap()` у продакшені
- `chrono::NaiveDate` для дат, `uuid::Uuid` для PK
- Async/await для всіх операцій з БД та файлами

### БД — ОБОВ'ЯЗКОВО
- DECIMAL(15,2) для сум, DECIMAL(15,4) для кількості — ніколи FLOAT
- Кожна таблиця: `id UUID`, `created_at`, `updated_at`
- `bas_id VARCHAR(100) UNIQUE` для документів з BAS
- Міграції — окремі файли в `/migrations/`
- Використовувати `sqlx::query_as!` макрос

### Slint
- UI логіка ТІЛЬКИ в .slint | Бізнес логіка ТІЛЬКИ в Rust
- Дані через `in`/`out` properties | Події через `callback`
- Використовувати `std-widgets`

## Домен
| Українська | Rust struct | Таблиця |
|-----------|-------------|---------|
| Контрагент | Counterparty | counterparties |
| Акт виконаних робіт | Act | acts |
| Позиція акту | ActItem | act_items |
| Видаткова накладна | Invoice | invoices |
| Позиція накладної | InvoiceItem | invoice_items |
| Договір | Contract | contracts |
| Платіж | Payment | payments |
| Стаття доходів/витрат | Category | categories |
| Шаблон документу | DocumentTemplate | document_templates |

## Структура проекту
```
acta/
├── src/
│   ├── main.rs
│   ├── db/        ← CRUD функції
│   ├── models/    ← Rust структури
│   ├── pdf/       ← Typst генерація
│   └── import/    ← Парсери BAS, банків
├── ui/            ← .slint файли
├── templates/     ← .typ шаблони Typst
├── migrations/    ← sqlx міграції
└── storage/       ← файли на диску
```

## Команди
```bash
cargo run                                         # запуск
sqlx migrate run                                  # міграції
cargo sqlx prepare                                # offline SQL (після зміни запитів)
cargo run --bin migrate -- --input ./bas-export/  # міграція з BAS
cargo test
```

## Уроки (поповнювати при помилках)
@.claude/lessons.md
