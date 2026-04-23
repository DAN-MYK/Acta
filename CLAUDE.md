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

## Відомі технічні борги (виправити у майбутньому)

### [2026-04-23] Read-Modify-Write на ViewData — latent fragility
**Де:** `src/ui/documents.rs:apply_documents_to_ui`, `src/ui/counterparties.rs:apply_counterparties_to_ui`  
**Проблема:** Патерн `let previous = ui.get_documents(); ui.set_documents(... previous.fields ...)` не є атомарним.
Зараз безпечний бо всі apply-функції викликаються лише з Slint event thread (через `upgrade_in_event_loop`).
Якщо у майбутньому додати другий паралельний запит що теж викликає `apply_documents_to_ui` — можливий race condition.  
**Виправлення:** Передавати в apply-функції явні поля замість read-back з UI, або гарантувати single writer через архітектуру.

### [2026-04-23] ShellChrome hardcoded — не відображає реальну компанію
**Де:** `src/bootstrap.rs:build_ui` (~рядок 263)  
**Проблема:** `company_name: "Acta"`, `user_name: "Адміністратор"` — статичні рядки. Не оновлюються при зміні компанії через налаштування.  
**Виправлення:** Завантажувати з `SettingsData.company_info.short_name` у `apply_initial_ui_data`, і оновлювати через `wire_settings_callbacks` при `settings-company-saved`.

### [2026-04-23] Відсутні тести на data wiring (не тільки callback wiring)
**Де:** `tests/ui_events.rs`  
**Проблема:** Тести перевіряють що callbacks зареєстровані, але не перевіряють що дані правильно потрапляють у Slint properties (`tasks-screen.open-count`, `shell.company-name` тощо). Якщо field name зміниться — Slint compile впіймає, але якщо property просто відключиться — тест не помітить.  
**Виправлення:** Додати тести у Epic 9 presenter layer: після `apply_*_to_ui` перевіряти що `ui.get_tasks_screen().open_count == expected`.
