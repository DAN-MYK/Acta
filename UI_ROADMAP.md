# UI Roadmap — привести Slint UI до вигляду прототипу

> Прототип: `C:\Users\MykhailoDan\Downloads\Acta` (React+Babel)
> Канонічний UI: `ui-redesign/` (Slint)
> `ui/` збережено лише як legacy reference під час міграції

---

## Поточний стан

### Вже виконано ✅
- Кольори: `theme.slint` повністю відповідає токенам прототипу (`#3D75F4`, `#F6F5F1`, hairline borders)
- Іконки: `ui/assets/icons/` — 37 SVG файлів, ідентичні прототипу
- Логотип: `ui/assets/logo/` — `Logo-Brand.svg`, `Logo-Light.svg`, `Logo-Dark.svg`, `Sidebar-Logo.svg`
- **Глобальний реєстр іконок:** `ui/components/icons.slint` — `Icons` global з усіма 37 іконками + логотипами ✅
- **SVG іконки скрізь:** KPI картки, EmptyState, кнопки дій, пошук, сайдбар, логотип ✅
- `EmptyState.slint` — drop-shadow з кнопки прибрано ✅

### Залишається (по пріоритету)
| # | Категорія | Опис | Складність |
|---|-----------|------|------------|
| 1 | Дизайн-система | Тіні на кнопках та form panels — заборонені прототипом | S |
| 2 | Дизайн-система | Радіуси відрізняються (sm=6 vs 4, md=8 vs 6...) | S |
| 3 | Дашборд | KPI Cards замість flat metric strip | M |
| 4 | Дашборд | Немає Inbox view (черга документів) | L |
| 5 | Документи | Немає chain view (Рахунок→Акт→Видаткова) | L |
| 6 | Контрагенти | Overlay замість master-detail layout | L |
| 7 | Задачі | Немає вкладок Open/Done/All та calendar sidebar | M |
| 8 | Налаштування | 3 секції з 6 (немає: Зовнішній вигляд, Нумерація, Інтеграції, Команда, Резервне) | L |
| 9 | Command Palette | Глобальний пошук Ctrl+K відсутній | XL |

---

## Як використовувати іконки та логотип

### Імпорт

```slint
import { Icons } from "../components/icons.slint";
// або з підпапки (acts/, invoices/, тощо):
import { Icons } from "../components/icons.slint";
```

Шлях `../components/icons.slint` — відносно поточного `.slint` файлу.

### Повна таблиця іконок

| Властивість | Файл | Де використовується |
|-------------|------|----------------------|
| `Icons.home` | `Home.svg` | Навігація — Головна |
| `Icons.documents` | `Documents.svg` | Навігація — Документи, KPI актів/рахунків/накладних, EmptyState |
| `Icons.counterparties` | `Counterparties.svg` | Навігація — Контрагенти |
| `Icons.payments` | `Payments.svg` | Навігація — Платежі, EmptyState платежів |
| `Icons.reports` | `Chart-Bar.svg` | Навігація — Звіти |
| `Icons.tasks` | `To-Do.svg` | Навігація — Задачі, EmptyState задач |
| `Icons.calendar` | `Calendar.svg` | Навігація — Календар |
| `Icons.settings` | `Settings.svg` | Навігація — Налаштування |
| `Icons.companies` | `Chart-Line.svg` | Навігація — Компанії |
| `Icons.draft` | `Draft.svg` | Статус: Чернетка, KPI непідписаних накладних |
| `Icons.issued` | `Issued.svg` | Статус: Виставлено, KPI неоплачених |
| `Icons.signed` | `Signed.svg` | Статус: Підписано |
| `Icons.paid` | `Paid.svg` | Статус: Оплачено, KPI відвантажених |
| `Icons.overdue` | `Overdue.svg` | Статус: Прострочено, KPI прострочених |
| `Icons.archive` | `Archive.svg` | Статус: Архів |
| `Icons.edit` | `Edit.svg` | Кнопка "Редагувати" у всіх списках |
| `Icons.delete` | `Delete.svg` | Кнопка "Видалити" у всіх списках |
| `Icons.new-doc` | `New.svg` | Кнопка "Новий документ" |
| `Icons.pdf` | `PDF-Print.svg` | Кнопка "PDF" у списках актів/рахунків |
| `Icons.download` | `Download.svg` | Кнопка "Завантажити" |
| `Icons.send` | `Send.svg` | Кнопка "Надіслати" |
| `Icons.refresh` | `Refresh.svg` | Кнопка "Оновити" |
| `Icons.search` | `Search.svg` | Пошукове поле у всіх списках |
| `Icons.filter` | `Filter.svg` | Кнопка "Фільтр" |
| `Icons.sort` | `Sort.svg` | Кнопка "Сортування" |
| `Icons.more` | `More.svg` | Кнопка "Ще..." (три крапки) |
| `Icons.uah` | `UAH.svg` | KPI виручки |
| `Icons.bank` | `Bank.svg` | Банківські рахунки |
| `Icons.incoming` | `Incoming.svg` | Напрямок: надходження |
| `Icons.outgoing` | `Outgoing.svg` | Напрямок: витрата |
| `Icons.check` | `Check.svg` | Позначка виконання |
| `Icons.uncheck` | `Uncheck.svg` | Знята позначка |
| `Icons.close` | `Close.svg` | Закрити overlay/діалог |
| `Icons.chevron-down` | `Chevron-Down.svg` | Розгорнути/згорнути (сайдбар) |
| `Icons.arrow-left` | `Arrow-Left.svg` | Назад |
| `Icons.arrow-right` | `Arrow-Right.svg` | Вперед / Наступний статус |
| `Icons.logo` | `Logo-Brand.svg` | Повний бренд-логотип |
| `Icons.logo-light` | `Logo-Light.svg` | Логотип для світлого фону |
| `Icons.logo-dark` | `Logo-Dark.svg` | Логотип для темного фону |
| `Icons.sidebar-logo` | `Sidebar-Logo.svg` | Логомарка у сайдбарі (28×28) |

### Патерни використання

#### Іконка в KPI картці
```slint
import { Icons } from "../components/icons.slint";
import { KpiCard } from "../components/KpiCard.slint";

KpiCard {
    accent: Theme.primary;
    icon: Icons.documents;    // SVG, colorize: accent автоматично
    value: "42";
    title: "Актів за місяць";
    subtitle: "поточний місяць";
}
```

#### Іконка в EmptyState
```slint
import { Icons } from "../components/icons.slint";
import { EmptyState } from "../components/EmptyState.slint";

EmptyState {
    icon: Icons.documents;    // SVG, colorize: Theme.text-muted автоматично
    title: "Актів ще немає";
    description: "Натисніть «+ Новий акт» щоб створити перший";
}
```

#### Іконка в ActionButton (кнопка дії в панелі вибору)
```slint
import { Icons } from "../components/icons.slint";
import { ActionButton } from "../components/action_button.slint";

ActionButton {
    width: 130px;
    icon: Icons.edit;         // показується зліва від тексту
    text: "Редагувати";
    clicked => { ... }
}

ActionButton {
    width: 180px;
    icon: Icons.arrow-right;
    text: "Наступний статус";
    primary: true;
    clicked => { ... }
}
```

#### Іконка пошуку у полі вводу
```slint
import { Icons } from "../components/icons.slint";

// Всередині Rectangle { clip: true; ... }
Image {
    x: 12px;
    y: (parent.height - 16px) / 2;
    width: 16px; height: 16px;
    source: Icons.search;
    image-fit: contain;
    colorize: Theme.text-muted;
}
HorizontalLayout {
    padding-left: 36px;   // 12px offset + 16px icon + 8px gap
    padding-right: 12px;
    search-input := TextInput { ... }
}
if search-input.text == "" : Text {
    x: 36px;
    height: parent.height;
    width: parent.width - 44px;
    text: "Пошук...";
    color: Theme.text-muted;
    vertical-alignment: center;
}
```

#### Іконка в кнопці рядка таблиці (icon-only)
```slint
import { Icons } from "../components/icons.slint";

Rectangle {
    width: 30px; height: 30px;
    border-radius: Theme.radius-sm;
    background: btn-ta.has-hover ? Theme.btn-action-bg : transparent;
    animate background { duration: 80ms; easing: ease; }

    Image {
        x: 8px; y: 8px;
        width: 14px; height: 14px;
        source: Icons.edit;
        image-fit: contain;
        colorize: btn-ta.has-hover ? Theme.primary : Theme.text-muted;
        animate colorize { duration: 80ms; easing: ease; }
    }

    btn-ta := TouchArea {
        mouse-cursor: pointer;
        clicked => { ... }
    }
}
```

#### Іконка в сайдбарі (вже реалізовано, патерн для довідки)
```slint
// NavItem отримує icon: image
NavItem {
    label: "Документи";
    icon: Icons.documents;
    active: root.current-feature == "acts";
    clicked => { root.navigate("acts"); }
}
```

#### Логотип у сайдбарі (вже реалізовано)
```slint
// Всередині синього прямокутника 28×28:
Image {
    x: 4px; y: 4px;
    width: 20px; height: 20px;
    source: Icons.sidebar-logo;
    image-fit: contain;
    colorize: Theme.text-white;   // робить SVG білим
}
```

### Правила colorize

`colorize` перезаписує всі пікселі SVG одним кольором (як CSS `color` для іконок).
Використовуй для монохромних іконок:

| Контекст | Колір |
|----------|-------|
| Неактивний стан | `Theme.text-muted` |
| Hover стан | `Theme.primary` або `Theme.text-primary` |
| Акцентний (KPI, сайдбар active) | відповідний `Theme.accent` |
| На темному фоні (кнопки, logo) | `Theme.text-white` |
| Небезпечна дія (delete hover) | `Theme.danger` |

### Де зберігаються файли

```
ui/
├── assets/
│   ├── icons/          ← 37 SVG іконок (Archive, Arrow-Left, Arrow-Right, Bank,
│   │                      Calendar, Chart-Bar, Chart-Line, Check, Chevron-Down,
│   │                      Close, Counterparties, Delete, Documents, Download,
│   │                      Draft, Edit, Filter, Home, Incoming, Issued, More,
│   │                      New, Outgoing, Overdue, PDF-Print, Paid, Payments,
│   │                      Refresh, Reports, Search, Send, Settings, Signed,
│   │                      Sort, To-Do, UAH, Uncheck)
│   └── logo/           ← Logo-Brand.svg, Logo-Dark.svg, Logo-Light.svg,
│                          Sidebar-Logo.svg
└── components/
    └── icons.slint     ← global Icons { ... } — єдина точка входу
```

---

## Крок 1 — Прибрати тіні з content елементів

> Правило прототипу: тіні ТІЛЬКИ на floating surfaces (overlay, popover, modal)

**Файли для змін:**

### `ui/components/shared.slint`
- Прибрати `drop-shadow-blur: 8px` та `drop-shadow-color` з `PrimaryButton` (~рядок 98–100)
- Прибрати `drop-shadow-blur: 12px` та `drop-shadow-color` з `DangerButton` (~рядок 159–160)

### `ui/acts/act_form.slint`
- Прибрати drop-shadow з form panel (~рядки 300–302, 916–918, 968–970)

### `ui/components/EmptyState.slint`
- Прибрати drop-shadow з іконки (~рядки 82–84) — замінити на `border: 1px solid Theme.border`

### Залишити тіні (коректно):
- `ui/app.slint:1086` — модальний overlay (правильно)

---

## Крок 2 — Виправити радіуси у theme.slint

> Прототип: sm=4, md=6, lg=8, xl=10 | Поточний: sm=6, md=8, lg=12, xl=16

```slint
// ui/theme.slint — замінити:
out property <length> radius-sm:   4px;   // було 6px
out property <length> radius-md:   6px;   // було 8px
out property <length> radius-lg:   8px;   // було 12px
out property <length> radius-xl:   10px;  // було 16px
```

> Після зміни перевірити візуально: кнопки, картки, badges, inputs

---

## Крок 3 — Розширити використання SVG іконок

> Зараз іконки використовуються ТІЛЬКИ у `sidebar.slint`. Всі інші компоненти — текстові заглушки або порожні.

### 3.1 Логотип у сайдбарі
**Файл:** `ui/components/sidebar.slint`

Замінити текстовий "Acta" логотип на SVG:
```slint
Image {
    source: @image-url("../assets/logo/Logo-Brand.svg");
    width: 72px;
    height: 24px;
    image-fit: contain;
}
```

### 3.2 Іконки у кнопках дій (таблиці)
**Файли:** `ui/acts/act_list.slint`, `ui/invoices/invoice_list.slint`, `ui/waybills/waybill_list.slint`, `ui/payments/payment_list.slint`

Додати `Image` до кнопок рядків таблиці:
| Кнопка | Іконка |
|--------|--------|
| Редагувати | `Edit.svg` |
| Видалити | `Delete.svg` |
| PDF | `PDF-Print.svg` |
| Завантажити | `Download.svg` |
| Надіслати | `Send.svg` |
| Ще | `More.svg` |
| Новий | `New.svg` |
| Оновити | `Refresh.svg` |

### 3.3 Іконки статусів документів
**Файл:** `ui/components/shared.slint` (StatusBadge)

Використати статусні іконки поруч з текстом badge:
| Статус | Іконка |
|--------|--------|
| Чернетка | `Draft.svg` |
| Видано | `Issued.svg` |
| Підписано | `Signed.svg` |
| Оплачено | `Paid.svg` |
| Прострочено | `Overdue.svg` |
| Архів | `Archive.svg` |

### 3.4 Іконки напрямку
**Файли:** `ui/payments/payment_list.slint`, `ui/documents/document_list.slint`

| Тип | Іконка |
|-----|--------|
| Надходження | `Incoming.svg` |
| Витрата | `Outgoing.svg` |

### 3.5 Іконка UAH у MetricStrip
Використати `UAH.svg` поруч з великими сумами на дашборді та у платежах.

---

## Крок 4 — MetricStrip компонент

> Прототип використовує **плоску смугу метрик** без карток. KpiCard з `accent border` зверху — відхилення від дизайну.

### 4.1 Створити `ui/components/MetricStrip.slint`

```slint
// Плоска горизонтальна смуга метрик з роздільниками
// Параметри: [{ label, value, subtext?, accent_color? }]
// Зовнішній вигляд:
//   [ Дохід ₴ 124 000  |  Витрати ₴ 48 000  |  Прибуток ₴ 76 000  |  Очікується ₴ 12 000 ]
//   border-bottom: 1px; padding: 16px 0; no background
export struct MetricItem {
    label: string,
    value: string,
    subtext: string,
    accent: color,
}

export component MetricStrip {
    in property <[MetricItem]> items;
    // горизонтальний HorizontalLayout з роздільниками між елементами
    // кожен елемент: label (font-sm, text-muted), value (font-display, serif)
}
```

### 4.2 Замінити KpiCard на MetricStrip
- **`ui/dashboard/dashboard.slint`** — 4 KPI cards → MetricStrip
- **`ui/payments/payment_list.slint`** — 2 KPI cards → MetricStrip
- **`ui/acts/act_list.slint`** — 4 KPI cards → MetricStrip

---

## Крок 5 — Дашборд: Inbox view

> Прототип: варіант "Вхідні" — черга документів що потребують уваги

### 5.1 Структура Inbox view (`ui/dashboard/dashboard_inbox.slint`)

```
┌─────────────────────────────────────────────────────────┐
│  MetricStrip (Income | Expenses | Net | Overdue)        │
├──────────────────────────┬──────────────────────────────┤
│  Список уваги (зліва)    │  Деталі вибраного (справа)  │
│                          │                              │
│  ⚠ Прострочено (3)       │  № РАХ-2025-042             │
│  ⊙ Без акту (2)          │  ТОВ "Альфа-Бізнес"         │
│  ≡ Непідписано (1)       │  ₴ 24 500  •  15 днів тому  │
│  ✉ Непоєднано (4)        │                              │
│  …                       │  [Підписати]  [Нагадати]    │
└──────────────────────────┴──────────────────────────────┘
```

**Типи елементів черги:**
| Тип | Опис | Дія |
|-----|------|-----|
| `overdue` | Прострочений платіж | Надіслати нагадування |
| `unsigned` | Акт без підпису | Підписати |
| `act_needed` | Рахунок без акту | Створити акт |
| `unmatched` | Платіж без документа | Поєднати |
| `draft` | Чернетка > 7 днів | Завершити або видалити |
| `waybill_needed` | Акт без видаткової | Створити видаткову |

### 5.2 Перемикач варіантів дашборду
У topbar додати: `[Огляд] [Вхідні]` — два режими дашборду.

---

## Крок 6 — Ланцюжок документів (Chain View)

> Прототип: Рахунок → Акт → Видаткова накладна — візуальний pipeline

### 6.1 Компонент `ui/components/DocChain.slint`

```
  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
  │  РАХ-2025-01 │ ──► │  АКТ-2025-01 │ ──► │  НАК-2025-01 │
  │  ₴ 24 500    │     │  ₴ 24 500    │     │   очікується  │
  │  Видано      │     │  Підписано   │     │  [Створити]  │
  └──────────────┘     └──────────────┘     └──────────────┘
```

Параметри компонента:
```slint
export struct ChainStep {
    doc_type: string,   // "invoice" | "act" | "waybill"
    doc_number: string, // "" якщо відсутній
    amount: string,
    status: string,
    exists: bool,
}

export component DocChain {
    in property <[ChainStep]> steps;
    callback create_next(string); // тип документа для створення
}
```

### 6.2 Використати DocChain у:
- **`ui/counterparties/counterparty_card.slint`** — вкладка "Документи"
- **`ui/documents/document_list.slint`** — новий режим "Ланцюжки"
- **`ui/acts/act_card.slint`** — в header секції картки акту

---

## Крок 7 — Контрагенти: Master-detail

> Прототип: список зліва + деталі справа (side-by-side), без overlay

### 7.1 Змінити layout у `ui/counterparties/counterparty_list.slint`

```
┌────────────┬───────────────────────────────────────────┐
│  Список    │  Деталі контрагента                       │
│            │                                           │
│  ◉ Альфа   │  ТОВ "Альфа-Бізнес"  [Редагувати]       │
│  ○ Бета    │  ЄДРПОУ: 12345678   •  Активний          │
│  ○ Гамма   │  ─────────────────────────────────────── │
│            │  [Документи] [Платежі] [Контракти] [Дії] │
│            │                                           │
│            │  Документи / Платежі / ...                │
└────────────┴───────────────────────────────────────────┘
```

- Ліва панель: 280px, список з пошуком
- Права панель: flexible, деталі + вкладки
- Перенести вміст `counterparty_card.slint` → inline права панель
- `counterparty_card.slint` — залишити або видалити (зробити alias)

### 7.2 Додати вкладку "Дії" (Activity feed)
```slint
// Хронологічний список подій:
// [Сьогодні] Виставлено рахунок РАХ-2025-042 — ₴ 24 500
// [Вчора]    Отримано платіж ПЛТ-2025-019 — ₴ 12 000
// [23 кві]   Підписано акт АКТ-2025-038
```

---

## Крок 8 — Задачі: вкладки + calendar sidebar

> Прототип: фільтр-вкладки Open/Done/All + sidebar з розкладом на сьогодні

### 8.1 Вкладки у `ui/tasks/task_list.slint`
Замінити тільки пошук на `StatusTabBar` з вкладками:
- Відкриті (count)
- Виконані
- Всі

### 8.2 Calendar sidebar (Today view)
```
┌────────────────────────────────┐
│  Сьогодні, 20 квітня 2026      │
├────────────────────────────────┤
│  09:00  Дзвінок з Альфа        │
│  11:00  Підписати акт          │
│  14:00  Виставити рахунок      │
│  16:00  ─ вільно ─             │
└────────────────────────────────┘
```
Нова секція праворуч від списку задач (~220px wide).

---

## Крок 9 — Налаштування: додаткові секції

> Поточний стан: Company, Categories, Templates
> Прототип: 6 секцій

### Додати у `ui/settings/settings.slint`:

**Вкладка 1: Зовнішній вигляд** *(нова)*
- Перемикач теми (Світла / Темна / Системна)
- Розмір шрифту (S / M / L)
- Щільність інтерфейсу (Компактна / Стандартна)
- Список клавіатурних скорочень (тільки для перегляду)

**Вкладка 4: Нумерація** *(нова)*
- Шаблони номерів документів: `АКТ-{YYYY}-{NNN}`, `РАХ-{YYYY}-{NNN}`, `НАК-{YYYY}-{NNN}`
- Поточний лічильник (скидається щороку)

**Вкладка 5: Інтеграції** *(нова)*
| Інтеграція | Статус | Дія |
|------------|--------|-----|
| BAS / M.E.Doc | Підключено / Не підключено | Налаштувати |
| Райффайзен | Не підключено | Підключити |
| Приватбанк | Не підключено | Підключити |
| Monobank | Не підключено | Підключити |
| Typst (PDF) | Встановлено | Змінити шлях |
| Дія.Підпис | Не підключено | Підключити |

**Вкладка 6: Резервне копіювання** *(нова)*
- Папка для резервних копій (path picker)
- Частота (щоденно / щотижнево / вручну)
- Список останніх резервних копій (дата, розмір)
- Кнопка "Створити зараз"

---

## Крок 10 — Command Palette (Ctrl+K)

> Глобальний пошук — найбільша одиночна фіча, XL складність

### 10.1 Overlay компонент `ui/components/CommandPalette.slint`

```
┌──────────────────────────────────────────────────────────┐
│  🔍  Пошук або введіть команду...              Ctrl+K  × │
├──────────────────────────────────────────────────────────┤
│  НАВІГАЦІЯ                                               │
│  ⌂  Головна                              G потім H      │
│  📄  Документи                            G потім D      │
│  👥  Контрагенти                          G потім C      │
├──────────────────────────────────────────────────────────┤
│  СТВОРИТИ                                                │
│  +  Новий акт                             C потім A      │
│  +  Новий рахунок                         C потім I      │
│  +  Нова видаткова                        C потім W      │
├──────────────────────────────────────────────────────────┤
│  ОСТАННІ ДОКУМЕНТИ                                       │
│  АКТ-2025-042  •  ТОВ Альфа  •  ₴ 24 500               │
│  РАХ-2025-039  •  ФОП Іваненко  •  ₴ 8 200             │
└──────────────────────────────────────────────────────────┘
```

### 10.2 Інтеграція
- В `app.slint`: global shortcut `Key.Control + "k"` → `show_command_palette = true`
- Логіка пошуку у Rust: по документах, контрагентах, командах
- Callback: `on_command_selected(action: string, id: string)`
- Клавіатурна навігація: ↑↓ Enter Esc

---

## Порядок виконання

```
Тиждень 1:  Крок 1 + Крок 2 + Крок 3      (polish дизайн-системи + іконки)
Тиждень 2:  Крок 4 + Крок 5               (MetricStrip + Inbox дашборд)
Тиждень 3:  Крок 6 + Крок 7               (Chain view + Master-detail)
Тиждень 4:  Крок 8 + Крок 9               (Задачі + Налаштування)
Тиждень 5:  Крок 10                        (Command Palette)
```

---

## Нотатки

- **Шрифти**: Slint обмежено підтримує кастомні шрифти. Спробувати через `font-family: "Source Serif 4"` у `Text {}` де потрібний serif-стиль (заголовки метрик, великі числа). Якщо системний шрифт виглядає прийнятно — залишити.
- **Іконки у Slint**: `Image { source: @image-url(...); colorize: Theme.text-muted; }` — для зміни кольору SVG через `colorize` property.
- **MetricStrip vs KpiCard**: KpiCard залишити для підекранів де картки доречні (наприклад, вкладки у counterparty_card) — замінювати тільки у header секціях списків.
