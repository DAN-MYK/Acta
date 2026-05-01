# Reports Finance Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Добудувати канонічний модуль звітів у Tauri/Svelte для управлінського обліку: P&L, залишок на рахунку, дебіторська заборгованість, кредиторська заборгованість та експорт у Excel.

**Architecture:** Поточний `reports`-MVP уже існує в `src/tauri_api/reports.rs` і `frontend/src/lib/screens/ReportsScreen.svelte`, але логіка змішана: SQL, домен, DTO та CSV-експорт живуть разом. Рекомендований напрямок: винести домен і SQL у `src/models/reports.rs` + `src/db/reports.rs`, залишити в `tauri_api/reports.rs` лише orchestration/DTO, а у frontend розширити існуючий `reports` screen без зміни загального shell-патерну.

**Tech Stack:** Rust, Tauri 2, Svelte 5, TypeScript, PostgreSQL, sqlx, rust_decimal, chrono, `rust_xlsxwriter` для `.xlsx`.

---

## Поточний стан

- Уже реалізовано `reports_load` і `reports_export_csv`.
- Уже є 3 вкладки: `bank`, `receivables`, `payables`.
- `bank` зараз є cashflow-звітом на базі `payments`, а не повним P&L.
- `receivables` уже тягне борги з `acts` + `invoices`, де статус не `paid` і не `draft`.
- `payables` уже тягне майбутні/прострочені виплати з `payment_schedule`.
- `dashboard` уже має частину агрегатів, які можна повторно використати для P&L і категорій.
- У Vault вже зафіксовано задум звітів у `Acta/Features/Reports.md`, але він ще орієнтується на старий Slint-етап і не відображає поточну Svelte/Tauri-структуру.
- У `Feature List.md` модуль звітів позначений як `В роботі`, а `P&L` та `Excel export` ще лишаються відкритими.

## Ключові доменні рішення перед реалізацією

- [ ] Зафіксувати, що `bank` = рух грошей і залишок за фактичними платежами, а не бухгалтерський баланс рахунків.
- [ ] Зафіксувати, що `P&L` = дохід/витрати за документами, а не за рухом грошей.
- [ ] Визначити правила включення статусів у P&L.
  Рекомендація: не включати `draft`; для MVP включати `issued`, `signed`, `paid`.
- [ ] Узгодити знак і класифікацію витрат.
  Рекомендація: використовувати `categories.kind = 'expense'` і `direction` документа, де це доступно.
- [ ] Визначити, чи потрібен окремий облік банківських рахунків.
  Рекомендація: для поточної хвилі ні; залишити один агрегований cash balance.

## Рекомендований scope v1

### Обов'язково

- [ ] P&L за період.
- [ ] Залишок на рахунку / cashflow summary за період.
- [ ] Дебіторська заборгованість з aging buckets.
- [ ] Кредиторська заборгованість з overdue сигналами.
- [ ] Експорт у `.xlsx` з кількома аркушами.

### Доцільно додати в ту ж хвилю

- [ ] Доходи/витрати по категоріях.
- [ ] Топ контрагенти за виручкою.
- [ ] Топ боржники за сумою дебіторки.

### Наступна хвиля

- [ ] Cashflow forecast 30/60/90 днів.
- [ ] Drill-down з P&L до документів.
- [ ] Порівняння періодів month-over-month.
- [ ] Окремий управлінський звіт по компаніях у multi-company режимі.

## Що варто додати як нові звіти

### 1. P&L по категоріях

- Найбільш природне продовження наявних `categories`.
- Дає управлінську відповідь "на чому заробляємо" і "куди йдуть витрати".
- Може стати базовим джерелом для діаграми та Excel summary.

### 2. Top counterparties

- Простий у реалізації на наявних `acts`, `invoices`, `payments`.
- Корисний і для sales-аналізу, і для контролю концентрації ризику.

### 3. Aging summary

- Не окремий великий screen, а компактний блок у дебіторці та Excel.
- Дає швидкий управлінський сигнал без читання таблиці рядків.

### 4. Cashflow forecast

- Найкращий кандидат після MVP.
- Джерела вже є: `payment_schedule` + `expected_payment_date`.
- Дає практичну цінність для планування касового розриву.

## Рекомендація по Excel

- [ ] Вибрати `rust_xlsxwriter` як основну бібліотеку.
- Причина: чистий Rust, записує нові `.xlsx`, підтримує кілька worksheet, формати, таблиці, формули й дати.
- Обмеження: бібліотека не редагує існуючі Excel-файли, але для нашого use case це не потрібно.
- Формат експорту:
  `Summary`, `P&L`, `Cashflow`, `Receivables`, `Payables`, за потреби `Top Counterparties`.
- У UI залишити кнопку рівня "Експортувати Excel", а `CSV` або прибрати, або сховати як fallback/debug.

## Цільова структура файлів

### Backend

- [ ] Створити `src/models/reports.rs`
  Відповідальність: доменні структури звітів, агрегати, aging buckets, типи вкладок і запитів.
- [ ] Створити `src/db/reports.rs`
  Відповідальність: SQL-агрегації для P&L, cashflow, receivables, payables, top counterparties.
- [ ] Скоригувати `src/db/mod.rs`
  Відповідальність: підключення нового модуля reports.
- [ ] Скоригувати `src/models/mod.rs`
  Відповідальність: експорт моделей reports.
- [ ] Переписати `src/tauri_api/reports.rs`
  Відповідальність: DTO, orchestration, formatters, виклик `db::reports`, генерація `.xlsx`.
- [ ] Скоригувати `src/tauri_api/mod.rs`
  Якщо потрібно для публічних експортів.
- [ ] Скоригувати `src-tauri/src/commands/reports.rs`
  Додати нову команду `reports_export_excel`.
- [ ] Скоригувати `src-tauri/src/lib.rs`
  Зареєструвати `reports_export_excel`.

### Frontend

- [ ] Скоригувати `frontend/src/lib/types.ts`
  Додати типи для `pnl`, aging summary, excel export result, category rows, top counterparties.
- [ ] Скоригувати `frontend/src/lib/api.ts`
  Додати `reportsExportExcel()`.
- [ ] Скоригувати `frontend/src/lib/stores/reports.ts`
  Додати підтримку нового таба `pnl`, excel-export state і, за потреби, richer filters.
- [ ] Скоригувати `frontend/src/lib/screens/ReportsScreen.svelte`
  Додати вкладку `P&L`, KPI для неї, таблицю/summary блоки, кнопку `Експортувати Excel`.
- [ ] Скоригувати `frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`
  Оновити рендер і взаємодію для нового таба та кнопки експорту.
- [ ] Скоригувати `frontend/src/lib/stores/__tests__/counterparties-payments-settings.test.ts` або винести окремий `reports` store test
  Перевірити виклик нових API і стан завантаження/помилки.

### Тести

- [ ] Додати backend unit/integration tests для `src/db/reports.rs`.
- [ ] Додати backend tests для Excel workbook generation.
- [ ] Оновити frontend tests для screen/store.

## Порядок реалізації

### Фаза 1. Нормалізація архітектури reports

- [ ] Винести SQL з `src/tauri_api/reports.rs` у `src/db/reports.rs`.
- [ ] Винести доменні структури в `src/models/reports.rs`.
- [ ] Залишити в `tauri_api` лише DTO і форматування money/date для UI.
- [ ] Після переносу перевірити, що поточні `bank`, `receivables`, `payables` працюють без функціональних змін.

### Фаза 2. P&L

- [ ] Додати нову вкладку `pnl`.
- [ ] Реалізувати summary:
  `income`, `expense`, `gross_result`/`net_result`.
- [ ] Реалізувати breakdown по категоріях.
- [ ] Реалізувати режим `scope=active|all`.
  Рекомендація: для `all` показувати спочатку загальний summary, потім рядки по компаніях.
- [ ] Для MVP використати документи `acts` + `invoices`, відфільтровані від `draft`.

### Фаза 3. Поліпшення дебіторки та кредиторки

- [ ] Додати aging buckets для дебіторки:
  `current`, `1-30`, `31-60`, `61-90`, `90+`.
- [ ] Додати агреговані KPI для payables:
  `до сплати`, `прострочено`, `найближчі 7 днів`.
- [ ] Додати сортування й фокус-рядки так, щоб першим ішов ризик.

### Фаза 4. Excel export

- [ ] Додати `reports_export_excel` у backend.
- [ ] Генерувати workbook у `storage/reports`.
- [ ] Додати щонайменше такі аркуші:
  `Summary`, `P&L`, `Cashflow`, `Receivables`, `Payables`.
- [ ] На Summary продублювати вибрані фільтри: компанія, період, дата генерації.
- [ ] Писати грошові суми в Excel як числа з форматом, а не як preformatted string.
  Це важливо: money contract `string` лишається для UI, але не для `.xlsx`.
- [ ] Залишити повернення шляху до файлу в DTO результату.

### Фаза 5. Додаткові управлінські звіти

- [ ] Додати `Top counterparties`.
- [ ] Додати `Top debtors`.
- [ ] Оцінити окрему вкладку `forecast` після стабілізації P&L і Excel.

## Логіка даних по звітах

### Bank / Cashflow

- Джерело: `payments`.
- Opening balance:
  всі платежі до `date_from`.
- Period movement:
  платежі між `date_from` і `date_to`.
- Closing balance:
  `opening + income - expense`.

### P&L

- Джерело: `acts` + `invoices` з категоріями.
- Доходи:
  документи з доходною категорією/напрямом.
- Витрати:
  документи з `categories.kind = 'expense'`.
- Не використовувати `payments`, щоби не змішувати касовий рух і результат періоду.

### Receivables

- Джерело: `acts`, `invoices`.
- Включати тільки не закриті документи.
- Aging рахувати від `expected_payment_date`, якщо вона задана.

### Payables

- Джерело MVP: `payment_schedule` з `direction = expense` і `is_completed = false`.
- Розширення v2:
  додати vendor invoice джерело, коли така сутність з'явиться.

## Ризики і перевірки

- [ ] Ризик доменного конфлікту між cashflow і P&L.
  Пом'якшення: чіткі назви вкладок і окремі KPI.
- [ ] Ризик дублювання логіки між `dashboard` і `reports`.
  Пом'якшення: повторно використати helper/query-layer, не копіювати SQL у два місця.
- [ ] Ризик розходження multi-company режиму.
  Пом'якшення: всюди використовувати єдиний фільтр `scope`.
- [ ] Ризик невалідного Excel через форматування рядками.
  Пом'якшення: у workbook передавати `Decimal -> f64/string-format` лише через контрольований адаптер і окремі тести на клітинки.

## Тестова стратегія

### Backend

- [ ] Unit tests на aging bucket calculation.
- [ ] Unit tests на P&L summary calculation.
- [ ] Integration tests на SQL вибірки `bank`, `pnl`, `receivables`, `payables`.
- [ ] Tests на `scope=active` vs `scope=all`.
- [ ] Tests на Excel export:
  файл створився, містить очікувані worksheet names, повертається коректний path.

### Frontend

- [ ] Screen test: вкладка `P&L` рендериться.
- [ ] Screen test: кнопка `Експортувати Excel` викликає новий API.
- [ ] Store test: loading/error/message для Excel export.
- [ ] Screen test: показ aging buckets у дебіторці.

## Рекомендована черговість релізу

- [ ] Release 1:
  архітектурний рефактор reports + стабілізація поточних трьох вкладок.
- [ ] Release 2:
  P&L + категорії + frontend tab.
- [ ] Release 3:
  Excel export `.xlsx`.
- [ ] Release 4:
  Top counterparties + top debtors + forecast backlog grooming.

## Висновок по пріоритету

- Найкраща перша хвиля для Acta:
  `Bank/Cashflow`, `P&L`, `Receivables`, `Payables`, `Excel export`.
- Найкраща друга хвиля:
  `Top counterparties`, `Top debtors`, `Cashflow forecast`.
- Головний технічний борг перед розширенням:
  рознести `reports` по `models/db/tauri_api`, бо зараз модуль уже робить забагато.

