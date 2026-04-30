# Slint — Безпечний чекліст видалення (2026-04-30)

Інвентар усього що залишилось Slint-only і живе виключно для legacy UI.
Lib-крейт (`src/lib.rs` і всі його підмодулі) вже **повністю Slint-free**.

---

## Інвентар Slint-only коду

### A. Бінарний entry point + wiring

| Файл | Що використовує |
|------|----------------|
| `src/main.rs` | `slint::include_modules!()`, `AppWindow`, `bootstrap::build_ui`, `ui.run()` |
| `src/bootstrap/refresh.rs` | `slint::Weak<AppWindow>`, `upgrade_in_event_loop`, усі `apply_*_to_ui` |
| `src/bootstrap/shell.rs` | `AppWindow` |
| `src/bootstrap/company_switcher.rs` | `AppWindow` |
| `src/bootstrap/document_chain.rs` | `AppWindow` |
| `src/bootstrap/inbox.rs` | `AppWindow` |
| `src/bootstrap/navigation.rs` | `AppWindow` |
| `src/bootstrap/palette.rs` | `AppWindow` |
| `src/bootstrap/wiring.rs` | `AppWindow` |
| `src/ui/mod.rs` | оголошення підмодулів |
| `src/ui/helpers.rs` | `slint::SharedString` утиліти |
| `src/ui/dashboard.rs` | `AppWindow`, Slint types |
| `src/ui/documents.rs` | `AppWindow`, Slint types |
| `src/ui/counterparties.rs` | `AppWindow`, Slint types |
| `src/ui/payments.rs` | `AppWindow`, Slint types |
| `src/ui/tasks.rs` | `AppWindow`, Slint types |
| `src/ui/reports.rs` | `AppWindow`, Slint types |
| `src/ui/settings.rs` | `AppWindow`, Slint types |

Усього: **src/main.rs + 8 bootstrap + 9 ui = 18 файлів**.

### B. Інтеграційний тест

| Файл | Що використовує |
|------|----------------|
| `tests/ui_events.rs` | Було legacy Slint callback-покриттям; замінено на Tauri/frontend test layer (`tests/tauri_vertical_slice.rs` + `frontend/src/lib/stores/__tests__/*`) |

### C. UI файли (13 штук)

```
ui/app.slint          ← root, компілюється у build.rs
ui/shell.slint        ← імпортується у tests/ui_events.rs
ui/types.slint        ← імпортується у tests/ui_events.rs
ui/design-tokens.slint
ui/components.slint
ui/icons.slint
ui/dashboard.slint
ui/documents.slint
ui/counterparties.slint
ui/payments.slint
ui/tasks.slint
ui/settings.slint
ui/reports.slint
```

### D. Build script

| Файл | Що робить |
|------|-----------|
| `build.rs` (root) | `slint_build::compile_with_config("ui/app.slint", ...)` → генерує Rust-код, який споживає `slint::include_modules!()` |

### E. Cargo.toml залежності

| Секція | Крейт | Причина |
|--------|-------|---------|
| `[dependencies]` | `slint = "1.15"` | runtime для binary і тестів |
| `[dev-dependencies]` | `i-slint-backend-testing = "=1.15.1"` | headless UI тести |
| `[build-dependencies]` | `slint-build = "1.15"` | компіляція .slint у build.rs |

---

## Категорії видалення

### Категорія 1 — Після settings Svelte екрану

Після того як Svelte/Tauri стає основним binary, Slint binary є мертвим кодом.
Можна видалити **одразу після переходу на `cargo tauri dev` як основний runner**:

- [ ] `src/main.rs` — замінюється `src-tauri/src/main.rs`
- [ ] `src/bootstrap/` — всі 8 файлів (wiring без UI аналога в Tauri)
- [ ] `src/ui/` — всі 9 файлів (presenter layer замінений `src/tauri_api/` + Svelte)

> **Умова:** `cargo build -p acta-tauri` компілюється без помилок і запускається.
> `cargo build` (Slint binary) перестає бути обов'язковим у CI.

### Категорія 2 — Після нового test/CI cutover

`tests/ui_events.rs` — єдина причина, чому build.rs, ui/*.slint і Slint-deps
ще потрібні після вимкнення Slint binary:

- [x] `tests/ui_events.rs` — замінено Vitest store-smoke тестами для Svelte frontend і Tauri vertical-slice smoke в Rust
- [ ] `build.rs` (root) — після видалення ui_events.rs
- [ ] `ui/` — всі 13 .slint файлів — після видалення build.rs
- [ ] Cargo.toml: `slint`, `i-slint-backend-testing`, `slint-build` — останніми

> **Умова:** Новий test suite (Playwright або unit тести Svelte) покриває
> callback-контракти, які зараз тестує `tests/ui_events.rs`.

### Категорія 3 — Поки не можна чіпати

Ніщо в lib-крейті не потребує видалення — він уже чистий.
До Категорії 1 не можна видаляти `src/ui/` бо `cargo build` (Slint binary) ще компілюється.

---

## Що блокує що

### Блокери для `build.rs`

| Блокер | Де | Що потрібно зробити |
|--------|----|---------------------|
| `slint::include_modules!()` | `src/main.rs:6` | Видалити разом з усім binary |
| `slint::include_modules!()` | `tests/ui_events.rs:6` | Видалити або переписати тест |

`build.rs` можна видалити **тільки після того як обидва файли більше не містять `slint::include_modules!()`**.

### Блокери для `ui/*.slint`

| Блокер | Де |
|--------|----|
| `slint_build::compile_with_config("ui/app.slint", ...)` | `build.rs` |
| `import { Shell, CommandPalette } from "../ui/shell.slint"` | `tests/ui_events.rs:10` |
| `import { NavScreen } from "../ui/types.slint"` | `tests/ui_events.rs:11` |

Видалення порядок: `tests/ui_events.rs` → `build.rs` → `ui/`.

### Блокери для `slint`, `slint-build`, `i-slint-backend-testing`

| Крейт | Останнє місце використання |
|-------|---------------------------|
| `slint` | `src/main.rs`, `src/bootstrap/*.rs`, `src/ui/*.rs`, `tests/ui_events.rs` |
| `slint-build` | `build.rs` |
| `i-slint-backend-testing` | більше не потрібен після видалення `tests/ui_events.rs` |

Deps видаляються **в останню чергу**, після повного видалення всього коду вище.

---

## Безпечна послідовність (одним поглядом)

```
[Зараз] Svelte settings екран
    ↓
[Категорія 1] src/main.rs + src/bootstrap/ + src/ui/ → 18 файлів
    ↓
[CI cutover] Замінити tests/ui_events.rs на Vitest/Tauri smoke
    ↓
[Категорія 2] tests/ui_events.rs → build.rs → ui/*.slint → Cargo.toml deps
    ↓
[Готово] Проект повністю без Slint
```
