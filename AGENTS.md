# Acta Codex Instructions

Це єдиний канонічний файл інструкцій для Codex у репозиторії `Acta`.

## Контекст проєкту
- Десктопна програма управлінського обліку для українського бізнесу.
- Основні сутності: акти виконаних робіт, видаткові накладні, контрагенти, договори, платежі, звіти.
- Мова інтерфейсу, коментарів і пояснень: українська.
- Базова база знань і To Do для побудови проєкту: `C:\Users\MykhailoDan\OneDrive - UDPR\Obsidian\Mykhailo_Dan\development`.
- Якщо потрібен контекст по рішенню, спочатку шукай його саме там, а вже потім у коді або Vault-файлах задачі.
- Використовуй цю папку як місце для фіксації рішень, планів, черги задач і коротких нотаток по прогресу.
- Якщо задача змінює архітектуру, правила або домен, онови відповідну нотатку в цій папці.

## Коли читати Vault
Перед роботою звіряйся з Obsidian Vault через MCP `obsidian-Codex-mcp`, якщо задача стосується:
- UI -> `Technologies/Slint UI.md`
- PDF -> `Technologies/PDF Generation.md`
- BAS -> `Integrations/BAS Integration.md`
- Банків -> `Integrations/Bank Integrations.md`
- Бази даних -> `Database/DB Schema.md`
- Функціоналу -> `Features/Feature List.md`

## Стек
- Rust.
- UI: Slint, без вебтехнологій.
- БД: PostgreSQL + sqlx, async, compile-time перевірка SQL.
- PDF: Typst CLI або Rust-бібліотека; для читання PDF можна використовувати `lopdf`.
- Файловий моніторинг: `notify`.
- XML: `quick-xml`.
- Excel: `calamine`.

## Жорсткі правила
### Rust
- Для всіх фінансових сум використовуй `rust_decimal::Decimal`.
- Не використовуй `f32` або `f64` для грошей чи сум.
- Для помилок використовуй `anyhow::Result`.
- Не використовуй `.unwrap()` у продакшені.
- Для дат використовуй `chrono::NaiveDate`.
- Для первинних ключів використовуй `uuid::Uuid`.
- Усі операції з БД та файлами мають бути async/await.

### База даних
- Для сум використовуй `DECIMAL(15,2)`.
- Для кількостей використовуй `DECIMAL(15,4)`.
- Не використовуй `FLOAT`, `REAL`, `DOUBLE PRECISION` для фінансових або кількісних полів.
- Кожна таблиця має містити `id UUID`, `created_at`, `updated_at`.
- Для документів з BAS використовуй `bas_id VARCHAR(100) UNIQUE`.
- Міграції зберігай окремими файлами в `migrations/`.
- Для SQL-запитів використовуй `sqlx::query_as!`.

### Slint
- UI-логіка тільки в `.slint`.
- Бізнес-логіка тільки в Rust.
- Дані передавай через `in` / `out` properties.
- Події передавай через `callback`.
- Використовуй `std-widgets`.

## Робочий порядок
1. Спочатку звір релевантний файл у Vault або коді.
2. Далі внеси зміни в найменшу потрібну частину системи.
3. Після змін перевір типи, SQL і Slint-логіку на відповідність правилам вище.
4. Якщо змінюєш SQL-запити, онови `cargo sqlx prepare`.
5. Якщо змінюєш міграції, перевір їх через `sqlx migrate run`.
6. Якщо задача зачіпає поведінку, додай або онови тести.

## Команди
```bash
cargo run
sqlx migrate run
cargo sqlx prepare
cargo run --bin migrate -- --input ./bas-export/
cargo test
```

## Доменні відповідності
| Українська | Rust struct | Таблиця |
|-----------|-------------|---------|
| Контрагент | `Counterparty` | `counterparties` |
| Акт виконаних робіт | `Act` | `acts` |
| Позиція акту | `ActItem` | `act_items` |
| Видаткова накладна | `Invoice` | `invoices` |
| Позиція накладної | `InvoiceItem` | `invoice_items` |
| Договір | `Contract` | `contracts` |
| Платіж | `Payment` | `payments` |
| Стаття доходів/витрат | `Category` | `categories` |
| Шаблон документу | `DocumentTemplate` | `document_templates` |

## Структура проєкту
```text
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

## Перший запуск
```bash
cargo install sqlx-cli --no-default-features --features native-tls,postgres
sqlx database create
sqlx migrate run
cargo build
```

## Не забувати
- `.env` не комітити, є `.env.example` без пароля.
- Якщо не вистачає контексту, спочатку шукай у Vault або коді.
- Якщо бачиш локальні незакомічені зміни, не перезатирай їх без потреби.
