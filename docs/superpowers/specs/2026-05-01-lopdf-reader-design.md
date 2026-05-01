# lopdf Reader — Design Spec

**Дата:** 2026-05-01  
**Статус:** Approved

## Контекст

У проекті вже є `src/pdf/generator.rs` для генерації PDF через Typst CLI.  
Потрібно додати функціонал читання та редагування існуючих PDF через бібліотеку `lopdf = "0.34"` (вже в `Cargo.toml`).

## Мета

Реалізувати дві функції:
1. **`read_pdf_text`** — витягти весь текст з PDF-файлу
2. **`replace_pdf_text`** — замінити текстовий рядок на всіх сторінках PDF

## Архітектура

### Новий файл: `src/pdf/reader.rs`

Окремий модуль, не змішувати з Typst-генерацією в `generator.rs`.  
`src/pdf/mod.rs` додає `pub mod reader;`.

### Публічний API

```rust
use std::path::Path;
use anyhow::Result;

/// Витягує весь текст з PDF-документу (всі сторінки, по порядку).
pub fn read_pdf_text(path: &Path) -> Result<String>

/// Замінює всі входження old_text → new_text на кожній сторінці PDF і зберігає файл.
///
/// Обмеження lopdf: замінює лише текст у Tj-операторах (точний рядок у одному операнді).
/// TJ-оператор (масив рядків, типовий для Typst) — НЕ обробляється.
/// Якщо old_text не знайдено — функція успішно завершується без змін.
pub fn replace_pdf_text(path: &Path, old_text: &str, new_text: &str) -> Result<()>
```

## Ключові деталі реалізації

### `read_pdf_text`

```
Document::load(path)
  → doc.get_pages().keys().copied().collect::<Vec<u32>>()  // номери сторінок
  → doc.extract_text(&page_numbers)                         // повертає lopdf::Result<String>
  → Ok(text)
```

**Важливо:** `get_pages()` повертає `BTreeMap<u32, ObjectId>`.  
`.keys()` = номери сторінок (u32) — те що потрібно для `extract_text`.  
`.values()` = ObjectId (tuple) — НЕ підходить.  
Vault-специфікація мала баг: використовувала `.values()`.

### `replace_pdf_text`

```
Document::load(path)
  → for page_number in doc.get_pages().keys()
      doc.replace_text(page_number, old_text, new_text)?
  → doc.save(path)
```

**Обмеження lopdf `replace_text`:**  
Реалізація в `parser_aux.rs` обробляє лише оператор `Tj`:
```rust
"Tj" => try_to_replace_encoded_text(...)  // ← є
// "TJ" => ...                            // ← відсутній
```
`TJ` — масив рядків, який Typst часто використовує. Тому на Typst-PDF заміна може не спрацювати — без помилки, але й без результату. Це обмеження бібліотеки, яке треба задокументувати в коментарі.

### Обробка помилок

- `Document::load` → `lopdf::Error` → конвертується в `anyhow::Error` через `?` + `.with_context()`
- `replace_text` → `lopdf::Result<()>` → пропагується через `?`
- `save` → `std::io::Result<File>` (writer.rs використовує `std::io::Result` напряму) → `std::io::Error: std::error::Error` → anyhow обгортає через `?`

## Тести

### Юніт (без реального PDF)

| Тест | Опис |
|------|------|
| `read_pdf_text_returns_err_for_missing_file` | `Path::new("nonexistent.pdf")` → `Err` |
| `replace_pdf_text_returns_err_for_missing_file` | `Path::new("nonexistent.pdf")` → `Err` |

### Інтеграційний (if typst available)

| Тест | Опис |
|------|------|
| `read_pdf_text_extracts_text_from_typst_pdf` | `generate_act_pdf` → `read_pdf_text` → рядок містить `"АКТ"` |

Інтеграційний тест використовує існуючий паттерн `typst_available()` + `typst_lock()` з `generator.rs`.

## Що НЕ входить у цей scope

- Читання PDF-форм, анотацій, метаданих
- Вставка зображень (lopdf підтримує, але не потрібно зараз)
- Tauri command для цих функцій (окрема задача)
- Вирішення обмеження TJ-оператора (потребує патчу lopdf або іншого підходу)
