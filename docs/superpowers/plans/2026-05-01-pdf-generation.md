# PDF Generation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Wire the existing `pdf/generator.rs` into the documents editor UI — кнопка «PDF» генерує файл і відкриває його в системному переглядачі.

**Architecture:** Тонкий Tauri command `document_generate_pdf` делегує у `src/tauri_api/documents.rs`, де приватні builders збирають `PdfActData`/`PdfInvoiceData` з моделей БД, а `open` crate відкриває готовий файл. Фронтенд додає кнопку тільки в editor-actions для act/invoice (не для waybill).

**Tech Stack:** Rust/Tauri 2, `pdf::generator` (Typst CLI), `open = "5"`, Svelte/TypeScript.

---

## File Map

| Файл | Дія |
|------|-----|
| `src/pdf/generator.rs` | Видалити `#![allow(dead_code)]` (рядок 4) |
| `src/tauri_api/documents.rs` | Нові imports + 4 приватні helper fn + pub `generate_document_pdf` |
| `src-tauri/src/commands/documents.rs` | Новий `#[tauri::command] document_generate_pdf` |
| `src-tauri/src/lib.rs` | +1 рядок в `invoke_handler!` |
| `frontend/src/lib/api.ts` | Нова функція `documentGeneratePdf` |
| `frontend/src/lib/stores/documents.ts` | Новий метод `generatePdf()` |
| `frontend/src/lib/screens/DocumentsScreen.svelte` | Кнопка «PDF» в `editor-actions` |

---

## Task 1: Видалити `#![allow(dead_code)]` з `generator.rs`

**Files:**
- Modify: `src/pdf/generator.rs:4`

- [x] **Step 1: Видалити атрибут**

У `src/pdf/generator.rs` знайди рядок 4:
```rust
#![allow(dead_code)]
```
Видали його повністю. Файл після видалення починатиметься з:
```rust
// Генерація PDF-актів через Typst CLI
//
// Алгоритм: структури даних → serde_json → JSON рядок → typst compile --input data=...
// Typst читає sys.inputs["data"] і будує PDF з шаблону templates/act.typ.

use std::path::{Path, PathBuf};
```

- [x] **Step 2: Перевірка компіляції**

```bash
cargo build --lib
```
Очікується: `Finished` без warnings типу "function is never used".

- [x] **Step 3: Коміт**

```bash
git add src/pdf/generator.rs
git commit -m "chore(pdf): remove dead_code allow — functions are now used"
```

---

## Task 2: Додати imports та helper-функції у `tauri_api/documents.rs`

**Files:**
- Modify: `src/tauri_api/documents.rs`

- [x] **Step 1: Написати тест для `to_pdf_company`**

Додай в кінець `src/tauri_api/documents.rs` (всередині майбутнього `#[cfg(test)] mod tests {}`):

Спочатку додай тестовий модуль в кінець файлу (якщо його ще немає):

```rust
#[cfg(test)]
mod pdf_tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    fn sample_company() -> crate::models::company::Company {
        crate::models::company::Company {
            id: uuid::Uuid::nil(),
            name: "ФОП Тестовий".into(),
            short_name: None,
            edrpou: Some("1234567890".into()),
            ipn: None,
            iban: Some("UA123456789012345678901234567".into()),
            legal_address: Some("вул. Юридична, 1".into()),
            actual_address: Some("вул. Фактична, 2".into()),
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
    fn to_pdf_company_prefers_actual_address() {
        let company = sample_company();
        let pdf = to_pdf_company(&company);
        assert_eq!(pdf.name, "ФОП Тестовий");
        assert_eq!(pdf.edrpou, "1234567890");
        assert_eq!(pdf.iban, "UA123456789012345678901234567");
        assert_eq!(pdf.address, "вул. Фактична, 2");
    }

    #[test]
    fn to_pdf_company_falls_back_to_legal_address() {
        let mut company = sample_company();
        company.actual_address = None;
        let pdf = to_pdf_company(&company);
        assert_eq!(pdf.address, "вул. Юридична, 1");
    }

    #[test]
    fn to_pdf_company_empty_when_no_address() {
        let mut company = sample_company();
        company.actual_address = None;
        company.legal_address = None;
        let pdf = to_pdf_company(&company);
        assert_eq!(pdf.address, "");
    }
}
```

- [x] **Step 2: Запустити тест — переконатися що не компілюється**

```bash
cargo test --lib pdf_tests 2>&1 | head -20
```
Очікується: помилка "cannot find function `to_pdf_company`".

- [x] **Step 3: Додати imports у `src/tauri_api/documents.rs`**

Знайди існуючий блок imports на початку файлу. Внеси такі зміни:

Змінити:
```rust
use crate::models::act::{ActItem, ActStatus, NewAct, NewActItem, UpdateAct};
```
На:
```rust
use crate::models::act::{Act, ActItem, ActStatus, NewAct, NewActItem, UpdateAct};
```

Змінити:
```rust
use crate::models::invoice::{
    InvoiceItem, InvoiceStatus, NewInvoice, NewInvoiceItem, UpdateInvoice,
};
```
На:
```rust
use crate::models::invoice::{
    Invoice, InvoiceItem, InvoiceStatus, NewInvoice, NewInvoiceItem, UpdateInvoice,
};
```

Додати нові рядки після існуючих `use crate::models::*`:
```rust
use crate::models::company::Company;
use crate::models::counterparty::Counterparty;
use crate::pdf::generator::{
    amount_to_words, ensure_invoice_output_dir, ensure_output_dir, generate_act_pdf,
    generate_invoice_pdf, PdfActData, PdfActItem, PdfCompany, PdfInvoiceData, PdfInvoiceItem,
};
```

- [x] **Step 4: Додати helper-функції `to_pdf_company` та `counterparty_to_pdf_company`**

Додай одразу після приватних функцій (наприклад після `fn normalize_chain_kind`) і до перших `pub async fn`:

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

- [x] **Step 5: Запустити тести helper-функцій — мають пройти**

```bash
cargo test --lib pdf_tests 2>&1 | head -30
```
Очікується: `test pdf_tests::to_pdf_company_prefers_actual_address ... ok` та ін.

---

## Task 3: Додати builder-функції + тести

**Files:**
- Modify: `src/tauri_api/documents.rs`

- [x] **Step 1: Написати тести для builders**

Додай до блоку `#[cfg(test)] mod pdf_tests` з Task 2:

```rust
    fn sample_counterparty() -> crate::models::counterparty::Counterparty {
        crate::models::counterparty::Counterparty {
            id: uuid::Uuid::nil(),
            name: "ТОВ Замовник".into(),
            edrpou: Some("9876543210".into()),
            ipn: None,
            iban: Some("UA987654321098765432109876543".into()),
            address: Some("вул. Замовника, 5".into()),
            phone: None,
            email: None,
            notes: None,
            is_archived: false,
            bas_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_act() -> crate::models::act::Act {
        use crate::models::act::ActStatus;
        use crate::models::DocumentDirection;
        crate::models::act::Act {
            id: uuid::Uuid::nil(),
            number: "АКТ-2026-001".into(),
            counterparty_id: uuid::Uuid::nil(),
            contract_id: None,
            category_id: None,
            direction: DocumentDirection::Outgoing,
            date: chrono::NaiveDate::from_ymd_opt(2026, 4, 15).unwrap(),
            expected_payment_date: None,
            total_amount: dec!(45000.00),
            status: ActStatus::Draft,
            notes: Some("Примітка".into()),
            bas_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_act_items() -> Vec<crate::models::act::ActItem> {
        vec![crate::models::act::ActItem {
            id: uuid::Uuid::nil(),
            act_id: uuid::Uuid::nil(),
            description: "Розробка ПЗ".into(),
            quantity: dec!(1),
            unit: "послуга".into(),
            unit_price: dec!(45000.00),
            amount: dec!(45000.00),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }]
    }

    #[test]
    fn build_act_pdf_data_maps_fields_correctly() {
        let act = sample_act();
        let items = sample_act_items();
        let company = sample_company();
        let client = sample_counterparty();

        let data = build_act_pdf_data(&act, &items, &company, &client);

        assert_eq!(data.number, "АКТ-2026-001");
        assert_eq!(data.date, "15.04.2026");
        assert_eq!(data.total, "45000.00");
        assert_eq!(data.notes, "Примітка");
        assert_eq!(data.items.len(), 1);

        let item = &data.items[0];
        assert_eq!(item.num, 1);
        assert_eq!(item.name, "Розробка ПЗ");
        assert_eq!(item.unit, "послуга");
        assert_eq!(item.price, "45000.00");  // unit_price → price у PdfActItem
        assert_eq!(item.amount, "45000.00");
    }

    fn sample_invoice() -> crate::models::invoice::Invoice {
        use crate::models::invoice::InvoiceStatus;
        use crate::models::DocumentDirection;
        crate::models::invoice::Invoice {
            id: uuid::Uuid::nil(),
            company_id: uuid::Uuid::nil(),
            number: "РАХ-2026-001".into(),
            counterparty_id: uuid::Uuid::nil(),
            contract_id: None,
            category_id: None,
            direction: DocumentDirection::Outgoing,
            date: chrono::NaiveDate::from_ymd_opt(2026, 4, 15).unwrap(),
            expected_payment_date: None,
            total_amount: dec!(1000.00),
            vat_amount: dec!(0.00),
            status: InvoiceStatus::Draft,
            notes: None,
            pdf_path: None,
            bas_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_invoice_items() -> Vec<crate::models::invoice::InvoiceItem> {
        vec![crate::models::invoice::InvoiceItem {
            id: uuid::Uuid::nil(),
            invoice_id: uuid::Uuid::nil(),
            position: 1,
            description: "Товар".into(),
            unit: Some("шт".into()),
            quantity: dec!(2),
            price: dec!(500.00),
            amount: dec!(1000.00),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }]
    }

    #[test]
    fn build_invoice_pdf_data_maps_fields_correctly() {
        let invoice = sample_invoice();
        let items = sample_invoice_items();
        let company = sample_company();
        let client = sample_counterparty();

        let data = build_invoice_pdf_data(&invoice, &items, &company, &client);

        assert_eq!(data.number, "РАХ-2026-001");
        assert_eq!(data.date, "15.04.2026");
        assert_eq!(data.total, "1000.00");
        assert_eq!(data.vat_amount, "0.00");
        assert_eq!(data.items.len(), 1);

        let item = &data.items[0];
        assert_eq!(item.unit, "шт");   // Option<String> → String
        assert_eq!(item.price, "500.00");  // InvoiceItem.price (не unit_price)
        assert_eq!(item.amount, "1000.00");
    }

    #[test]
    fn build_invoice_pdf_data_handles_none_unit() {
        let invoice = sample_invoice();
        let mut items = sample_invoice_items();
        items[0].unit = None;  // Option<String> = None
        let data = build_invoice_pdf_data(&invoice, &items, &sample_company(), &sample_counterparty());
        assert_eq!(data.items[0].unit, "");  // unwrap_or_default
    }
```

- [x] **Step 2: Запустити тести — переконатися що не компілюються**

```bash
cargo test --lib pdf_tests 2>&1 | head -20
```
Очікується: помилка "cannot find function `build_act_pdf_data`".

- [x] **Step 3: Додати builder-функції**

Додай одразу після `counterparty_to_pdf_company` (з Task 2):

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
        items: items
            .iter()
            .enumerate()
            .map(|(i, item)| PdfActItem {
                num: (i + 1) as u32,
                name: item.description.clone(),
                qty: format!("{:.4}", item.quantity),
                unit: item.unit.clone(),
                price: format!("{:.2}", item.unit_price),
                amount: format!("{:.2}", item.amount),
            })
            .collect(),
        total: format!("{:.2}", act.total_amount),
        total_words: amount_to_words(&act.total_amount),
        notes: act.notes.clone().unwrap_or_default(),
    }
}

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
        items: items
            .iter()
            .enumerate()
            .map(|(i, item)| PdfInvoiceItem {
                num: (i + 1) as u32,
                name: item.description.clone(),
                qty: format!("{:.4}", item.quantity),
                unit: item.unit.clone().unwrap_or_default(),
                price: format!("{:.2}", item.price),
                amount: format!("{:.2}", item.amount),
            })
            .collect(),
        total: format!("{:.2}", invoice.total_amount),
        vat_amount: format!("{:.2}", invoice.vat_amount),
        total_words: amount_to_words(&invoice.total_amount),
        notes: invoice.notes.clone().unwrap_or_default(),
    }
}
```

- [x] **Step 4: Запустити всі тести builders — мають пройти**

```bash
cargo test --lib pdf_tests 2>&1 | head -40
```
Очікується: всі `pdf_tests::*` — `ok`.

- [x] **Step 5: Коміт**

```bash
git add src/tauri_api/documents.rs
git commit -m "feat(pdf): add PdfCompany helpers and act/invoice builder functions"
```

---

## Task 4: Додати `generate_document_pdf` у `tauri_api/documents.rs`

**Files:**
- Modify: `src/tauri_api/documents.rs`

- [x] **Step 1: Написати тест для waybill-branch**

Додай до `#[cfg(test)] mod pdf_tests`:

```rust
    #[test]
    fn generate_document_pdf_rejects_waybill_id() {
        // Перевірка що waybill повертає Err без DB запитів
        let wbl_id = format!("wbl:{}", uuid::Uuid::nil());
        // parse_document_ref → DocumentRef::Waybill → bail!
        // Тестуємо логіку через parse: waybill ref має правильно парситись
        let doc_ref = parse_document_ref(&wbl_id);
        assert!(matches!(doc_ref, Some(DocumentRef::Waybill(_))));
    }
```

- [x] **Step 2: Запустити тест — перевірити що компілюється і проходить**

```bash
cargo test --lib pdf_tests::generate_document_pdf_rejects_waybill_id 2>&1
```
Очікується: `ok` — `parse_document_ref` і `DocumentRef` вже існують.

- [x] **Step 3: Додати `generate_document_pdf`**

Додай після `build_invoice_pdf_data` (і перед `pub async fn documents_list`):

```rust
pub async fn generate_document_pdf(ctx: &AppCtx, doc_id: String) -> Result<MutationResultDto> {
    let doc_ref = parse_document_ref(&doc_id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документу: {doc_id}"))?;

    let pool = ctx.pool();
    let company_id = ctx.company_id();

    let path = match doc_ref {
        DocumentRef::Act(id) => {
            let (act, items) = db::acts::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Акт не знайдено"))?;

            let (company_res, counterparty_res) = tokio::join!(
                db::companies::get_by_id(pool, company_id),
                db::counterparties::get_by_id(pool, company_id, act.counterparty_id)
            );
            let company = company_res?.ok_or_else(|| anyhow!("Компанію не знайдено"))?;
            let counterparty =
                counterparty_res?.ok_or_else(|| anyhow!("Контрагента не знайдено"))?;

            let data = build_act_pdf_data(&act, &items, &company, &counterparty);
            let path = ensure_output_dir(&act.number)?;
            generate_act_pdf(&data, &path)?;
            path
        }
        DocumentRef::Invoice(id) => {
            let (invoice, items) = db::invoices::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Рахунок не знайдено"))?;

            let (company_res, counterparty_res) = tokio::join!(
                db::companies::get_by_id(pool, company_id),
                db::counterparties::get_by_id(pool, company_id, invoice.counterparty_id)
            );
            let company = company_res?.ok_or_else(|| anyhow!("Компанію не знайдено"))?;
            let counterparty =
                counterparty_res?.ok_or_else(|| anyhow!("Контрагента не знайдено"))?;

            let data = build_invoice_pdf_data(&invoice, &items, &company, &counterparty);
            let path = ensure_invoice_output_dir(&invoice.number)?;
            generate_invoice_pdf(&data, &path)?;
            path
        }
        DocumentRef::Waybill(_) => {
            anyhow::bail!("PDF для накладних не підтримується");
        }
    };

    if let Err(e) = open::that(&path) {
        tracing::warn!("Не вдалось відкрити PDF: {e}");
    }

    Ok(MutationResultDto {
        ok: true,
        document_id: doc_id,
        message: format!("PDF збережено: {}", path.display()),
    })
}
```

- [x] **Step 4: Перевірка компіляції**

```bash
cargo build --lib 2>&1 | tail -5
```
Очікується: `Finished` без errors.

---

## Task 5: Tauri command shim + реєстрація

**Files:**
- Modify: `src-tauri/src/commands/documents.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Додати command shim**

В кінці `src-tauri/src/commands/documents.rs` додай:

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

- [x] **Step 2: Зареєструвати в `invoke_handler!`**

У `src-tauri/src/lib.rs` знайди блок `invoke_handler!`. Після рядка:
```rust
commands::documents::document_chain_create_draft,
```
Додай:
```rust
commands::documents::document_generate_pdf,
```

- [x] **Step 3: Повна компіляція з тестами**

```bash
cargo build --tests 2>&1 | tail -5
```
Очікується: `Finished` без errors.

- [x] **Step 4: Коміт**

```bash
git add src/tauri_api/documents.rs src-tauri/src/commands/documents.rs src-tauri/src/lib.rs
git commit -m "feat(pdf): add document_generate_pdf Tauri command"
```

---

## Task 6: Frontend — API + store + кнопка

**Files:**
- Modify: `frontend/src/lib/api.ts`
- Modify: `frontend/src/lib/stores/documents.ts`
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`

- [x] **Step 1: Додати `documentGeneratePdf` у `api.ts`**

У `frontend/src/lib/api.ts` знайди останній `export function` перед кінцем файлу (наразі це `importBasExecute`). Додай перед ним:

```ts
export function documentGeneratePdf(docId: string): Promise<MutationResultDto> {
  return appInvoke("document_generate_pdf", { docId });
}
```

- [x] **Step 2: Додати `generatePdf` у `stores/documents.ts`**

У `frontend/src/lib/stores/documents.ts` в об'єкт що повертає `createDocumentsStore()` додай метод після `advanceStatus`:

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

Додай `documentGeneratePdf` до існуючого import з `"../api"`:
```ts
import {
  // ... існуючі імпорти ...
  documentGeneratePdf
} from "../api";
```

- [x] **Step 3: Додати кнопку у `DocumentsScreen.svelte`**

У `frontend/src/lib/screens/DocumentsScreen.svelte` знайди блок `editor-actions`:
```svelte
<div class="editor-actions">
  <button class="btn-ghost" on:click={() => documents.addItem()}>Додати позицію</button>
  <button class="btn-primary" on:click={() => documents.save()}>Зберегти</button>
  <button class="btn-secondary" on:click={() => documents.advanceStatus()}>Наступний статус</button>
  <button class="btn-danger" on:click={onDeleteCurrent}>Видалити</button>
  <button class="btn-ghost" on:click={() => documents.closeEditor()}>Закрити</button>
</div>
```

Замінь на:
```svelte
<div class="editor-actions">
  <button class="btn-ghost" on:click={() => documents.addItem()}>Додати позицію</button>
  <button class="btn-primary" on:click={() => documents.save()}>Зберегти</button>
  <button class="btn-secondary" on:click={() => documents.advanceStatus()}>Наступний статус</button>
  {#if ['act', 'invoice'].includes($documents.editor.form.kind)}
    <button class="btn-secondary" on:click={() => documents.generatePdf()}>PDF</button>
  {/if}
  <button class="btn-danger" on:click={onDeleteCurrent}>Видалити</button>
  <button class="btn-ghost" on:click={() => documents.closeEditor()}>Закрити</button>
</div>
```

- [x] **Step 4: TypeScript перевірка**

```bash
cd frontend && npx tsc --noEmit 2>&1 | head -20
```
Очікується: без errors.

- [x] **Step 5: Коміт**

```bash
git add frontend/src/lib/api.ts frontend/src/lib/stores/documents.ts frontend/src/lib/screens/DocumentsScreen.svelte
git commit -m "feat(pdf): wire PDF button in document editor frontend"
```

---

## Перевірка завершення

Після виконання всіх задач:

```bash
cargo build --tests && cargo test --lib pdf_tests
```
Очікується: `Finished` + всі pdf_tests `ok`.

Для ручного end-to-end тесту:
1. `cd src-tauri && cargo tauri dev`
2. Відкрити документ типу "акт" або "рахунок"
3. Натиснути кнопку «PDF» — має відкритись PDF файл у системному переглядачі
4. Перевірити що файл з'явився в `storage/documents/acts/{рік}/` або `storage/documents/invoices/{рік}/`
5. Переконатись що для "накладна" кнопки немає

---

## Статус реалізації

**Повністю реалізовано** — 2026-05-01

| Задача | Коміт | Статус |
|--------|-------|--------|
| Task 1: видалити `#![allow(dead_code)]` | `8be3db1` | ✅ |
| Task 2: imports + `to_pdf_company` / `counterparty_to_pdf_company` | `8be3db1` | ✅ |
| Task 3: `build_act_pdf_data` / `build_invoice_pdf_data` + тести | `cb00619` | ✅ |
| Task 4: `generate_document_pdf` | `06eeca9` | ✅ |
| Task 5: Tauri command shim + реєстрація | `af86b68` | ✅ |
| Task 6: frontend API + store + кнопка «PDF» | `991a6d8` | ✅ |
| Post-plan: шляхи для production builds | `eb289fb` | ✅ |

**Пост-план зміни (не в оригінальному scope):**
- `generator.rs` — `generate_act_pdf` / `generate_invoice_pdf` / `ensure_output_dir` / `ensure_invoice_output_dir` приймають явні `template_path: &Path` і `storage_dir: &Path` замість CWD-відносних рядків.
- `AppCtx` — додано `template_dir` / `storage_dir` поля з CWD-defaults, конструктор `with_dirs()`, аксесори.
- `runtime.rs` — `init_app_ctx_with_paths(template_dir, storage_dir)`.
- `src-tauri/src/lib.rs` — розв'язує шляхи через `app.path().resource_dir()` / `app_local_data_dir()`.
- `tauri.conf.json` — `"resources": ["../templates/*"]` для бандлу шаблонів.
