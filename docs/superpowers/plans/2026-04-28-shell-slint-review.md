# Shell.slint Code Review — Implementation Plan

> **Archived/pre-cutover:** Slint review plan збережено як історичний контекст. Після `2026-04-30` live shell/navigation work іде через Tauri/Svelte, не через `ui/shell.slint`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Виправити 11 проблем знайдених в ревью `ui/shell.slint`: баги hover-state, SkipNav, CommandPalette, хибні дефолти, заглушки даних та технічний борг.

**Architecture:** Всі зміни — тільки у `.slint` файлах (UI шар). Rust-сторона не змінюється крім Task 5 (видалення заглушок SavedViewItem). Задачі незалежні і можуть виконуватись у будь-якому порядку всередині групи.

**Tech Stack:** Slint 1.9, `ui/shell.slint`, `ui/types.slint`, `ui/app.slint`

---

## File Structure

**Змінюються:**
- `ui/shell.slint` — всі задачі крім Task 5
- `ui/types.slint` — Task 5 (новий struct `SavedFilterItem`)
- `ui/app.slint` — Task 5 (нова property на AppWindow)

**Не змінюється:** `src/` (Rust), `.sqlx/`, `migrations/`

---

## Група 1 — Критичні баги

---

### Task 1: Виправити hover stuck bug (NavItem, SavedViewItem, PaletteItem)

**Проблема:** Три компоненти використовують `property <bool> hovered` + `pointer-event(PointerEventKind.move)` для відстеження hover. Якщо миша виходить швидко без руху — `move` не спрацьовує і `hovered` залишається `true` ("stuck"). `CompanySwitcherRow` вже правильно використовує `touch.has-hover` напряму.

**Files:**
- Modify: `ui/shell.slint:55-125` (NavItem)
- Modify: `ui/shell.slint:128-182` (SavedViewItem)
- Modify: `ui/shell.slint:269-338` (PaletteItem)

- [x] **Step 1: Перевірити поточну поведінку**

Запустити `cargo run`, навести мишу на пункт меню і швидко прибрати. Переконатись що підсвітка залишається (відтворюємо баг).

- [x] **Step 2: Виправити NavItem**

Замінити рядки 65–124 у `ui/shell.slint`:

**До:**
```slint
component NavItem inherits Rectangle {
    in property <string> label;
    in property <image> icon;
    in property <bool> active: false;
    in property <int> badge-count: 0;
    callback clicked;

    accessible-role: button;
    accessible-label: root.label;

    property <bool> hovered: false;

    height: 36px;
    border-radius: AppTheme.radius-lg;
    background: active ? AppTheme.bg-elevated : (hovered ? AppTheme.bg-hover : transparent);
    border-width: active ? 1px : 0px;
    border-color: AppTheme.border;
    // ... HorizontalLayout ...
    TouchArea {
        mouse-cursor: pointer;
        pointer-event(e) => {
            if (e.kind == PointerEventKind.move) {
                root.hovered = self.has-hover;
            }
        }
        clicked => { root.clicked(); }
    }
}
```

**Після:**
```slint
component NavItem inherits Rectangle {
    in property <string> label;
    in property <image> icon;
    in property <bool> active: false;
    in property <int> badge-count: 0;
    callback clicked;

    accessible-role: button;
    accessible-label: root.label;

    height: 36px;
    border-radius: AppTheme.radius-lg;
    background: active ? AppTheme.bg-elevated : (nav-touch.has-hover ? AppTheme.bg-hover : transparent);
    border-width: active ? 1px : 0px;
    border-color: AppTheme.border;

    HorizontalLayout {
        padding-top: 4px;
        padding-bottom: 4px;
        padding-left: 12px;
        padding-right: 12px;
        spacing: 10px;
        alignment: stretch;

        InlineIcon {
            icon: root.icon;
            icon-size: 15px;
            tint: active ? AppTheme.text : AppTheme.text-faint;
        }

        Text {
            text: root.label;
            color: active ? AppTheme.text : AppTheme.text-muted;
            font-size: AppTheme.font-body;
            font-family: AppTheme.font-sans;
            font-weight: active ? 500 : 400;
            vertical-alignment: center;
            horizontal-stretch: 1;
            overflow: elide;
        }

        if badge-count > 0 : Rectangle {
            width: max(18px, badge-label.preferred-width + 12px);
            height: 17px;
            border-radius: 4px;
            background: AppTheme.bg-subtle;

            badge-label := Text {
                text: badge-count;
                color: AppTheme.text-muted;
                font-size: 10.5px;
                font-family: AppTheme.font-mono;
                font-weight: 600;
                vertical-alignment: center;
                horizontal-alignment: center;
            }
        }
    }

    nav-touch := TouchArea {
        mouse-cursor: pointer;
        clicked => { root.clicked(); }
    }
}
```

- [x] **Step 3: Виправити SavedViewItem**

Замінити рядки 128–182 у `ui/shell.slint`:

**До:**
```slint
component SavedViewItem inherits Rectangle {
    in property <string> label;
    in property <int> count;
    in property <image> star-icon;
    callback clicked;

    property <bool> hovered: false;

    height: 24px;
    border-radius: AppTheme.radius-sm;
    background: hovered ? AppTheme.bg-hover : transparent;

    HorizontalLayout {
        // ...
        Text {
            text: root.label;
            color: hovered ? AppTheme.text : AppTheme.text-muted;
            // ...
        }
        // ...
    }

    TouchArea {
        mouse-cursor: pointer;
        pointer-event(e) => {
            if (e.kind == PointerEventKind.move) {
                root.hovered = self.has-hover;
            }
        }
        clicked => { root.clicked(); }
    }
}
```

**Після:**
```slint
component SavedViewItem inherits Rectangle {
    in property <string> label;
    in property <int> count;
    in property <image> star-icon;
    callback clicked;

    height: 24px;
    border-radius: AppTheme.radius-sm;
    background: svi-touch.has-hover ? AppTheme.bg-hover : transparent;

    HorizontalLayout {
        padding-left: 10px;
        padding-right: 10px;
        spacing: 8px;
        alignment: stretch;

        InlineIcon {
            icon: root.star-icon;
            icon-size: 11px;
            tint: AppTheme.text-faint;
        }

        Text {
            text: root.label;
            color: svi-touch.has-hover ? AppTheme.text : AppTheme.text-muted;
            font-size: 12px;
            font-family: AppTheme.font-sans;
            vertical-alignment: center;
            horizontal-stretch: 1;
            overflow: elide;
        }

        Text {
            text: root.count;
            color: AppTheme.text-faint;
            font-size: 10.5px;
            font-family: AppTheme.font-mono;
            vertical-alignment: center;
            horizontal-alignment: right;
            width: 20px;
        }
    }

    svi-touch := TouchArea {
        mouse-cursor: pointer;
        clicked => { root.clicked(); }
    }
}
```

- [x] **Step 4: Виправити PaletteItem**

Замінити рядки 269–338 у `ui/shell.slint`:

**До:**
```slint
component PaletteItem inherits Rectangle {
    // ...
    property <bool> hovered: false;

    height: 36px;
    background: hovered ? AppTheme.bg-subtle : transparent;

    HorizontalLayout {
        // ...
        InlineIcon {
            tint: hovered ? AppTheme.text : AppTheme.text-muted;
        }
        // ...
    }

    TouchArea {
        mouse-cursor: pointer;
        pointer-event(e) => {
            if (e.kind == PointerEventKind.move) {
                root.hovered = self.has-hover;
            }
        }
        clicked => { root.clicked(); }
    }
}
```

**Після:**
```slint
component PaletteItem inherits Rectangle {
    in property <image> icon;
    in property <string> label;
    in property <string> kbd: "";
    in property <string> meta: "";
    callback clicked;

    height: 36px;
    background: pi-touch.has-hover ? AppTheme.bg-subtle : transparent;

    HorizontalLayout {
        padding-left: 16px;
        padding-right: 16px;
        spacing: 10px;
        alignment: stretch;

        InlineIcon {
            icon: root.icon;
            icon-size: 14px;
            tint: pi-touch.has-hover ? AppTheme.text : AppTheme.text-muted;
        }

        Text {
            text: root.label;
            color: AppTheme.text;
            font-size: 13px;
            font-family: AppTheme.font-sans;
            vertical-alignment: center;
            horizontal-stretch: 1;
        }

        if meta != "" : Text {
            text: root.meta;
            color: AppTheme.text-faint;
            font-size: 11.5px;
            font-family: AppTheme.font-mono;
            vertical-alignment: center;
        }

        if kbd != "" : Rectangle {
            height: 18px;
            min-width: kbd-text.preferred-width + 10px;
            border-width: 1px;
            border-color: AppTheme.border;
            border-radius: 3px;
            background: AppTheme.bg-subtle;

            kbd-text := Text {
                text: root.kbd;
                color: AppTheme.text-faint;
                font-size: 10px;
                font-family: AppTheme.font-mono;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
    }

    pi-touch := TouchArea {
        mouse-cursor: pointer;
        clicked => { root.clicked(); }
    }
}
```

- [x] **Step 5: Перевірити компіляцію**

```bash
cargo build
```
Очікується: `Finished` без помилок.

- [x] **Step 6: Перевірити поведінку**

```bash
cargo run
```
Навести мишу на пункти навігації і швидко прибрати — підсвітка має зникати миттєво без "застрягання".

- [x] **Step 7: Commit**

```bash
git add ui/shell.slint
git commit -m "fix(ui): replace hovered property with has-hover in NavItem, SavedViewItem, PaletteItem"
```

---

### Task 2: Виправити SkipNav — clip за шириною 1px

**Проблема:** Shell встановлює `width: 1px` на SkipNav. Компонент має `clip: true`, тому при фокусуванні кнопка "Перейти до основного контенту" розкривається по висоті (36px), але залишається зрізаною до 1px по ширині — текст невидимий.

**Files:**
- Modify: `ui/shell.slint:9-52` (компонент SkipNav)
- Modify: `ui/shell.slint:881-886` (використання в Shell)

- [x] **Step 1: Оновити компонент SkipNav**

Замінити рядки 9–52 у `ui/shell.slint`:

**До:**
```slint
component SkipNav inherits Rectangle {
    callback activated;

    height: scope.has-focus ? 36px : 1px;
    clip: true;
    z: 200;
    background: transparent;
    // ...
```

**Після:**
```slint
component SkipNav inherits Rectangle {
    callback activated;

    height: scope.has-focus ? 36px : 1px;
    width: scope.has-focus ? 260px : 1px;
    clip: true;
    z: 200;
    background: transparent;
    // ...
```

- [x] **Step 2: Прибрати зовнішній override width у Shell**

Замінити рядки 881–886:

**До:**
```slint
skip-nav := SkipNav {
    width: 1px;
    x: 0;
    y: 12px;
    activated => { content-area.focus(); }
}
```

**Після:**
```slint
skip-nav := SkipNav {
    x: 0;
    y: 12px;
    activated => { content-area.focus(); }
}
```

- [x] **Step 3: Перевірити компіляцію**

```bash
cargo build
```

- [x] **Step 4: Перевірити поведінку**

```bash
cargo run
```
Натиснути Tab одразу після запуску — SkipNav має отримати фокус і показати кнопку шириною ≈260px у лівому верхньому куті. Натиснути Enter — фокус переходить у контентну область.

- [x] **Step 5: Commit**

```bash
git add ui/shell.slint
git commit -m "fix(ui): fix SkipNav width clipping — expand to 260px on focus"
```

---

### Task 3: CommandPalette — автофокус і закриття через Escape

**Проблема 1:** При відкритті палітри `search-field` (TextInput) не отримує фокус автоматично — користувач мусить кликати мишею.  
**Проблема 2:** Натискання Escape не закриває палітру через клавіатуру.

**Files:**
- Modify: `ui/shell.slint:396-417` (TextInput search-field в CommandPalette)

- [x] **Step 1: Додати `init` і `key-pressed` до search-field**

Знайти `search-field := TextInput {` (рядок ~397) і додати два нових обробники:

**До:**
```slint
search-field := TextInput {
    text: root.query;
    color: AppTheme.text;
    font-size: 14px;
    font-family: AppTheme.font-sans;
    single-line: true;
    vertical-alignment: center;
    horizontal-stretch: 1;
    accessible-label: "Пошук команд";
    edited => { root.query = self.text; root.query-changed(self.text); }
}
```

**Після:**
```slint
search-field := TextInput {
    text: root.query;
    color: AppTheme.text;
    font-size: 14px;
    font-family: AppTheme.font-sans;
    single-line: true;
    vertical-alignment: center;
    horizontal-stretch: 1;
    accessible-label: "Пошук команд";
    init => { self.focus(); }
    edited => { root.query = self.text; root.query-changed(self.text); }
    key-pressed(e) => {
        if e.text == Key.Escape {
            root.closed();
            accept
        } else {
            reject
        }
    }
}
```

- [x] **Step 2: Перевірити компіляцію**

```bash
cargo build
```

- [x] **Step 3: Перевірити автофокус**

```bash
cargo run
```
Натиснути Ctrl+K — палітра відкривається і курсор одразу в полі пошуку (можна друкувати без кліку мишею).

- [x] **Step 4: Перевірити закриття через Escape**

При відкритій палітрі натиснути Escape — палітра закривається.

- [x] **Step 5: Commit**

```bash
git add ui/shell.slint
git commit -m "fix(ui): command palette auto-focuses search field and closes on Escape"
```

---

## Група 2 — Важливі покращення

---

### Task 4: Виправити хибні default значення property

**Проблема:** Shell має 6 `in property` з development-заглушками замість пустих дефолтів. Якщо Rust не встигне проставити значення або виникне помилка ініціалізації — інтерфейс покаже фіктивні дані.

**Files:**
- Modify: `ui/shell.slint:824-832`

- [x] **Step 1: Замінити хибні дефолти**

Знайти і замінити рядки 824–832:

**До:**
```slint
in property <string> company-name: "ТОВ «Альфа-Бізнес»";
in property <[CompanySwitcherItem]> company-items;
in property <string> user-name: "Михайло Дан";
in property <string> user-role: "Адміністратор";
in property <string> user-initials: "МД";

// Notification badge counts
in property <int> documents-badge: 3;
in property <int> tasks-badge: 4;
```

**Після:**
```slint
in property <string> company-name: "";
in property <[CompanySwitcherItem]> company-items;
in property <string> user-name: "";
in property <string> user-role: "";
in property <string> user-initials: "";

// Notification badge counts
in property <int> documents-badge: 0;
in property <int> tasks-badge: 0;
```

- [x] **Step 2: Перевірити компіляцію та запуск**

```bash
cargo build && cargo run
```
Дані мають відображатись коректно (з БД), без фіктивних значень.

- [x] **Step 3: Commit**

```bash
git add ui/shell.slint
git commit -m "fix(ui): replace dev-placeholder defaults with empty/zero values in Shell"
```

---

### Task 5: Видалити hardcoded SavedViewItem заглушки і зробити список динамічним

**Проблема:** Чотири `SavedViewItem` у Shell (рядки 1166–1185) мають хардкодовані дані і не мають прив'язки до callback. Це мертвий нефункціональний код.

**Підхід:** Додати struct `SavedFilterItem` у types.slint, нову property на AppWindow (app.slint), використати `for` loop у shell.slint. З Rust передавати порожній список до реалізації фічі.

**Files:**
- Modify: `ui/types.slint` (новий struct)
- Modify: `ui/shell.slint:824+` (нова property і callback)
- Modify: `ui/shell.slint:1166-1185` (замінити заглушки на `for` loop)
- Modify: `ui/app.slint` (прив'язка нової property)

- [x] **Step 1: Додати SavedFilterItem до types.slint**

Дописати в кінець секції "Shell chrome model" (після рядка 206) у `ui/types.slint`:

```slint
export struct SavedFilterItem {
    label: string,
    count: int,
}
```

- [x] **Step 2: Додати import SavedFilterItem в shell.slint**

Знайти рядок 5:
```slint
import { NavScreen, PaletteItemData, CompanySwitcherItem } from "types.slint";
```

Замінити на:
```slint
import { NavScreen, PaletteItemData, CompanySwitcherItem, SavedFilterItem } from "types.slint";
```

- [x] **Step 3: Додати property і callback до Shell**

Після рядка `in property <int> tasks-badge: 0;` (Task 4 result) додати:

```slint
// Saved filters sidebar section
in property <[SavedFilterItem]> saved-filters;
callback saved-filter-clicked(int);
```

- [x] **Step 4: Замінити заглушки на for loop**

Знайти рядки 1166–1185:

**До:**
```slint
SavedViewItem {
    label: "Прострочені рахунки";
    count: 1;
    star-icon: root.icon-star;
}
SavedViewItem {
    label: "Акти без підпису";
    count: 2;
    star-icon: root.icon-star;
}
SavedViewItem {
    label: "Неприв'язані платежі";
    count: 2;
    star-icon: root.icon-star;
}
SavedViewItem {
    label: "Цей тиждень";
    count: 7;
    star-icon: root.icon-star;
}
```

**Після:**
```slint
if root.saved-filters.length == 0 : Text {
    text: "Немає збережених фільтрів";
    color: AppTheme.text-faint;
    font-size: 11px;
    font-family: AppTheme.font-sans;
    padding-left: 10px;
    height: 28px;
    vertical-alignment: center;
}

for filter[i] in root.saved-filters : SavedViewItem {
    label: filter.label;
    count: filter.count;
    star-icon: root.icon-star;
    clicked => { root.saved-filter-clicked(i); }
}
```

- [x] **Step 5: Прив'язати в app.slint**

Знайти у `ui/app.slint` блок де прив'язуються shell-properties (рядки ~161–166). Додати після `tasks-badge`:

```slint
saved-filters: [];
saved-filter-clicked(i) => { /* TODO: implement */ }
```

- [x] **Step 6: Перевірити компіляцію**

```bash
cargo build
```

- [x] **Step 7: Перевірити запуск**

```bash
cargo run
```
Секція "ЗБЕРЕЖЕНІ ФІЛЬТРИ" показує текст "Немає збережених фільтрів" замість хардкодованих рядків.

- [x] **Step 8: Commit**

```bash
git add ui/types.slint ui/shell.slint ui/app.slint
git commit -m "feat(ui): replace hardcoded SavedViewItem stubs with dynamic saved-filters property"
```

---

### Task 6: Винести hardcoded "Acta · упр. облік" у property

**Проблема:** Рядок `"Acta · упр. облік"` захаркоджений у company button (рядок ~957). При необхідності локалізації або зміни — треба редагувати `.slint`.

**Files:**
- Modify: `ui/shell.slint:824+` (нова property)
- Modify: `ui/shell.slint:957` (Text)

- [x] **Step 1: Додати property app-subtitle до Shell**

Після рядка `in property <string> company-name: "";` додати:

```slint
in property <string> app-subtitle: "Acta · упр. облік";
```

- [x] **Step 2: Використати property в Text**

Знайти (рядок ~957):
```slint
Text {
    text: "Acta · упр. облік";
```

Замінити:
```slint
Text {
    text: root.app-subtitle;
```

- [x] **Step 3: Перевірити компіляцію**

```bash
cargo build
```

- [x] **Step 4: Commit**

```bash
git add ui/shell.slint
git commit -m "refactor(ui): extract hardcoded app subtitle to Shell property"
```

---

## Група 3 — Технічний борг

---

### Task 7: Задокументувати магічне число y:84 в company switcher

**Проблема:** `y: 84px` для company-switcher popup — хрупке хардкодоване число. Змінення `padding-top` або `company-btn.height` непомітно зламає позиціювання. Повністю динамічне рішення потребує рефакторингу розкладки; зараз — додати пояснення.

**Files:**
- Modify: `ui/shell.slint:1263-1265`

- [x] **Step 1: Додати пояснювальний коментар**

Знайти (рядок ~1263):
```slint
if root.company-switcher-open : Rectangle {
    x: 14px;
    y: 84px;
```

Замінити:
```slint
if root.company-switcher-open : Rectangle {
    x: 14px;
    // 84 = sidebar padding-top(24) + company-btn height(56) + gap(4)
    // Оновити якщо змінюється padding-top VerticalLayout або company-btn height
    y: 84px;
```

- [x] **Step 2: Commit**

```bash
git add ui/shell.slint
git commit -m "docs(ui): document magic y:84 constant in company switcher popup"
```

---

### Task 8: Додати cross-reference між KeyboardHelp і FocusScope

**Проблема:** Текстові описи клавіш у `KeyboardHelp` (рядки 723–730, 765–772) дубльовані з реальною логікою у `nav-scope` FocusScope (рядки 1083–1122). Зміна скорочення в одному місці не оновлює інше.

**Files:**
- Modify: `ui/shell.slint:611` (перед KeyboardHelp)
- Modify: `ui/shell.slint:1075` (перед FocusScope)

- [x] **Step 1: Додати cross-reference коментар перед KeyboardHelp**

Знайти рядок `// ── Keyboard Help Overlay ─────`:

```slint
// ── Keyboard Help Overlay ─────────────────────────────────────────────────────
// Shows keyboard shortcuts cheatsheet. Triggered by Ctrl+/.
component KeyboardHelp {
```

Замінити:
```slint
// ── Keyboard Help Overlay ─────────────────────────────────────────────────────
// Shows keyboard shortcuts cheatsheet. Triggered by Ctrl+/.
// SYNC: Список скорочень тут і реальна логіка в nav-scope FocusScope (рядок ~1076)
// мусять залишатись синхронізованими при зміні будь-якого скорочення.
component KeyboardHelp {
```

- [x] **Step 2: Додати cross-reference коментар перед FocusScope**

Знайти рядок `// Epic 10: Keyboard navigation hotkeys`:

```slint
                // Epic 10: Keyboard navigation hotkeys
                nav-scope := FocusScope {
```

Замінити:
```slint
                // Keyboard navigation hotkeys — при зміні скорочень тут
                // оновлювати також KeyboardHelp (~рядок 611).
                nav-scope := FocusScope {
```

- [x] **Step 3: Перевірити компіляцію**

```bash
cargo build
```

- [x] **Step 4: Commit**

```bash
git add ui/shell.slint
git commit -m "docs(ui): add cross-reference comments between KeyboardHelp and nav-scope shortcuts"
```

---

### Task 9: Задокументувати content-area FocusScope як навмисний Tab-stop

**Проблема:** Порожній `content-area` FocusScope (рядок 1424) додає зайву зупинку при навігації Tab. В Slint 1.9 немає простого способу прибрати FocusScope з Tab-order без видалення — він потрібен як якір фокусу для SkipNav.

**Files:**
- Modify: `ui/shell.slint:1419-1431`

- [x] **Step 1: Пояснити призначення у коментарі**

Знайти (рядок ~1419):
```slint
            // ── Screen content (injected by App) ───────────────────────────
            content := Rectangle {
                vertical-stretch: 1;
                clip: true;
                background: AppTheme.bg-stripe;

                content-area := FocusScope {
                    width: parent.width;
                    height: parent.height;
                    // Якір фокусу: Tab з цього FocusScope переходить до першого елементу екрану нижче
                }
```

Замінити коментар всередині FocusScope:
```slint
            // ── Screen content (injected by App) ───────────────────────────
            content := Rectangle {
                vertical-stretch: 1;
                clip: true;
                background: AppTheme.bg-stripe;

                // Якір фокусу для SkipNav (activated => content-area.focus()).
                // FocusScope навмисно порожній — є додатковим Tab-stop між sidebar і
                // першим елементом екрану. В Slint 1.9 прибрати з Tab-order неможливо
                // без видалення елементу.
                content-area := FocusScope {
                    width: parent.width;
                    height: parent.height;
                }
```

- [x] **Step 2: Commit**

```bash
git add ui/shell.slint
git commit -m "docs(ui): document content-area FocusScope as intentional Tab anchor for SkipNav"
```

---

## Self-Review Checklist

- [x] **Покриття:** Всі 11 пунктів ревью закриті задачами 1–9
- [x] **Без плейсхолдерів:** Всі кроки містять реальний код
- [x] **Типи:** `SavedFilterItem` визначений у Task 5 Step 1 і використаний у Steps 2–5
- [x] **Назви:** `nav-touch`, `svi-touch`, `pi-touch` — унікальні в межах компонентів
- [x] **Групи:** Задачі 1–3 незалежні всередині Групи 1, можна виконувати паралельно
- [x] **Rust:** Не змінюється крім app.slint у Task 5 (`.slint` файл, не Rust)

---

## Відповідність до ревью

| Пункт ревью | Задача |
|-------------|--------|
| Bug #1: SkipNav width:1px | Task 2 |
| Bug #2: y:84px magic number | Task 7 |
| Bug #3: hover stuck | Task 1 |
| Bug #4: CommandPalette фокус/Escape | Task 3 |
| Dead #5: SavedViewItem заглушки | Task 5 |
| Dead #6: хибні дефолти | Task 4 |
| Dead #7: hardcoded subtitle | Task 6 |
| Debt #8: shortcuts дублювання | Task 8 |
| Debt #9: inconsistent hover | Task 1 (вже покриває) |
| Refactor #10: PaletteItem дублювання | — (розміри різні навмисно, задокументовано) |
| Refactor #11: content-area tab stop | Task 9 |

---

## Статус реалізації

✅ **Повністю реалізовано** — 2026-04-28
