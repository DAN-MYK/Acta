# Acta — Claude Code Instructions

## Проект
Десктопна програма управлінського обліку для українського бізнесу.
Акти виконаних робіт, видаткові накладні, контрагенти, звіти.
Мова інтерфейсу та коментарів: **українська**.

## Документація (Obsidian Vault)
Підключено через MCP (obsidian-claude-code-mcp).
Перед задачею читати відповідний файл vault:

- Svelte компонент → `Technologies/Svelte UI.md`
- PDF → `Technologies/PDF Generation.md`
- BAS імпорт → `Integrations/BAS Integration.md`
- Банки → `Integrations/Bank Integrations.md`
- Схема БД → `Database/DB Schema.md`
- Функціонал → `Features/Feature List.md`

## Стек
- Мова: Rust (навчальний проект — пояснювати концепції при написанні коду)
- UI: Tauri 2 + Svelte (frontend/) + TypeScript
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
cargo build --lib
cd src-tauri && cargo tauri dev   # Tauri dev режим
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

### Svelte / Tauri
- UI логіка в frontend/src/ (.svelte, .ts) | Бізнес логіка в Rust (src/tauri_api/)
- Дані через Tauri commands (invoke) | події через Svelte stores
- Типи між Rust і TS: serde → JSON → TypeScript interfaces

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
├── src/               ← lib-крейт (Slint-free)
│   ├── lib.rs
│   ├── db/            ← CRUD функції
│   ├── models/        ← Rust структури
│   ├── tauri_api/     ← команди для Tauri (invoke handlers)
│   ├── actions/       ← бізнес-операції
│   ├── pdf/           ← Typst генерація
│   └── import/        ← Парсери BAS, банків
├── src-tauri/         ← Tauri binary (src-tauri/src/)
│   └── src/commands/  ← #[tauri::command] handlers
├── frontend/          ← Svelte UI (src/lib/, src/routes/)
├── templates/         ← .typ шаблони Typst
├── migrations/        ← sqlx міграції
└── storage/           ← файли на диску
```

## Команди
```bash
# Основний запуск — Tauri + Svelte
cd src-tauri && cargo tauri dev                   # dev режим (hot reload)
cd src-tauri && cargo tauri build                 # production build

# Бібліотека та утиліти
cargo build --lib                                 # компіляція lib-крейту
cargo run --bin migrate -- --input ./bas-export/  # міграція з BAS
sqlx migrate run                                  # міграції БД
cargo sqlx prepare                                # offline SQL (після зміни запитів)
cargo build --tests                               # повна компіляція: lib + тести
cargo test                                        # всі тести
```
> `cargo run` більше не працює — немає default binary. Запуск тільки через `cargo tauri dev`.

## Уроки (поповнювати при помилках)
@.claude/lessons.md
