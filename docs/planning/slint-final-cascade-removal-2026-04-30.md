# Slint — Фінальне каскадне видалення

> **Archived execution note:** документ збережено як audit trail фінального Slint cutover. Поточний runtime — Tauri/Svelte; не використовуй перелік Slint файлів як live backlog.

> **Статус: ВИКОНАНО** — 2026-04-30
>
> Всі кроки виконані в одному cleanup pass разом із Category 1.
> `tests/ui_events.rs` вже видалено до початку цього пасу,
> тому каскад пройшов повністю без окремого кроку.

## Що було видалено (2026-04-30)

### Category 1 — Legacy Slint binary (src/main.rs + wiring)

| Видалено | Розмір |
|----------|--------|
| `src/main.rs` | Slint entry point |
| `src/bootstrap.rs` | module root |
| `src/bootstrap/company_switcher.rs` | AppWindow wiring |
| `src/bootstrap/document_chain.rs` | AppWindow wiring |
| `src/bootstrap/inbox.rs` | AppWindow wiring |
| `src/bootstrap/navigation.rs` | AppWindow wiring |
| `src/bootstrap/palette.rs` | AppWindow wiring |
| `src/bootstrap/refresh.rs` | Weak<AppWindow> + upgrade_in_event_loop |
| `src/bootstrap/shell.rs` | AppWindow wiring |
| `src/bootstrap/wiring.rs` | AppWindow wiring |
| `src/ui/mod.rs` | presenter layer root |
| `src/ui/helpers.rs` | SharedString utils |
| `src/ui/dashboard.rs` | presenter |
| `src/ui/documents.rs` | presenter |
| `src/ui/counterparties.rs` | presenter |
| `src/ui/payments.rs` | presenter |
| `src/ui/tasks.rs` | presenter |
| `src/ui/reports.rs` | presenter |
| `src/ui/settings.rs` | presenter |

### Category 2 — Slint runtime (замінений)

| Видалено | Причина |
|----------|---------|
| `tests/ui_events.rs` | замінено `tests/tauri_vertical_slice.rs` + Vitest store tests |
| `build.rs` | більше не потрібен — ui/*.slint gone |
| `ui/` (13 .slint файлів) | весь Slint UI directory |
| Cargo.toml: `slint = "1.15"` | залежність видалена |
| Cargo.toml: `slint-build = "1.15"` | build-dep видалена |
| Cargo.toml: `i-slint-backend-testing` | вже було видалено раніше |
| Cargo.toml: `default-run = "acta"` | Slint binary entry |
| Cargo.toml: `[[bin]] name = "acta"` | Slint binary entry |

## Поточний стан після cleanup

- **lib-крейт** (`src/lib.rs`) — повністю Slint-free, завжди був
- **Tauri binary** (`src-tauri/`) — основний UI entry point
- **Тести** — `db_integration`, `tauri_vertical_slice`, `unit_business_logic`
- **Утиліти** — `cargo run --bin migrate`, `cargo run --bin reseed`
- **Desktop build check** — канонічний локальний build тепер `cargo build --manifest-path src-tauri/Cargo.toml`

## Перевірка

```bash
# Має проходити без помилок
cargo build --tests
cargo test

# Запуск додатку
cd src-tauri && cargo tauri dev
```
