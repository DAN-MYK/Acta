use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use super::api::{parse_document_ref, DocumentRef};
use super::dto::{DocumentPdfStateDto, MutationResultDto};
use crate::app_ctx::AppCtx;
use crate::db;
use crate::models::act::{Act, ActItem};
use crate::models::company::Company;
use crate::models::counterparty::Counterparty;
use crate::models::invoice::{Invoice, InvoiceItem};
use crate::pdf::generator::{
    amount_to_words, ensure_invoice_output_dir, ensure_output_dir, generate_act_pdf,
    generate_invoice_pdf, PdfActData, PdfActItem, PdfCompany, PdfInvoiceData, PdfInvoiceItem,
};
use crate::pdf::reader::inspect_pdf;

pub(super) async fn load_existing_pdf_path(
    storage_dir: &Path,
    pool: &PgPool,
    doc_ref: DocumentRef,
) -> Result<Option<String>> {
    let stored_path = match doc_ref {
        DocumentRef::Act(_) => None,
        DocumentRef::Invoice(id) => db::invoices::get_by_id(pool, id)
            .await?
            .and_then(|(invoice, _)| invoice.pdf_path),
        DocumentRef::Waybill(id) => db::waybills::get_by_id(pool, id)
            .await?
            .and_then(|(waybill, _)| waybill.pdf_path),
    };

    Ok(stored_path.map(|path| {
        resolve_stored_pdf_path(storage_dir, &path)
            .display()
            .to_string()
    }))
}

pub(super) async fn persist_existing_pdf_path(
    pool: &PgPool,
    doc_ref: DocumentRef,
    path: String,
) -> Result<()> {
    match doc_ref {
        DocumentRef::Act(_) => anyhow::bail!("Для актів flow існуючого PDF поки не підтримується"),
        DocumentRef::Invoice(id) => {
            db::invoices::set_pdf_path(pool, id, Some(path))
                .await?
                .ok_or_else(|| anyhow!("Рахунок не знайдено"))?;
        }
        DocumentRef::Waybill(id) => {
            db::waybills::set_pdf_path(pool, id, Some(path))
                .await?
                .ok_or_else(|| anyhow!("Накладну не знайдено"))?;
        }
    }

    Ok(())
}

pub(super) async fn load_document_kind_and_number(
    pool: &PgPool,
    doc_ref: DocumentRef,
) -> Result<(String, String)> {
    match doc_ref {
        DocumentRef::Act(id) => {
            let (act, _) = db::acts::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Акт не знайдено"))?;
            Ok(("act".to_string(), act.number))
        }
        DocumentRef::Invoice(id) => {
            let (invoice, _) = db::invoices::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Рахунок не знайдено"))?;
            Ok(("invoice".to_string(), invoice.number))
        }
        DocumentRef::Waybill(id) => {
            let (waybill, _) = db::waybills::get_by_id(pool, id)
                .await?
                .ok_or_else(|| anyhow!("Накладну не знайдено"))?;
            Ok(("waybill".to_string(), waybill.number))
        }
    }
}

pub(super) fn supports_existing_pdf_flow(kind: &str) -> bool {
    matches!(kind, "invoice" | "waybill")
}

pub(super) fn document_ref_uuid(doc_ref: DocumentRef) -> Uuid {
    match doc_ref {
        DocumentRef::Act(id) | DocumentRef::Invoice(id) | DocumentRef::Waybill(id) => id,
    }
}

pub(super) fn sanitize_pdf_fragment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            'A'..='Z'
            | 'a'..='z'
            | '0'..='9'
            | 'А'..='Я'
            | 'а'..='я'
            | 'І'
            | 'і'
            | 'Ї'
            | 'ї'
            | 'Є'
            | 'є'
            | '_'
            | '-' => ch,
            _ => '_',
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if sanitized.is_empty() {
        "document".to_string()
    } else {
        sanitized
    }
}

pub(super) fn managed_existing_pdf_dir(
    storage_dir: &Path,
    kind: &str,
    doc_id: Uuid,
    number: &str,
) -> PathBuf {
    storage_dir.join(managed_existing_pdf_relative_dir(kind, doc_id, number))
}

pub(super) fn managed_existing_pdf_relative_dir(kind: &str, doc_id: Uuid, number: &str) -> PathBuf {
    PathBuf::from("existing_pdf")
        .join(kind)
        .join(format!("{doc_id}_{}", sanitize_pdf_fragment(number)))
}

pub(super) fn managed_existing_pdf_relative_path(
    kind: &str,
    doc_id: Uuid,
    number: &str,
) -> PathBuf {
    managed_existing_pdf_relative_dir(kind, doc_id, number).join("working.pdf")
}

pub(super) fn resolve_stored_pdf_path(storage_dir: &Path, stored_path: &str) -> PathBuf {
    let path = PathBuf::from(stored_path);
    if path.is_absolute() {
        path
    } else {
        storage_dir.join(path)
    }
}

fn normalize_stored_pdf_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Перевіряє що `source_path` має розширення PDF і не перетинається з керованою
/// директорією `storage_dir/existing_pdf/` після canonicalize. Це захищає attach-flow
/// від випадку коли користувач (або зловмисник) вказує існуючий керований PDF як
/// джерело — інакше copy(source -> working.pdf) переписав би оригінал самим собою
/// або вкрав би чужий PDF в managed dir.
pub(super) fn ensure_attach_source_safe(
    storage_dir: &Path,
    source_path: &Path,
) -> Result<PathBuf> {
    let extension_ok = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);
    anyhow::ensure!(
        extension_ok,
        "Файл має бути PDF (розширення .pdf)"
    );

    let canonical_source = std::fs::canonicalize(source_path).with_context(|| {
        format!(
            "Не вдалось нормалізувати шлях до PDF-файлу: {}",
            source_path.display()
        )
    })?;

    let canonical_managed_root = std::fs::canonicalize(storage_dir.join("existing_pdf"))
        .ok();
    if let Some(root) = canonical_managed_root {
        anyhow::ensure!(
            !canonical_source.starts_with(&root),
            "PDF-джерело знаходиться всередині керованої директорії — виберіть зовнішній файл"
        );
    }

    Ok(canonical_source)
}

/// Перевіряє що `db_path` (звідки PDF буде прочитано/перезаписано) після canonicalize
/// знаходиться саме у `managed_existing_pdf_dir(storage_dir, kind, doc_id, number)`.
/// Захищає replace/open flow від redirect на довільний шлях через manipulation БД
/// або застарілі absolute paths поза storage_dir.
pub(super) fn ensure_managed_pdf_path(
    storage_dir: &Path,
    kind: &str,
    doc_id: Uuid,
    number: &str,
    db_path: &Path,
) -> Result<PathBuf> {
    let canonical_path = std::fs::canonicalize(db_path).with_context(|| {
        format!(
            "Не вдалось нормалізувати збережений шлях до PDF: {}",
            db_path.display()
        )
    })?;

    let expected_dir = managed_existing_pdf_dir(storage_dir, kind, doc_id, number);
    let canonical_expected = std::fs::canonicalize(&expected_dir).with_context(|| {
        format!(
            "Керована директорія для PDF не існує: {}",
            expected_dir.display()
        )
    })?;

    anyhow::ensure!(
        canonical_path.starts_with(&canonical_expected),
        "Збережений PDF лежить поза керованою директорією документа — операцію скасовано"
    );

    Ok(canonical_path)
}

pub(super) async fn inspect_document_pdf_state(file_path: String) -> DocumentPdfStateDto {
    let task_path = file_path.clone();
    match tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(&task_path);
        if !path.exists() {
            return DocumentPdfStateDto {
                file_path: task_path,
                page_count: 0,
                extracted_text: String::new(),
                has_text_ops: false,
                editable: false,
                warnings: vec!["Керований PDF-файл не знайдено на диску.".to_string()],
            };
        }

        match inspect_pdf(&path) {
            Ok(summary) => DocumentPdfStateDto {
                file_path: path.display().to_string(),
                page_count: summary.page_count,
                extracted_text: summary.extracted_text,
                has_text_ops: summary.has_text_ops,
                editable: summary.editable,
                warnings: summary.warnings,
            },
            Err(error) => DocumentPdfStateDto {
                file_path: path.display().to_string(),
                page_count: 0,
                extracted_text: String::new(),
                has_text_ops: false,
                editable: false,
                warnings: vec![error.to_string()],
            },
        }
    })
    .await
    {
        Ok(state) => state,
        Err(error) => DocumentPdfStateDto {
            file_path,
            page_count: 0,
            extracted_text: String::new(),
            has_text_ops: false,
            editable: false,
            warnings: vec![format!("Не вдалось виконати inspection PDF: {error}")],
        },
    }
}

pub(super) async fn attach_existing_pdf_copy(
    storage_dir: PathBuf,
    kind: String,
    doc_id: Uuid,
    number: String,
    source_path: String,
) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let source = PathBuf::from(&source_path);
        if !source.exists() {
            return Err(anyhow!("PDF-файл не знайдено: {}", source.display()));
        }

        let target_dir = managed_existing_pdf_dir(&storage_dir, &kind, doc_id, &number);
        std::fs::create_dir_all(&target_dir).with_context(|| {
            format!(
                "Не вдалось створити директорію для керованого PDF: {}",
                target_dir.display()
            )
        })?;

        let original_path = target_dir.join("original.pdf");
        let working_path = target_dir.join("working.pdf");

        std::fs::copy(&source, &original_path).with_context(|| {
            format!(
                "Не вдалось скопіювати оригінальний PDF у керовану папку: {}",
                original_path.display()
            )
        })?;
        std::fs::copy(&source, &working_path).with_context(|| {
            format!(
                "Не вдалось створити робочу копію PDF: {}",
                working_path.display()
            )
        })?;

        Ok(normalize_stored_pdf_path(&managed_existing_pdf_relative_path(
            &kind, doc_id, &number,
        )))
    })
    .await
    .context("PDF copy thread error")?
}

pub(super) async fn open_pdf_file(file_path: String) -> Result<()> {
    tokio::task::spawn_blocking(move || open::that(file_path))
        .await
        .context("PDF open thread error")?
        .context("Не вдалось відкрити PDF у системному переглядачі")?;
    Ok(())
}

pub(super) fn to_pdf_company(c: &Company) -> PdfCompany {
    PdfCompany {
        name: c.name.clone(),
        edrpou: c.edrpou.clone().unwrap_or_default(),
        iban: c.iban.clone().unwrap_or_default(),
        address: c
            .actual_address
            .clone()
            .or_else(|| c.legal_address.clone())
            .unwrap_or_default(),
    }
}

pub(super) fn counterparty_to_pdf_company(cp: &Counterparty) -> PdfCompany {
    PdfCompany {
        name: cp.name.clone(),
        edrpou: cp
            .edrpou
            .clone()
            .or_else(|| cp.ipn.clone())
            .unwrap_or_default(),
        iban: cp.iban.clone().unwrap_or_default(),
        address: cp.address.clone().unwrap_or_default(),
    }
}

pub(super) fn build_act_pdf_data(
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

pub(super) fn build_invoice_pdf_data(
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
            let path = ensure_output_dir(ctx.storage_dir(), &act.number)?;
            let template = ctx.template_dir().join("act.typ");
            let out = path.clone();
            tokio::task::spawn_blocking(move || generate_act_pdf(&data, &template, &out))
                .await
                .context("PDF thread error")??;
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
            let path = ensure_invoice_output_dir(ctx.storage_dir(), &invoice.number)?;
            let template = ctx.template_dir().join("invoice.typ");
            let out = path.clone();
            tokio::task::spawn_blocking(move || generate_invoice_pdf(&data, &template, &out))
                .await
                .context("PDF thread error")??;
            path
        }
        DocumentRef::Waybill(_) => {
            anyhow::bail!("PDF для накладних не підтримується");
        }
    };

    let open_path = path.clone();
    if let Ok(Err(e)) = tokio::task::spawn_blocking(move || open::that(open_path)).await {
        tracing::warn!("Не вдалось відкрити PDF: {e}");
    }

    Ok(MutationResultDto {
        ok: true,
        document_id: doc_id,
        message: format!("PDF збережено: {}", path.display()),
    })
}

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
        assert_eq!(item.price, "45000.00"); // unit_price → price у PdfActItem
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
        assert_eq!(item.unit, "шт"); // Option<String> → String
        assert_eq!(item.price, "500.00"); // InvoiceItem.price (not unit_price)
        assert_eq!(item.amount, "1000.00");
    }

    #[test]
    fn build_invoice_pdf_data_handles_none_unit() {
        let invoice = sample_invoice();
        let mut items = sample_invoice_items();
        items[0].unit = None; // Option<String> = None
        let data =
            build_invoice_pdf_data(&invoice, &items, &sample_company(), &sample_counterparty());
        assert_eq!(data.items[0].unit, ""); // unwrap_or_default
    }

    #[test]
    fn generate_document_pdf_rejects_waybill_id() {
        // parse_document_ref → DocumentRef::Waybill → bail!
        let wbl_id = format!("wbl:{}", uuid::Uuid::nil());
        let doc_ref = parse_document_ref(&wbl_id);
        assert!(matches!(doc_ref, Some(DocumentRef::Waybill(_))));
    }

    #[test]
    fn managed_existing_pdf_dir_is_unique_per_document_id() {
        let storage_dir = Path::new("storage");
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();

        let first = managed_existing_pdf_dir(storage_dir, "invoice", first_id, "INV-001");
        let second = managed_existing_pdf_dir(storage_dir, "invoice", second_id, "INV-001");

        assert_ne!(first, second);
        assert!(first.to_string_lossy().contains(&first_id.to_string()));
        assert!(second.to_string_lossy().contains(&second_id.to_string()));
    }

    #[test]
    fn managed_existing_pdf_relative_path_targets_working_copy() {
        let doc_id = Uuid::new_v4();
        let path = managed_existing_pdf_relative_path("invoice", doc_id, "INV-001");
        let normalized = path.to_string_lossy().replace('\\', "/");

        assert!(normalized.starts_with("existing_pdf/invoice/"));
        assert!(normalized.ends_with("/working.pdf"));
        assert!(normalized.contains(&doc_id.to_string()));
    }

    #[test]
    fn resolve_stored_pdf_path_joins_relative_and_preserves_absolute() {
        let storage_dir = Path::new("storage/documents");
        let relative = "existing_pdf/invoice/test/working.pdf";
        let resolved_relative = resolve_stored_pdf_path(storage_dir, relative);
        let expected_relative = storage_dir.join(PathBuf::from(relative));

        assert_eq!(resolved_relative, expected_relative);

        let absolute = std::env::temp_dir().join("acta-working.pdf");
        let resolved_absolute =
            resolve_stored_pdf_path(storage_dir, absolute.to_string_lossy().as_ref());

        assert_eq!(resolved_absolute, absolute);
    }
}
