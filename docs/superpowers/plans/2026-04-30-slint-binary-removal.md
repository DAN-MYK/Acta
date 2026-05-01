# Slint Binary Removal — Implementation Plan

> **Archived/pre-cutover:** execution plan збережено як історичний cleanup trace. Поточний runtime — Tauri/Svelte; Slint file lists тут не є live backlog.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Видалити legacy Slint binary (`[[bin]] acta`) разом із усіма його приватними модулями (`src/bootstrap/`, `src/ui/`), зберігши при цьому Slint runtime для `tests/ui_events.rs`, та підготувати фінальний cascade-removal diff.

**Architecture:** Slint binary (`src/main.rs` + `src/bootstrap/` + `src/ui/`) — приватний compilation unit, ізольований від lib-крейту (`src/lib.rs`). Видалення `[[bin]] acta` з Cargo.toml та файлів не зачіпає: lib-крейт, tauri-крейт (`src-tauri/`), тести, migrate/reseed binaries. `build.rs` залишається бо `tests/ui_events.rs` потребує `slint::include_modules!()`.

**Tech Stack:** Rust, Cargo, slint 1.15, sqlx

---

## File Map

| Дія | Файл | Причина |
|-----|------|---------|
| Delete | `src/main.rs` | Слint binary entry point |
| Delete | `src/bootstrap.rs` | bootstrap module root |
| Delete | `src/bootstrap/refresh.rs` | Slint Weak + upgrade_in_event_loop |
| Delete | `src/bootstrap/shell.rs` | AppWindow wiring |
| Delete | `src/bootstrap/company_switcher.rs` | AppWindow wiring |
| Delete | `src/bootstrap/document_chain.rs` | AppWindow wiring |
| Delete | `src/bootstrap/inbox.rs` | AppWindow wiring |
| Delete | `src/bootstrap/navigation.rs` | AppWindow wiring |
| Delete | `src/bootstrap/palette.rs` | AppWindow wiring |
| Delete | `src/bootstrap/wiring.rs` | AppWindow wiring |
| Delete | `src/ui/mod.rs` | ui module root |
| Delete | `src/ui/helpers.rs` | SharedString utils |
| Delete | `src/ui/dashboard.rs` | Slint presenter |
| Delete | `src/ui/documents.rs` | Slint presenter |
| Delete | `src/ui/counterparties.rs` | Slint presenter |
| Delete | `src/ui/payments.rs` | Slint presenter |
| Delete | `src/ui/tasks.rs` | Slint presenter |
| Delete | `src/ui/reports.rs` | Slint presenter |
| Delete | `src/ui/settings.rs` | Slint presenter |
| Modify | `Cargo.toml` | Прибрати `[[bin]] acta` та `default-run` |
| Modify | `CLAUDE.md` | Оновити команди запуску |
| Create | `docs/planning/slint-final-cascade-removal-2026-04-30.md` | Скрипт фінального видалення |

**Залишаються без змін:** `build.rs`, `ui/*.slint` (13 файлів), `tests/ui_events.rs`, `slint`/`slint-build`/`i-slint-backend-testing` у Cargo.toml.

---

## Task 1: Baseline — зафіксувати поточний стан тестів

**Files:** читання, без змін

- [x] **Step 1: Зберегти baseline тестів**

```bash
cargo test 2>&1 | tail -20
```

Очікувано: всі тести проходять. Запиши кількість тестів — вона має залишитись такою ж після видалення.

- [x] **Step 2: Перевірити що migrate та reseed компілюються без Slint**

```bash
cargo build --bin migrate --bin reseed 2>&1 | tail -5
```

Очікувано: `Finished` без помилок.

---

## Task 2: Редагувати Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [x] **Step 1: Видалити `default-run` та `[[bin]] acta`**

Знайти і видалити рядок:
```toml
default-run = "acta"
```

Знайти і видалити весь блок:
```toml
[[bin]]
name = "acta"
path = "src/main.rs"
```

Результат після правки — залишаються тільки:
```toml
[[bin]]
name = "migrate"
path = "src/bin/migrate.rs"

[[bin]]
name = "reseed"
path = "src/bin/reseed.rs"
```

- [x] **Step 2: Перевірити що Cargo.toml валідний**

```bash
cargo metadata --no-deps --quiet 2>&1 | head -5
```

Очікувано: JSON без помилок.

---

## Task 3: Видалити src/main.rs

**Files:**
- Delete: `src/main.rs`

- [x] **Step 1: Видалити файл**

```bash
rm src/main.rs
```

- [x] **Step 2: Перевірити що lib і тести ще компілюються**

```bash
cargo build --lib 2>&1 | tail -5
```

Очікувано: `Finished` без помилок.

---

## Task 4: Видалити src/bootstrap.rs та src/bootstrap/

**Files:**
- Delete: `src/bootstrap.rs`
- Delete: `src/bootstrap/` (8 файлів)

- [x] **Step 1: Видалити bootstrap module root**

```bash
rm src/bootstrap.rs
```

- [x] **Step 2: Видалити всі файли в src/bootstrap/**

```bash
rm src/bootstrap/company_switcher.rs
rm src/bootstrap/document_chain.rs
rm src/bootstrap/inbox.rs
rm src/bootstrap/navigation.rs
rm src/bootstrap/palette.rs
rm src/bootstrap/refresh.rs
rm src/bootstrap/shell.rs
rm src/bootstrap/wiring.rs
rmdir src/bootstrap
```

- [x] **Step 3: Перевірити що lib ще компілюється**

```bash
cargo build --lib 2>&1 | tail -5
```

Очікувано: `Finished` без помилок.

---

## Task 5: Видалити src/ui/

**Files:**
- Delete: `src/ui/` (9 файлів)

- [x] **Step 1: Видалити всі файли в src/ui/**

```bash
rm src/ui/mod.rs
rm src/ui/helpers.rs
rm src/ui/dashboard.rs
rm src/ui/documents.rs
rm src/ui/counterparties.rs
rm src/ui/payments.rs
rm src/ui/tasks.rs
rm src/ui/reports.rs
rm src/ui/settings.rs
rmdir src/ui
```

---

## Task 6: Перевірити повну компіляцію

**Files:** без змін, тільки запуск

- [x] **Step 1: Повна компіляція включно з тестами**

```bash
cargo build --tests 2>&1 | tail -10
```

Очікувано: `Finished` без помилок.  
Якщо є помилки компіляції — розбити за модулями і виправити.

- [x] **Step 2: Перевірити що tests/ui_events.rs ще компілюється окремо**

```bash
cargo test --test ui_events --no-run 2>&1 | tail -5
```

Очікувано: `Finished` або `Compiling ... Finished`.

- [x] **Step 3: Запустити всі тести**

```bash
cargo test 2>&1 | tail -20
```

Очікувано: та сама кількість тестів що і в Task 1, усі проходять.

---

## Task 7: Оновити CLAUDE.md — секція Команди

**Files:**
- Modify: `CLAUDE.md`

- [x] **Step 1: Оновити секцію `## Команди`**

Знайти поточну секцію:
```markdown
## Команди
```bash
cargo run                                         # запуск
...
```

Замінити на:
```markdown
## Команди
```bash
# Основний запуск — Tauri+Svelte (src-tauri/)
cd src-tauri && cargo tauri dev                   # dev режим
cd src-tauri && cargo tauri build                 # production build

# Бібліотека та утиліти
cargo build --lib                                 # компіляція lib-крейту
cargo build --bin migrate                         # BAS import utility
cargo build --bin reseed                          # DB reseed utility
cargo run --bin migrate -- --input ./bas-export/  # міграція з BAS
sqlx migrate run                                  # міграції БД
cargo sqlx prepare                                # offline SQL (після зміни запитів)
cargo build --tests                               # повна компіляція: lib + tests
cargo test                                        # всі тести
```
```

> Примітка: `cargo run` більше не працює (кілька binaries без default-run).
> Legacy Slint binary видалено. Єдиний UI entry — `cargo tauri dev` з src-tauri/.

---

## Task 8: Commit

**Files:** всі змінені/видалені файли

- [x] **Step 1: Перевірити що staging виглядає правильно**

```bash
git status
```

Очікувано: видалені файли (`D`) для src/main.rs, src/bootstrap.*, src/ui/*.

- [x] **Step 2: Stage і commit**

```bash
git add -u
git add CLAUDE.md Cargo.toml
git commit -m "chore: remove legacy Slint binary — src/main.rs, bootstrap/, ui/ presenters

Tauri+Svelte is now the primary UI path. The Slint runtime (deps, build.rs,
ui/*.slint) remains for tests/ui_events.rs headless contract tests.
Next: replace ui_events.rs with Playwright/Vitest, then cascade-remove all Slint."
```

---

## Task 9: Підготувати фінальний cascade-removal документ

**Files:**
- Create: `docs/planning/slint-final-cascade-removal-2026-04-30.md`

- [x] **Step 1: Створити документ з точними командами**

```markdown
# Slint — Фінальне каскадне видалення

Виконати після заміни `tests/ui_events.rs` на Playwright/Vitest тести.

## Передумова

`tests/ui_events.rs` замінений або видалений. `cargo build --tests` компілюється без нього.

## Команди (в порядку)

```bash
# 1. Видалити Slint test file
rm tests/ui_events.rs

# 2. Перевірити що cargo build --tests ще проходить
cargo build --tests

# 3. Видалити build.rs
rm build.rs

# 4. Видалити всі .slint файли
rm -rf ui/

# 5. Редагувати Cargo.toml — видалити Slint залежності
```

### Зміни в Cargo.toml

Видалити з `[dependencies]`:
```toml
slint = { version = "1.15", features = ["serde"] }
```

Видалити з `[dev-dependencies]`:
```toml
i-slint-backend-testing = "=1.15.1"
```

Видалити всю секцію `[build-dependencies]`:
```toml
[build-dependencies]
slint-build = "1.15"
```

```bash
# 6. Перевірити повну компіляцію
cargo build --tests

# 7. Запустити тести
cargo test

# 8. Commit
git add -u
git commit -m "chore!: remove Slint runtime — build.rs, ui/*.slint, slint deps

tests/ui_events.rs replaced with Playwright/Vitest. Slint fully gone."
```

## Файли що видаляються в цьому кроці

| Файл | Розмір |
|------|--------|
| `tests/ui_events.rs` | ~1050 рядків |
| `build.rs` | 10 рядків |
| `ui/app.slint` | root |
| `ui/shell.slint` | |
| `ui/types.slint` | |
| `ui/design-tokens.slint` | |
| `ui/components.slint` | |
| `ui/icons.slint` | |
| `ui/dashboard.slint` | |
| `ui/documents.slint` | |
| `ui/counterparties.slint` | |
| `ui/payments.slint` | |
| `ui/tasks.slint` | |
| `ui/settings.slint` | |
| `ui/reports.slint` | |
| Cargo.toml: `slint`, `slint-build`, `i-slint-backend-testing` | |

**Разом: 14 файлів + 3 dep записи.**
```

- [x] **Step 2: Зберегти документ**

Зберегти як `docs/planning/slint-final-cascade-removal-2026-04-30.md`.

- [x] **Step 3: Додати в commit**

```bash
git add docs/planning/slint-final-cascade-removal-2026-04-30.md
git commit -m "docs: add Slint final cascade removal script"
```

---

## Self-Review

**Spec coverage:**
- [x] Точний code removal plan — Task 2–5
- [x] Перший безпечний cleanup pass — Tasks 2–8
- [x] Без видалення Slint runtime — build.rs, ui/*.slint, deps залишаються
- [x] Оновлення docs/CI notes — Task 7 (CLAUDE.md)
- [x] Окремий мінімальний diff для фінального видалення — Task 9

**Placeholder scan:** Немає TBD або TODO.

**Type consistency:** `AppWindow` згадується тільки в контексті видалення, не в новому коді.
