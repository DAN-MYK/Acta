use anyhow::{anyhow, Result};
use chrono::{TimeZone, Utc};
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};
use rust_decimal_macros::dec;
use std::sync::Arc;
use std::sync::OnceLock;
use uuid::Uuid;

fn tauri_vertical_slice_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn newest_file_path(dir: &str) -> Result<Option<std::path::PathBuf>> {
    let dir = std::path::Path::new(dir);
    if !dir.exists() {
        return Ok(None);
    }

    let mut newest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }

        let modified = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        match &newest {
            Some((current, _)) if modified <= *current => {}
            _ => newest = Some((modified, path)),
        }
    }

    Ok(newest.map(|(_, path)| path))
}

fn save_supported_pdf(path: &std::path::Path) -> Result<()> {
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
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
            Operation::new("Td", vec![50.into(), 750.into()]),
            Operation::new("Tj", vec![Object::string_literal("DRAFT STATUS")]),
            Operation::new("ET", vec![]),
        ],
    }
    .encode()?;
    let content_id = doc.add_object(Stream::new(dictionary! {}, content));

    doc.objects.insert(
        page_id,
        Object::Dictionary(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        }),
    );
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();
    doc.save(path)?;
    Ok(())
}

#[tokio::test]
async fn tauri_vertical_slice_existing_pdf_flow_stores_relative_paths_and_blocks_unsafe_reuse(
) -> Result<()> {
    let _guard = tauri_vertical_slice_lock().lock().await;
    let _ = dotenvy::dotenv();

    let pool = acta::runtime::connect_pool().await?;
    let company_id = acta::runtime::get_first_company_id(&pool).await;
    let suffix = Uuid::new_v4().simple().to_string();
    let temp_root = std::env::temp_dir().join(format!("acta-existing-pdf-flow-{suffix}"));
    let storage_dir = temp_root.join("storage").join("documents");
    std::fs::create_dir_all(&storage_dir)?;
    let ctx = Arc::new(acta::app_ctx::AppCtx::with_dirs(
        pool,
        company_id,
        std::env::current_dir()?.join("templates"),
        storage_dir.clone(),
    ));

    let counterparty = acta::db::counterparties::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewCounterparty {
            name: format!("Existing Pdf Counterparty {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("existing-pdf-cp-{suffix}")),
        },
    )
    .await?;

    let invoice = acta::db::invoices::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewInvoice {
            number: format!("VERT-PDF-INV-{suffix}"),
            counterparty_id: counterparty.id,
            contract_id: None,
            category_id: None,
            direction: acta::models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            notes: None,
            bas_id: Some(format!("existing-pdf-invoice-{suffix}")),
            items: vec![acta::models::NewInvoiceItem {
                position: 1,
                description: "Invoice PDF".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(250.00),
            }],
        },
    )
    .await?;

    let waybill = acta::db::waybills::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewWaybill {
            number: format!("VERT-PDF-WBL-{suffix}"),
            counterparty_id: counterparty.id,
            contract_id: None,
            category_id: None,
            direction: acta::models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            notes: None,
            bas_id: Some(format!("existing-pdf-waybill-{suffix}")),
            items: vec![acta::models::NewWaybillItem {
                position: 1,
                description: "Waybill PDF".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(175.00),
            }],
        },
    )
    .await?;

    let invoice_doc_id = format!("inv:{}", invoice.id);
    let waybill_doc_id = format!("wbl:{}", waybill.id);
    let source_pdf = temp_root.join("source.pdf");
    save_supported_pdf(&source_pdf)?;

    let invoice_attach = acta::tauri_api::documents::document_pdf_attach_existing(
        &ctx,
        invoice_doc_id.clone(),
        source_pdf.display().to_string(),
    )
    .await?;
    let waybill_attach = acta::tauri_api::documents::document_pdf_attach_existing(
        &ctx,
        waybill_doc_id.clone(),
        source_pdf.display().to_string(),
    )
    .await?;

    let stored_invoice_path = sqlx::query_scalar::<_, Option<String>>(
        "SELECT pdf_path FROM invoices WHERE id = $1",
    )
    .bind(invoice.id)
    .fetch_one(ctx.pool())
    .await?;
    let stored_waybill_path = sqlx::query_scalar::<_, Option<String>>(
        "SELECT pdf_path FROM waybills WHERE id = $1",
    )
    .bind(waybill.id)
    .fetch_one(ctx.pool())
    .await?;

    assert_eq!(
        stored_invoice_path.as_deref(),
        Some(
            format!(
                "existing_pdf/invoice/{}_VERT-PDF-INV-{suffix}/working.pdf",
                invoice.id
            )
            .as_str()
        )
    );
    assert_eq!(
        stored_waybill_path.as_deref(),
        Some(
            format!(
                "existing_pdf/waybill/{}_VERT-PDF-WBL-{suffix}/working.pdf",
                waybill.id
            )
            .as_str()
        )
    );
    assert!(
        invoice_attach
            .editor
            .pdf
            .as_ref()
            .is_some_and(|pdf| pdf.extracted_text.contains("DRAFT"))
    );
    assert!(
        waybill_attach
            .editor
            .pdf
            .as_ref()
            .is_some_and(|pdf| pdf.extracted_text.contains("DRAFT"))
    );

    let invoice_replace = acta::tauri_api::documents::document_pdf_apply_text_replace(
        &ctx,
        acta::tauri_api::documents::ReplaceDocumentPdfTextRequest {
            doc_id: invoice_doc_id.clone(),
            find_text: "DRAFT".to_string(),
            replace_text: "PAID".to_string(),
        },
    )
    .await?;
    let waybill_replace = acta::tauri_api::documents::document_pdf_apply_text_replace(
        &ctx,
        acta::tauri_api::documents::ReplaceDocumentPdfTextRequest {
            doc_id: waybill_doc_id.clone(),
            find_text: "DRAFT".to_string(),
            replace_text: "SIGNED".to_string(),
        },
    )
    .await?;
    assert!(
        invoice_replace
            .editor
            .pdf
            .as_ref()
            .is_some_and(|pdf| pdf.extracted_text.contains("PAID"))
    );
    assert!(
        waybill_replace
            .editor
            .pdf
            .as_ref()
            .is_some_and(|pdf| pdf.extracted_text.contains("SIGNED"))
    );

    let managed_invoice_path = storage_dir.join(
        stored_invoice_path
            .clone()
            .expect("invoice pdf path має існувати після attach"),
    );
    let reuse_err = acta::tauri_api::documents::document_pdf_attach_existing(
        &ctx,
        invoice_doc_id.clone(),
        managed_invoice_path.display().to_string(),
    )
    .await
    .expect_err("attach не має приймати вже керований working.pdf як source");
    assert!(reuse_err.to_string().contains("керованої директорії"));

    let outside_pdf = temp_root.join("outside.pdf");
    save_supported_pdf(&outside_pdf)?;
    sqlx::query("UPDATE invoices SET pdf_path = $2 WHERE id = $1")
        .bind(invoice.id)
        .bind(outside_pdf.display().to_string())
        .execute(ctx.pool())
        .await?;
    sqlx::query("UPDATE waybills SET pdf_path = $2 WHERE id = $1")
        .bind(waybill.id)
        .bind(outside_pdf.display().to_string())
        .execute(ctx.pool())
        .await?;

    let invoice_replace_err = acta::tauri_api::documents::document_pdf_apply_text_replace(
        &ctx,
        acta::tauri_api::documents::ReplaceDocumentPdfTextRequest {
            doc_id: invoice_doc_id.clone(),
            find_text: "PAID".to_string(),
            replace_text: "DONE".to_string(),
        },
    )
    .await
    .expect_err("replace має відхиляти pdf path поза керованою директорією");
    assert!(invoice_replace_err
        .to_string()
        .contains("поза керованою директорією"));

    let waybill_open_err =
        acta::tauri_api::documents::document_pdf_open_current(&ctx, waybill_doc_id.clone())
            .await
            .expect_err("open має відхиляти pdf path поза керованою директорією");
    assert!(waybill_open_err
        .to_string()
        .contains("поза керованою директорією"));

    let _ = acta::db::invoices::delete(ctx.pool(), invoice.id).await;
    let _ = acta::db::waybills::delete(ctx.pool(), waybill.id).await;
    let _ = sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(counterparty.id)
        .execute(ctx.pool())
        .await;
    let _ = std::fs::remove_dir_all(&temp_root);

    Ok(())
}

#[tokio::test]
async fn tauri_vertical_slice_shell_and_documents_smoke() -> Result<()> {
    let _guard = tauri_vertical_slice_lock().lock().await;
    let _ = dotenvy::dotenv();
    std::env::set_var("ACTA_CONFIG_DIR", "storage/test-config");

    let pool = acta::runtime::connect_pool().await?;
    let company_id = acta::runtime::get_first_company_id(&pool).await;
    let ctx = Arc::new(acta::app_ctx::AppCtx::new(pool, company_id));

    let shell = acta::tauri_api::shell::shell_load(&ctx).await?;
    assert!(
        !shell.company_items.is_empty(),
        "має бути хоча б одна компанія"
    );

    let palette = acta::tauri_api::shell::shell_palette_search(
        &ctx,
        acta::tauri_api::shell::PaletteSearchRequestDto {
            query: "док".to_string(),
            selected_counterparty_id: None,
        },
    )
    .await?;
    assert!(
        !palette.items.is_empty(),
        "палітра має повертати хоча б базові navigation entries"
    );

    let dashboard = acta::tauri_api::dashboard::dashboard_load(&ctx).await?;
    assert!(
        !dashboard.kpis.is_empty(),
        "дашборд має повертати KPI для Tauri shell"
    );
    assert!(
        dashboard.kpis.iter().any(|kpi| kpi.label == "Документи"),
        "дашборд має включати документний KPI"
    );

    let list = acta::tauri_api::documents::documents_list(
        &ctx,
        acta::tauri_api::documents::DocumentsListRequest::default(),
    )
    .await?;
    if let Some(item) = list.items.first() {
        let editor = acta::tauri_api::documents::document_open(&ctx, item.id.clone()).await?;
        assert_eq!(editor.form.id, item.id);
        assert!(
            !editor.form.kind.is_empty(),
            "тип документа має бути заповнений"
        );
    }

    let counterparty = acta::db::counterparties::list(ctx.pool(), ctx.company_id())
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("для smoke-test потрібен хоча б один контрагент"))?;

    let mut cleanup_doc_ids = Vec::new();

    let document_result: Result<()> = async {
        let draft = acta::tauri_api::documents::document_create_draft(
            &ctx,
            acta::tauri_api::documents::CreateDocumentDraftRequest {
                counterparty_id: counterparty.id.to_string(),
                kind: "invoice".to_string(),
                direction: "outgoing".to_string(),
            },
        )
        .await?;
        cleanup_doc_ids.push(draft.form.id.clone());

        let save_response = acta::tauri_api::documents::document_save(
            &ctx,
            acta::tauri_api::documents::SaveDocumentRequest {
                form: acta::tauri_api::documents::DocumentDraftFormDto {
                    notes: "Smoke vertical slice".to_string(),
                    ..draft.form.clone()
                },
                items: vec![acta::tauri_api::documents::DocumentDraftItemDto {
                    description: "Тестова позиція".to_string(),
                    unit: "послуга".to_string(),
                    quantity: "1".to_string(),
                    price: "1234.50".to_string(),
                }],
            },
        )
        .await?;
        assert_eq!(save_response.document_id, draft.form.id);

        let saved_editor =
            acta::tauri_api::documents::document_open(&ctx, draft.form.id.clone()).await?;
        assert_eq!(saved_editor.form.notes, "Smoke vertical slice");
        assert_eq!(saved_editor.items.len(), 1);

        let initial_chain =
            acta::tauri_api::documents::document_chain_get(&ctx, draft.form.id.clone()).await?;
        assert!(
            !initial_chain.steps.is_empty(),
            "ланцюжок має повертати хоча б базові кроки"
        );

        let chained_editor = acta::tauri_api::documents::document_chain_create_draft(
            &ctx,
            acta::tauri_api::documents::CreateChainDraftRequest {
                source_id: draft.form.id.clone(),
                target_kind: "act".to_string(),
            },
        )
        .await?;
        cleanup_doc_ids.push(chained_editor.form.id.clone());
        assert_eq!(chained_editor.form.kind, "act");
        assert_eq!(chained_editor.items.len(), 1);

        let updated_chain =
            acta::tauri_api::documents::document_chain_get(&ctx, draft.form.id.clone()).await?;
        assert!(
            updated_chain
                .steps
                .iter()
                .any(|step| step.doc_type == "act" && step.exists),
            "після chain-create у ланцюжку має з'явитися акт"
        );

        let status_result =
            acta::tauri_api::documents::document_advance_status(&ctx, draft.form.id.clone())
                .await?;
        assert!(status_result.ok, "advance status має завершитись успішно");

        let refreshed_list = acta::tauri_api::documents::documents_list(
            &ctx,
            acta::tauri_api::documents::DocumentsListRequest::default(),
        )
        .await?;
        let advanced_item = refreshed_list
            .items
            .iter()
            .find(|item| item.id == draft.form.id)
            .ok_or_else(|| anyhow!("створений документ має бути присутній у списку"))?;
        assert_ne!(
            advanced_item.status_label, "Чернетка",
            "після зміни статусу документ не має лишатися у чернетці"
        );

        let bulk_draft = acta::tauri_api::documents::document_create_draft(
            &ctx,
            acta::tauri_api::documents::CreateDocumentDraftRequest {
                counterparty_id: counterparty.id.to_string(),
                kind: "invoice".to_string(),
                direction: "outgoing".to_string(),
            },
        )
        .await?;
        cleanup_doc_ids.push(bulk_draft.form.id.clone());

        let bulk_delete_result = acta::tauri_api::documents::documents_bulk_delete_live(
            &ctx,
            acta::tauri_api::documents::BulkDocumentRequest {
                doc_ids: vec![bulk_draft.form.id.clone()],
            },
        )
        .await?;
        assert_eq!(bulk_delete_result.total, 1);
        assert_eq!(bulk_delete_result.succeeded, 1);
        assert_eq!(bulk_delete_result.failed, 0);
        assert!(
            bulk_delete_result.message.contains("Видалено 1 документ"),
            "bulk delete має повертати зрозуміле повідомлення"
        );

        let bulk_status_draft = acta::tauri_api::documents::document_create_draft(
            &ctx,
            acta::tauri_api::documents::CreateDocumentDraftRequest {
                counterparty_id: counterparty.id.to_string(),
                kind: "invoice".to_string(),
                direction: "outgoing".to_string(),
            },
        )
        .await?;
        cleanup_doc_ids.push(bulk_status_draft.form.id.clone());

        let bulk_status_result = acta::tauri_api::documents::documents_bulk_advance_status_live(
            &ctx,
            acta::tauri_api::documents::BulkDocumentRequest {
                doc_ids: vec![bulk_status_draft.form.id.clone()],
            },
        )
        .await?;
        assert_eq!(bulk_status_result.total, 1);
        assert_eq!(bulk_status_result.succeeded, 1);
        assert_eq!(bulk_status_result.failed, 0);
        assert!(
            bulk_status_result.message.contains("1"),
            "bulk advance status має повертати повідомлення з лічильником оброблених документів"
        );

        let bulk_status_list = acta::tauri_api::documents::documents_list(
            &ctx,
            acta::tauri_api::documents::DocumentsListRequest::default(),
        )
        .await?;
        let advanced_bulk_item = bulk_status_list
            .items
            .iter()
            .find(|item| item.id == bulk_status_draft.form.id)
            .ok_or_else(|| anyhow!("bulk-status draft має бути присутній у списку"))?;
        assert_ne!(
            advanced_bulk_item.status_label, "Чернетка",
            "після bulk advance status документ не має лишатися у чернетці"
        );

        Ok(())
    }
    .await;

    for doc_id in cleanup_doc_ids.into_iter().rev() {
        let _ = acta::tauri_api::documents::document_delete(&ctx, doc_id).await;
    }

    document_result?;

    let counterparties_before = acta::tauri_api::counterparties::counterparties_list(
        &ctx,
        acta::tauri_api::counterparties::CounterpartiesListRequest::default(),
    )
    .await?;
    let initial_counterparty_count = counterparties_before.items.len();
    let initial_selected_counterparty_id = counterparties_before
        .items
        .first()
        .map(|item| item.id.clone())
        .ok_or_else(|| anyhow!("для counterparties smoke потрібен хоча б один контрагент"))?;

    let selected_detail = acta::tauri_api::counterparties::counterparty_get(
        &ctx,
        initial_selected_counterparty_id.clone(),
    )
    .await?;
    assert_eq!(selected_detail.info.id, initial_selected_counterparty_id);

    let new_counterparty_editor =
        acta::tauri_api::counterparties::counterparty_open_editor(&ctx, None).await?;
    assert!(
        new_counterparty_editor.form.id.is_empty(),
        "новий редактор контрагента має повертати порожній id"
    );

    let counterparties_result: Result<()> = async {
        let suffix = &Uuid::new_v4().simple().to_string()[..8];
        let save_result = acta::tauri_api::counterparties::counterparty_save(
            &ctx,
            acta::tauri_api::counterparties::CounterpartySaveRequest {
                form: acta::tauri_api::counterparties::CounterpartyDraftFormDto {
                    id: String::new(),
                    title: "Новий контрагент".to_string(),
                    name: format!("Smoke Counterparty {suffix}"),
                    edrpou: String::new(),
                    ipn: String::new(),
                    iban: "UA123456789012345678901234567".to_string(),
                    address: "м. Київ".to_string(),
                    phone: "+380501112233".to_string(),
                    email: "smoke@example.com".to_string(),
                    notes: "Created by Tauri smoke".to_string(),
                },
            },
        )
        .await?;
        assert!(save_result.ok);
        assert_eq!(
            save_result.updated_list.len(),
            initial_counterparty_count + 1
        );

        let saved_id = save_result.saved_id.clone();
        let edit_editor =
            acta::tauri_api::counterparties::counterparty_open_editor(&ctx, Some(saved_id.clone()))
                .await?;
        assert_eq!(edit_editor.form.id, saved_id);
        assert!(edit_editor.form.name.contains("Smoke Counterparty"));

        let updated_name = format!("{} Updated", edit_editor.form.name);
        let updated_result = acta::tauri_api::counterparties::counterparty_save(
            &ctx,
            acta::tauri_api::counterparties::CounterpartySaveRequest {
                form: acta::tauri_api::counterparties::CounterpartyDraftFormDto {
                    name: updated_name.clone(),
                    ..edit_editor.form
                },
            },
        )
        .await?;
        assert!(updated_result.ok);
        assert_eq!(
            updated_result
                .updated_detail
                .as_ref()
                .map(|detail| detail.info.name.as_str()),
            Some(updated_name.as_str())
        );

        let create_doc_context =
            acta::tauri_api::counterparties::counterparty_create_document_context(
                &ctx,
                saved_id.clone(),
            )
            .await?;
        assert_eq!(create_doc_context.counterparty_id, saved_id);
        assert_eq!(create_doc_context.counterparty_name, updated_name);

        let archive_result =
            acta::tauri_api::counterparties::counterparty_archive(&ctx, saved_id.clone()).await?;
        assert!(archive_result.ok);

        let after_archive = acta::tauri_api::counterparties::counterparties_list(
            &ctx,
            acta::tauri_api::counterparties::CounterpartiesListRequest::default(),
        )
        .await?;
        assert!(
            after_archive.items.iter().all(|item| item.id != saved_id),
            "архівований контрагент не має лишатися у списку активних"
        );

        Ok(())
    }
    .await;

    counterparties_result?;

    let tasks_before = acta::tauri_api::tasks::tasks_list(
        &ctx,
        acta::tauri_api::tasks::TasksListRequest::default(),
    )
    .await?;
    let initial_task_count = tasks_before.items.len();

    let created_task = acta::tauri_api::tasks::task_save(
        &ctx,
        acta::tauri_api::tasks::TaskSaveRequest {
            form: acta::tauri_api::tasks::TaskDraftFormDto {
                id: String::new(),
                title: "Smoke task".to_string(),
                description: "Перевірити Tauri tasks slice".to_string(),
                priority: "high".to_string(),
                due_date: Utc::now().format("%Y-%m-%d").to_string(),
                reminder_at: String::new(),
                status: "open".to_string(),
                counterparty_id: String::new(),
                act_id: String::new(),
                link_kind: String::new(),
                link_label: String::new(),
            },
        },
    )
    .await?;
    assert!(created_task.ok);
    assert_eq!(
        created_task.updated_list.items.len(),
        initial_task_count + 1
    );

    let task_id = created_task.saved_id.clone();
    let task_editor = acta::tauri_api::tasks::task_open_editor(&ctx, Some(task_id.clone())).await?;
    assert_eq!(task_editor.form.title, "Smoke task");
    assert_eq!(task_editor.form.priority, "high");

    let updated_task = acta::tauri_api::tasks::task_save(
        &ctx,
        acta::tauri_api::tasks::TaskSaveRequest {
            form: acta::tauri_api::tasks::TaskDraftFormDto {
                title: "Smoke task updated".to_string(),
                status: "in_progress".to_string(),
                ..task_editor.form
            },
        },
    )
    .await?;
    assert_eq!(updated_task.saved_id, task_id);
    assert_eq!(
        updated_task
            .updated_editor
            .as_ref()
            .map(|editor| editor.form.title.as_str()),
        Some("Smoke task updated")
    );
    assert_eq!(
        updated_task
            .updated_editor
            .as_ref()
            .map(|editor| editor.form.status.as_str()),
        Some("in_progress")
    );

    let status_result =
        acta::tauri_api::tasks::task_set_status(&ctx, task_id.clone(), "done".to_string()).await?;
    assert!(status_result.ok);

    let done_editor = acta::tauri_api::tasks::task_open_editor(&ctx, Some(task_id.clone())).await?;
    assert_eq!(done_editor.form.status, "done");

    let delete_result = acta::tauri_api::tasks::task_delete(&ctx, task_id.clone()).await?;
    assert!(delete_result.ok);

    let tasks_after = acta::tauri_api::tasks::tasks_list(
        &ctx,
        acta::tauri_api::tasks::TasksListRequest::default(),
    )
    .await?;
    assert_eq!(tasks_after.items.len(), initial_task_count);
    assert!(
        tasks_after.items.iter().all(|item| item.id != task_id),
        "видалене завдання не має повертатися у списку"
    );

    let original_config = acta::config::AppConfig::load();
    let original_settings = acta::tauri_api::settings::settings_load(&ctx).await?;
    assert!(!original_settings.numbering.is_empty());

    let integration_path = std::path::PathBuf::from("storage/integrations/bas.json");
    let integration_existed = integration_path.exists();
    let integration_backup = if integration_existed {
        Some(std::fs::read_to_string(&integration_path)?)
    } else {
        None
    };
    let team_before = original_settings.team.len();
    let backup_before = newest_file_path("storage/backups")?;
    let invite_before = newest_file_path("storage/team/invites")?;

    let settings_result: Result<()> = async {
        let toggled_preferences = acta::tauri_api::settings::settings_save_preferences(
            &ctx,
            acta::tauri_api::settings::SettingsPreferencesRequest {
                dark_mode: !original_settings.preferences.dark_mode,
            },
        )
        .await?;
        assert!(toggled_preferences.ok);
        assert_eq!(
            toggled_preferences.screen.preferences.dark_mode,
            !original_settings.preferences.dark_mode
        );

        let shell_after_preferences = acta::tauri_api::shell::shell_load(&ctx).await?;
        assert_eq!(
            shell_after_preferences.is_dark,
            toggled_preferences.screen.preferences.dark_mode
        );

        let mut updated_company = original_settings.company.clone();
        let suffix = Uuid::new_v4().simple().to_string();
        updated_company.full_name = format!("{} smoke {}", updated_company.full_name, &suffix[..8]);

        let saved_company = acta::tauri_api::settings::settings_save_company(
            &ctx,
            acta::tauri_api::settings::SettingsSaveCompanyRequest {
                company: updated_company.clone(),
            },
        )
        .await?;
        assert!(saved_company.ok);
        assert_eq!(
            saved_company.screen.company.full_name,
            updated_company.full_name
        );

        let configured_integration = acta::tauri_api::settings::settings_configure_integration(
            &ctx,
            acta::tauri_api::settings::SettingsIntegrationActionRequest {
                tag: "bas".to_string(),
            },
        )
        .await?;
        assert!(configured_integration.ok);
        assert!(configured_integration
            .screen
            .integrations
            .iter()
            .any(|item| item.tag == "bas" && item.enabled));
        assert!(integration_path.exists());

        let invited = acta::tauri_api::settings::settings_team_invite(&ctx).await?;
        assert!(invited.ok);
        assert_eq!(invited.screen.team.len(), team_before + 1);

        let backup_created = acta::tauri_api::settings::settings_backup_now(&ctx).await?;
        assert!(backup_created.ok);

        let opened_backup = acta::tauri_api::settings::settings_backup_open_latest(&ctx).await?;
        assert!(opened_backup.ok);
        assert!(std::path::Path::new(&opened_backup.path).exists());

        Ok(())
    }
    .await;

    let restore_preferences = acta::tauri_api::settings::settings_save_preferences(
        &ctx,
        acta::tauri_api::settings::SettingsPreferencesRequest {
            dark_mode: original_config.dark_mode,
        },
    )
    .await;

    let restore_company = acta::tauri_api::settings::settings_save_company(
        &ctx,
        acta::tauri_api::settings::SettingsSaveCompanyRequest {
            company: original_settings.company.clone(),
        },
    )
    .await;

    match integration_backup {
        Some(text) => {
            let _ = std::fs::write(&integration_path, text);
        }
        None if integration_path.exists() => {
            let _ = std::fs::remove_file(&integration_path);
        }
        None => {}
    }

    if let Ok(Some(path)) = newest_file_path("storage/backups") {
        if Some(path.clone()) != backup_before {
            let _ = std::fs::remove_file(path);
        }
    }

    if let Ok(Some(path)) = newest_file_path("storage/team/invites") {
        if Some(path.clone()) != invite_before {
            let _ = std::fs::remove_file(path);
        }
    }

    if let Ok(result) = restore_preferences {
        assert_eq!(
            result.screen.preferences.dark_mode,
            original_config.dark_mode
        );
    }
    if let Ok(result) = restore_company {
        assert_eq!(
            result.screen.company.full_name,
            original_settings.company.full_name
        );
    }

    original_config.save();
    settings_result?;

    Ok(())
}

#[tokio::test]
async fn dashboard_load_uses_expected_receivables_for_upcoming_payments() -> Result<()> {
    let _guard = tauri_vertical_slice_lock().lock().await;
    let _ = dotenvy::dotenv();
    std::env::set_var("ACTA_CONFIG_DIR", "storage/test-config");

    let pool = acta::runtime::connect_pool().await?;
    let company_id = acta::runtime::get_first_company_id(&pool).await;
    let ctx = Arc::new(acta::app_ctx::AppCtx::new(pool, company_id));

    let suffix = Uuid::new_v4().simple().to_string();
    let counterparty = acta::db::counterparties::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewCounterparty {
            name: format!("Dashboard Receivable Counterparty {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("dash-rec-cp-{}", &suffix[..24])),
        },
    )
    .await?;

    let due_date = chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
        .ok_or_else(|| anyhow!("не вдалося розрахувати дату очікуваної оплати"))?;
    let act = acta::db::acts::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewAct {
            number: format!("DASH-REC-{}", &suffix[..24]),
            counterparty_id: counterparty.id,
            contract_id: None,
            category_id: None,
            direction: acta::models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: Some(due_date),
            status: acta::models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![acta::models::NewActItem {
                description: "Очікувана оплата для dashboard".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(432.10),
            }],
        },
    )
    .await?;

    let payment_ref = format!("DASH-UNMATCHED-{}", &suffix[..24]);
    acta::tauri_api::payments::payment_create_or_update(
        &ctx,
        acta::tauri_api::payments::PaymentCreateOrUpdateRequest {
            id: String::new(),
            date: Utc::now().date_naive().format("%Y-%m-%d").to_string(),
            amount: "432.10".to_string(),
            direction: "income".to_string(),
            counterparty_id: counterparty.id.to_string(),
            counterparty_name: String::new(),
            bank_name: String::new(),
            reference: payment_ref.clone(),
            description: "Нерознесений платіж не має бути upcoming receivable".to_string(),
        },
    )
    .await?;
    let payment_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM payments WHERE company_id = $1 AND bank_ref = $2 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(ctx.company_id())
    .bind(&payment_ref)
    .fetch_one(ctx.pool())
    .await?;

    let dashboard = acta::tauri_api::dashboard::dashboard_load(&ctx).await?;
    let upcoming_ids = dashboard
        .upcoming_payments
        .iter()
        .map(|payment| payment.id.as_str())
        .collect::<Vec<_>>();
    let act_id = act.id.to_string();
    let payment_id_text = payment_id.to_string();

    assert!(
        upcoming_ids.contains(&act_id.as_str()),
        "dashboard upcoming payments мають брати id акту з expected_payment_date"
    );
    assert!(
        !upcoming_ids.contains(&payment_id_text.as_str()),
        "нерознесений банківський платіж не має виглядати як очікувана оплата"
    );

    acta::db::payments::delete_scoped(ctx.pool(), ctx.company_id(), payment_id).await?;
    let _ = acta::db::acts::delete(ctx.pool(), act.id).await;
    let _ = sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(counterparty.id)
        .execute(ctx.pool())
        .await;

    Ok(())
}

#[tokio::test]
async fn tauri_vertical_slice_payments_smoke() -> Result<()> {
    let _guard = tauri_vertical_slice_lock().lock().await;
    let _ = dotenvy::dotenv();
    std::env::set_var("ACTA_CONFIG_DIR", "storage/test-config");

    let pool = acta::runtime::connect_pool().await?;
    let company_id = acta::runtime::get_first_company_id(&pool).await;
    let ctx = Arc::new(acta::app_ctx::AppCtx::new(pool, company_id));

    // 1. Завантажити список платежів і перевірити KPI
    let screen_before = acta::tauri_api::payments::payments_list(&ctx).await?;
    let kpi = &screen_before.kpi;
    assert!(
        !kpi.incoming_str.is_empty(),
        "kpi.incoming_str має бути непорожнім"
    );
    assert!(
        !kpi.outgoing_str.is_empty(),
        "kpi.outgoing_str має бути непорожнім"
    );
    assert!(!kpi.net_str.is_empty(), "kpi.net_str має бути непорожнім");
    assert!(
        !kpi.unmatched_str.is_empty(),
        "kpi.unmatched_str має бути непорожнім"
    );
    let count_before = screen_before.items.len();
    let suffix = Uuid::new_v4().simple().to_string();
    let counterparty = acta::db::counterparties::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewCounterparty {
            name: format!("Tauri Payment Counterparty {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("tauri-payments-cp-{suffix}")),
        },
    )
    .await?;
    let act = acta::db::acts::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewAct {
            number: format!("TAURI-PAYMENT-ACT-{suffix}"),
            counterparty_id: counterparty.id,
            contract_id: None,
            category_id: None,
            direction: acta::models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: acta::models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![acta::models::NewActItem {
                description: "Вертикальний slice".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(100.00),
            }],
        },
    )
    .await?;

    // 2. Створити тестовий платіж
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let create_result = acta::tauri_api::payments::payment_create_or_update(
        &ctx,
        acta::tauri_api::payments::PaymentCreateOrUpdateRequest {
            id: String::new(),
            date: today,
            amount: "100.00".to_string(),
            direction: "income".to_string(),
            counterparty_id: counterparty.id.to_string(),
            counterparty_name: String::new(),
            bank_name: String::new(),
            reference: String::new(),
            description: String::new(),
        },
    )
    .await?;
    assert!(
        create_result.ok,
        "payment_create_or_update має повернути ok=true"
    );

    // 3. Перезавантажити список і знайти створений платіж
    let screen_after = acta::tauri_api::payments::payments_list(&ctx).await?;
    assert_eq!(
        screen_after.items.len(),
        count_before + 1,
        "після створення список має збільшитись на 1"
    );

    // Знайти новий платіж — перший елемент якого не було у попередньому списку
    let before_ids: std::collections::HashSet<&str> =
        screen_before.items.iter().map(|p| p.id.as_str()).collect();
    let new_payment = screen_after
        .items
        .iter()
        .find(|p| !before_ids.contains(p.id.as_str()))
        .ok_or_else(|| anyhow!("новий платіж не знайдено у списку після створення"))?;
    let new_payment_id = new_payment.id.clone();

    let dashboard_after_create = acta::tauri_api::dashboard::dashboard_load(&ctx).await?;
    assert!(
        !dashboard_after_create
            .upcoming_payments
            .iter()
            .any(|payment| payment.id == new_payment_id),
        "dashboard upcoming payments не мають показувати фактичні банківські платежі"
    );

    // Виконати решту кроків з гарантованим cleanup
    let payment_result: Result<()> = async {
        let payment_uuid = Uuid::parse_str(&new_payment_id)?;

        let malformed_amount_err = acta::tauri_api::payments::payment_reconcile(
            &ctx,
            acta::tauri_api::payments::PaymentReconcileRequest {
                payment_id: new_payment_id.clone(),
                document_kind: "act".to_string(),
                document_id: act.id.to_string(),
                amount: "abc".to_string(),
            },
        )
        .await
        .expect_err("malformed amount має відхилятися");
        assert!(
            malformed_amount_err
                .to_string()
                .contains("Невірна сума платежу"),
            "payment_reconcile має повертати узгоджену помилку парсингу суми"
        );

        let zero_amount_err = acta::tauri_api::payments::payment_reconcile(
            &ctx,
            acta::tauri_api::payments::PaymentReconcileRequest {
                payment_id: new_payment_id.clone(),
                document_kind: "act".to_string(),
                document_id: act.id.to_string(),
                amount: "0".to_string(),
            },
        )
        .await
        .expect_err("нульова amount має відхилятися");
        assert!(
            zero_amount_err
                .to_string()
                .contains("Сума платежу має бути більшою за нуль"),
            "payment_reconcile не має приймати нульові або від'ємні суми"
        );

        let link_exists_after_failed_requests = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM payment_acts WHERE payment_id = $1 AND act_id = $2)",
        )
        .bind(payment_uuid)
        .bind(act.id)
        .fetch_one(ctx.pool())
        .await?;
        assert!(
            !link_exists_after_failed_requests,
            "після невалідного payload reconcile не має створювати link"
        );

        // 4. Позначити як зведений
        let reconcile_result = acta::tauri_api::payments::payment_reconcile(
            &ctx,
            acta::tauri_api::payments::PaymentReconcileRequest {
                payment_id: new_payment_id.clone(),
                document_kind: "act".to_string(),
                document_id: act.id.to_string(),
                amount: "100,00".to_string(),
            },
        )
        .await?;
        assert!(
            reconcile_result.ok,
            "payment_reconcile має повернути ok=true"
        );

        // 5. Зняти позначку зведення
        let act_link_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM payment_acts WHERE payment_id = $1 AND act_id = $2)",
        )
        .bind(payment_uuid)
        .bind(act.id)
        .fetch_one(ctx.pool())
        .await?;
        assert!(
            act_link_exists,
            "reconcile має зберігати link у payment_acts"
        );

        let act_link_amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
            "SELECT amount FROM payment_acts WHERE payment_id = $1 AND act_id = $2",
        )
        .bind(payment_uuid)
        .bind(act.id)
        .fetch_one(ctx.pool())
        .await?;
        assert_eq!(
            act_link_amount,
            dec!(100.00),
            "localized amount має парситися до канонічного Decimal"
        );

        let reconciled_payment =
            acta::db::payments::get_by_id_scoped(ctx.pool(), ctx.company_id(), payment_uuid)
                .await?
                .ok_or_else(|| anyhow!("створений платіж має існувати після reconcile"))?;
        assert!(
            reconciled_payment.is_reconciled,
            "is_reconciled має бути derived=true після створення link"
        );

        let unreconcile_result = acta::tauri_api::payments::payment_unreconcile(
            &ctx,
            acta::tauri_api::payments::PaymentUnreconcileRequest {
                payment_id: new_payment_id.clone(),
                document_kind: "act".to_string(),
                document_id: act.id.to_string(),
            },
        )
        .await?;
        assert!(
            unreconcile_result.ok,
            "payment_unreconcile має повернути ok=true"
        );

        // 6. Імпорт CSV — допустимі як Ok, так і Err (файл може бути відсутній)
        let act_link_exists_after = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM payment_acts WHERE payment_id = $1 AND act_id = $2)",
        )
        .bind(payment_uuid)
        .bind(act.id)
        .fetch_one(ctx.pool())
        .await?;
        assert!(
            !act_link_exists_after,
            "unreconcile має видаляти link із payment_acts"
        );

        let unreconciled_payment =
            acta::db::payments::get_by_id_scoped(ctx.pool(), ctx.company_id(), payment_uuid)
                .await?
                .ok_or_else(|| anyhow!("створений платіж має існувати після unreconcile"))?;
        assert!(
            !unreconciled_payment.is_reconciled,
            "після видалення всіх links derived state має ставати false"
        );

        let _ = acta::tauri_api::payments::payments_import_latest_csv(&ctx).await;

        Ok(())
    }
    .await;

    // 7. Cleanup: видалити тестовий платіж
    let payment_uuid = uuid::Uuid::parse_str(&new_payment_id)
        .map_err(|e| anyhow!("не вдалося розпарсити UUID платежу: {}", e))?;
    acta::db::payments::delete_scoped(ctx.pool(), ctx.company_id(), payment_uuid).await?;
    let _ = acta::db::acts::delete(ctx.pool(), act.id).await;
    let _ = sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(counterparty.id)
        .execute(ctx.pool())
        .await;

    payment_result?;

    Ok(())
}

#[tokio::test]
async fn tauri_vertical_slice_payment_match_preview_and_apply_auto_exact() -> Result<()> {
    let _guard = tauri_vertical_slice_lock().lock().await;
    let _ = dotenvy::dotenv();
    std::env::set_var("ACTA_CONFIG_DIR", "storage/test-config");

    let pool = acta::runtime::connect_pool().await?;
    let company_id = acta::runtime::get_first_company_id(&pool).await;
    let ctx = Arc::new(acta::app_ctx::AppCtx::new(pool, company_id));

    let suffix = Uuid::new_v4().simple().to_string();
    let counterparty = acta::db::counterparties::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewCounterparty {
            name: format!("Preview Exact Counterparty {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: Some("UA123456789012345678901234567".to_string()),
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("preview-exact-cp-{suffix}")),
        },
    )
    .await?;

    let act_number = format!("AUTO-ACT-{suffix}");
    let invoice_number = format!("AUTO-INV-{suffix}");
    let payment_ref = format!("BANK-REF-{suffix}");
    let today = Utc::now().date_naive();

    let act = acta::db::acts::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewAct {
            number: act_number.clone(),
            counterparty_id: counterparty.id,
            contract_id: None,
            category_id: None,
            direction: acta::models::DocumentDirection::Outgoing,
            date: today,
            expected_payment_date: Some(today),
            status: acta::models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![acta::models::NewActItem {
                description: "Preview exact act".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(150.00),
            }],
        },
    )
    .await?;

    let invoice = acta::db::invoices::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewInvoice {
            number: invoice_number,
            counterparty_id: counterparty.id,
            contract_id: None,
            category_id: None,
            direction: acta::models::DocumentDirection::Outgoing,
            date: today,
            expected_payment_date: Some(today),
            notes: None,
            items: vec![acta::models::NewInvoiceItem {
                position: 1,
                description: "Preview exact invoice".to_string(),
                quantity: dec!(1.0000),
                unit: Some("шт".to_string()),
                price: dec!(150.00),
            }],
            bas_id: None,
        },
    )
    .await?;
    acta::db::invoices::change_status(
        ctx.pool(),
        invoice.id,
        acta::models::invoice::InvoiceStatus::Issued,
    )
    .await?
    .ok_or_else(|| anyhow!("створена накладна має існувати для preview exact"))?;

    acta::tauri_api::payments::payment_create_or_update(
        &ctx,
        acta::tauri_api::payments::PaymentCreateOrUpdateRequest {
            id: String::new(),
            date: today.format("%Y-%m-%d").to_string(),
            amount: "150.00".to_string(),
            direction: "income".to_string(),
            counterparty_id: counterparty.id.to_string(),
            counterparty_name: String::new(),
            bank_name: String::new(),
            reference: payment_ref.clone(),
            description: format!("Оплата {act_number}"),
        },
    )
    .await?;

    let payment_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM payments WHERE company_id = $1 AND bank_ref = $2 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(ctx.company_id())
    .bind(&payment_ref)
    .fetch_one(ctx.pool())
    .await?;

    let preview = acta::tauri_api::payments::payment_match_preview(
        &ctx,
        acta::tauri_api::payments::PaymentMatchPreviewRequest {
            payment_id: payment_id.to_string(),
        },
    )
    .await?;
    assert_eq!(preview.payment_id, payment_id.to_string());
    assert!(!preview.is_reconciled);
    assert_eq!(preview.decision_kind, "exact");
    assert!(
        preview.candidates.len() >= 2,
        "preview має повертати список кандидатів для review"
    );
    let top_candidate = preview
        .candidates
        .first()
        .ok_or_else(|| anyhow!("для exact preview має бути хоча б один кандидат"))?;
    assert_eq!(top_candidate.document_id, act.id.to_string());
    assert_eq!(top_candidate.document_kind, "act");
    assert!(top_candidate.same_iban);
    assert!(top_candidate.text_hits >= 1);
    let recommendation = preview
        .auto_match
        .ok_or_else(|| anyhow!("exact preview має повертати auto-match recommendation"))?;
    assert_eq!(recommendation.document_id, act.id.to_string());
    assert_eq!(recommendation.document_kind, "act");

    let apply_result = acta::tauri_api::payments::payment_match_apply_auto(
        &ctx,
        acta::tauri_api::payments::PaymentMatchApplyAutoRequest {
            payment_id: payment_id.to_string(),
        },
    )
    .await?;
    assert!(
        apply_result.ok,
        "apply-auto має завершуватися успішно для exact match"
    );

    let act_link_amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_acts WHERE payment_id = $1 AND act_id = $2",
    )
    .bind(payment_id)
    .bind(act.id)
    .fetch_one(ctx.pool())
    .await?;
    assert_eq!(
        act_link_amount,
        dec!(150.00),
        "auto-apply має звіряти платіж на повну суму exact кандидата"
    );

    let payment = acta::db::payments::get_by_id_scoped(ctx.pool(), ctx.company_id(), payment_id)
        .await?
        .ok_or_else(|| anyhow!("створений платіж має існувати після apply-auto"))?;
    assert!(payment.is_reconciled);

    acta::db::payments::delete_scoped(ctx.pool(), ctx.company_id(), payment_id).await?;
    let _ = acta::db::invoices::delete(ctx.pool(), invoice.id).await;
    let _ = acta::db::acts::delete(ctx.pool(), act.id).await;
    let _ = sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(counterparty.id)
        .execute(ctx.pool())
        .await;

    Ok(())
}

#[tokio::test]
async fn tauri_vertical_slice_payment_match_preview_ambiguous_refuses_auto_apply() -> Result<()> {
    let _guard = tauri_vertical_slice_lock().lock().await;
    let _ = dotenvy::dotenv();
    std::env::set_var("ACTA_CONFIG_DIR", "storage/test-config");

    let pool = acta::runtime::connect_pool().await?;
    let company_id = acta::runtime::get_first_company_id(&pool).await;
    let ctx = Arc::new(acta::app_ctx::AppCtx::new(pool, company_id));

    let suffix = Uuid::new_v4().simple().to_string();
    let counterparty = acta::db::counterparties::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewCounterparty {
            name: format!("Preview Ambiguous Counterparty {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("preview-ambiguous-cp-{suffix}")),
        },
    )
    .await?;

    let today = Utc::now().date_naive();
    let act = acta::db::acts::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewAct {
            number: format!("AMB-ACT-{suffix}"),
            counterparty_id: counterparty.id,
            contract_id: None,
            category_id: None,
            direction: acta::models::DocumentDirection::Outgoing,
            date: today,
            expected_payment_date: Some(today),
            status: acta::models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![acta::models::NewActItem {
                description: "Ambiguous act".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(210.00),
            }],
        },
    )
    .await?;
    let invoice = acta::db::invoices::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewInvoice {
            number: format!("AMB-INV-{suffix}"),
            counterparty_id: counterparty.id,
            contract_id: None,
            category_id: None,
            direction: acta::models::DocumentDirection::Outgoing,
            date: today,
            expected_payment_date: Some(today),
            notes: None,
            items: vec![acta::models::NewInvoiceItem {
                position: 1,
                description: "Ambiguous invoice".to_string(),
                quantity: dec!(1.0000),
                unit: Some("шт".to_string()),
                price: dec!(210.00),
            }],
            bas_id: None,
        },
    )
    .await?;
    acta::db::invoices::change_status(
        ctx.pool(),
        invoice.id,
        acta::models::invoice::InvoiceStatus::Issued,
    )
    .await?
    .ok_or_else(|| anyhow!("створена накладна має існувати для preview ambiguous"))?;

    let payment_ref = format!("AMB-PAYMENT-{suffix}");
    acta::tauri_api::payments::payment_create_or_update(
        &ctx,
        acta::tauri_api::payments::PaymentCreateOrUpdateRequest {
            id: String::new(),
            date: today.format("%Y-%m-%d").to_string(),
            amount: "210.00".to_string(),
            direction: "income".to_string(),
            counterparty_id: counterparty.id.to_string(),
            counterparty_name: String::new(),
            bank_name: String::new(),
            reference: payment_ref.clone(),
            description: "Оплата послуг".to_string(),
        },
    )
    .await?;

    let payment_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM payments WHERE company_id = $1 AND bank_ref = $2 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(ctx.company_id())
    .bind(&payment_ref)
    .fetch_one(ctx.pool())
    .await?;

    let preview = acta::tauri_api::payments::payment_match_preview(
        &ctx,
        acta::tauri_api::payments::PaymentMatchPreviewRequest {
            payment_id: payment_id.to_string(),
        },
    )
    .await?;
    assert_eq!(preview.decision_kind, "ambiguous");
    assert!(
        preview.candidates.len() >= 2,
        "ambiguous preview має повертати всі рівноцінні або релевантні exact кандидати"
    );
    let act_id = act.id.to_string();
    let invoice_id = invoice.id.to_string();
    let candidate_ids = preview
        .candidates
        .iter()
        .map(|candidate| candidate.document_id.as_str())
        .collect::<Vec<_>>();
    assert!(candidate_ids.contains(&act_id.as_str()));
    assert!(candidate_ids.contains(&invoice_id.as_str()));
    assert!(
        preview.auto_match.is_none(),
        "ambiguous preview не має пропонувати auto-match"
    );

    let apply_err = acta::tauri_api::payments::payment_match_apply_auto(
        &ctx,
        acta::tauri_api::payments::PaymentMatchApplyAutoRequest {
            payment_id: payment_id.to_string(),
        },
    )
    .await
    .expect_err("ambiguous preview має блокувати auto-apply");
    assert!(
        apply_err.to_string().contains("Автозіставлення неможливе"),
        "для ambiguous apply-auto має повертати зрозумілу помилку"
    );

    let payment = acta::db::payments::get_by_id_scoped(ctx.pool(), ctx.company_id(), payment_id)
        .await?
        .ok_or_else(|| anyhow!("створений платіж має існувати після відмови auto-apply"))?;
    assert!(
        !payment.is_reconciled,
        "ambiguous auto-apply не має змінювати reconcile state"
    );

    let has_act_link = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM payment_acts WHERE payment_id = $1)",
    )
    .bind(payment_id)
    .fetch_one(ctx.pool())
    .await?;
    let has_invoice_link = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM payment_invoices WHERE payment_id = $1)",
    )
    .bind(payment_id)
    .fetch_one(ctx.pool())
    .await?;
    assert!(!has_act_link && !has_invoice_link);

    acta::db::payments::delete_scoped(ctx.pool(), ctx.company_id(), payment_id).await?;
    let _ = acta::db::invoices::delete(ctx.pool(), invoice.id).await;
    let _ = acta::db::acts::delete(ctx.pool(), act.id).await;
    let _ = sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(counterparty.id)
        .execute(ctx.pool())
        .await;

    Ok(())
}

#[tokio::test]
async fn tauri_vertical_slice_payment_reconcile_split_is_atomic() -> Result<()> {
    let _guard = tauri_vertical_slice_lock().lock().await;
    let _ = dotenvy::dotenv();
    std::env::set_var("ACTA_CONFIG_DIR", "storage/test-config");

    let pool = acta::runtime::connect_pool().await?;
    let company_id = acta::runtime::get_first_company_id(&pool).await;
    let ctx = Arc::new(acta::app_ctx::AppCtx::new(pool, company_id));

    let suffix = Uuid::new_v4().simple().to_string();
    let counterparty = acta::db::counterparties::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewCounterparty {
            name: format!("Vertical Split Counterparty {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("vertical-split-cp-{suffix}")),
        },
    )
    .await?;

    let today = Utc::now().date_naive();
    let act = acta::db::acts::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewAct {
            number: format!("VERT-SPLIT-ACT-{suffix}"),
            counterparty_id: counterparty.id,
            contract_id: None,
            category_id: None,
            direction: acta::models::DocumentDirection::Outgoing,
            date: today,
            expected_payment_date: None,
            status: acta::models::ActStatus::Issued,
            notes: None,
            bas_id: None,
            items: vec![acta::models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "шт".to_string(),
                unit_price: dec!(1500.00),
            }],
        },
    )
    .await?;

    let invoice = acta::db::invoices::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewInvoice {
            number: format!("VERT-SPLIT-INV-{suffix}"),
            counterparty_id: counterparty.id,
            contract_id: None,
            category_id: None,
            direction: acta::models::DocumentDirection::Outgoing,
            date: today,
            expected_payment_date: None,
            notes: None,
            bas_id: None,
            items: vec![acta::models::NewInvoiceItem {
                position: 1,
                description: "Товар".to_string(),
                unit: Some("шт".to_string()),
                quantity: dec!(1.0000),
                price: dec!(1500.00),
            }],
        },
    )
    .await?;

    let payment = acta::db::payments::create(
        ctx.pool(),
        acta::models::payment::NewPayment {
            company_id: ctx.company_id(),
            date: today,
            amount: dec!(3000.00),
            direction: acta::models::payment::PaymentDirection::Income,
            counterparty_id: Some(counterparty.id),
            bank_name: Some("Тест Банк".to_string()),
            bank_ref: Some(format!("VERT-SPLIT-{suffix}")),
            description: Some("Тест vertical split reconcile".to_string()),
        },
    )
    .await?;

    let ok_result = acta::tauri_api::payments::payment_reconcile_split(
        &ctx,
        acta::tauri_api::payments::PaymentReconcileSplitRequest {
            payment_id: payment.id.to_string(),
            allocations: vec![
                acta::tauri_api::payments::PaymentReconcileSplitAllocationRequest {
                    document_kind: "invoice".to_string(),
                    document_id: invoice.id.to_string(),
                    amount: "1 500,00".to_string(),
                },
                acta::tauri_api::payments::PaymentReconcileSplitAllocationRequest {
                    document_kind: "act".to_string(),
                    document_id: act.id.to_string(),
                    amount: "1 500,00".to_string(),
                },
            ],
        },
    )
    .await?;
    assert!(ok_result.ok);
    assert_eq!(ok_result.message, "Розподіл платежу підтверджено");
    assert_eq!(ok_result.payment_id, payment.id.to_string());
    assert_eq!(ok_result.allocation_count, 2);
    assert_eq!(ok_result.total_allocated_str, "3\u{00a0}000,00");
    let allocation_titles = ok_result
        .allocations
        .iter()
        .map(|allocation| allocation.title.clone())
        .collect::<Vec<_>>();
    assert!(allocation_titles.contains(&format!("Накладна VERT-SPLIT-INV-{suffix}")));
    assert!(allocation_titles.contains(&format!("Акт VERT-SPLIT-ACT-{suffix}")));

    let act_amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_acts WHERE payment_id = $1 AND act_id = $2",
    )
    .bind(payment.id)
    .bind(act.id)
    .fetch_one(ctx.pool())
    .await?;
    let invoice_amount = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_invoices WHERE payment_id = $1 AND invoice_id = $2",
    )
    .bind(payment.id)
    .bind(invoice.id)
    .fetch_one(ctx.pool())
    .await?;
    assert_eq!(act_amount, dec!(1500.00));
    assert_eq!(invoice_amount, dec!(1500.00));

    let failed_result = acta::tauri_api::payments::payment_reconcile_split(
        &ctx,
        acta::tauri_api::payments::PaymentReconcileSplitRequest {
            payment_id: payment.id.to_string(),
            allocations: vec![
                acta::tauri_api::payments::PaymentReconcileSplitAllocationRequest {
                    document_kind: "invoice".to_string(),
                    document_id: invoice.id.to_string(),
                    amount: "1 400,00".to_string(),
                },
                acta::tauri_api::payments::PaymentReconcileSplitAllocationRequest {
                    document_kind: "act".to_string(),
                    document_id: act.id.to_string(),
                    amount: "1 600,00".to_string(),
                },
            ],
        },
    )
    .await
    .expect_err("невалідний allocation має відхиляти весь split request");
    assert!(failed_result.to_string().contains("документа"));

    let act_amount_after_error = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_acts WHERE payment_id = $1 AND act_id = $2",
    )
    .bind(payment.id)
    .bind(act.id)
    .fetch_one(ctx.pool())
    .await?;
    let invoice_amount_after_error = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT amount FROM payment_invoices WHERE payment_id = $1 AND invoice_id = $2",
    )
    .bind(payment.id)
    .bind(invoice.id)
    .fetch_one(ctx.pool())
    .await?;
    assert_eq!(act_amount_after_error, dec!(1500.00));
    assert_eq!(invoice_amount_after_error, dec!(1500.00));

    acta::db::payments::delete_scoped(ctx.pool(), ctx.company_id(), payment.id).await?;
    let _ = acta::db::invoices::delete(ctx.pool(), invoice.id).await;
    let _ = acta::db::acts::delete(ctx.pool(), act.id).await;
    let _ = sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(counterparty.id)
        .execute(ctx.pool())
        .await;

    Ok(())
}

#[tokio::test]
async fn tauri_vertical_slice_payment_calendar_load_includes_schedule_and_task_deadlines(
) -> Result<()> {
    let _guard = tauri_vertical_slice_lock().lock().await;
    let _ = dotenvy::dotenv();
    std::env::set_var("ACTA_CONFIG_DIR", "storage/test-config");

    let pool = acta::runtime::connect_pool().await?;
    let company_id = acta::runtime::get_first_company_id(&pool).await;
    let ctx = Arc::new(acta::app_ctx::AppCtx::new(pool, company_id));

    let suffix = Uuid::new_v4().simple().to_string();
    let counterparty = acta::db::counterparties::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewCounterparty {
            name: format!("Calendar Counterparty {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("calendar-cp-{suffix}")),
        },
    )
    .await?;

    let schedule = acta::db::payments::create_schedule(
        ctx.pool(),
        acta::models::payment::NewPaymentSchedule {
            company_id: ctx.company_id(),
            title: format!("Оренда офісу {suffix}"),
            amount: Some(dec!(12500.00)),
            direction: acta::models::payment::PaymentDirection::Expense,
            scheduled_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 6).expect("валидна дата"),
            recurrence: acta::models::payment::ScheduleRecurrence::Monthly,
            recurrence_end: None,
            counterparty_id: Some(counterparty.id),
            notes: Some("Оплатити до обіду".to_string()),
        },
    )
    .await?;

    let task = acta::db::tasks::create(
        ctx.pool(),
        ctx.company_id(),
        &acta::models::NewTask {
            title: format!("Підтвердити платіж {suffix}"),
            description: Some("Звірити суму та реквізити".to_string()),
            priority: acta::models::task::TaskPriority::High,
            due_date: Some(
                chrono::Utc
                    .with_ymd_and_hms(2026, 5, 6, 9, 0, 0)
                    .single()
                    .expect("валидний дедлайн"),
            ),
            reminder_at: None,
            counterparty_id: Some(counterparty.id),
            act_id: None,
        },
    )
    .await?;

    let result = acta::tauri_api::payments::payments_calendar_load(
        &ctx,
        acta::tauri_api::payments::PaymentCalendarMonthRequest {
            month: "2026-05".to_string(),
            selected_date: Some("2026-05-06".to_string()),
        },
    )
    .await?;

    assert_eq!(result.month_label, "Травень 2026");
    let day = result
        .days
        .iter()
        .find(|day| day.date == "2026-05-06")
        .ok_or_else(|| anyhow!("календар має містити день 2026-05-06"))?;
    assert_eq!(day.event_count, 2);
    assert_eq!(day.expense_total_str, "12\u{00a0}500,00");
    assert!(day
        .events
        .iter()
        .any(|event| event.id == schedule.id.to_string() && event.kind == "schedule"));
    assert!(day
        .events
        .iter()
        .any(|event| event.id == task.id.to_string() && event.kind == "task"));

    let complete = acta::tauri_api::payments::payment_schedule_complete(
        &ctx,
        acta::tauri_api::payments::PaymentScheduleCompleteRequest {
            schedule_id: schedule.id.to_string(),
        },
    )
    .await?;
    assert!(complete.ok);

    let updated_schedule =
        sqlx::query_scalar::<_, bool>("SELECT is_completed FROM payment_schedule WHERE id = $1")
            .bind(schedule.id)
            .fetch_one(ctx.pool())
            .await?;
    assert!(updated_schedule);

    let _ = acta::db::tasks::delete(ctx.pool(), task.id).await;
    sqlx::query("DELETE FROM payment_schedule WHERE id = $1")
        .bind(schedule.id)
        .execute(ctx.pool())
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(counterparty.id)
        .execute(ctx.pool())
        .await?;

    Ok(())
}

#[tokio::test]
async fn documents_direction_filter() -> Result<()> {
    use acta::models::DocumentDirection;
    use acta::tauri_api::documents::{
        CreateDocumentDraftRequest, DocumentsListRequest, documents_list, document_create_draft,
        document_delete,
    };

    let _guard = tauri_vertical_slice_lock().lock().await;
    let _ = dotenvy::dotenv();
    std::env::set_var("ACTA_CONFIG_DIR", "storage/test-config");

    let pool = acta::runtime::connect_pool().await?;
    let company_id = acta::runtime::get_first_company_id(&pool).await;
    let ctx = Arc::new(acta::app_ctx::AppCtx::new(pool, company_id));

    let counterparty = acta::db::counterparties::list(ctx.pool(), ctx.company_id())
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("потрібен хоча б один контрагент"))?;

    // Create one outgoing invoice and one incoming act
    let outgoing = document_create_draft(
        &ctx,
        CreateDocumentDraftRequest {
            counterparty_id: counterparty.id.to_string(),
            kind: "invoice".to_string(),
            direction: "outgoing".to_string(),
        },
    )
    .await?;

    let incoming = document_create_draft(
        &ctx,
        CreateDocumentDraftRequest {
            counterparty_id: counterparty.id.to_string(),
            kind: "act".to_string(),
            direction: "incoming".to_string(),
        },
    )
    .await?;

    let cleanup: Vec<String> = vec![outgoing.form.id.clone(), incoming.form.id.clone()];

    let result: Result<()> = async {
        // direction field is populated
        assert_eq!(outgoing.form.direction, "outgoing");
        assert_eq!(incoming.form.direction, "incoming");

        // filter by outgoing — only invoice returned (not the incoming act)
        let outgoing_list = documents_list(
            &ctx,
            DocumentsListRequest { direction: Some(DocumentDirection::Outgoing), ..Default::default() },
        )
        .await?;
        assert!(
            outgoing_list.items.iter().any(|i| i.id == outgoing.form.id),
            "outgoing filter must include outgoing invoice"
        );
        assert!(
            !outgoing_list.items.iter().any(|i| i.id == incoming.form.id),
            "outgoing filter must exclude incoming act"
        );
        assert!(
            outgoing_list.items.iter().all(|i| i.direction == "outgoing"),
            "all items in outgoing filter must have direction=outgoing"
        );

        // filter by incoming — only act returned
        let incoming_list = documents_list(
            &ctx,
            DocumentsListRequest { direction: Some(DocumentDirection::Incoming), ..Default::default() },
        )
        .await?;
        assert!(
            incoming_list.items.iter().any(|i| i.id == incoming.form.id),
            "incoming filter must include incoming act"
        );
        assert!(
            !incoming_list.items.iter().any(|i| i.id == outgoing.form.id),
            "incoming filter must exclude outgoing invoice"
        );

        // filter by kind=act — only acts returned
        let act_list = documents_list(
            &ctx,
            DocumentsListRequest { kind: Some("act".to_string()), ..Default::default() },
        )
        .await?;
        assert!(
            act_list.items.iter().all(|i| i.direction == "incoming" || i.direction == "outgoing"),
            "all items must have direction set"
        );
        assert!(
            act_list.items.iter().all(|i| matches!(i.kind, acta::tauri_api::documents::DocumentKindDto::Act)),
            "kind=act filter must only return acts"
        );

        Ok(())
    }
    .await;

    // cleanup
    for id in cleanup {
        let _ = document_delete(&ctx, id).await;
    }

    result
}
