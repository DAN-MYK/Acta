# Skill: Slint UI Components

Використовується при створенні або зміні .slint файлів.

## Стандартні патерни

### Список документів
```slint
import { StandardTableView, Button, LineEdit } from "std-widgets.slint";

export component DocumentList {
    in property <[[StandardListViewItem]]> items;
    callback row-selected(int);
    callback create-new();

    VerticalLayout {
        padding: 16px; spacing: 8px;
        LineEdit { placeholder-text: "Пошук..."; }
        StandardTableView {
            rows: root.items;
            current-row-changed(row) => { root.row-selected(row); }
        }
        Button { text: "Новий"; clicked => { root.create-new(); } }
    }
}
```

### Форма документу
```slint
import { Button, LineEdit, GroupBox, DatePicker } from "std-widgets.slint";

export component DocumentForm {
    in property <string> title;
    in property <[string]> counterparties;
    callback save(string, string, string);  // (number, date, counterparty_id)
    callback cancel();

    VerticalLayout {
        padding: 16px; spacing: 12px;
        Text { text: root.title; font-size: 16px; font-weight: 700; }
        GroupBox {
            title: "Реквізити";
            GridLayout {
                Row {
                    Text { text: "Номер:"; }
                    LineEdit { placeholder-text: "001"; }
                }
                Row {
                    Text { text: "Дата:"; }
                    DatePicker { }
                }
            }
        }
        HorizontalLayout {
            spacing: 8px;
            Button { text: "Зберегти"; primary: true; clicked => { root.save("", "", ""); } }
            Button { text: "Скасувати"; clicked => { root.cancel(); } }
        }
    }
}
```

## Типи властивостей
```slint
in property      // Rust → Slint (тільки читання)
out property     // Slint → Rust (тільки запис)
in-out property  // двосторонній
private property // тільки всередині компоненту
```

## Підключення Rust ↔ Slint
```rust
let ui = ActList::new()?;
ui.set_items(load_from_db().await?);
ui.on_row_selected(|row| { /* ... */ });
ui.on_create_new(|| { /* ... */ });
ui.run()?;
```

## Графіки через plotters
```rust
fn render_chart(data: &[f64]) -> slint::Image {
    let mut buf = SharedPixelBuffer::new(800, 400);
    let backend = BitMapBackend::with_buffer(buf.make_mut_bytes(), (800, 400));
    // ... малюємо через plotters ...
    slint::Image::from_rgb8(buf)
}
ui.set_chart_image(render_chart(&data));
```

```slint
Image { source: root.chart-image; width: 100%; height: 400px; }
```
