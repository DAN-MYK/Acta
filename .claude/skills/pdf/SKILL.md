# Skill: PDF Generation (Typst)

Використовується при генерації PDF документів.

## Варіант A: Typst CLI (простіше для старту)

```rust
use std::process::Command;
use serde_json::json;

pub fn generate_pdf(
    template: &Path,
    data: &serde_json::Value,
    output: &Path,
) -> Result<()> {
    let data_str = serde_json::to_string(data)?;
    let status = Command::new("typst")
        .args([
            "compile",
            "--input", &format!("data={}", data_str),
            &template.to_string_lossy(),
            &output.to_string_lossy(),
        ])
        .status()?;
    anyhow::ensure!(status.success(), "typst compile failed");
    Ok(())
}
```

## Варіант B: Typst як Rust бібліотека
Потребує реалізації `typst::World` trait.
Документація: https://docs.rs/typst/latest/typst/trait.World.html

## Шаблон акту (templates/act.typ)
```typst
#let data = json(sys.inputs.data)

#align(center)[
  #text(size: 14pt, weight: "bold")[АКТ ВИКОНАНИХ РОБІТ № #data.number]
]

*Дата:* #data.date \
*Замовник:* #data.client_name \
*Виконавець:* #data.company_name

#table(
  columns: (auto, 1fr, auto, auto, auto),
  stroke: 0.5pt,
  [*№*], [*Найменування*], [*К-сть*], [*Ціна*], [*Сума*],
  ..data.items.map(i => (
    str(i.num), i.name,
    str(i.qty) + " " + i.unit,
    i.price + " грн",
    i.amount + " грн"
  )).flatten()
)

#align(right)[*Загальна сума: #data.total грн*]
```

## Дані для шаблону (Rust)
```rust
let data = json!({
    "number": act.number,
    "date": act.date.format("%d.%m.%Y").to_string(),
    "client_name": act.counterparty.name,
    "company_name": "ФОП Іваненко І.І.",
    "items": act.items.iter().map(|i| json!({
        "num": i.position,
        "name": i.name,
        "qty": i.quantity,
        "unit": i.unit,
        "price": format!("{:.2}", i.price),
        "amount": format!("{:.2}", i.amount),
    })).collect::<Vec<_>>(),
    "total": format!("{:.2}", act.total_amount),
});
```

## Вставка зображення в шаблон (формула, схема)
```typst
#if data.has("image_path") {
  figure(image(data.image_path, width: 80%))
}
```

## Збереження файлів
```
storage/documents/acts/2026/03/АКТ-001-2026.pdf
storage/documents/invoices/2026/03/НАК-001-2026.pdf
```
