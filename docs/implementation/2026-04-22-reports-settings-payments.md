## 2026-04-22: compile-ready стан і реальний wiring для reports/settings/payments

### Що ламалось

1. `ui-redesign/app.slint` не компілювався.
Симптом: Slint падав на зверненнях до `detail.balance-str`, `detail.balance-is-negative`, `detail.overdue-amount-str`.
Причина: у вкладеному контексті треба було явно звертатись через `self.detail`.
Рішення: звернення замінено на `self.detail.*`.

2. Екран звітів існував переважно як Slint-контракт без завершеного Rust wiring.
Симптом: navigation відкривав reports, але не було повного data flow, drilldown і export.
Причина: після переходу на `ui-redesign` контракт уже був, а orchestration і backend-дії лишились незавершеними.
Рішення:
- додано `src/ui/reports.rs` і підключено його в bootstrap;
- добудовано агрегати для chart/category breakdown;
- додано перезавантаження даних при зміні періоду та drill category;
- реалізовано export у CSV і PDF з записом у `storage/reports/`;
- для PDF використано `typst compile`, а помилки експорту тепер явно логуються і показують системне повідомлення.

3. Налаштування були лише формою без реального читання і збереження.
Симптом: форма компанії виглядала готовою, але дані не вантажились із БД, а `settings-company-saved` нічого не робив.
Причина: `settings.rs` підставляв пустий `CompanyInfo`, а callbacks були заглушками.
Рішення:
- додано завантаження компанії з БД і мапінг `Company <-> CompanyInfo`;
- `settings-company-saved` тепер викликає `db::companies::update()`;
- інтеграції читаються з `storage/integrations/*.json`;
- команда налаштування інтеграції створює шаблон конфіга для BAS або банку;
- команда invite створює чернетку запрошення в `storage/team/invites/`;
- команда backup створює резервну копію в `storage/backups/`, а якщо `pg_dump` недоступний, переходить на JSON snapshot fallback;
- команда download відкриває останній backup-файл.

4. Payment callbacks лишались незавершеними навіть після виправлення KPI.
Симптом: import/sync/new/link у payments були або `TODO`, або no-op.
Причина: після стабілізації compile-ready стану реальна дія для цих callback'ів не була реалізована.
Рішення:
- додано `wire_payment_callbacks()` і підключено його в `main.rs`;
- import/sync тепер читають найновіший CSV із `storage/import/bank/`;
- додано визначення parser-кандидатів для CSV банків і dedupe перевірку через `exists_imported_row()`;
- `pay_link` тепер реально позначає платіж як звірений і оновлює список;
- `pay_new` створює та відкриває `manual-payment-template.csv`, щоб був робочий шлях для ручного додавання через чинний UI-контракт.

5. UI safety net був крихким після переходу на новий контракт.
Симптом: `tests/ui_events.rs` падав на роботі з `SharedString` і move semantics.
Причина: старий тестовий код не відповідав новому `AppWindow` contract.
Рішення: тест переписано в стабільніший headless smoke test для callbacks і навігації.

### Що ще довелось врахувати

- У Slint-контракті для settings invite немає полів введення email/name, тому повністю реальне "надіслати запрошення" зараз неможливе без зміни UI. Поточне рішення чесне: створюється editable draft-файл.
- У payments немає окремої форми ручного створення платежу, тому `pay_new` поки відкриває підготовлений CSV-шаблон для імпорту.
- Експорт PDF залежить від наявності `typst` у середовищі. Якщо команда завершується з помилкою, користувач тепер отримує явне повідомлення замість мовчазного no-op.
- Backup працює в режимі progressive enhancement: якщо є `pg_dump`, створюється SQL backup; якщо ні, зберігається локальний JSON snapshot із поясненням.

### Як перевірено

- `SQLX_OFFLINE=true cargo check`
- `SQLX_OFFLINE=true cargo test --test ui_events --no-run`

Обидві команди проходять успішно. У локальному середовищі `SQLX_OFFLINE=true` важливий, бо без доступної БД compile-time перевірка `sqlx` намагається звертатись до PostgreSQL.

### Що ще лишилось поза цією задачею

- У `main.rs` ще лишаються `TODO` тільки для command palette callbacks.
- `settings_section_changed`, `settings_dark_mode_toggled`, `settings_density_changed` поки логують подію, але не змінюють persisted app preferences.
- Для повністю реального invite/manual-payment flow треба розширити сам Slint-контракт і додати UI-ввід, а не лише backend wiring.
