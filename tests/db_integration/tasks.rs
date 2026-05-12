use super::super::*;

#[tokio::test]
async fn tasks_create_update_and_delete_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Task Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-task-cp-{suffix}")),
        },
    )
    .await?;

    let new_task = models::NewTask {
        title: format!("ІТ Задача {suffix}"),
        description: Some("перевірка нагадування".to_string()),
        priority: models::TaskPriority::High,
        due_date: Some(Utc::now() + Duration::days(1)),
        reminder_at: Some(Utc::now()),
        counterparty_id: Some(cp.id),
        act_id: None,
    };

    let created = db::tasks::create(&pool, DEFAULT_COMPANY_ID, &new_task).await?;
    assert_eq!(created.status, models::TaskStatus::Open);

    let open_tasks = db::tasks::list_open(&pool, DEFAULT_COMPANY_ID, "").await?;
    assert!(open_tasks.iter().any(|t| t.id == created.id));

    let due = db::tasks::due_reminders(&pool).await?;
    assert!(due.iter().any(|t| t.id == created.id));

    let done = db::tasks::set_status_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        created.id,
        models::TaskStatus::Done,
    )
    .await?
    .expect("status updated");
    assert_eq!(done.status, models::TaskStatus::Done);

    let deleted = db::tasks::delete_scoped(&pool, DEFAULT_COMPANY_ID, created.id).await?;
    assert!(deleted);

    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn tasks_update_and_get_by_id_in_db() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Task Update Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-task-update-cp-{suffix}")),
        },
    )
    .await?;

    let created = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Оновити задачу {suffix}"),
            description: Some("Початковий опис".to_string()),
            priority: models::TaskPriority::Normal,
            due_date: Some(Utc::now() + Duration::days(2)),
            reminder_at: None,
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?;

    let updated = db::tasks::update_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        created.id,
        &models::NewTask {
            title: format!("Оновлена задача {suffix}"),
            description: Some("Оновлений опис".to_string()),
            priority: models::TaskPriority::Critical,
            due_date: Some(Utc::now() + Duration::days(3)),
            reminder_at: Some(Utc::now() + Duration::hours(1)),
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?
    .expect("task updated");

    assert_eq!(updated.title, format!("Оновлена задача {suffix}"));
    assert_eq!(updated.description.as_deref(), Some("Оновлений опис"));
    assert_eq!(updated.priority, models::TaskPriority::Critical);

    let fetched = db::tasks::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, created.id)
        .await?
        .expect("task exists");
    assert_eq!(fetched.title, updated.title);
    assert_eq!(fetched.priority, updated.priority);

    let missing = db::tasks::update_scoped(
        &pool,
        DEFAULT_COMPANY_ID,
        Uuid::new_v4(),
        &models::NewTask {
            title: "Missing".to_string(),
            description: None,
            priority: models::TaskPriority::Low,
            due_date: None,
            reminder_at: None,
            counterparty_id: None,
            act_id: None,
        },
    )
    .await?;
    assert!(missing.is_none());

    sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(created.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn tasks_scoped_mutations_reject_foreign_company() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let foreign_company_id: Uuid =
        sqlx::query_scalar("INSERT INTO companies (name) VALUES ($1) RETURNING id")
            .bind(format!("IT Foreign Task Company {suffix}"))
            .fetch_one(&pool)
            .await?;

    let created = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("IT Scoped Task {suffix}"),
            description: None,
            priority: models::TaskPriority::Normal,
            due_date: None,
            reminder_at: None,
            counterparty_id: None,
            act_id: None,
        },
    )
    .await?;

    assert!(
        db::tasks::get_by_id_scoped(&pool, foreign_company_id, created.id)
            .await?
            .is_none()
    );
    assert!(db::tasks::update_scoped(
        &pool,
        foreign_company_id,
        created.id,
        &models::NewTask {
            title: "Foreign update".to_string(),
            description: None,
            priority: models::TaskPriority::Critical,
            due_date: None,
            reminder_at: None,
            counterparty_id: None,
            act_id: None,
        },
    )
    .await?
    .is_none());
    assert!(db::tasks::set_status_scoped(
        &pool,
        foreign_company_id,
        created.id,
        models::TaskStatus::Done,
    )
    .await?
    .is_none());
    assert!(!db::tasks::delete_scoped(&pool, foreign_company_id, created.id).await?);

    let own = db::tasks::get_by_id_scoped(&pool, DEFAULT_COMPANY_ID, created.id)
        .await?
        .expect("own company still sees task");
    assert_eq!(own.title, created.title);
    assert_eq!(own.status, models::TaskStatus::Open);

    assert!(db::tasks::delete_scoped(&pool, DEFAULT_COMPANY_ID, created.id).await?);
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(foreign_company_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn tasks_list_by_act_returns_only_related_tasks() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Task Act Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-task-act-cp-{suffix}")),
        },
    )
    .await?;

    let act = db::acts::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewAct {
            number: format!("TASK-ACT-{suffix}"),
            counterparty_id: cp.id,
            contract_id: None,
            category_id: None,
            direction: models::DocumentDirection::Outgoing,
            date: Utc::now().date_naive(),
            expected_payment_date: None,
            status: models::ActStatus::Draft,
            notes: None,
            bas_id: Some(format!("it-task-act-{suffix}")),
            items: vec![models::NewActItem {
                description: "Послуга".to_string(),
                quantity: dec!(1.0000),
                unit: "год".to_string(),
                unit_price: dec!(100.00),
            }],
        },
    )
    .await?;

    let related = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Задача по акту {suffix}"),
            description: None,
            priority: models::TaskPriority::High,
            due_date: None,
            reminder_at: None,
            counterparty_id: Some(cp.id),
            act_id: Some(act.id),
        },
    )
    .await?;

    let unrelated = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Інша задача {suffix}"),
            description: None,
            priority: models::TaskPriority::Low,
            due_date: None,
            reminder_at: None,
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?;

    let tasks = db::tasks::list_by_act_scoped(&pool, DEFAULT_COMPANY_ID, act.id).await?;
    assert!(tasks.iter().any(|t| t.id == related.id));
    assert!(!tasks.iter().any(|t| t.id == unrelated.id));

    sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(related.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(unrelated.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM acts WHERE id = $1")
        .bind(act.id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn tasks_due_reminders_and_list_open_filter_correctly() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let cp = db::counterparties::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewCounterparty {
            name: format!("ІТ Reminder Контрагент {suffix}"),
            edrpou: Some(suffix[..8].to_string()),
            ipn: None,
            iban: None,
            address: None,
            phone: None,
            email: None,
            notes: None,
            bas_id: Some(format!("it-task-reminder-cp-{suffix}")),
        },
    )
    .await?;

    let urgent = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Термінова задача {suffix}"),
            description: Some("Містить ключове-слово для пошуку".to_string()),
            priority: models::TaskPriority::Critical,
            due_date: Some(Utc::now() + Duration::hours(1)),
            reminder_at: Some(Utc::now()),
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?;

    let later = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Пізніша задача {suffix}"),
            description: None,
            priority: models::TaskPriority::Low,
            due_date: Some(Utc::now() + Duration::days(3)),
            reminder_at: Some(Utc::now() + Duration::days(2)),
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?;

    let done = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Закрита задача {suffix}"),
            description: None,
            priority: models::TaskPriority::High,
            due_date: Some(Utc::now() + Duration::hours(2)),
            reminder_at: Some(Utc::now()),
            counterparty_id: Some(cp.id),
            act_id: None,
        },
    )
    .await?;
    db::tasks::set_status_scoped(&pool, DEFAULT_COMPANY_ID, done.id, models::TaskStatus::Done)
        .await?
        .expect("done task updated");

    let open_tasks = db::tasks::list_open(&pool, DEFAULT_COMPANY_ID, "").await?;
    let urgent_pos = open_tasks
        .iter()
        .position(|t| t.id == urgent.id)
        .expect("urgent in open list");
    let later_pos = open_tasks
        .iter()
        .position(|t| t.id == later.id)
        .expect("later in open list");
    assert!(urgent_pos < later_pos);
    assert!(!open_tasks.iter().any(|t| t.id == done.id));

    let due = db::tasks::due_reminders(&pool).await?;
    assert!(due.iter().any(|t| t.id == urgent.id));
    assert!(!due.iter().any(|t| t.id == later.id));
    assert!(!due.iter().any(|t| t.id == done.id));

    let filtered_open = db::tasks::list_open(&pool, DEFAULT_COMPANY_ID, "Термінова").await?;
    assert_eq!(filtered_open.len(), 1);
    assert_eq!(filtered_open[0].id, urgent.id);

    let all_filtered = db::tasks::list_all(&pool, DEFAULT_COMPANY_ID, "Пізніша").await?;
    assert_eq!(all_filtered.len(), 1);
    assert_eq!(all_filtered[0].id, later.id);

    let description_filtered =
        db::tasks::list_open(&pool, DEFAULT_COMPANY_ID, "ключове-слово").await?;
    assert_eq!(description_filtered.len(), 1);
    assert_eq!(description_filtered[0].id, urgent.id);

    let due_today = db::tasks::list_due_today(&pool, DEFAULT_COMPANY_ID).await?;
    assert!(due_today.iter().any(|t| t.id == urgent.id));
    assert!(!due_today.iter().any(|t| t.id == later.id));
    assert!(!due_today.iter().any(|t| t.id == done.id));

    for id in [urgent.id, later.id, done.id] {
        sqlx::query("DELETE FROM tasks WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await?;
    }
    sqlx::query("DELETE FROM counterparties WHERE id = $1")
        .bind(cp.id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
async fn tasks_list_queries_are_scoped_to_company() -> Result<()> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };

    let suffix = unique_suffix();
    let other_company = db::companies::create(
        &pool,
        &models::NewCompany {
            name: format!("ІТ Компанія Tasks Scope {suffix}"),
            short_name: Some(format!("ITS-{suffix}")),
            edrpou: Some(suffix[..8].to_string()),
            ipn: Some(format!("4{}", &suffix[..9])),
            iban: None,
            legal_address: None,
            director_name: None,
            tax_system: Some("simplified".to_string()),
            is_vat_payer: false,
        },
    )
    .await?;

    let task_default = db::tasks::create(
        &pool,
        DEFAULT_COMPANY_ID,
        &models::NewTask {
            title: format!("Задача default company {suffix}"),
            description: None,
            priority: models::TaskPriority::Normal,
            due_date: None,
            reminder_at: None,
            counterparty_id: None,
            act_id: None,
        },
    )
    .await?;

    let task_other = db::tasks::create(
        &pool,
        other_company.id,
        &models::NewTask {
            title: format!("Задача other company {suffix}"),
            description: None,
            priority: models::TaskPriority::Normal,
            due_date: None,
            reminder_at: None,
            counterparty_id: None,
            act_id: None,
        },
    )
    .await?;

    let default_open = db::tasks::list_open(&pool, DEFAULT_COMPANY_ID, "").await?;
    assert!(default_open.iter().any(|task| task.id == task_default.id));
    assert!(!default_open.iter().any(|task| task.id == task_other.id));

    let other_all = db::tasks::list_all(&pool, other_company.id, "").await?;
    assert!(other_all.iter().any(|task| task.id == task_other.id));
    assert!(!other_all.iter().any(|task| task.id == task_default.id));

    for id in [task_default.id, task_other.id] {
        sqlx::query("DELETE FROM tasks WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await?;
    }
    sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(other_company.id)
        .execute(&pool)
        .await?;

    Ok(())
}
