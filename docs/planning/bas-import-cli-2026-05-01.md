# План доробки BAS import CLI

Оновлено: `2026-05-01`

## Поточний стан

- `src/bin/migrate.rs` вже містить discovery, dry-run/import flow, агрегацію звітів і запуск імпортерів у правильному порядку.
- `src/import/bas_counterparties.rs`, `src/import/bas_contracts.rs`, `src/import/bas_invoices.rs` вже підтримують XML та Excel (`.xlsx` / `.xls`).
- `src/import/bas_payments.rs` уже закриває CLI-частину для платежів через CSV банківських виписок.
- `src/import/bas_acts.rs` поки підтримує тільки XML, через що CLI не завершує вимогу по BAS Excel export end-to-end.
- У `tests/db_integration.rs` вже є інтеграційні перевірки імпортної логіки, а у `src/bin/migrate.rs` та `src/import/bas_*.rs` є unit-тести, але бракує перевірок для Excel-актів і повного CLI сценарію.

## Кроки реалізації

1. Додати Excel-парсинг для актів у `src/import/bas_acts.rs` без зміни доменного контракту імпорту.
2. Розширити CLI discovery/import flow у `src/bin/migrate.rs`, щоб акти з Excel вважалися повноцінно підтриманими артефактами.
3. Додати unit-тести на parsing Excel/extension routing для актів і оновити CLI-тести на підтримуваний формат.
4. Додати або оновити інтеграційну перевірку dry-run / CLI flow на тестових BAS-даних, наскільки це дозволить локальне оточення.
5. Запустити збірку, релевантні unit/integration тести та окремий прогін `cargo run --bin migrate -- --input ...`.

## Ризики і залежності

- Excel parsing у Rust потребує або генерації тимчасового `.xlsx` fixture, або наявного тестового файлу; важливо не заводити зайву інфраструктуру.
- CLI dry-run усе одно залежить від `DATABASE_URL` і наявності компанії у БД, тому для повного прогону може знадобитися локальна тестова база.
- У репозиторії є сторонні незакомічені зміни; під час правок не торкатися чужих файлів поза BAS import областю.

## Перевірки

- `cargo build --tests`
- `cargo test --bin migrate`
- релевантні unit / integration тести для `bas_acts`, `bas_invoices`, `db_integration`
- `cargo run --bin migrate -- --input <test-dir> --dry-run`
