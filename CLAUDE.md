# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
- Всі рядки UI — в `frontend/src/lib/config/ui.ts`, не hardcode в компонентах
- CSS — screen-scoped файли в `frontend/src/styles/`, дизайн-токени з `--acta-*` префіксом
- Гроші у frontend: bigint minor-units через `money.ts` (parseMoneyToMinor / formatMinorMoney)

## Frontend Архітектура

### Шар виклику Tauri команд (api.ts)
`frontend/src/lib/api.ts` — єдина точка входу для всіх 61 Tauri команд.
`appInvoke<T>(command, payload?)` автоматично роутить:
- в Tauri runtime → `invoke()` з `@tauri-apps/api/core`
- в браузері / тестах → `browserFixtureInvoke()` з `browser-fixtures.ts`

Це дає змогу запускати всі тести без Tauri runtime.

### Svelte Stores (стан екранів)
Кожен екран має свій store в `frontend/src/lib/stores/`.
Типовий публічний API store:
- `load()` / `refresh()` — завантаження даних через api.ts
- `openEditor(id?)` / `closeEditor(force?)` — lifecycle редактора
- Dirty-check: `cloneSnapshot` + `isEditorFormDirty` з `editorDirty.ts`

### Presentation Layer
`*Presentation.ts` файли (payments, tasks, counterparty) — чиста бізнес-логіка форматування та фільтрації без store/UI залежностей. Тестуються без моків.

### Тестування
- Screen/component тести: `@vitest-environment jsdom` + `vi.hoisted()` для mock stores
- Store тести: мокування subscribe, перевірка load/error стейтів
- Presentation тести: чисті функції, без моків

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
├── src/               ← lib-крейт
│   ├── db/            ← CRUD функції (14 модулів, по одному на таблицю)
│   ├── models/        ← Rust структури
│   ├── tauri_api/     ← бізнес-логіка між командами і БД
│   ├── actions/       ← складні бізнес-операції
│   ├── pdf/           ← Typst генерація
│   └── import/        ← Парсери BAS, банків (CSV/XLSX)
├── src-tauri/src/commands/  ← тонкі #[tauri::command] обгортки (61 команда)
├── frontend/src/lib/
│   ├── screens/       ← 7 екранів (.svelte) + __tests__/
│   ├── stores/        ← Svelte stores (14 файлів) + __tests__/
│   ├── components/    ← Button, Modal, Table, KPI, StatusBadge...
│   ├── config/        ← UI рядки, мітки, конфіги (ui.ts — головний)
│   ├── api.ts         ← виклик Tauri команд
│   ├── browser-fixtures.ts  ← mock-дані для тестів без Tauri
│   └── *Presentation.ts     ← presentation layer (бізнес-логіка UI)
├── frontend/src/styles/     ← screen-scoped CSS файли
├── tests/             ← Rust integration tests (db_integration, tauri_vertical_slice)
├── templates/         ← .typ шаблони Typst
├── migrations/        ← sqlx міграції
└── storage/           ← файли на диску
```

## Команди
```bash
# Основний запуск — Tauri + Svelte
cd src-tauri && cargo tauri dev                   # dev режим (hot reload)
cd src-tauri && cargo tauri build                 # production build

# Frontend
cd frontend && npm run test:frontend              # всі frontend тести (vitest)
cd frontend && npx vitest run --config vitest.config.mjs -t "назва тесту"  # один тест
cd frontend && npm run check                      # svelte-check типізація
cd frontend && npm run dev                        # Vite dev server (port 1420)

# Rust
cargo build --lib                                 # компіляція lib-крейту
cargo build --tests                               # повна компіляція: lib + тести
cargo test                                        # всі тести
cargo test --test unit_business_logic             # unit тести (без БД)
TEST_DATABASE_URL=... cargo test --test db_integration      # DB інтеграційні
TEST_DATABASE_URL=... cargo test --test tauri_vertical_slice # vertical slice
cargo run --bin reseed                            # seed тестової БД

# БД
sqlx migrate run                                  # міграції БД
cargo sqlx prepare                                # offline SQL (після зміни запитів)
```
> `cargo run` більше не працює — немає default binary. Запуск тільки через `cargo tauri dev`.

## Уроки (поповнювати при помилках)
@.claude/lessons.md
