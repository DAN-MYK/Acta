# Slint — Безпечний чекліст видалення (2026-04-30)

> **Archived execution note:** checklist збережено як історія cutover. Після `2026-04-30` live safety net описаний у `docs/testing/ui-safety-net.md`.

> Статус на 2026-04-30: історичний документ.
> Slint runtime вже видалено, тому цей файл не описує активне дерево і зберігається як архів cleanup-рішення.

## Актуальний стан дерева

- `src/main.rs` відсутній
- `build.rs` відсутній
- `ui/` відсутня
- `tests/ui_events.rs` відсутній
- root `Cargo.toml` не містить `slint`, `slint-build`, `i-slint-backend-testing`
- єдиний desktop entrypoint: `src-tauri/src/main.rs`
- канонічні перевірки цього етапу:
  - `cargo build --manifest-path src-tauri/Cargo.toml`
  - `cargo test --no-run`
  - `cargo test --test tauri_vertical_slice`
  - `npm run check`

## Як читати цей документ

- Усі старі секції про `src/main.rs`, `ui/*.slint`, `tests/ui_events.rs` і Slint-залежності треба трактувати як вже виконаний інвентар.
- Якщо потрібен фактичний поточний стан міграції, джерелом істини є `docs/architecture/tauri-runtime.md`.
- Якщо потрібен підтверджений факт фінального cleanup, див. `slint-final-cascade-removal-2026-04-30.md`.

## Архівний зміст

Початкове призначення цього чекліста було зафіксувати порядок безпечного видалення Slint-only шару:

1. прибрати legacy entrypoint і wiring;
2. прибрати headless Slint tests;
3. прибрати `build.rs` і `ui/*.slint`;
4. прибрати Slint-залежності з `Cargo.toml`;
5. перевести канонічний desktop runner на Tauri.

На 2026-04-30 всі ці кроки вже завершені.
