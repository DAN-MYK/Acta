# Roadmap міграції на Tauri — 2026-04-29

## Принцип

Міграція має йти паралельно до чинного Slint UI, а не через миттєвий cutover. Найменш ризиковий шлях:

1. Підняти Tauri поруч.
2. Винести backend contract.
3. Перенести shell і екрани по одному.
4. Переписати тести й CI.
5. Лише після цього прибрати Slint.

## Етап 1. Tauri scaffold

### Ціль

Підготувати новий runtime без впливу на Slint.

### Нові файли

- `src-tauri/Cargo.toml`
- `src-tauri/src/main.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/build.rs`
- `src-tauri/tauri.conf.json`
- `package.json`
- `vite.config.ts`
- `svelte.config.js`
- `tsconfig.json`
- `index.html`
- `src/` frontend

### Ризик

Низький.

## Етап 2. Shared backend bootstrap

### Ціль

Зробити так, щоб Tauri міг використовувати той самий Rust backend, що й нинішній Slint app.

### Поточні точки інтеграції

- [src/main.rs](/C:/Users/MykhailoDan/apps/Acta/src/main.rs:13)
- [src/bootstrap.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap.rs:34)
- [src/app_ctx.rs](/C:/Users/MykhailoDan/apps/Acta/src/app_ctx.rs:54)

### Що переносимо

- `AppCtx`
- `PgPool`
- міграції
- background tasks
- active company context

### Ризик

Низький-середній.

## Етап 3. Command contract

### Ціль

Замінити Slint callbacks на Tauri commands.

### Джерела контракту

- [src/ui/mod.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/mod.rs:1)
- [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:1)
- presenter-модулі у [src/ui](/C:/Users/MykhailoDan/apps/Acta/src/ui)

### Ключове правило

- гроші лишаються рядками;
- дати на межі API лишаються рядками;
- валідація і `Decimal` лишаються в Rust.

### Ризик

Середній.

## Етап 4. Shell і navigation

### Ціль

Перенести root shell, navigation, company switcher, palette, theme toggle, shortcuts.

### Поточні Slint джерела

- [ui/app.slint](/C:/Users/MykhailoDan/apps/Acta/ui/app.slint:30)
- [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint)
- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:210)
- [src/bootstrap/navigation.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/navigation.rs:35)
- [src/bootstrap/shell.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/shell.rs)
- [src/bootstrap/palette.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/palette.rs:154)

### Нові frontend модулі

- `src/App.svelte`
- `src/lib/stores/navigation.ts`
- `src/lib/stores/shell.ts`
- `src/lib/stores/theme.ts`
- `src/lib/stores/palette.ts`
- `src/lib/components/Shell.svelte`
- `src/lib/components/Sidebar.svelte`
- `src/lib/components/CommandPalette.svelte`

### Ризик

Середній.

## Етап 5. Перенос feature screens

### Рекомендований порядок

1. `dashboard`
2. `documents`
3. `counterparties`
4. `payments`
5. `tasks`
6. `reports`
7. `settings`

### Поточні Rust presenter-модулі

- [src/ui/dashboard.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/dashboard.rs:145)
- [src/ui/documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:1046)
- [src/ui/counterparties.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/counterparties.rs:229)
- [src/ui/payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:364)
- [src/ui/tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:239)
- [src/ui/reports.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/reports.rs:370)
- [src/ui/settings.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/settings.rs:512)

### Ризик

Середній-високий.

## Етап 6. Заміна refresh/wiring моделі

### Поточна модель

- `Weak<AppWindow>`
- `apply_*_to_ui`
- `wire_*_callbacks`
- `VecModel` / `ModelRc`

### Цільова модель

- `invoke()` / `#[tauri::command]`
- Svelte stores
- targeted re-fetch після mutation
- мінімум глобального imperative refresh

### Поточні файли

- [src/bootstrap/refresh.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/refresh.rs:99)
- [src/bootstrap/wiring.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/wiring.rs:7)
- [src/bootstrap/document_chain.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/document_chain.rs:9)

### Ризик

Високий.

## Етап 7. Дизайн-система

### Джерела

- [ui/design-tokens.slint](/C:/Users/MykhailoDan/apps/Acta/ui/design-tokens.slint)
- [ui/components.slint](/C:/Users/MykhailoDan/apps/Acta/ui/components.slint)
- [ui/icons.slint](/C:/Users/MykhailoDan/apps/Acta/ui/icons.slint)
- `ui/assets/...`

### Ціль

- перенести токени в CSS custom properties;
- перенести reusable components;
- перепідключити SVG assets.

### Ризик

Середній.

## Етап 8. Тести

### Поточний стан

- [tests/ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:6) є Slint-specific test suite.

### Цільовий стан

- Rust unit/integration tests на backend commands;
- frontend component tests;
- Tauri/e2e smoke tests.

### Мінімальний набір

- command tests
- navigation smoke test
- documents CRUD smoke test
- payments flow smoke test
- settings persistence smoke test

### Ризик

Середній.

## Етап 9. CI

### Поточний стан

- [ci.yml](/C:/Users/MykhailoDan/apps/Acta/.github/workflows/ci.yml:39) має Slint UI job.

### Цільовий стан

- frontend install
- typecheck
- frontend build
- Tauri build smoke test
- backend tests
- DB integration tests

### Ризик

Низький-середній.

## Етап 10. Фінальний cutover

### Прибираємо лише після green build

- `ui/*.slint`
- поточний [build.rs](/C:/Users/MykhailoDan/apps/Acta/build.rs:1)
- `slint`
- `slint-build`
- `i-slint-backend-testing`
- Slint bootstrap/wiring

### Ризик

Високий, якщо зробити передчасно.

## Найкращий практичний порядок виконання

1. Tauri scaffold
2. shared backend bootstrap
3. shell/navigation
4. `dashboard` як пілот
5. `documents`
6. `payments`
7. решта screens
8. тести
9. CI
10. видалення Slint

## Пов'язані документи

- [tauri-migration-audit-2026-04-29.md](./tauri-migration-audit-2026-04-29.md)
- [tauri-migration-contract-matrix-2026-04-29.md](./tauri-migration-contract-matrix-2026-04-29.md)
- [tauri-documents-command-spec-2026-04-29.md](./tauri-documents-command-spec-2026-04-29.md)
