# Figma Design System Rules — Acta

> Цей документ описує як перекладати Figma-дизайни в код для проекту Acta.
> Стек: **Rust + Slint** (десктопний UI, НЕ веб).

---

## КРИТИЧНО ВАЖЛИВО: Це не веб-застосунок

Acta — десктопна програма. UI написаний на **Slint DSL** (файли `.slint`).
Ніякого React, Vue, HTML, CSS, Tailwind, styled-components тут немає.

При реалізації Figma-дизайнів:
- НЕ генерувати JSX, TSX, HTML, CSS
- НЕ використовувати веб-класи (flex, grid, p-4, text-sm тощо)
- ЗАВЖДИ генерувати `.slint` файли

---

## 1. Дизайн-токени (Design Tokens)

Токени НЕ мають окремого файлу — вони визначені **інлайн у .slint файлах**.

### Поточна кольорова палітра (з `ui/main.slint`)

```
Фон сайдбару:    #1e293b   (темно-синій)
Фон сторінки:    #f8fafc   (світло-сірий)
Активний пункт:  #3b82f6   (синій)
Текст активний:  #ffffff
Текст неактивний:#94a3b8   (сіро-блакитний)
Роздільник:      #334155
Версія (текст):  #475569
Заголовок:       #1e293b
```

### Де визначати токени у Slint

Для нових токенів — створити `ui/tokens.slint`:

```slint
export global Tokens {
    // Кольори
    out property <color> surface-sidebar: #1e293b;
    out property <color> surface-page:    #f8fafc;
    out property <color> accent-primary:  #3b82f6;
    out property <color> text-primary:    #1e293b;
    out property <color> text-secondary:  #94a3b8;
    out property <color> border:          #334155;

    // Типографія
    out property <length> font-body:    14px;
    out property <length> font-heading: 22px;
    out property <length> font-small:   11px;

    // Відступи
    out property <length> spacing-sm:  8px;
    out property <length> spacing-md:  12px;
    out property <length> spacing-lg:  20px;

    // Радіус
    out property <length> radius-sm: 6px;
    out property <length> radius-md: 8px;
}
```

Імпорт у компоненті: `import { Tokens } from "../tokens.slint";`

---

## 2. Компонентна бібліотека

### Розташування

```
ui/
├── main.slint                      ← кореневий файл (MainWindow)
├── tokens.slint                    ← (створити) дизайн-токени
├── counterparties/
│   └── counterparty_list.slint     ← список контрагентів
└── [нові модулі]/
    └── [module]_list.slint         ← за аналогією
```

### Архітектура компонентів

Кожен `.slint` файл = один або кілька компонентів.
- `export component` — публічний (імпортується в інших файлах)
- `component` (без export) — приватний (тільки в цьому файлі)

### Приклад патерну компонента

```slint
import { Button, LineEdit, StandardTableView } from "std-widgets.slint";
import { Tokens } from "../tokens.slint";

export component MyList {
    // Вхідні дані (Rust → Slint)
    in property <[[StandardListViewItem]]> rows;
    in property <[string]> row-ids;
    in property <bool> loading: false;

    // Колбеки (Slint → Rust)
    callback item-selected(string);   // передає UUID
    callback create-clicked();

    // Внутрішній стан
    private property <int>    selected-row: -1;
    private property <string> selected-id:  "";

    // Розмітка
    VerticalLayout {
        padding: Tokens.spacing-lg;
        spacing: Tokens.spacing-md;
        // ...
    }
}
```

---

## 3. Фреймворк та збірка

| Компонент        | Технологія            |
|------------------|-----------------------|
| Мова             | Rust (edition 2024)   |
| UI DSL           | Slint 1.9             |
| БД               | PostgreSQL + sqlx 0.8 |
| Async runtime    | tokio 1               |
| PDF              | Typst / lopdf         |
| XML/Excel import | quick-xml, calamine   |
| Build script     | `build.rs` → `slint_build::compile("ui/main.slint")` |

### Збірка

```bash
cargo build   # компілює Rust + генерує код з .slint
cargo run     # запуск
```

`.slint` → Rust код генерується автоматично в `build.rs`.
Доступ у Rust: `slint::include_modules!()` у `src/main.rs`.

---

## 4. Управління активами (Assets)

- Немає CDN, немає web assets pipeline
- Зображення: Slint підтримує `@image-url("path/to/image.png")` — відносно `.slint` файлу
- Іконки: вбудовані через Slint або через `Image { source: @image-url(...); }`
- Шрифти: системні або через `slint::platform::set_platform`

---

## 5. Система іконок

Зараз іконок немає (використовується тільки текст та емодзі як placeholder у заглушках).

Рекомендований підхід для нових іконок:

```slint
// SVG іконка через Image
Image {
    source: @image-url("../assets/icons/edit.svg");
    width: 20px;
    height: 20px;
    colorize: #3b82f6;   // перефарбування SVG
}
```

Зберігати в: `ui/assets/icons/`

---

## 6. Підхід до стилізації

### Немає CSS — є Slint властивості

| CSS/Web аналог       | Slint еквівалент                        |
|----------------------|-----------------------------------------|
| `display: flex`      | `HorizontalLayout {}` / `VerticalLayout {}` |
| `gap: 8px`           | `spacing: 8px`                          |
| `padding: 20px`      | `padding: 20px`                         |
| `border-radius: 6px` | `border-radius: 6px`                    |
| `background: #fff`   | `background: #ffffff`                   |
| `font-size: 14px`    | `font-size: 14px`                       |
| `font-weight: 700`   | `font-weight: 700`                      |
| `color: #333`        | `color: #333333`                        |
| `width: 200px`       | `width: 200px`                          |
| `min-width`          | `min-width`                             |
| `overflow: hidden`   | `clip: true`                            |
| `text-overflow: ellipsis` | `overflow: elide`                  |
| `cursor: pointer`    | `mouse-cursor: pointer`                 |
| `transition`         | `animate prop { duration: 150ms; }`     |
| `z-index`            | порядок визначається порядком у коді    |
| `@media query`       | умовна логіка через `if` або properties |

### Адаптивність

Slint — десктоп, немає медіа-запитів у звичному розумінні.
Адаптивність через: `min-width`, `horizontal-stretch`, `vertical-stretch`.

---

## 7. Структура проекту

```
acta/
├── src/
│   ├── main.rs              ← точка входу, Rust ↔ Slint зв'язка
│   ├── db/                  ← CRUD функції (async, sqlx)
│   │   ├── mod.rs
│   │   └── counterparties.rs
│   ├── models/              ← Rust структури (FromRow)
│   │   ├── mod.rs
│   │   └── counterparty.rs
│   ├── pdf/                 ← (TODO) Typst генерація
│   └── import/              ← (TODO) BAS/банківські парсери
│   └── bin/
│       └── migrate.rs       ← (TODO) BAS міграція
├── ui/                      ← ВСІ .slint файли
│   ├── main.slint           ← MainWindow (кореневий)
│   ├── tokens.slint         ← (створити) дизайн-токени
│   └── counterparties/
│       └── counterparty_list.slint
├── templates/               ← (TODO) .typ Typst шаблони
├── migrations/              ← SQL міграції
│   └── 001_initial.sql
├── storage/                 ← файли на диску
├── build.rs                 ← slint_build::compile
├── Cargo.toml
└── .env
```

---

## 8. Як реалізовувати Figma → Slint

### Алгоритм

1. **Отримати скріншот та контекст** з Figma (`get_design_context`)
2. **Визначити тип компонента**: новий екран, нова картка, новий діалог
3. **Знайти аналог у `std-widgets`**: `Button`, `LineEdit`, `ComboBox`, `CheckBox`, `StandardTableView`, `ScrollView`, `TabWidget`, `ProgressIndicator`, `Spinner`
4. **Створити `.slint` файл** у відповідній папці `ui/`
5. **Визначити `in property` та `callback`** — межа UI/бізнес-логіки
6. **Підключити у `main.slint`** та передати дані з Rust

### Шаблон нового екрану

```slint
// ui/[module]/[module]_form.slint
import { Button, LineEdit, TextEdit } from "std-widgets.slint";

export component MyForm {
    // Режим: true = редагування, false = створення
    in property <bool> edit-mode: false;
    in property <string> initial-name: "";

    // Виходи
    callback save-clicked(string /* name */, string /* ...other fields */);
    callback cancel-clicked();

    VerticalLayout {
        padding: 24px;
        spacing: 16px;

        Text {
            text: root.edit-mode ? "Редагувати" : "Новий запис";
            font-size: 20px;
            font-weight: 700;
            color: #1e293b;
        }

        name-field := LineEdit {
            text: root.initial-name;
            placeholder-text: "Назва *";
        }

        HorizontalLayout {
            alignment: end;
            spacing: 8px;

            Button {
                text: "Скасувати";
                clicked => { root.cancel-clicked(); }
            }
            Button {
                text: root.edit-mode ? "Зберегти" : "Створити";
                primary: true;
                clicked => { root.save-clicked(name-field.text); }
            }
        }
    }
}
```

### Rust-сторона для нового компонента

```rust
// Підключення колбеку в main.rs
ui.on_my_form_save_clicked(move |name| {
    let pool = pool.clone();
    let ui_handle = ui_weak.clone();
    tokio::spawn(async move {
        match db::module::create(&pool, &name).await {
            Ok(_) => { /* оновити список */ }
            Err(e) => tracing::error!("Помилка: {e}"),
        }
    });
});
```

---

## 9. Іменування (конвенції)

| Контекст         | Конвенція             | Приклад                        |
|------------------|-----------------------|--------------------------------|
| Slint property   | kebab-case            | `selected-row`, `row-ids`      |
| Slint callback   | kebab-case з дієсловом| `create-clicked`, `row-selected`|
| Slint component  | PascalCase            | `CounterpartyList`, `NavItem`  |
| Rust функція     | snake_case            | `list`, `get_by_id`, `archive` |
| Rust struct      | PascalCase            | `Counterparty`, `NewCounterparty`|
| БД таблиця       | snake_case plural     | `counterparties`, `act_items`  |
| Файл .slint      | snake_case            | `counterparty_list.slint`      |
| Папка UI модуля  | snake_case plural     | `counterparties/`, `acts/`     |

---

## 10. Обмеження Slint (для Figma-перекладу)

- Немає `box-shadow` → використовувати `drop-shadow` filter або вкладені Rectangle
- Немає `overflow: scroll` на довільному елементі → `ScrollView { ... }`
- Немає `position: absolute` → вкладені Rectangle з явними розмірами
- Немає `display: grid` → вкладені HorizontalLayout/VerticalLayout
- Немає web-шрифтів → системні шрифти або вбудовані через ресурси
- `Text` не підтримує вбудований HTML → тільки plain text або `wrap: word-wrap`
