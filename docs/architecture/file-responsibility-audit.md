# Аудит відповідальності файлів

Оновлено: 2026-05-05.

## Канонічні правила

- Один файл має одну відповідальність.
- Public facade має бути коротким і делегувати у підмодулі.
- DTO, mappers, import, pdf, chain, calendar, reconcile живуть в окремих модулях.
- Файл понад 500 рядків потребує явного виправдання.
- Файл понад 1000 рядків отримує окрему задачу на split.

## Зроблено в цьому проході

- `src/tauri_api/documents.rs` став facade-файлом і делегує в `src/tauri_api/documents/api.rs`, `dto.rs`, `pdf.rs`.
- `src/tauri_api/payments.rs` став facade-файлом і делегує в `src/tauri_api/payments/api.rs`, `dto.rs`.
- `src/tauri_api/documents/pdf.rs` отримав явні імпорти замість залежності від монолітного `super::*`.
- `src/tauri_api/payments/api.rs` зменшено з 1266 до 780 рядків: `calendar` і unit tests винесено в `src/tauri_api/payments/api/calendar.rs` та `src/tauri_api/payments/api/tests.rs`.

## Задачі на split для файлів понад 1000 рядків

- [ ] `tests/tauri_vertical_slice.rs` (1496): розбити за вертикалями command surface: shell, documents, payments, reports/tasks.
- [ ] `src/tauri_api/documents/api.rs` (1286): винести `references`, `mappers`, `chain`, `bulk`, `editor` в окремі підмодулі.
- [ ] `frontend/src/lib/stores/__tests__/counterparties-payments-settings.test.ts` (1272): розділити на три suite-файли за store/domain.
- [ ] `tests/db_integration/reports.rs` (1191): розбити за сценаріями P&L, CSV/Excel export, top counterparties, debtors.
- [ ] `src/bin/migrate.rs` (1179): винести CLI parsing, BAS plan, execution, reporting в окремі модулі.
- [ ] `tests/db_integration/payments.rs` (1151): розділити на CRUD, schedule, import, reconcile, matching.
- [ ] `frontend/src/lib/stores/payments.ts` (1146): винести calendar/reconcile/import utilities зі store-файлу.
- [ ] `src/db/invoices.rs` (1066): розділити repository на CRUD, numbering, status transitions, PDF path helpers.

## Тимчасово виправдані файли понад 500 рядків

Ці файли поки лишаються понад 500 рядків, бо містять уже згруповані сценарії або тестові fixture-и. Їх треба торкатися під час найближчих змін у відповідній області.

- `src/import/bas_invoices.rs`, `src/import/bas_acts.rs`: BAS parsers; split на parser/mappers/persist під час наступної роботи з BAS.
- `frontend/src/lib/screens/PaymentsScreen.svelte`, `ReportsScreen.svelte`, `DocumentsScreen.svelte`, `TasksScreen.svelte`, `SettingsScreen.svelte`: screen-level UI; split на panels/forms/lists при наступній зміні цих screen.
- `src/db/payments.rs`, `src/db/reports.rs`, `src/db/acts.rs`, `src/db/dashboard.rs`: repository-файли; split лише разом із тестовим покриттям конкретної вертикалі.
- `src/tauri_api/reports_excel.rs`, `src/pdf/generator.rs`, `src/tauri_api/settings.rs`, `src/tauri_api/dashboard.rs`, `src/tauri_api/documents/pdf.rs`: окремі use-case/IO модулі, але вже на межі; наступна зміна має або зменшити файл, або дописати точне виправдання.
- `frontend/src/lib/types.ts`, `frontend/src/lib/browser-fixtures.ts`, великі frontend tests: контрактні типи й fixture-и; split за domain namespaces.
- `tests/db_integration/*.rs` понад 500: інтеграційні suite-и; split за сценаріями без зміни тестової БД fixture.
