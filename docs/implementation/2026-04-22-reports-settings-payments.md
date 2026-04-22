## 2026-04-22: відновлення compile-ready стану та wiring для reports/settings/payments

### Що ламалось

1. `ui-redesign/app.slint` не компілювався.
Симптом: Slint падав на зверненні до полів `detail.balance-str`, `detail.balance-is-negative`, `detail.overdue-amount-str`.
Причина: у вкладеному контексті треба було явно звертатись через `self.detail`.
Рішення: замінено звернення на `self.detail.*`.

2. Екран звітів був лише частково описаний у Slint, але не мав реального Rust wiring.
Симптом: `NavScreen::Reports` не підтягував реальні дані, модуля `src/ui/reports.rs` не існувало.
Причина: після переходу на `ui-redesign` контракт уже був, а orchestration та DB-агрегації не були доведені.
Рішення:
- додано `src/ui/reports.rs`;
- підключено `pub mod reports;` у `src/ui/mod.rs`;
- додано агрегації `expenses_by_month()` і `category_breakdown()` у `src/db/dashboard.rs`;
- додано drill-down завантаження рядків та застосування даних у Slint;
- підключено reports в initial load і navigation wiring у `src/main.rs`.

3. Налаштування були stub-екраном без завантаження і збереження.
Симптом: UI показував форму компанії, але дані не читались із БД і `settings-company-saved` нічого не робив.
Причина: `src/ui/settings.rs` підставляв порожній `CompanyInfo`, а save callback був `TODO`.
Рішення:
- переписано `src/ui/settings.rs`;
- додано завантаження компанії з БД;
- додано мапінг `Company -> CompanyInfo` та `CompanyInfo -> UpdateCompany`;
- `settings-company-saved` тепер викликає `db::companies::update()`;
- тимчасові сценарії без бекенду залишені як явні `tracing::info!("TODO: ...")`.

4. KPI на екрані платежів були неточні.
Симптом: вхідні/вихідні суми рахувались по всьому списку, а unmatched фактично дорівнював кількості всіх платежів.
Причина: KPI формувались у UI-шарі з локального списку без окремої бізнес-логіки.
Рішення:
- додано `PaymentKpi` та `payment_kpi()` у `src/db/payments.rs`;
- `src/ui/payments.rs` тепер завантажує список і KPI паралельно через `tokio::join!`;
- у Slint передаються вже підготовлені рядки сум та коректний `unmatched_count`.

5. `main.rs` мав зайву orchestration-логіку і неповне підключення нових екранів.
Симптом: reports/settings не були повністю в bootstrap/navigation шляху, а match містив старий fallback.
Причина: після попередніх змін bootstrap залишився в проміжному стані.
Рішення:
- `main.rs` приведено до ролі bootstrap;
- reports/settings включені в initial preload;
- додано `wire_reports_callbacks()` і `wire_settings_callbacks()`;
- прибрано unreachable fallback у navigation match.

6. UI safety net тест не проходив навіть після виправлення app-коду.
Симптом: `tests/ui_events.rs` падав на `Rc<Cell<SharedString>>`, move-помилках та невдалому зберіганні non-Copy значень.
Причина: `Cell` не підходить для `SharedString`, а старий тестовий код був крихким після оновлення контракту.
Рішення:
- переписано `tests/ui_events.rs` у простіший і стабільніший headless smoke-test;
- для рядків використано `Rc<RefCell<SharedString>>`;
- прибрано непотрібні helper'и, які переносили `ui` у closure;
- збережено покриття callback-контракту для navigation, documents, counterparties, payments, reports, tasks, settings, command palette.

### Як перевірено

- `SQLX_OFFLINE=true cargo check`
- `SQLX_OFFLINE=true cargo test --test ui_events --no-run`

Обидві команди завершились успішно. Для локального середовища `SQLX_OFFLINE=true` важливий, бо без доступної БД існуючі `sqlx` macro-запити в репозиторії намагаються ходити в PostgreSQL під час compile-time перевірки.

### Що лишилось поза цією задачею

- Експорт reports у CSV/PDF поки лишився явним `TODO`.
- Частина попередніх dead-code warning у `documents.rs` та `tasks.rs` існувала окремо від цієї задачі і не блокує compile-ready стан.
