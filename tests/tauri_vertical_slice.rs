use anyhow::{anyhow, Result};
use chrono::Utc;
use std::sync::Arc;

#[tokio::test]
async fn tauri_vertical_slice_shell_and_documents_smoke() -> Result<()> {
    let _ = dotenvy::dotenv();

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

        Ok(())
    }
    .await;

    for doc_id in cleanup_doc_ids.into_iter().rev() {
        let _ = acta::tauri_api::documents::document_delete(&ctx, doc_id).await;
    }

    document_result?;

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

    Ok(())
}
