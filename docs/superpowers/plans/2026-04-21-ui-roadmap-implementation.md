# UI Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Додати Dashboard Inbox view і DocChain компонент до ui-redesign/, потім повністю підключити всі екрани до PostgreSQL backend.

**Architecture:** Дві фази: Фаза 1 — Slint-only зміни (не потребують БД, перевіряються через `cargo build`); Фаза 2 — Rust backend wiring (потребує запущеної PostgreSQL і `.env` з `DATABASE_URL`). Патерн: prepare\_data (async) → apply\_to\_ui (sync) → wire\_callbacks (реєструє handlers). TDD для Rust коду.

**Tech Stack:** Rust, Slint 1.x (`slint::include_modules!()`), sqlx (runtime-style), tokio::join! для паралельних запитів, `rust_decimal::Decimal` для фінансових значень.

---

## File Map

| Файл | Дія | Відповідальність |
|------|-----|-----------------|
| `ui-redesign/types.slint` | Modify | Додати InboxItem, ChainStep, DocChainGroup, ChartBar structs |
| `ui-redesign/components.slint` | Modify | Додати DocChain export component |
| `ui-redesign/dashboard.slint` | Modify | Додати InboxView component і mode toggle |
| `ui-redesign/documents.slint` | Modify | Додати chain accordion до DocRow |
| `ui-redesign/counterparties.slint` | Modify | Додати DocChainGroup в docs tab |
| `ui-redesign/app.slint` | Modify | Додати нові props/callbacks, виправити chart-bars тип |
| `src/db/dashboard.rs` | Modify | Додати `inbox_items` query |
| `src/ui/dashboard.rs` | Modify | Оновити DashboardData + inbox wiring |
| `src/ui/settings.rs` | Modify | Повна реалізація з DB loading |
| `src/ui/reports.rs` | Create | Новий модуль Reports |
| `src/ui/mod.rs` | Modify | Додати `pub mod reports` |
| `src/main.rs` | Modify | Завершити wiring всіх callbacks |

---

## Task 1: Додати Slint type structs

**Файли:**
- Modify: `ui-redesign/types.slint` (після рядка 216, після `DayEvent` struct)

- [ ] **Step 1: Додати structs в кінець types.slint**

Відкрити `ui-redesign/types.slint`. Після останнього `}` (рядок 216, кінець `DayEvent`) додати:

```slint
// ── Dashboard Inbox ──────────────────────────────────────────────────────────

export struct InboxItem {
    kind: string,          // "overdue" | "unsigned" | "act-needed" | "unmatched"
    doc-id: string,
    doc-number: string,
    counterparty: string,
    amount-str: string,    // pre-formatted Rust
    age-days: int,
    action-label: string,  // "Нагадати" | "Підписати" | "Поєднати"
}

// ── DocChain ─────────────────────────────────────────────────────────────────

export struct ChainStep {
    doc-type: string,    // "invoice" | "act" | "waybill"
    doc-number: string,  // "" якщо документ відсутній
    amount-str: string,  // pre-formatted Rust
    status: string,      // "draft"|"issued"|"signed"|"paid"|"overdue"|""
    exists: bool,
}

export struct DocChainGroup {
    doc-id: string,      // ID батьківського документа
    steps: [ChainStep],
}

// ── Chart bar ────────────────────────────────────────────────────────────────
// Named type to replace anonymous {rev-h,exp-h,month} — necessary for Rust access.

export struct ChartBar {
    rev-h: float,    // revenue bar height 0.0–1.0
    exp-h: float,    // expenses bar height 0.0–1.0
    month: string,   // abbreviated label "Січ", "Лют", etc.
}
```

- [ ] **Step 2: Перевірити build**

```
cargo build 2>&1 | head -30
```
Очікується: помилок компіляції немає (types.slint використовується скрізь, зміни — лише доповнення).

- [ ] **Step 3: Commit**

```bash
git add ui-redesign/types.slint
git commit -m "feat(ui): add InboxItem, ChainStep, DocChainGroup, ChartBar structs to types.slint"
```

---

## Task 2: DocChain component

**Файли:**
- Modify: `ui-redesign/components.slint` (після рядка 665, кінець файлу)

- [ ] **Step 1: Додати DocChain в кінець components.slint**

Відкрити `ui-redesign/components.slint`. Після рядка 665 (після `}` що закриває `SimpleProgressBar`) додати:

```slint
// ── DocChain ──────────────────────────────────────────────────────────────────
// Horizontal pipeline: Invoice → Act → Waybill.
// Existing docs: solid box with number, status dot, amount.
// Missing docs: dashed border + ghost "Створити" button.

component ChainStepBox inherits Rectangle {
    in property <ChainStep> step;
    callback create-next(string);

    property <bool> hovered: false;

    width: 158px;
    height: 80px;
    border-radius: AppTheme.radius-md;
    border-width: 1px;
    border-color: step.exists ? AppTheme.border : AppTheme.border;
    border-style: step.exists ? solid : dashed;
    background: step.exists ? AppTheme.bg-elevated : transparent;

    VerticalLayout {
        padding: 10px;
        spacing: 4px;

        Text {
            text:
                step.doc-type == "invoice" ? "РАХУНОК" :
                step.doc-type == "act"     ? "АКТ" :
                "НАКЛАДНА";
            color: AppTheme.text-faint;
            font-size: 10px;
            font-family: AppTheme.font-sans;
            font-weight: 600;
            letter-spacing: 1px;
        }

        if step.exists : Text {
            text: step.doc-number;
            color: AppTheme.accent-text;
            font-family: AppTheme.font-mono;
            font-size: 12px;
            font-weight: 500;
        }

        if step.exists : HorizontalLayout {
            spacing: 6px;
            alignment: start;

            StatusDot {
                tone:
                    step.status == "paid"    ? "success" :
                    step.status == "overdue" ? "danger"  :
                    step.status == "signed"  ? "success" :
                    step.status == "issued"  ? "info"    :
                    "muted";
            }
            Text {
                text: step.amount-str;
                color: AppTheme.text-muted;
                font-size: 11px;
                font-family: AppTheme.font-mono;
                vertical-alignment: center;
            }
        }

        if !step.exists : GhostButton {
            text: "Створити";
            small: true;
            clicked => { root.create-next(root.step.doc-type); }
        }
    }
}

export component DocChain inherits HorizontalLayout {
    in property <[ChainStep]> steps;
    callback create-next(string);  // (doc-type) → Rust handles

    spacing: 0px;
    alignment: start;

    for step[i] in steps : HorizontalLayout {
        spacing: 0px;

        if i > 0 : Text {
            text: "→";
            color: AppTheme.text-faint;
            font-size: 14px;
            font-family: AppTheme.font-sans;
            vertical-alignment: center;
            width: 28px;
            horizontal-alignment: center;
        }

        ChainStepBox {
            step: step;
            create-next(t) => { root.create-next(t); }
        }
    }
}
```

- [ ] **Step 2: Перевірити build**

```
cargo build 2>&1 | head -30
```
Очікується: помилок немає.

- [ ] **Step 3: Commit**

```bash
git add ui-redesign/components.slint
git commit -m "feat(ui): add DocChain component to components.slint"
```

---

## Task 3: Dashboard Inbox view

**Файли:**
- Modify: `ui-redesign/dashboard.slint`

- [ ] **Step 1: Додати InboxRow і InboxView компоненти**

Відкрити `ui-redesign/dashboard.slint`. Знайти рядок де закінчується `DashboardTaskItem` і починається `// ── Dashboard ────`:

```slint
// ── Dashboard ─────────────────────────────────────────────────────────────────
export component Dashboard inherits Rectangle {
```

Перед цим рядком вставити:

```slint
// ── Inbox row ─────────────────────────────────────────────────────────────────
component InboxRow inherits Rectangle {
    in property <InboxItem> item;
    callback action-clicked(string, string);

    property <bool> hovered: false;

    height: 56px;
    background: hovered ? AppTheme.bg-hover : transparent;

    // Kind color bar
    Rectangle {
        x: 0;
        y: 0;
        width: 3px;
        height: root.height;
        background:
            item.kind == "overdue"    ? AppTheme.danger  :
            item.kind == "unsigned"   ? AppTheme.warning :
            item.kind == "unmatched"  ? AppTheme.info    :
            AppTheme.text-faint;
    }

    // Hairline bottom
    Rectangle {
        y: root.height - 1px;
        width: root.width;
        height: 1px;
        background: AppTheme.border;
    }

    HorizontalLayout {
        padding-left: 16px;
        padding-right: 16px;
        spacing: 0px;
        alignment: stretch;

        VerticalLayout {
            horizontal-stretch: 1;
            alignment: center;
            spacing: 3px;

            HorizontalLayout {
                spacing: 10px;

                Text {
                    text: item.doc-number;
                    color: AppTheme.accent-text;
                    font-family: AppTheme.font-mono;
                    font-size: 12px;
                    font-weight: 500;
                    vertical-alignment: center;
                }

                Text {
                    text: item.counterparty;
                    color: AppTheme.text;
                    font-size: 12.5px;
                    font-family: AppTheme.font-sans;
                    vertical-alignment: center;
                    overflow: elide;
                    horizontal-stretch: 1;
                }

                Text {
                    text: item.amount-str;
                    color: AppTheme.text;
                    font-family: AppTheme.font-mono;
                    font-size: 12px;
                    vertical-alignment: center;
                }
            }

            Text {
                text: item.age-days + " дн. тому";
                color: AppTheme.text-faint;
                font-size: 11px;
                font-family: AppTheme.font-mono;
            }
        }

        GhostButton {
            text: item.action-label;
            small: true;
            clicked => { root.action-clicked(item.doc-id, item.kind); }
        }
    }

    TouchArea {
        pointer-event(e) => {
            if (e.kind == PointerEventKind.move) {
                root.hovered = self.has-hover;
            }
        }
    }
}

// ── Inbox empty state ─────────────────────────────────────────────────────────
component InboxEmpty inherits VerticalLayout {
    alignment: center;
    spacing: 8px;
    height: 200px;

    Text {
        text: "✓";
        color: AppTheme.success;
        font-size: 32px;
        horizontal-alignment: center;
    }
    Text {
        text: "Відмінно! Немає документів що потребують уваги.";
        color: AppTheme.text-muted;
        font-size: 13px;
        font-family: AppTheme.font-sans;
        horizontal-alignment: center;
        wrap: word-wrap;
    }
}

// ── InboxView ─────────────────────────────────────────────────────────────────
component InboxView inherits Rectangle {
    in property <[InboxItem]> items;
    callback action(string, string);

    background: transparent;

    if items.length == 0 : InboxEmpty { }

    if items.length > 0 : VerticalLayout {
        for item in items : InboxRow {
            item: item;
            action-clicked(id, kind) => { root.action(id, kind); }
        }
    }
}
```

- [ ] **Step 2: Додати props і mode toggle до Dashboard component**

Знайти рядок `export component Dashboard inherits Rectangle {` (~рядок 359).
Після нього знайти блок `// ── Data bindings from Rust ────` і ПЕРЕД ним додати:

```slint
    // ── Inbox mode ────────────────────────────────────────────────────────
    in property <[InboxItem]> inbox;
    callback inbox-action(string, string);
    property <string> mode-state: "journal";
```

- [ ] **Step 3: Додати mode toggle і InboxView в layout**

Знайти всередині `Dashboard` рядок де починається `content-col := VerticalLayout {`. Перед першим `Card {` в content-col вставити:

```slint
                // ── Mode toggle ────────────────────────────────────────────
                HorizontalLayout {
                    spacing: 2px;
                    alignment: start;
                    height: 32px;

                    Rectangle {
                        width: journal-btn.preferred-width + 20px;
                        height: 28px;
                        border-radius: AppTheme.radius-md;
                        background: mode-state == "journal" ? AppTheme.bg-elevated : transparent;
                        border-width: mode-state == "journal" ? 1px : 0px;
                        border-color: AppTheme.border;

                        journal-btn := Text {
                            text: "Огляд";
                            color: mode-state == "journal" ? AppTheme.text : AppTheme.text-muted;
                            font-size: 12.5px;
                            font-family: AppTheme.font-sans;
                            font-weight: mode-state == "journal" ? 500 : 400;
                            vertical-alignment: center;
                            horizontal-alignment: center;
                        }
                        TouchArea { mouse-cursor: pointer; clicked => { root.mode-state = "journal"; } }
                    }

                    Rectangle {
                        width: inbox-btn.preferred-width + 20px;
                        height: 28px;
                        border-radius: AppTheme.radius-md;
                        background: mode-state == "inbox" ? AppTheme.bg-elevated : transparent;
                        border-width: mode-state == "inbox" ? 1px : 0px;
                        border-color: AppTheme.border;

                        inbox-btn := Text {
                            text: "Вхідні" + (root.inbox.length > 0 ? " (" + root.inbox.length + ")" : "");
                            color: mode-state == "inbox" ? AppTheme.text : AppTheme.text-muted;
                            font-size: 12.5px;
                            font-family: AppTheme.font-sans;
                            font-weight: mode-state == "inbox" ? 500 : 400;
                            vertical-alignment: center;
                            horizontal-alignment: center;
                        }
                        TouchArea { mouse-cursor: pointer; clicked => { root.mode-state = "inbox"; } }
                    }
                }
```

Потім загорнути існуючі три `Card { }` блоки в content-col (KPI strip, Chart, Journal) в `if`:

Перед першим `// ── KPI Metric strip ───────` вставити:
```slint
                if mode-state == "journal" : VerticalLayout {
                    spacing: 20px;
                    horizontal-stretch: 1;
```

Після останнього закриваючого `}` Journal Card (перед `}` що закриває content-col) вставити:
```slint
                }

                if mode-state == "inbox" : InboxView {
                    horizontal-stretch: 1;
                    items: root.inbox;
                    action(id, kind) => { root.inbox-action(id, kind); }
                }
```

**Важливо:** `InboxView` і `InboxRow` потребують `import { InboxItem } from "types.slint"`. Знайти рядок `import { ... } from "types.slint"` у dashboard.slint і додати `InboxItem` до списку.

- [ ] **Step 4: Перевірити build**

```
cargo build 2>&1 | head -40
```
Очікується: помилок немає.

- [ ] **Step 5: Commit**

```bash
git add ui-redesign/dashboard.slint
git commit -m "feat(ui): add Dashboard Inbox view with mode toggle"
```

---

## Task 4: DocChain accordion в Documents

**Файли:**
- Modify: `ui-redesign/documents.slint`

- [ ] **Step 1: Оновити imports у documents.slint**

Знайти рядок з `import { DocumentItem, DocumentKind, DocumentStatus } from "types.slint";` і замінити на:

```slint
import { DocumentItem, DocumentKind, DocumentStatus, ChainStep } from "types.slint";
```

Знайти рядок з `import { Card, HDivider, VDivider, Badge, StatusDot, ...` і додати `DocChain` до списку:

```slint
import {
    Card, HDivider, VDivider, Badge, StatusDot,
    GhostButton, SecondaryButton, PrimaryButton, IconButton,
    SearchInput, HeadingSm, BulkBar, Checkbox, DocChain
} from "components.slint";
```

- [ ] **Step 2: Розширити DocRow для chain accordion**

Знайти компонент `component DocRow inherits Rectangle {` (приблизно рядок 111).

Після останнього `in property` перед `property <bool> hovered` додати:

```slint
    in property <bool> chain-expanded: false;
    in property <[ChainStep]> chain-steps;
    callback chain-toggle(string);
```

Знайти рядок `height: 46px;` і замінити на:

```slint
    height: chain-expanded ? 136px : 46px;
    animate height { duration: 150ms; easing: ease-out; }
```

Знайти рядок з `TouchArea {` що обробляє hover (в самому кінці DocRow, перед останнім `}`):

```slint
    TouchArea {
        mouse-cursor: pointer;
        pointer-event(e) => {
            if (e.kind == PointerEventKind.move) {
```

Після нього (після `}` що закриває цей TouchArea, але перед закриваючим `}` DocRow) додати:

```slint
    if chain-expanded : Rectangle {
        y: 46px;
        height: 90px;
        width: root.width;
        background: AppTheme.bg-subtle;

        Rectangle {
            y: 0;
            width: root.width;
            height: 1px;
            background: AppTheme.border;
        }

        DocChain {
            x: 18px;
            y: 10px;
            width: root.width - 36px;
            steps: root.chain-steps;
            create-next(_t) => { }
        }
    }
```

В HorizontalLayout (рядок дій документа), знайти `// Actions` секцію і ПЕРЕД нею додати кнопку розкриття ланцюжка:

```slint
        // Chain expand toggle
        Rectangle {
            width: 30px;
            height: 30px;
            border-radius: AppTheme.radius-sm;
            background: chain-ta.has-hover ? AppTheme.bg-hover : transparent;

            Text {
                text: root.chain-expanded ? "▲" : "▼";
                color: AppTheme.text-faint;
                font-size: 9px;
                horizontal-alignment: center;
                vertical-alignment: center;
            }

            chain-ta := TouchArea {
                mouse-cursor: pointer;
                clicked => { root.chain-toggle(root.doc.id); }
            }
        }
```

- [ ] **Step 3: Додати props і callbacks до Documents component**

Знайти `export component Documents inherits Rectangle {` (рядок 313).
Після `// Callbacks` блоку додати:

```slint
    callback chain-load(string);           // (doc-id) Rust завантажує chain
    callback chain-create(string, string); // (doc-type, source-id) stub
```

Після `property <string> active-tab: "all";` додати:

```slint
    in property <[ChainStep]> chain-steps;
    property <string> expanded-doc-id: "";
```

- [ ] **Step 4: Прокинути props у for loop**

Знайти `for doc[i] in visible-rows : DocRow {` і додати після останнього існуючого prop:

```slint
        chain-expanded: doc.id == root.expanded-doc-id;
        chain-steps: root.chain-steps;
        chain-toggle(id) => {
            if (root.expanded-doc-id == id) {
                root.expanded-doc-id = "";
            } else {
                root.expanded-doc-id = id;
                root.chain-load(id);
            }
        }
```

- [ ] **Step 5: Перевірити build**

```
cargo build 2>&1 | head -40
```
Очікується: помилок немає.

- [ ] **Step 6: Commit**

```bash
git add ui-redesign/documents.slint
git commit -m "feat(ui): add DocChain accordion to Documents screen"
```

---

## Task 5: DocChain у Counterparties detail docs tab

**Файли:**
- Modify: `ui-redesign/counterparties.slint`

- [ ] **Step 1: Оновити imports у counterparties.slint**

Знайти `import { CounterpartyItem, CounterpartyDetails, DocumentItem, PaymentItem, DocumentKind, DocumentStatus } from "types.slint";` і додати нові типи:

```slint
import { CounterpartyItem, CounterpartyDetails, DocumentItem, PaymentItem, DocumentKind, DocumentStatus, ChainStep, DocChainGroup } from "types.slint";
```

Знайти import з components.slint і додати `DocChain`:

```slint
import {
    Card, HDivider, VDivider, Badge, StatusDot,
    GhostButton, SecondaryButton, PrimaryButton, IconButton,
    SearchInput, HeadingSm, HeadingLg, Avatar, SectionLabel, DocChain
} from "components.slint";
```

- [ ] **Step 2: Додати props до Counterparties component**

Знайти `export component Counterparties inherits Rectangle {` (рядок 201).
Знайти кінець блоку `in property` і перед `// Callbacks` додати:

```slint
    in property <[DocChainGroup]> doc-chains;
    callback chain-create(string, string);  // (doc-type, source-id) stub
```

- [ ] **Step 3: Додати секцію DocChain в docs tab**

Знайти в docs tab блок `for doc in detail-docs : Rectangle {` (рядок ~448).
Після закриваючого `}` цього for loop (але ще всередині `if active-tab == "docs"`) додати:

```slint
                    if root.doc-chains.length > 0 : VerticalLayout {
                        padding-top: 16px;
                        spacing: 12px;

                        HorizontalLayout {
                            padding-left: 28px;

                            Text {
                                text: "ЛАНЦЮЖКИ ДОКУМЕНТІВ";
                                color: AppTheme.text-faint;
                                font-size: 10.5px;
                                font-family: AppTheme.font-sans;
                                font-weight: 600;
                                letter-spacing: 1.2px;
                            }
                        }

                        HDivider { }

                        for group in root.doc-chains : Rectangle {
                            height: 100px;

                            HorizontalLayout {
                                padding-left: 28px;
                                padding-right: 28px;
                                alignment: start;

                                DocChain {
                                    steps: group.steps;
                                    create-next(t) => { root.chain-create(t, group.doc-id); }
                                }
                            }
                        }
                    }
```

- [ ] **Step 4: Перевірити build**

```
cargo build 2>&1 | head -40
```
Очікується: помилок немає.

- [ ] **Step 5: Commit**

```bash
git add ui-redesign/counterparties.slint
git commit -m "feat(ui): add DocChain groups to Counterparties docs tab"
```

---

## Task 6: Оновити app.slint + verify cargo build

**Файли:**
- Modify: `ui-redesign/app.slint`

- [ ] **Step 1: Оновити imports з types.slint**

Знайти рядок:
```slint
import {
    DocumentItem, CounterpartyItem, PaymentItem, TaskItem,
    MonthlyData, AccountItem, DashboardMetrics, JournalRow,
    CounterpartyDetails, ReportMetrics, ExpenseCategory, DrillRow,
    CompanyInfo, IntegrationItem, TeamMember, NumberingRow, DayEvent,
    DocumentKind, DocumentStatus, Priority, Direction,
} from "types.slint";
```
Замінити на:
```slint
import {
    DocumentItem, CounterpartyItem, PaymentItem, TaskItem,
    MonthlyData, AccountItem, DashboardMetrics, JournalRow,
    CounterpartyDetails, ReportMetrics, ExpenseCategory, DrillRow,
    CompanyInfo, IntegrationItem, TeamMember, NumberingRow, DayEvent,
    DocumentKind, DocumentStatus, Priority, Direction,
    InboxItem, ChainStep, DocChainGroup, ChartBar,
} from "types.slint";
```

- [ ] **Step 2: Виправити анонімний тип chart-bars у AppWindow**

Знайти:
```slint
    in property <[{rev-h: float, exp-h: float, month: string}]> dash-chart-bars;
```
Замінити на:
```slint
    in property <[ChartBar]> dash-chart-bars;
```

Знайти:
```slint
    in property <[{rev-h: float, exp-h: float, month: string}]> rep-chart-bars;
```
Замінити на:
```slint
    in property <[ChartBar]> rep-chart-bars;
```

- [ ] **Step 3: Додати нові properties до AppWindow**

Знайти `// ── Dashboard data ─────────────────────────────────────────────────────────────` і після блоку dashboard properties додати:

```slint
    in property <[InboxItem]> dash-inbox;
```

Знайти `// ── Documents data ─────────────────────────────────────────────────────────────` і після блоку documents properties додати:

```slint
    in property <[ChainStep]> doc-chain-steps;
```

Знайти `// ── Counterparties data ────────────────────────────────────────────────────────` і після блоку counterparties properties додати:

```slint
    in property <[DocChainGroup]> cp-doc-chains;
```

- [ ] **Step 4: Додати нові callbacks до AppWindow**

Знайти `// ── Outbound callbacks ─────────────────────────────────────────────────────────` і після `callback nav-changed(NavScreen);` додати:

```slint
    callback inbox-action(string, string);   // (doc-id, kind) → stub Rust
    callback doc-chain-load(string);         // (doc-id) → Rust loads chain steps
    callback doc-chain-create(string, string); // (doc-type, source-id) → stub
```

- [ ] **Step 5: Прокинути нові props до Dashboard**

Знайти `if root.current-screen == NavScreen.Dashboard : Dashboard {` і після останнього існуючого prop/callback додати:

```slint
            inbox: root.dash-inbox;
            inbox-action(id, kind) => { root.inbox-action(id, kind); }
```

- [ ] **Step 6: Прокинути chain props до Documents**

Знайти `if root.current-screen == NavScreen.Documents : Documents {` і після останнього existing callback (`page-changed`) додати:

```slint
            chain-steps: root.doc-chain-steps;
            chain-load(id) => { root.doc-chain-load(id); }
            chain-create(t, id) => { root.doc-chain-create(t, id); }
```

- [ ] **Step 7: Прокинути doc-chains до Counterparties**

Знайти `if root.current-screen == NavScreen.Counterparties : Counterparties {` і після останнього existing callback (`create-doc`) додати:

```slint
            doc-chains: root.cp-doc-chains;
            chain-create(t, id) => { root.doc-chain-create(t, id); }
```

- [ ] **Step 8: Перевірити що chart-bars в Dashboard правильно використовується**

Знайти в dashboard.slint рядок:
```slint
    in property <[{rev-h: float, exp-h: float, month: string}]> chart-bars;
```
Замінити на:
```slint
    in property <[ChartBar]> chart-bars;
```

Також перевірити reports.slint — там теж може бути анонімний тип. Знайти аналогічний рядок в `ui-redesign/reports.slint` і замінити.

- [ ] **Step 9: Перевірити повний build**

```
cargo build 2>&1 | head -50
```
Очікується: помилок немає. Якщо є помилки пов'язані з chart-bars — це означає що Rust код досі використовує `Default::default()`, що коректно для зараз.

- [ ] **Step 10: Commit**

```bash
git add ui-redesign/app.slint ui-redesign/dashboard.slint ui-redesign/reports.slint
git commit -m "feat(ui): wire new props/callbacks in app.slint; fix ChartBar named type"
```

---

## Task 7: Dashboard Rust inbox — DB query та DashboardData оновлення

**Файли:**
- Modify: `src/db/dashboard.rs`
- Modify: `src/ui/dashboard.rs`

> **Потребує:** запущений PostgreSQL + DATABASE_URL у `.env`

- [ ] **Step 1: Написати failing test для `inbox_items_from_row`**

Додати в `src/ui/dashboard.rs` в `#[cfg(test)] mod tests`:

```rust
#[test]
fn inbox_item_from_row_maps_kind_and_action() {
    let item = inbox_item_from_row(
        "act:abc123",
        "АКТ-2026-001",
        "ТОВ Тест",
        rust_decimal_macros::dec!(15000),
        45,
        "overdue",
        "Нагадати",
    );
    assert_eq!(item.kind.as_str(), "overdue");
    assert_eq!(item.doc_number.as_str(), "АКТ-2026-001");
    assert_eq!(item.counterparty.as_str(), "ТОВ Тест");
    assert_eq!(item.age_days, 45);
    assert_eq!(item.action_label.as_str(), "Нагадати");
    assert!(item.amount_str.contains("15000"), "amount: {}", item.amount_str);
}
```

- [ ] **Step 2: Запустити test щоб перевірити що fails**

```
cargo test inbox_item_from_row_maps_kind_and_action 2>&1 | tail -10
```
Очікується: FAIL — `inbox_item_from_row` not found.

- [ ] **Step 3: Додати inbox DB query в `src/db/dashboard.rs`**

Додати після функції `revenue_by_month` (або в кінці файлу перед `#[cfg(test)]`):

```rust
/// Рядок результату запиту Inbox.
pub struct InboxRow {
    pub doc_id: String,
    pub doc_number: String,
    pub counterparty: String,
    pub amount: rust_decimal::Decimal,
    pub age_days: i32,
    pub kind: String,
    pub action_label: String,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for InboxRow {
    fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row as _;
        Ok(Self {
            doc_id:       r.try_get("doc_id")?,
            doc_number:   r.try_get("doc_number")?,
            counterparty: r.try_get("counterparty")?,
            amount:       r.try_get("amount")?,
            age_days:     r.try_get("age_days")?,
            kind:         r.try_get("kind")?,
            action_label: r.try_get("action_label")?,
        })
    }
}

/// Документи що потребують уваги: прострочені акти + неузгоджені платежі.
pub async fn inbox_items(pool: &PgPool, company_id: Uuid) -> Result<Vec<InboxRow>> {
    sqlx::query_as::<_, InboxRow>(
        r#"
        SELECT
            'act:' || a.id::text          AS doc_id,
            a.number                       AS doc_number,
            c.name                         AS counterparty,
            a.total_amount                 AS amount,
            (CURRENT_DATE - a.date)::int   AS age_days,
            'overdue'                      AS kind,
            'Нагадати'                     AS action_label
        FROM acts a
        JOIN counterparties c ON c.id = a.counterparty_id
        WHERE a.company_id = $1
          AND a.status = 'issued'
          AND a.date < CURRENT_DATE - INTERVAL '14 days'
        UNION ALL
        SELECT
            'pay:' || p.id::text,
            'ПЛТ-' || LEFT(p.id::text, 8),
            COALESCE(c.name, '—'),
            p.amount,
            (CURRENT_DATE - p.date)::int,
            'unmatched',
            'Поєднати'
        FROM payments p
        LEFT JOIN counterparties c ON c.id = p.counterparty_id
        WHERE p.company_id = $1
          AND p.is_reconciled = false
        ORDER BY age_days DESC
        LIMIT 20
        "#,
    )
    .bind(company_id)
    .bind(company_id)
    .fetch_all(pool)
    .await
    .map_err(anyhow::Error::from)
}
```

- [ ] **Step 4: Додати `inbox_item_from_row` та оновити `DashboardData` в `src/ui/dashboard.rs`**

Додати функцію перед `prepare_dashboard_data`:

```rust
pub fn inbox_item_from_row(
    doc_id: &str,
    doc_number: &str,
    counterparty: &str,
    amount: rust_decimal::Decimal,
    age_days: i32,
    kind: &str,
    action_label: &str,
) -> crate::InboxItem {
    use rust_decimal::prelude::ToPrimitive;
    crate::InboxItem {
        kind: kind.into(),
        doc_id: doc_id.into(),
        doc_number: doc_number.into(),
        counterparty: counterparty.into(),
        amount_str: format!("{:.0}", amount.to_f64().unwrap_or(0.0)).into(),
        age_days,
        action_label: action_label.into(),
    }
}
```

Оновити `DashboardData` — додати поле:
```rust
pub struct DashboardData {
    pub metrics: crate::DashboardMetrics,
    pub revenue_str: String,
    pub expenses_str: String,
    pub net_str: String,
    pub outstanding_str: String,
    pub overdue_str: String,
    pub journal: Vec<crate::JournalRow>,
    pub tasks: Vec<crate::TaskItem>,
    pub inbox: Vec<crate::InboxItem>,  // нове поле
}
```

Оновити `prepare_dashboard_data` — додати `inbox_res` до `tokio::join!`:

```rust
pub async fn prepare_dashboard_data(pool: &PgPool, company_id: Uuid) -> DashboardData {
    let (kpi_res, recent_res, tasks_res, inbox_res) = tokio::join!(
        db::dashboard::get_kpi_summary(pool, company_id),
        db::dashboard::get_recent_acts(pool, company_id, 20),
        db::tasks::list_open(pool),
        db::dashboard::inbox_items(pool, company_id),
    );

    let kpi = kpi_res.unwrap_or_else(|e| {
        tracing::error!("dashboard kpi failed: {e}");
        KpiSummary {
            revenue_this_month: rust_decimal::Decimal::ZERO,
            unpaid_total: rust_decimal::Decimal::ZERO,
            acts_this_month: 0,
            active_counterparties: 0,
        }
    });

    let recent = recent_res.unwrap_or_default();
    let tasks = tasks_res.unwrap_or_default();
    let inbox_rows = inbox_res.unwrap_or_default();

    let journal: Vec<crate::JournalRow> = recent.iter().map(recent_act_to_journal_row).collect();
    let task_items: Vec<crate::TaskItem> = tasks.iter().map(task_to_item).collect();
    let inbox: Vec<crate::InboxItem> = inbox_rows
        .iter()
        .map(|r| inbox_item_from_row(
            &r.doc_id, &r.doc_number, &r.counterparty,
            r.amount, r.age_days, &r.kind, &r.action_label,
        ))
        .collect();
    let revenue = decimal_to_f32(kpi.revenue_this_month);

    DashboardData {
        revenue_str: format!("{:.0}", revenue),
        expenses_str: "0".to_string(),
        net_str: format!("{:.0}", revenue),
        outstanding_str: format!("{:.0}", kpi.unpaid_total),
        overdue_str: "0".to_string(),
        metrics: kpi_to_metrics(&kpi),
        journal,
        tasks: task_items,
        inbox,
    }
}
```

Оновити `apply_dashboard_to_ui` — додати set\_dash\_inbox:

```rust
pub fn apply_dashboard_to_ui(ui: &crate::AppWindow, data: DashboardData) {
    ui.set_dash_metrics(data.metrics);
    ui.set_dash_revenue_str(data.revenue_str.into());
    ui.set_dash_expenses_str(data.expenses_str.into());
    ui.set_dash_net_str(data.net_str.into());
    ui.set_dash_outstanding_str(data.outstanding_str.into());
    ui.set_dash_overdue_str(data.overdue_str.into());
    ui.set_dash_journal(ModelRc::new(VecModel::from(data.journal)));
    ui.set_dash_tasks(ModelRc::new(VecModel::from(data.tasks)));
    ui.set_dash_inbox(ModelRc::new(VecModel::from(data.inbox)));
    ui.set_dash_chart_bars(ModelRc::new(VecModel::<crate::ChartBar>::default()));
    ui.set_dash_accounts(ModelRc::new(VecModel::<crate::AccountItem>::default()));
}
```

- [ ] **Step 5: Запустити test щоб перевірити що passes**

```
cargo test inbox_item_from_row_maps_kind_and_action 2>&1 | tail -5
```
Очікується: PASS.

- [ ] **Step 6: Запустити всі dashboard тести**

```
cargo test ui::dashboard 2>&1 | tail -10
```
Очікується: всі тести проходять.

- [ ] **Step 7: Commit**

```bash
git add src/db/dashboard.rs src/ui/dashboard.rs
git commit -m "feat: add inbox_items DB query; wire dash-inbox in DashboardData"
```

---

## Task 8: Settings — повна реалізація з DB

**Файли:**
- Modify: `src/ui/settings.rs`

- [ ] **Step 1: Написати failing test**

Додати в `src/ui/settings.rs` новий `#[cfg(test)] mod tests` (або додати до існуючого):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use acta::models::company::Company;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_company() -> Company {
        Company {
            id: Uuid::new_v4(),
            name: "ТОВ Тест".into(),
            short_name: None,
            edrpou: Some("12345678".into()),
            ipn: None,
            iban: None,
            legal_address: None,
            actual_address: None,
            phone: None,
            email: None,
            director_name: None,
            accountant_name: None,
            tax_system: None,
            is_vat_payer: false,
            logo_path: None,
            notes: None,
            is_archived: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn company_to_info_maps_optional_fields_to_empty_string() {
        let c = make_company();
        let info = company_to_info(&c);
        assert_eq!(info.full_name.as_str(), "ТОВ Тест");
        assert_eq!(info.edrpou.as_str(), "12345678");
        assert_eq!(info.short_name.as_str(), "");
        assert_eq!(info.ipn.as_str(), "");
        assert!(!info.vat_registered);
    }
}
```

- [ ] **Step 2: Запустити test щоб перевірити що fails**

```
cargo test company_to_info_maps_optional_fields 2>&1 | tail -5
```
Очікується: FAIL — `company_to_info` not found.

- [ ] **Step 3: Повністю замінити `src/ui/settings.rs`**

```rust
use slint::{ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;
use std::sync::{Arc, Mutex};

use acta::db;
use acta::models::company::Company;

pub struct SettingsData {
    pub company_info: crate::CompanyInfo,
}

pub fn company_to_info(c: &Company) -> crate::CompanyInfo {
    crate::CompanyInfo {
        full_name:      c.name.clone().into(),
        short_name:     c.short_name.clone().unwrap_or_default().into(),
        edrpou:         c.edrpou.clone().unwrap_or_default().into(),
        ipn:            c.ipn.clone().unwrap_or_default().into(),
        address:        c.legal_address.clone().unwrap_or_default().into(),
        director:       c.director_name.clone().unwrap_or_default().into(),
        iban:           c.iban.clone().unwrap_or_default().into(),
        bank:           slint::SharedString::default(),
        vat_registered: c.is_vat_payer,
        vat_cert:       slint::SharedString::default(),
    }
}

pub async fn prepare_settings_data(pool: &PgPool, company_id: Uuid) -> SettingsData {
    let company_info = db::companies::get_by_id(pool, company_id)
        .await
        .ok()
        .flatten()
        .map(|c| company_to_info(&c))
        .unwrap_or_else(|| crate::CompanyInfo {
            full_name:      slint::SharedString::default(),
            short_name:     slint::SharedString::default(),
            edrpou:         slint::SharedString::default(),
            ipn:            slint::SharedString::default(),
            address:        slint::SharedString::default(),
            director:       slint::SharedString::default(),
            iban:           slint::SharedString::default(),
            bank:           slint::SharedString::default(),
            vat_registered: false,
            vat_cert:       slint::SharedString::default(),
        });
    SettingsData { company_info }
}

pub fn apply_settings_to_ui(ui: &crate::AppWindow, data: SettingsData) {
    ui.set_company_info(data.company_info);

    ui.set_integrations(ModelRc::new(VecModel::from(vec![
        crate::IntegrationItem {
            label: "BAS / 1C".into(),
            description: "Імпорт документів та довідників з BAS".into(),
            tag: "bas".into(),
            enabled: false,
        },
        crate::IntegrationItem {
            label: "ПриватБанк".into(),
            description: "Синхронізація банківських виписок".into(),
            tag: "privatbank".into(),
            enabled: false,
        },
        crate::IntegrationItem {
            label: "Монобанк".into(),
            description: "Синхронізація банківських виписок".into(),
            tag: "monobank".into(),
            enabled: false,
        },
    ])));

    ui.set_team_members(ModelRc::new(VecModel::<crate::TeamMember>::default()));

    ui.set_numbering_rows(ModelRc::new(VecModel::from(vec![
        crate::NumberingRow {
            doc_type: "Акт виконаних робіт".into(),
            template: "АКТ-{YYYY}-{NNN}".into(),
            example: "АКТ-2026-001".into(),
            next_number: 1,
        },
        crate::NumberingRow {
            doc_type: "Видаткова накладна".into(),
            template: "НАК-{YYYY}-{NNN}".into(),
            example: "НАК-2026-001".into(),
            next_number: 1,
        },
        crate::NumberingRow {
            doc_type: "Рахунок-фактура".into(),
            template: "РАХ-{YYYY}-{NNN}".into(),
            example: "РАХ-2026-001".into(),
            next_number: 1,
        },
    ])));

    ui.set_last_backup_label("Резервна копія не створювалась".into());
    ui.set_last_backup_file(slint::SharedString::default());
}

pub fn wire_settings_callbacks(
    ui: &crate::AppWindow,
    pool: &Arc<PgPool>,
    company_id: &Arc<Mutex<Uuid>>,
) {
    use slint::ComponentHandle;

    ui.on_settings_company_saved({
        let pool = pool.clone();
        let company_id = company_id.clone();
        move |info| {
            let pool = pool.clone();
            let cid = *company_id.lock().unwrap();
            let update = acta::models::company::UpdateCompany {
                name:            info.full_name.as_str().trim().to_string(),
                short_name:      opt_str(&info.short_name),
                edrpou:          opt_str(&info.edrpou),
                iban:            opt_str(&info.iban),
                legal_address:   opt_str(&info.address),
                director_name:   opt_str(&info.director),
                accountant_name: None,
                tax_system:      None,
                is_vat_payer:    info.vat_registered,
                logo_path:       None,
            };
            tokio::spawn(async move {
                match db::companies::update(&pool, cid, &update).await {
                    Ok(Some(_)) => tracing::info!("settings: company saved"),
                    Ok(None)    => tracing::warn!("settings: company id={cid} not found"),
                    Err(e)      => tracing::error!("settings: save failed: {e}"),
                }
            });
        }
    });

    ui.on_settings_section_changed(|_| {});
    ui.on_settings_dark_mode_toggled(|_| {});
    ui.on_settings_density_changed(|_| {});
    ui.on_settings_integration_configure(|_| {});
    ui.on_settings_team_invite(|| {});
    ui.on_settings_backup_now(|| {});
    ui.on_settings_backup_download(|| {});
}

fn opt_str(s: &slint::SharedString) -> Option<String> {
    let v = s.as_str().trim().to_string();
    if v.is_empty() { None } else { Some(v) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use acta::models::company::Company;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_company() -> Company {
        Company {
            id: Uuid::new_v4(),
            name: "ТОВ Тест".into(),
            short_name: None,
            edrpou: Some("12345678".into()),
            ipn: None,
            iban: None,
            legal_address: None,
            actual_address: None,
            phone: None,
            email: None,
            director_name: None,
            accountant_name: None,
            tax_system: None,
            is_vat_payer: false,
            logo_path: None,
            notes: None,
            is_archived: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn company_to_info_maps_optional_fields_to_empty_string() {
        let c = make_company();
        let info = company_to_info(&c);
        assert_eq!(info.full_name.as_str(), "ТОВ Тест");
        assert_eq!(info.edrpou.as_str(), "12345678");
        assert_eq!(info.short_name.as_str(), "");
        assert_eq!(info.ipn.as_str(), "");
        assert!(!info.vat_registered);
    }
}
```

- [ ] **Step 4: Запустити test**

```
cargo test ui::settings 2>&1 | tail -10
```
Очікується: всі тести PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/settings.rs
git commit -m "feat: settings persistence — load company from DB, wire save callback"
```

---

## Task 9: Reports module

**Файли:**
- Create: `src/ui/reports.rs`
- Modify: `src/ui/mod.rs`

Цей модуль реалізує повний список запитів описаний в `docs/superpowers/plans/2026-04-21-remaining-features.md` (Tasks 3–6). Нижче наведено повну реалізацію.

- [ ] **Step 1: Додати `pub mod reports` до `src/ui/mod.rs`**

Відкрити `src/ui/mod.rs` і додати рядок:

```rust
pub mod helpers;
pub mod dashboard;
pub mod documents;
pub mod counterparties;
pub mod payments;
pub mod reports;
pub mod tasks;
pub mod settings;
```

- [ ] **Step 2: Написати failing test**

Створити `src/ui/reports.rs` з тільки тестовим модулем:

```rust
fn period_to_months(period: i32) -> u32 { todo!() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_to_months_maps_correctly() {
        assert_eq!(period_to_months(0), 1);
        assert_eq!(period_to_months(1), 3);
        assert_eq!(period_to_months(2), 12);
        assert_eq!(period_to_months(3), 3);
        assert_eq!(period_to_months(99), 3);
    }
}
```

- [ ] **Step 3: Запустити test щоб перевірити що fails**

```
cargo test ui::reports::tests::period_to_months 2>&1 | tail -5
```
Очікується: FAIL (todo! panic або compile error).

- [ ] **Step 4: Замінити `src/ui/reports.rs` повною реалізацією**

```rust
use slint::{ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;
use std::sync::{Arc, Mutex};

use acta::db;
use acta::models::dashboard::CategoryRevenue;

/// Кількість місяців для rep-period-state (0=місяць, 1=квартал, 2=рік, _=квартал).
pub fn period_to_months(period: i32) -> u32 {
    match period {
        0 => 1,
        1 => 3,
        2 => 12,
        _ => 3,
    }
}

pub struct ReportsData {
    pub metrics: crate::ReportMetrics,
    pub chart_bars: Vec<crate::ChartBar>,
    pub categories: Vec<crate::ExpenseCategory>,
    pub drill_rows: Vec<crate::DrillRow>,
    pub revenue_str: String,
    pub expenses_str: String,
    pub profit_str: String,
    pub margin_str: String,
}

pub async fn prepare_reports_data(pool: &PgPool, company_id: Uuid, period: i32) -> ReportsData {
    let months = period_to_months(period);

    let (rev_res, exp_res, cats_res) = tokio::join!(
        db::dashboard::revenue_by_month(pool, company_id, months),
        db::dashboard::expenses_by_month(pool, company_id, months),
        db::dashboard::category_breakdown(pool, company_id, months),
    );

    let rev_months = rev_res.unwrap_or_default();
    let exp_months = exp_res.unwrap_or_default();
    let cats = cats_res.unwrap_or_default();

    let total_rev: rust_decimal::Decimal = rev_months.iter().map(|m| m.amount).sum();
    let total_exp: rust_decimal::Decimal = exp_months.iter().map(|m| m.amount).sum();
    let profit = total_rev - total_exp;
    let margin = if total_rev > rust_decimal::Decimal::ZERO {
        (profit / total_rev * rust_decimal::Decimal::from(100)).round_dp(1)
    } else {
        rust_decimal::Decimal::ZERO
    };

    use rust_decimal::prelude::ToPrimitive;
    let metrics = crate::ReportMetrics {
        revenue:        total_rev.to_f32().unwrap_or(0.0),
        expenses:       total_exp.to_f32().unwrap_or(0.0),
        profit:         profit.to_f32().unwrap_or(0.0),
        margin:         margin.to_f32().unwrap_or(0.0),
        delta_revenue:  "".into(),
        delta_expenses: "".into(),
        delta_profit:   "".into(),
        delta_margin:   "".into(),
    };

    ReportsData {
        revenue_str:  format_amount(total_rev),
        expenses_str: format_amount(total_exp),
        profit_str:   format_amount(profit),
        margin_str:   format!("{margin:.1}%"),
        metrics,
        chart_bars:   build_chart_bars(&rev_months, &exp_months),
        categories:   build_expense_categories(&cats),
        drill_rows:   vec![],
    }
}

pub fn apply_reports_to_ui(ui: &crate::AppWindow, data: ReportsData) {
    ui.set_rep_metrics(data.metrics);
    ui.set_rep_chart_bars(ModelRc::new(VecModel::from(data.chart_bars)));
    ui.set_rep_categories(ModelRc::new(VecModel::from(data.categories)));
    ui.set_rep_drill_rows(ModelRc::new(VecModel::from(data.drill_rows)));
    ui.set_rep_revenue_str(data.revenue_str.into());
    ui.set_rep_expenses_str(data.expenses_str.into());
    ui.set_rep_profit_str(data.profit_str.into());
    ui.set_rep_margin_str(data.margin_str.into());
}

pub fn wire_reports_callbacks(
    ui: &crate::AppWindow,
    pool: &Arc<PgPool>,
    company_id: &Arc<Mutex<Uuid>>,
) {
    use slint::ComponentHandle;

    ui.on_rep_period_changed({
        let pool = pool.clone();
        let ui_weak = ui.as_weak();
        let company_id = company_id.clone();
        move |period| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let cid = *company_id.lock().unwrap();
            tokio::spawn(async move {
                let data = prepare_reports_data(&pool, cid, period).await;
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    apply_reports_to_ui(&ui, data);
                });
            });
        }
    });

    ui.on_rep_category_drilled(|_| {
        tracing::debug!("rep_category_drilled: not yet implemented");
    });
    ui.on_rep_export_csv(|| tracing::info!("rep_export_csv: not implemented"));
    ui.on_rep_export_pdf(|| tracing::info!("rep_export_pdf: not implemented"));
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn build_chart_bars(
    rev: &[acta::models::dashboard::MonthRevenue],
    exp: &[acta::models::dashboard::MonthRevenue],
) -> Vec<crate::ChartBar> {
    use rust_decimal::prelude::ToPrimitive;

    let max_val = rev.iter().map(|m| m.amount)
        .chain(exp.iter().map(|m| m.amount))
        .map(|a| a.to_f64().unwrap_or(0.0))
        .fold(0.0f64, f64::max);

    let norm = |a: rust_decimal::Decimal| -> f32 {
        let v = a.to_f64().unwrap_or(0.0);
        if max_val > 0.0 { (v / max_val) as f32 } else { 0.0 }
    };

    let n = rev.len().max(exp.len());
    (0..n).map(|i| {
        let r = rev.get(i);
        let e = exp.get(i);
        crate::ChartBar {
            rev_h: r.map(|m| norm(m.amount)).unwrap_or(0.0),
            exp_h: e.map(|m| norm(m.amount)).unwrap_or(0.0),
            month: r.map(|m| m.month_label().to_string())
                    .or_else(|| e.map(|m| m.month_label().to_string()))
                    .unwrap_or_default()
                    .into(),
        }
    }).collect()
}

pub fn build_expense_categories(cats: &[CategoryRevenue]) -> Vec<crate::ExpenseCategory> {
    use rust_decimal::prelude::ToPrimitive;
    let total: rust_decimal::Decimal = cats.iter().map(|c| c.amount).sum();

    cats.iter().map(|c| {
        let pct = if total > rust_decimal::Decimal::ZERO {
            ((c.amount / total) * rust_decimal::Decimal::from(100))
                .to_i32().unwrap_or(0)
        } else { 0 };
        crate::ExpenseCategory {
            label:   c.label.clone().into(),
            amount:  c.amount.to_f32().unwrap_or(0.0),
            percent: pct,
        }
    }).collect()
}

fn format_amount(amt: rust_decimal::Decimal) -> String {
    use rust_decimal::prelude::ToPrimitive;
    let val = amt.to_f64().unwrap_or(0.0);
    if val == 0.0 { return "0 ₴".to_string(); }
    let s = format!("{:.0}", val.abs());
    let digits: Vec<char> = s.chars().collect();
    let mut result = String::new();
    let len = digits.len();
    for (i, d) in digits.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 { result.push('\u{00A0}'); }
        result.push(*d);
    }
    if val < 0.0 { format!("−{result} ₴") } else { format!("{result} ₴") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_to_months_maps_correctly() {
        assert_eq!(period_to_months(0), 1);
        assert_eq!(period_to_months(1), 3);
        assert_eq!(period_to_months(2), 12);
        assert_eq!(period_to_months(3), 3);
        assert_eq!(period_to_months(99), 3);
    }

    #[test]
    fn build_expense_categories_normalizes_percent() {
        use rust_decimal_macros::dec;
        let cats = vec![
            CategoryRevenue { label: "А".into(), amount: dec!(750) },
            CategoryRevenue { label: "Б".into(), amount: dec!(250) },
        ];
        let result = build_expense_categories(&cats);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].percent, 75);
        assert_eq!(result[1].percent, 25);
    }

    #[test]
    fn format_amount_zero_returns_zero_hryvnia() {
        assert_eq!(format_amount(rust_decimal::Decimal::ZERO), "0 ₴");
    }

    #[test]
    fn format_amount_negative_starts_with_minus() {
        use rust_decimal_macros::dec;
        let s = format_amount(dec!(-5000));
        assert!(s.starts_with('−'), "очікується від'ємний знак: {s}");
    }
}
```

**Примітка:** `db::dashboard::revenue_by_month`, `expenses_by_month`, `category_breakdown` та `acta::models::dashboard::{CategoryRevenue, MonthRevenue}` мають бути вже визначені. Якщо `expenses_by_month` або `category_breakdown` відсутні — реалізувати їх в `src/db/dashboard.rs` за зразком з `docs/superpowers/plans/2026-04-21-remaining-features.md` (Tasks 5, Steps 4–5).

- [ ] **Step 5: Запустити тести**

```
cargo test ui::reports 2>&1 | tail -15
```
Очікується: всі тести PASS.

- [ ] **Step 6: Commit**

```bash
git add src/ui/reports.rs src/ui/mod.rs
git commit -m "feat: add reports UI module — prepare/apply/wire pattern"
```

---

## Task 10: main.rs — завершення wiring + cargo build

**Файли:**
- Modify: `src/main.rs`

- [ ] **Step 1: Оновити початковий `tokio::join!` — додати reports і settings**

Знайти блок:
```rust
    let (dash_data, doc_data, cp_data, pay_data, task_data) = rt.block_on(async {
        tokio::join!(
            ui::dashboard::prepare_dashboard_data(&pool, company_id),
            ui::documents::prepare_documents_data(&pool, company_id, None, None),
            ui::counterparties::prepare_counterparties_data(&pool, company_id, None),
            ui::payments::prepare_payments_data(&pool, company_id),
            ui::tasks::prepare_tasks_data(&pool),
        )
    });
```

Замінити на:
```rust
    let (dash_data, doc_data, cp_data, pay_data, task_data, rep_data, set_data) = rt.block_on(async {
        tokio::join!(
            ui::dashboard::prepare_dashboard_data(&pool, company_id),
            ui::documents::prepare_documents_data(&pool, company_id, None, None),
            ui::counterparties::prepare_counterparties_data(&pool, company_id, None),
            ui::payments::prepare_payments_data(&pool, company_id),
            ui::tasks::prepare_tasks_data(&pool),
            ui::reports::prepare_reports_data(&pool, company_id, 1),
            ui::settings::prepare_settings_data(&pool, company_id),
        )
    });
```

- [ ] **Step 2: Оновити apply calls — замінити settings stub**

Знайти:
```rust
    ui::settings::apply_settings_to_ui(&ui);
```
Замінити на:
```rust
    ui::reports::apply_reports_to_ui(&ui, rep_data);
    ui::settings::apply_settings_to_ui(&ui, set_data);
```

- [ ] **Step 3: Додати Reports і Settings до `on_nav_changed`**

Знайти блок `NavScreen::Tasks => {` в `on_nav_changed`. Після його закриваючого `}` (але перед `_ => {}`) додати:

```rust
                    NavScreen::Reports => {
                        let data = ui::reports::prepare_reports_data(&pool, cid, 1).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::reports::apply_reports_to_ui(&ui, data);
                        });
                    }
                    NavScreen::Settings => {
                        let data = ui::settings::prepare_settings_data(&pool, cid).await;
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui::settings::apply_settings_to_ui(&ui, data);
                        });
                    }
```

- [ ] **Step 4: Замінити заглушки Reports і Settings на wire функції**

Знайти блок `// ── Заглушки для нереалізованих callback'ів ────────────────────────────────`:

Видалити ці рядки:
```rust
    ui.on_rep_period_changed(|_| {});
    ui.on_rep_category_drilled(|_| {});
    ui.on_rep_export_csv(|| {});
    ui.on_rep_export_pdf(|| {});
    ui.on_settings_section_changed(|_| {});
    ui.on_settings_dark_mode_toggled(|_| {});
    ui.on_settings_density_changed(|_| {});
    ui.on_settings_company_saved(|_| {});
    ui.on_settings_integration_configure(|_| {});
    ui.on_settings_team_invite(|| {});
    ui.on_settings_backup_now(|| {});
    ui.on_settings_backup_download(|| {});
```

Замінити додавши поряд з іншими wire функціями:
```rust
    // ── Звіти ────────────────────────────────────────────────────────────────────
    ui::reports::wire_reports_callbacks(&ui, &pool, &active_company_id);

    // ── Налаштування ─────────────────────────────────────────────────────────────
    ui::settings::wire_settings_callbacks(&ui, &pool, &active_company_id);
```

- [ ] **Step 5: Додати stub callbacks для Inbox і DocChain**

Після блоку оплат (де `ui.on_pay_import_csv(|| {});` тощо) додати:

```rust
    // ── Inbox + DocChain (MVP stubs) ──────────────────────────────────────────────
    ui.on_inbox_action(|id, kind| {
        tracing::info!("inbox_action: doc={id} kind={kind} (not yet implemented)");
    });
    ui.on_doc_chain_load(|_id| {
        // Rust може завантажити chain steps і передати через set_doc_chain_steps
        // MVP: залишити порожнім — chain показується без даних
    });
    ui.on_doc_chain_create(|doc_type, source_id| {
        tracing::info!("doc_chain_create: type={doc_type} source={source_id} (not yet implemented)");
    });
```

Також додати stub для `cp-doc-chains` (поки Rust не формує ланцюжки для контрагентів):
```rust
    ui.set_cp_doc_chains(ModelRc::new(VecModel::<crate::DocChainGroup>::default()));
```

- [ ] **Step 6: Запустити cargo build**

```
cargo build 2>&1
```
Очікується: 0 errors. Можливі warnings — ок. Якщо є errors — виправити за повідомленнями компілятора.

Типові помилки і виправлення:
- `no method found set_dash_inbox` → перевірити що `dash-inbox` додано до AppWindow в app.slint (Task 6 Step 3)
- `expected ChartBar, found anonymous struct` → перевірити що chart-bars типи оновлені в app.slint і dashboard.slint (Task 6 Step 2, 8)
- `apply_settings_to_ui takes 1 argument` → знайти інші виклики старої сигнатури і оновити

- [ ] **Step 7: Запустити всі тести**

```
cargo test 2>&1 | tail -20
```
Очікується: всі тести PASS.

- [ ] **Step 8: Commit**

```bash
git add src/main.rs
git commit -m "feat: complete main.rs wiring — reports, settings, inbox, doc-chain callbacks"
```

---

## Self-Review

**Spec coverage:**
- ✅ InboxItem struct → types.slint (Task 1)
- ✅ ChainStep + DocChainGroup structs → types.slint (Task 1)
- ✅ ChartBar named type → types.slint (Task 1) + app.slint/dashboard.slint (Task 6)
- ✅ DocChain component → components.slint (Task 2)
- ✅ Dashboard mode toggle [Огляд][Вхідні] → dashboard.slint (Task 3)
- ✅ InboxView component → dashboard.slint (Task 3)
- ✅ DocChain accordion в Documents → documents.slint (Task 4)
- ✅ DocChain groups в Counterparties docs tab → counterparties.slint (Task 5)
- ✅ Нові props/callbacks в app.slint (Task 6)
- ✅ inbox_items DB query → src/db/dashboard.rs (Task 7)
- ✅ DashboardData.inbox + apply_dashboard_to_ui → src/ui/dashboard.rs (Task 7)
- ✅ Settings з DB loading + wire_settings_callbacks → src/ui/settings.rs (Task 8)
- ✅ Reports module → src/ui/reports.rs (Task 9)
- ✅ NavScreen::Reports + NavScreen::Settings у on_nav_changed → src/main.rs (Task 10)
- ✅ inbox-action + doc-chain-load + doc-chain-create callbacks → src/main.rs (Task 10)

**Type consistency:**
- `crate::InboxItem` — визначений у types.slint (Task 1), використаний у dashboard.rs (Task 7), wired у main.rs (Task 10) ✅
- `crate::ChartBar` — визначений у types.slint (Task 1), app.slint оновлено (Task 6), `set_rep_chart_bars`/`set_dash_chart_bars` у reports.rs/dashboard.rs ✅
- `SettingsData` — визначений у settings.rs (Task 8), `apply_settings_to_ui(&ui, data)` сигнатура оновлена скрізь ✅
- `DocChainGroup` — визначений у types.slint (Task 1), використаний у counterparties.slint (Task 5), `cp-doc-chains` у app.slint (Task 6), stub у main.rs (Task 10) ✅
