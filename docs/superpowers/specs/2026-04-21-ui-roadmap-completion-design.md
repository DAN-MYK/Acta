# UI Roadmap Completion — Design Spec
**Date:** 2026-04-21  
**Scope:** Дописати відсутні UI фічі в ui-redesign/, потім підключити весь UI до PostgreSQL backend.

---

## Контекст

`ui-redesign/` — повністю нова Slint UI (6 600 рядків), яка замінює старий `ui/`. `build.rs` вже вказує на `ui-redesign/app.slint`. Більшість пунктів `UI_ROADMAP.md` вже реалізовано в новому дизайні.

**Вже є в ui-redesign:**
- Flat KPI metric strip (KpiMetric component)
- Sidebar nav + Command Palette (shell.slint)
- Master-detail контрагенти (380px list + detail panel)
- Задачі: вкладки Open/Done/All + day-view calendar sidebar
- Settings: 6 секцій (Appearance, Company, Numbering, Integrations, Team, Backup)
- AppTheme design tokens: коректні радіуси (sm=4, md=6, lg=8, xl=10), без тіней на content

**Відсутні UI фічі (потребують реалізації):**
- Dashboard Inbox view
- DocChain component

**Backend:** Dashboard/Documents/Counterparties/Payments/Tasks підключені. Reports і Settings — заглушки. Rust типи потребують адаптації під нові Slint struct'и.

---

## Фаза 1: UI — відсутні компоненти

### 1.1 Dashboard Inbox view

**Мета:** Другий режим дашборду — черга документів що потребують дії.

**Перемикач:** у topbar дашборду додати `[Огляд] [Вхідні]`. Стан: `property <string> dash-mode-state: "journal"` (вже є в app.slint як internal state).

**Новий struct у `types.slint`:**
```slint
export struct InboxItem {
    kind: string,          // "overdue" | "unsigned" | "act-needed" | "unmatched"
    doc-id: string,
    doc-number: string,
    counterparty: string,
    amount-str: string,    // pre-formatted Rust
    age-days: int,
    action-label: string,  // "Нагадати" | "Підписати" | "Створити акт" | "Поєднати"
}
```

**Нові properties в `app.slint`:**
```slint
in property <[InboxItem]> dash-inbox;
callback inbox-action(string, string);  // (doc-id, kind)
```

**Layout Inbox view** (показується коли `dash-mode-state == "inbox"`):
- Ліва панель: список `InboxItem`, згрупований за `kind` із секційними заголовками
- Кожен рядок: кольоровий лівий бар (danger/warning/info за типом) + номер документа + контрагент + сума + вік + кнопка дії
- Права панель: деталі вибраного (аналогічно journal sidebar — рахунок, статус, дії)
- Empty state: якщо `dash-inbox.length == 0` — "Чудово! Немає документів що потребують уваги"

**Файл:** `dashboard.slint` — додати `InboxView` component і перемикач у `Dashboard` export.

---

### 1.2 DocChain component

**Мета:** Візуальний pipeline Рахунок → Акт → Видаткова накладна.

**Новий struct у `types.slint`:**
```slint
export struct ChainStep {
    doc-type: string,    // "invoice" | "act" | "waybill"
    doc-number: string,  // "" якщо документ відсутній
    amount-str: string,
    status: string,      // "draft" | "issued" | "signed" | "paid" | "overdue" | ""
    exists: bool,
}
```

**Component у `components.slint`:**
```slint
export component DocChain {
    in property <[ChainStep]> steps;
    callback create-next(string);  // (doc-type)
}
```

**Зовнішній вигляд:** горизонтальний `HorizontalLayout` з трьох блоків + двох стрілок між ними.
- Існуючий документ: border-radius прямокутник з номером, статусом (StatusDot), сумою
- Відсутній документ: пунктирний border, текст "Немає", ghost-кнопка "Створити"
- Стрілка між блоками: `→` символ або `Image` з `Icons.arrow-right`, colorize: text-faint

**Новий struct у `types.slint`** (для групування ланцюжків):
```slint
export struct DocChainGroup {
    doc-id: string,
    steps: [ChainStep],
}
```
Slint не підтримує масиви анонімних struct → `DocChainGroup` обов'язковий.

**Інтеграція:**
- `documents.slint`: клік на рядку таблиці розкриває DocChain під рядком (accordion). Стан: `property <string> expanded-doc-id: ""`. Коли `data.id == expanded-doc-id` — рядок показує DocChain нижче (фіксована висота 80px, `visible: self.expanded`). При кліку знову — згортається. Rust завантажує chain через callback `doc-chain-load(doc-id)` → встановлює `in property <[ChainStep]> doc-chain-steps`.
- `counterparties.slint`: у вкладці "Документи" detail panel — список `DocChainGroup`, кожна група як `DocChain`.

**Нові properties в `app.slint`:**
```slint
in property <[ChainStep]> doc-chain-steps;  // для поточного expanded документа
callback doc-chain-load(string);             // (doc-id) → Rust завантажує chain
callback doc-chain-create(string, string);   // (doc-type, source-id) → stub у MVP
```

---

## Фаза 2: Backend wiring

### 2.1 Адаптація існуючих src/ui/ модулів

**`src/ui/dashboard.rs`:**
- `DashboardMetrics` тепер має: `revenue_month`, `expenses_month`, `net_month`, `outstanding`, `overdue`, `delta_revenue/expenses/net` (f32 %)
- Оновити `kpi_to_metrics()` — обчислювати `net_month = revenue - expenses`, `delta_*` з порівняння з попереднім місяцем (або залишити 0.0 спочатку)
- Додати `dash_expenses_str`, `dash_net_str`, `dash_overdue_str` → `set_dash_expenses_str()` тощо
- Додати `prepare_inbox_data()` → повертає `Vec<InboxItem>` з БД (акти зі статусом overdue/issued, рахунки без linked act, платежі без matched-doc)

**`src/ui/tasks.rs`:**
- `AppWindow` тепер має `tasks_open`, `tasks_done`, `tasks_all` + `day_events` + `tasks_open_count`, `tasks_high_count`, `tasks_done_count`, `tasks_today_label`
- Розділити `apply_tasks_to_ui()` — фільтрувати задачі на open/done/all, конвертувати `Priority` enum
- `day_events`: список подій на сьогодні з задач де `due_date == today`

**`src/ui/counterparties.rs`:**
- `CounterpartyDetails` тепер має: `overdue_amount`, `last_contact_days`, `last_contact_date` — додати в `db::counterparties::get_details()` або обчислити в Rust

**`src/ui/payments.rs`:**
- `Direction::In/Out` замість рядка — оновити маппінг `"income"/"expense"` → `Direction::In/Out`

### 2.2 Нові src/ui/ модулі

**`src/ui/reports.rs`** — вже описано в `docs/superpowers/plans/2026-04-21-remaining-features.md` (Tasks 3–6). Перевикористати без змін.

**`src/ui/settings.rs`** — повна версія (Task 7 з того ж плану).

### 2.3 `src/main.rs`

- Розширити початковий `tokio::join!` → додати `rep_data`, `set_data`
- Замінити заглушки Reports/Settings на `wire_reports_callbacks` + `wire_settings_callbacks`
- Додати `NavScreen::Reports` і `NavScreen::Settings` до `on_nav_changed`
- Додати callback: `ui.on_inbox_action(...)` → Rust визначає дію за `kind`
- Додати callback: `ui.on_doc_chain_create(...)` → router до відповідного модуля

---

## Порядок реалізації

```
Фаза 1 (UI):
  1.1  InboxItem struct → types.slint
  1.2  InboxView component → dashboard.slint  
  1.3  ChainStep struct → types.slint
  1.4  DocChain component → components.slint
  1.5  DocChain у documents.slint (accordion)
  1.6  DocChain у counterparties.slint (detail tab)
  1.7  Нові props/callbacks → app.slint

Фаза 2 (Backend):
  2.1  dashboard.rs адаптація + inbox_data query
  2.2  tasks.rs розділення open/done/all + day_events
  2.3  counterparties.rs detail оновлення
  2.4  payments.rs Direction enum
  2.5  reports.rs (новий)
  2.6  settings.rs (повний)
  2.7  main.rs завершення wiring
  2.8  cargo build → виправити помилки компіляції
```

---

## Обмеження (YAGNI)

- `inbox-action` → stub у Rust (log + ігнорувати) для MVP; реальна логіка (відкрити форму підпису тощо) — окрема задача
- `doc-chain-create` → stub у Rust для MVP
- `delta_*` в DashboardMetrics → передавати 0.0 до появи реального порівняння місяців
- DocChain accordion у documents — тільки read-only перегляд, без редагування inline
