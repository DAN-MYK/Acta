# UI Safety Net

Оновлено: `2026-04-30`

## Статус

Цей документ описує post-cutover safety net для канонічного Tauri/Svelte UI.

Старий `tests/ui_events.rs` був Slint-era headless safety net і більше не є live test contract. Якщо треба читати його або `ui/*.slint`, трактуй їх лише як archived/pre-cutover reference.

## Поточний набір перевірок

- `npm run check` - Svelte/TypeScript typecheck.
- `npm run build` - Vite production build і перевірка frontend assets для Tauri.
- `npm run test:frontend` - store tests + screen-level component tests.
- `npm run test:e2e` - app-level Tauri WebView smoke через `tauri-driver`.
- `cargo test -j 1 --test tauri_vertical_slice` - Rust/Tauri command vertical slice.

## Frontend component tests

Компонентні тести живуть у `frontend/src/lib/screens/__tests__/`.

Поточний P1 coverage:

- `DashboardScreen.test.ts` - ініціалізація summary секцій, drill-in у documents/payments/tasks, empty-state платежів.
- `DocumentsScreen.test.ts` - list/editor/chain controls, пошук, створення draft, save/advance.
- `PaymentsScreen.test.ts` - KPI/rows, відкриття editor, reconcile/unreconcile, створення платежу.

Ці тести навмисно мокають stores на межі screen contract. Бізнес-логіку, DTO parsing і DB behavior треба покривати нижче, у store/Rust tests.

## App-level desktop smoke

`e2e-tests/` запускає реальний Tauri desktop shell і перевіряє, що WebView ініціалізується та проходить базову навігацію:

- старт на `Дашборд`;
- перехід у `Документи`;
- перехід у `Платежі`;
- shortcut `Ctrl+1` назад на dashboard.

У CI smoke працює під `xvfb-run` на Linux і використовує `tauri-driver`.

## Правила розширення

1. Якщо змінюється screen-level UX або wiring store -> screen, додай/онови компонентний тест у `frontend/src/lib/screens/__tests__/`.
2. Якщо змінюється store behavior або frontend invoke contract, додай/онови store test у `frontend/src/lib/stores/__tests__/`.
3. Якщо змінюється Tauri command або backend DTO, додай Rust vertical/integration coverage.
4. Якщо змінюється shell startup, routing або WebView initialization, онови `e2e-tests/test/specs/app-smoke.e2e.js`.
5. Не додавай нові Slint safety-net tests; Slint references лишаються historical/pre-cutover.
