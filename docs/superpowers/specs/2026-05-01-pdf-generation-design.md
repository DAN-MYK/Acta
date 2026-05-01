# PDF Generation — Design Spec
_2026-05-01_

## Мета
Підключити вже реалізований `src/pdf/generator.rs` до UI: кнопка «PDF» в редакторі документа генерує файл і одразу відкриває його в системному переглядачі.

---

## Обсяг змін

6 файлів:

| Файл | Зміна |
|------|-------|
| `src/pdf/generator.rs` | Видалити `#![allow(dead_code)]` (рядок 4) |
| `src/tauri_api/documents.rs` | Додати `generate_document_pdf` + два приватні builders |
| `src-tauri/src/commands/documents.rs` | Додати `#[tauri::command] document_generate_pdf` |
| `src-tauri/src/lib.rs` | Зареєструвати команду в `invoke_handler!` |
| `frontend/src/lib/api.ts` | Додати `documentGeneratePdf` |
| `frontend/src/lib/stores/documents.ts` | Додати `generatePdf()` action |
| `frontend/src/lib/screens/DocumentsScreen.svelte` | Кнопка «PDF» в `editor-actions` |

---

## Формат `doc_id`

Ідентифікатор документа в системі — **рядок з префіксом**:
- `"act:UUID"` — акт
- `"inv:UUID"` — рахунок
- `"wbl:UUID"` — накладна

Такий формат використовується скрізь (списки, `document_open`, форма редактора). `parse_document_ref()` (вже є у `tauri_api/documents.rs`) розбирає цей рядок. Тому `generate_document_pdf` приймає тільки `doc_id: String` — окремий параметр `kind` не потрібен.

---

## Потік даних (Rust)

```
generate_document_pdf(ctx, doc_id)
  └─ parse_document_ref(&doc_id) → DocumentRef::Act(uuid) | ::Invoice(uuid) | ::Waybill(_)

  Для Act:
    1. db::acts::get_by_id(pool, uuid) → (Act, Vec<ActItem>)
    2. tokio::join!(
           db::companies::get_by_id(pool, company_id),
           db::counterparties::get_by_id(pool, company_id, act.counterparty_id)
       )
    3. build_act_pdf_data(&act, &items, &company, &counterparty) → PdfActData
    4. pdf::generator::ensure_output_dir(&act.number) → PathBuf
    5. pdf::generator::generate_act_pdf(&data, &path)
    6. open::that(&path)  // ignore Err, логуємо warn

  Для Invoice: аналогічно (ensure_invoice_output_dir, generate_invoice_pdf)

  Для Waybill: anyhow::bail!("PDF для накладних не підтримується")

  → MutationResultDto { ok: true, document_id: doc_id, message: "PDF збережено: {path}" }
```

**Критично:** `tokio::join!` тільки для company + counterparty, після того як документ вже завантажено. Company і counterparty незалежні між собою — їх можна паралелити.

---

## Нові імпорти у `src/tauri_api/documents.rs`

```rust
use crate::models::act::Act;           // додати до існуючого use crate::models::act::{...}
use crate::models::invoice::Invoice;   // додати до існуючого use crate::models::invoice::{...}
use crate::models::company::Company;
use crate::models::counterparty::Counterparty;
use crate::pdf::generator::{
    amount_to_words, ensure_invoice_output_dir, ensure_output_dir,
    generate_act_pdf, generate_invoice_pdf,
    PdfActData, PdfActItem, PdfCompany, PdfInvoiceData, PdfInvoiceItem,
};
```

---

## Builder-функції

### `build_act_pdf_data`

```rust
fn build_act_pdf_data(
    act: &Act,
    items: &[ActItem],
    company: &Company,
    client: &Counterparty,
) -> PdfActData {
    PdfActData {
        number: act.number.clone(),
        date: act.date.format("%d.%m.%Y").to_string(),
        company: to_pdf_company(company),
        client: counterparty_to_pdf_company(client),
        items: items.iter().enumerate().map(|(i, item)| PdfActItem {
            num: (i + 1) as u32,
            name: item.description.clone(),
            qty: format!("{:.4}", item.quantity),
            unit: item.unit.clone(),              // ActItem.unit: String
            price: format!("{:.2}", item.unit_price), // ActItem.unit_price (не price!)
            amount: format!("{:.2}", item.amount),
        }).collect(),
        total: format!("{:.2}", act.total_amount),
        total_words: amount_to_words(&act.total_amount),
        notes: act.notes.clone().unwrap_or_default(),
    }
}
```

### `build_invoice_pdf_data`

```rust
fn build_invoice_pdf_data(
    invoice: &Invoice,
    items: &[InvoiceItem],
    company: &Company,
    client: &Counterparty,
) -> PdfInvoiceData {
    PdfInvoiceData {
        number: invoice.number.clone(),
        date: invoice.date.format("%d.%m.%Y").to_string(),
        company: to_pdf_company(company),
        client: counterparty_to_pdf_company(client),
        items: items.iter().enumerate().map(|(i, item)| PdfInvoiceItem {
            num: (i + 1) as u32,
            name: item.description.clone(),
            qty: format!("{:.4}", item.quantity),
            unit: item.unit.clone().unwrap_or_default(), // InvoiceItem.unit: Option<String>
            price: format!("{:.2}", item.price),         // InvoiceItem.price (не unit_price!)
            amount: format!("{:.2}", item.amount),
        }).collect(),
        total: format!("{:.2}", invoice.total_amount),
        vat_amount: format!("{:.2}", invoice.vat_amount),
        total_words: amount_to_words(&invoice.total_amount),
        notes: invoice.notes.clone().unwrap_or_default(),
    }
}
```

### Допоміжні функції PdfCompany

```rust
fn to_pdf_company(c: &Company) -> PdfCompany {
    PdfCompany {
        name: c.name.clone(),
        edrpou: c.edrpou.clone().unwrap_or_default(),
        iban: c.iban.clone().unwrap_or_default(),
        address: c.actual_address.clone()
            .or_else(|| c.legal_address.clone())
            .unwrap_or_default(),
    }
}

fn counterparty_to_pdf_company(cp: &Counterparty) -> PdfCompany {
    PdfCompany {
        name: cp.name.clone(),
        edrpou: cp.edrpou.clone().or_else(|| cp.ipn.clone()).unwrap_or_default(),
        iban: cp.iban.clone().unwrap_or_default(),
        address: cp.address.clone().unwrap_or_default(),
    }
}
```

---

## Tauri command shim

Патерн такий самий як решта у `src-tauri/src/commands/documents.rs`:

```rust
#[tauri::command]
pub async fn document_generate_pdf(
    state: State<'_, TauriState>,
    doc_id: String,
) -> CommandResult<MutationResultDto> {
    acta::tauri_api::documents::generate_document_pdf(&state.ctx, doc_id)
        .await
        .map_err(|error| error.to_string())
}
```

Реєстрація в `src-tauri/src/lib.rs` — додати рядок у `invoke_handler!`:
```rust
commands::documents::document_generate_pdf,
```

---

## Frontend

### `api.ts`
```ts
export function documentGeneratePdf(docId: string): Promise<MutationResultDto> {
  return invoke("document_generate_pdf", { docId });
}
```

### `stores/documents.ts`
```ts
async generatePdf() {
  const snapshot = get({ subscribe });
  const docId = snapshot.editor?.form.id;
  if (!docId) return;

  update((state) => ({ ...state, loading: true, error: null, message: null }));
  try {
    const response = await documentGeneratePdf(docId);
    update((state) => ({ ...state, loading: false, message: response.message }));
  } catch (error) {
    update((state) => ({ ...state, loading: false, error: String(error) }));
  }
},
```

### `DocumentsScreen.svelte`

Кнопка в блоці `editor-actions`, видима тільки для act та invoice:
```svelte
{#if ['act', 'invoice'].includes($documents.editor.form.kind)}
  <button class="btn-secondary" on:click={() => documents.generatePdf()}>PDF</button>
{/if}
```

---

## Підводні камені

| Підводний камінь | Рішення |
|-----------------|---------|
| `ActItem.unit_price` vs `InvoiceItem.price` | Правильні поля прописані у builders |
| `InvoiceItem.unit: Option<String>` | `.clone().unwrap_or_default()` |
| `db::counterparties::get_by_id` — 3 аргументи | `(pool, company_id, counterparty_id)` |
| `parse_document_ref` є приватною функцією | Викликати всередині того ж модуля — ОК |
| `open` crate v5 | Вже є в Cargo.toml |
| Typst шляхи відносні до CWD | Тільки `cargo tauri dev` з кореня проекту. Production — out of scope |

---

## Non-goals (не входить в цей спек)

- Збереження PDF-шляху в БД (Acts не має поля `pdf_path` на відміну від Invoices — залишаємо на майбутнє)
- PDF для накладних (немає шаблону)
- Налаштування шляхів для production build (релятивні шляхи Typst)
- Progress indicator під час генерації
