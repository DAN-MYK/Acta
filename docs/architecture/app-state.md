# App State

Канонічний state path у `Acta`:

- shared runtime state живе в [src/app_ctx.rs](/C:/Users/MykhailoDan/apps/Acta/src/app_ctx.rs)
- screen refresh coordination живе в [src/bootstrap.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap.rs)

## Що зберігає `AppCtx`

- active company
- active screen
- documents list snapshot
- counterparties list snapshot
- reports snapshot
- tasks snapshot

Feature callbacks не повинні будувати другий паралельний state container.
Вони оновлюють snapshot у `AppCtx`, після чого викликають canonical refresh API з bootstrap coordinator-а.

Починаючи з `Epic 5`, callback-и також не повинні працювати з `Mutex` напряму.
Для цього `AppCtx` дає `update_documents_state()`, `update_counterparty_state()`,
`update_reports_state()` і `update_task_state()`, а mutex-backed поля лишаються внутрішніми.

## Canonical refresh API

- `load_initial_ui_data()` для стартового завантаження
- `refresh_screen()` для явного screen refresh
- `refresh_current_screen()` коли треба перевантажити поточний екран
- `spawn_refresh_screen()` як безпечний async entrypoint з UI callback-ів

Це прибирає дублювання між initial load, navigation refresh і feature-triggered reload.

## UI contract після Epic 6

`AppWindow` більше не повинен розростатися через flat data properties під кожен екран.
Канонічний підхід для redesign:

- shell/chrome дані йдуть окремим `ShellChrome`
- screen data йде через feature-specific view models:
  `DashboardViewData`, `DocumentsViewData`, `CounterpartiesViewData`,
  `PaymentsViewData`, `ReportsViewData`, `TasksViewData`, `SettingsViewData`
- screen-local UI state на кшталт активної вкладки, пошуку чи drill selection
  лишається локальним `AppWindow`/screen state, якщо це не shared runtime snapshot

Це зменшує surface area root component і відділяє shell contract від feature payload-ів.
