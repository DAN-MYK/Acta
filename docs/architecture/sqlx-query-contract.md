# SQLx query contract

Оновлено: 2026-05-08.

## Правило

Для нових стабільних CRUD-запитів пріоритет має `sqlx::query!` / `sqlx::query_as!`, бо вони дають compile-time перевірку SQL і типів.

Runtime-запити `sqlx::query_as::<_, T>()`, `sqlx::query_scalar::<_, T>()` і `QueryBuilder` допускаються, коли:

- запит будується динамічно з опційних фільтрів;
- модуль уже історично побудований на runtime SQL і зміна на макроси не є частиною поточної задачі;
- запит повертає локальний row-тип або агрегат, який складно підтримувати через offline metadata без непропорційного churn;
- зміна стосується міграційного/імпортного коду, де integration tests є основним захистом контракту.

## Verification

- Якщо додано або змінено `sqlx::query!`, `sqlx::query_as!` чи `sqlx::query_scalar!`, потрібно запускати `cargo sqlx prepare` і комітити зміни `.sqlx`.
- Якщо змінено тільки runtime SQL, обов'язковими є `cargo test --test db_integration` і `cargo build --tests`.
- Для runtime SQL, який торкається меж компаній, integration test має явно перевіряти `company_id` isolation.

## Поточний стан

У backend уже є значний runtime SQL шар у `src/db/acts.rs`, `src/db/invoices.rs`, `src/db/waybills.rs`, `src/db/payments.rs`, `src/db/contracts.rs`, `src/db/reports.rs` і dashboard/report API. Це не скасовує бажаний напрямок на compile-time макроси, але робить перехід окремою refactoring-задачею, а не побічним ефектом кожної функціональної зміни.

Поточний пріоритет для безпеки даних: будь-який Tauri-facing read/update/delete/status/import lookup повинен або приймати `company_id`, або делегувати у scoped helper (`*_scoped`).
