use anyhow::{anyhow, Result};
use chrono::Utc;
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
            "bulk advance status РјР°С” РїРѕРІРµСЂС‚Р°С‚Рё РїРѕРІС–РґРѕРјР»РµРЅРЅСЏ Р· Р»С–С‡РёР»СЊРЅРёРєРѕРј РѕР±СЂРѕР±Р»РµРЅРёС… РґРѕРєСѓРјРµРЅС‚С–РІ"
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
            .ok_or_else(|| anyhow!("bulk-status draft РјР°С” Р±СѓС‚Рё РїСЂРёСЃСѓС‚РЅС–Р№ Сѓ СЃРїРёСЃРєСѓ"))?;
        assert_ne!(
            advanced_bulk_item.status_label, "Р§РµСЂРЅРµС‚РєР°",
            "РїС–СЃР»СЏ bulk advance status РґРѕРєСѓРјРµРЅС‚ РЅРµ РјР°С” Р»РёС€Р°С‚РёСЃСЏ Сѓ С‡РµСЂРЅРµС‚С†С–"
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
                density: if original_settings.preferences.density >= 2 {
                    0
                } else {
                    original_settings.preferences.density + 1
                },
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
            density: i32::from(original_config.density),
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

    // 2. Створити тестовий платіж
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let create_result = acta::tauri_api::payments::payment_create_or_update(
        &ctx,
        acta::tauri_api::payments::PaymentCreateOrUpdateRequest {
            id: String::new(),
            date: today,
            amount: "100.00".to_string(),
            direction: "income".to_string(),
            counterparty_id: String::new(),
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
        dashboard_after_create
            .upcoming_payments
            .iter()
            .any(|payment| payment.id == new_payment_id),
        "dashboard upcoming payments мають віддавати id реального payment record для drill-in"
    );

    // Виконати решту кроків з гарантованим cleanup
    let payment_result: Result<()> = async {
        // 4. Позначити як зведений
        let reconcile_result =
            acta::tauri_api::payments::payment_reconcile(&ctx, new_payment_id.clone()).await?;
        assert!(
            reconcile_result.ok,
            "payment_reconcile має повернути ok=true"
        );

        // 5. Зняти позначку зведення
        let unreconcile_result =
            acta::tauri_api::payments::payment_unreconcile(&ctx, new_payment_id.clone()).await?;
        assert!(
            unreconcile_result.ok,
            "payment_unreconcile має повернути ok=true"
        );

        // 6. Імпорт CSV — допустимі як Ok, так і Err (файл може бути відсутній)
        let _ = acta::tauri_api::payments::payments_import_latest_csv(&ctx).await;

        Ok(())
    }
    .await;

    // 7. Cleanup: видалити тестовий платіж
    let payment_uuid = uuid::Uuid::parse_str(&new_payment_id)
        .map_err(|e| anyhow!("не вдалося розпарсити UUID платежу: {}", e))?;
    acta::db::payments::delete_scoped(ctx.pool(), ctx.company_id(), payment_uuid).await?;

    payment_result?;

    Ok(())
}
