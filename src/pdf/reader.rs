use std::path::Path;

use anyhow::{anyhow, Context, Result};
use lopdf::content::Content;
use lopdf::{Document, Encoding, Object, ObjectId};

/// Підсумок inspection для існуючого PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfInspection {
    pub page_count: usize,
    pub extracted_text: String,
    pub has_text_ops: bool,
    pub editable: bool,
    pub warnings: Vec<String>,
    pub text_operator_count: usize,
}

/// Результат exact replace у керованій копії PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfReplaceReport {
    pub changed: bool,
    pub occurrences_before: usize,
    pub occurrences_after: usize,
    pub page_count: usize,
    pub warnings: Vec<String>,
    pub extracted_text: String,
}

fn page_numbers(doc: &Document) -> Vec<u32> {
    doc.get_pages().keys().copied().collect()
}

fn count_text_show_operators(doc: &Document, page_id: ObjectId) -> Result<usize> {
    let content = doc
        .get_and_decode_page_content(page_id)
        .context("Не вдалось декодувати content stream сторінки")?;

    Ok(content
        .operations
        .iter()
        .filter(|operation| matches!(operation.operator.as_str(), "Tj" | "TJ"))
        .count())
}

fn replace_substring_in_operands(
    operation: &mut lopdf::content::Operation,
    encoding: &Encoding,
    old_text: &str,
    new_text: &str,
) -> Result<usize> {
    let mut replacements = 0usize;

    for bytes in operation.operands.iter_mut().flat_map(Object::as_str_mut) {
        let decoded_text = Document::decode_text(encoding, bytes)?;
        let occurrence_count = decoded_text.matches(old_text).count();
        if occurrence_count == 0 {
            continue;
        }

        let replaced_text = decoded_text.replace(old_text, new_text);
        *bytes = Document::encode_text(encoding, &replaced_text);
        replacements += occurrence_count;
    }

    Ok(replacements)
}

fn replace_text_in_document(doc: &mut Document, old_text: &str, new_text: &str) -> Result<usize> {
    let mut total_replacements = 0usize;
    let pages = doc.get_pages();

    for page_id in pages.values().copied() {
        let encodings = doc
            .get_page_fonts(page_id)?
            .into_iter()
            .map(|(name, font)| font.get_font_encoding(doc).map(|encoding| (name, encoding)))
            .collect::<Result<std::collections::BTreeMap<Vec<u8>, Encoding>, _>>()?;

        let content_data = doc.get_page_content(page_id)?;
        let mut content = Content::decode(&content_data)?;
        let mut current_encoding: Option<&Encoding> = None;
        let mut page_replacements = 0usize;

        for operation in &mut content.operations {
            match operation.operator.as_str() {
                "Tf" => {
                    let current_font = operation
                        .operands
                        .first()
                        .ok_or_else(|| anyhow!("У PDF відсутній font operand у Tf"))?
                        .as_name()?;
                    current_encoding = encodings.get(current_font);
                }
                "Tj" | "TJ" => {
                    if let Some(encoding) = current_encoding {
                        page_replacements +=
                            replace_substring_in_operands(operation, encoding, old_text, new_text)?;
                    }
                }
                _ => {}
            }
        }

        if page_replacements > 0 {
            doc.change_page_content(page_id, content.encode()?)?;
            total_replacements += page_replacements;
        }
    }

    Ok(total_replacements)
}

fn extract_text_with_warnings(doc: &Document, path: &Path) -> (String, Vec<String>) {
    match doc.extract_text(&page_numbers(doc)) {
        Ok(text) if !text.trim().is_empty() => (text, Vec::new()),
        Ok(_) => (
            String::new(),
            vec![format!(
                "lopdf не витягнув читабельний текст із PDF: {}",
                path.display()
            )],
        ),
        Err(error) => (
            String::new(),
            vec![format!(
                "lopdf не зміг витягти текст із PDF {}: {error}",
                path.display()
            )],
        ),
    }
}

fn can_exact_replace(text_operator_count: usize, extracted_text: &str) -> bool {
    text_operator_count > 0 && !extracted_text.trim().is_empty()
}

/// Перевіряє, чи містить PDF читабельний текст і підтримуваний текстовий шар для exact replace.
pub fn inspect_pdf(path: &Path) -> Result<PdfInspection> {
    let doc = Document::load(path)
        .with_context(|| format!("Не вдалось відкрити PDF: {}", path.display()))?;
    let page_map = doc.get_pages();
    let page_count = page_map.len();
    let (extracted_text, mut warnings) = extract_text_with_warnings(&doc, path);

    let mut text_operator_count = 0usize;
    for page_id in page_map.values().copied() {
        text_operator_count += count_text_show_operators(&doc, page_id)?;
    }

    if text_operator_count == 0 {
        warnings.push(
            "У PDF не знайдено підтримуваних текстових операторів Tj/TJ, тому exact replace недоступний."
                .to_string(),
        );
    }

    if extracted_text.trim().is_empty() && text_operator_count > 0 {
        warnings.push(
            "Text extraction не дав читабельного результату. Це типове обмеження PDF зі складними шрифтами або ToUnicode/CMap."
                .to_string(),
        );
    }

    let has_text_ops = text_operator_count > 0;
    let editable = can_exact_replace(text_operator_count, &extracted_text);

    Ok(PdfInspection {
        page_count,
        extracted_text,
        has_text_ops,
        editable,
        warnings,
        text_operator_count,
    })
}

/// Витягує весь текст з PDF-документу.
pub fn read_pdf_text(path: &Path) -> Result<String> {
    let inspection = inspect_pdf(path)?;
    if inspection.extracted_text.trim().is_empty() {
        return Err(anyhow!(
            "Не вдалось витягти читабельний текст з PDF: {}",
            path.display()
        ));
    }

    Ok(inspection.extracted_text)
}

/// Замінює всі входження `old_text` → `new_text` у підтримуваному текстовому шарі PDF.
///
/// Обмеження `lopdf`: це exact replace поверх content stream і не є універсальним PDF-редактором.
/// Найкраще працює для тексту, який лежить у підтримуваних `Tj`/`TJ`-операторах і коректно
/// декодується через `lopdf`.
pub fn replace_pdf_text(path: &Path, old_text: &str, new_text: &str) -> Result<()> {
    replace_pdf_text_with_report(path, old_text, new_text).map(|_| ())
}

/// Те саме, що `replace_pdf_text`, але з поверненням діагностичного звіту для product-flow.
pub fn replace_pdf_text_with_report(
    path: &Path,
    old_text: &str,
    new_text: &str,
) -> Result<PdfReplaceReport> {
    let trimmed_old_text = old_text.trim();
    if trimmed_old_text.is_empty() {
        return Err(anyhow!("Текст для пошуку в PDF не може бути порожнім"));
    }

    let before = inspect_pdf(path)?;
    let mut doc = Document::load(path)
        .with_context(|| format!("Не вдалось відкрити PDF: {}", path.display()))?;

    let replacements = replace_text_in_document(&mut doc, old_text, new_text)?;

    doc.save(path)
        .with_context(|| format!("Не вдалось зберегти PDF: {}", path.display()))?;

    let after = inspect_pdf(path)?;
    let occurrences_before = before.extracted_text.matches(trimmed_old_text).count();
    let occurrences_after = after.extracted_text.matches(trimmed_old_text).count();
    let changed = before.extracted_text != after.extracted_text;

    let mut warnings = before.warnings;
    for warning in after.warnings {
        if !warnings.contains(&warning) {
            warnings.push(warning);
        }
    }

    if !before.editable {
        warnings.push(
            "У цьому PDF не знайдено підтримуваного текстового шару для exact replace.".to_string(),
        );
    }

    if replacements == 0 || !changed {
        warnings.push(
            "lopdf не змінив текст. Імовірно, точне входження лежить поза підтримуваним сценарієм content stream або PDF не містить такого фрагмента."
                .to_string(),
        );
    }

    Ok(PdfReplaceReport {
        changed,
        occurrences_before,
        occurrences_after,
        page_count: after.page_count,
        warnings,
        extracted_text: after.extracted_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Operation;
    use lopdf::{dictionary, Stream};

    fn save_simple_pdf(path: &Path, operations: Vec<Operation>) {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();

        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });

        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            }
        });

        let content = Content { operations }
            .encode()
            .expect("контент PDF має кодуватися");
        let content_id = doc.add_object(Stream::new(dictionary! {}, content));

        let page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects.insert(page_id, Object::Dictionary(page));

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.compress();
        doc.save(path).expect("PDF має зберегтися");
    }

    fn save_supported_pdf(path: &Path) {
        save_simple_pdf(
            path,
            vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
                Operation::new("Td", vec![50.into(), 750.into()]),
                Operation::new("Tj", vec![Object::string_literal("DRAFT STATUS")]),
                Operation::new("ET", vec![]),
            ],
        );
    }

    fn save_unsupported_pdf(path: &Path) {
        save_simple_pdf(path, vec![]);
    }

    #[test]
    fn read_pdf_text_returns_err_for_missing_file() {
        let result = read_pdf_text(Path::new("__nonexistent__.pdf"));
        assert!(result.is_err());
    }

    #[test]
    fn replace_pdf_text_returns_err_for_missing_file() {
        let result = replace_pdf_text(Path::new("__nonexistent__.pdf"), "DRAFT", "PAID");
        assert!(result.is_err());
    }

    #[test]
    fn inspect_pdf_reports_supported_text_layer() {
        let path = std::env::temp_dir().join("acta_supported_pdf_reader_test.pdf");
        save_supported_pdf(&path);

        let inspection = inspect_pdf(&path).expect("inspect_pdf має завершитись успішно");
        assert_eq!(inspection.page_count, 1);
        assert!(inspection.has_text_ops);
        assert!(inspection.editable);
        assert!(inspection.text_operator_count > 0);
        assert!(inspection.extracted_text.contains("DRAFT"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn inspect_pdf_reports_unsupported_pdf_predictably() {
        let path = std::env::temp_dir().join("acta_unsupported_pdf_reader_test.pdf");
        save_unsupported_pdf(&path);

        let inspection = inspect_pdf(&path).expect("inspect_pdf має завершитись успішно");
        assert_eq!(inspection.page_count, 1);
        assert!(!inspection.has_text_ops);
        assert!(!inspection.editable);
        assert!(inspection
            .warnings
            .iter()
            .any(|warning| warning.contains("Tj/TJ")));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn exact_replace_requires_supported_text_ops_and_readable_text() {
        assert!(!can_exact_replace(0, "DRAFT"));
        assert!(!can_exact_replace(2, ""));
        assert!(!can_exact_replace(2, "   "));
        assert!(can_exact_replace(1, "DRAFT"));
    }

    #[test]
    fn replace_pdf_text_with_report_updates_supported_pdf() {
        let path = std::env::temp_dir().join("acta_replace_pdf_reader_test.pdf");
        save_supported_pdf(&path);

        let report = replace_pdf_text_with_report(&path, "DRAFT", "PAID")
            .expect("replace_pdf_text_with_report має завершитись успішно");
        assert!(report.changed);
        assert!(report.extracted_text.contains("PAID"));
        assert_eq!(report.occurrences_before, 1);
        assert_eq!(report.occurrences_after, 0);

        let _ = std::fs::remove_file(path);
    }

    fn typst_available() -> bool {
        std::process::Command::new("typst")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn typst_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    fn sample_act_data() -> crate::pdf::generator::PdfActData {
        use crate::pdf::generator::{PdfActData, PdfActItem, PdfCompany};
        PdfActData {
            number: "АКТ-2026-001".into(),
            date: "01.05.2026".into(),
            company: PdfCompany {
                name: "ФОП Тест".into(),
                edrpou: "1234567890".into(),
                iban: "UA123456789012345678901234567".into(),
                address: "м. Київ".into(),
            },
            client: PdfCompany {
                name: "ТОВ Замовник".into(),
                edrpou: "9876543210".into(),
                iban: "UA987654321098765432109876543".into(),
                address: "м. Львів".into(),
            },
            items: vec![PdfActItem {
                num: 1,
                name: "Послуга".into(),
                qty: "1.0000".into(),
                unit: "послуга".into(),
                price: "1000.00".into(),
                amount: "1000.00".into(),
            }],
            total: "1000.00".into(),
            total_words: "одна тисяча гривень 00 копійок".into(),
            notes: String::new(),
        }
    }

    #[test]
    #[ignore = "lopdf не підтримує ToUnicode CMap із Typst PDF; запускати вручну через -- --ignored"]
    fn read_pdf_text_extracts_text_from_typst_pdf() {
        if !typst_available() {
            eprintln!("пропуск: typst не встановлено");
            return;
        }
        let _guard = typst_lock().lock().expect("mutex має блокуватись");

        let out = std::env::temp_dir().join("acta_reader_integration.pdf");
        crate::pdf::generator::generate_act_pdf(
            &sample_act_data(),
            std::path::Path::new("templates/act.typ"),
            &out,
        )
        .expect("generate_act_pdf має завершитись успішно");

        let text = read_pdf_text(&out).expect("read_pdf_text має повернути Ok");
        assert!(
            !text.is_empty(),
            "витягнутий текст не повинен бути порожнім"
        );

        let _ = std::fs::remove_file(&out);
    }
}
