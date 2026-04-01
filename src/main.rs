// Acta — програма управлінського обліку
//
// Підключаємо Rust типи, згенеровані з .slint файлів.
// Після цього доступний MainWindow (та інші export компоненти).
// ВАЖЛИВО: має бути на рівні модуля — не всередині функції.
slint::include_modules!();

mod db;
mod models;

use anyhow::Result;
use slint::{ModelRc, SharedString, StandardListViewItem, VecModel};
use sqlx::postgres::PgPoolOptions;

use models::ActStatus;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let _ = dotenvy::dotenv();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL не задано. Перевір .env файл.");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Міграції застосовано.");

    // ── Створення вікна ──────────────────────────────────────────────────────
    // MainWindow — тип згенерований з ui/main.slint
    let ui = MainWindow::new()?;

    // ── Початкове завантаження ───────────────────────────────────────────────
    // Тут ми ще в main thread (до ui.run()), тому ModelRc будувати безпечно.
    {
        let counterparties = db::counterparties::list(&pool).await?;
        let (rows, ids) = build_models(to_table_data(&counterparties));
        ui.set_counterparty_rows(rows);
        ui.set_counterparty_ids(ids);
        tracing::info!("Завантажено {} контрагентів.", counterparties.len());
    }

    // ── Початкове завантаження актів ─────────────────────────────────────────
    {
        let acts = db::acts::list(&pool, None).await?;
        let (rows, ids, statuses) = build_acts_models(to_acts_table_data(&acts));
        ui.set_act_rows(rows);
        ui.set_act_row_ids(ids);
        ui.set_act_row_statuses(statuses);
        tracing::info!("Завантажено {} актів.", acts.len());
    }

    // ── Колбек: пошук ────────────────────────────────────────────────────────
    //
    // Ключова проблема async + Slint:
    //   Slint event loop → main thread (не Send)
    //   tokio::spawn    → інший потік (потребує Send)
    //
    // ModelRc базується на Rc (не atomic), тому НЕ є Send.
    // Рішення: передаємо між потоками Vec<_> (Send),
    //   а ModelRc будуємо всередині upgrade_in_event_loop (main thread).
    let pool_search = pool.clone();
    let ui_weak = ui.as_weak();

    ui.on_counterparty_search_changed(move |query| {
        let pool = pool_search.clone();
        let ui_handle = ui_weak.clone();
        let query_str = query.to_string();

        tokio::spawn(async move {
            let result = if query_str.trim().is_empty() {
                db::counterparties::list(&pool).await
            } else {
                db::counterparties::search(&pool, &query_str).await
            };

            match result {
                Ok(cps) => {
                    // to_table_data повертає Vec<Vec<SharedString>> — є Send
                    let data = to_table_data(&cps);

                    // upgrade_in_event_loop: ставимо задачу в чергу Slint event loop.
                    // Closure виконається в main thread — тут безпечно будувати ModelRc.
                    ui_handle
                        .upgrade_in_event_loop(move |ui| {
                            let (rows, ids) = build_models(data);
                            ui.set_counterparty_rows(rows);
                            ui.set_counterparty_ids(ids);
                        })
                        // unwrap безпечний: повертає Err лише якщо вікно вже закрите,
                        // але tokio::spawn завершується раніше ніж ui дропається.
                        .unwrap();
                }
                Err(e) => tracing::error!("Помилка пошуку: {e}"),
            }
        });
    });

    // ── Колбек: вибір рядка ──────────────────────────────────────────────────
    ui.on_counterparty_selected(|id| {
        tracing::debug!("Вибрано контрагента: {id}");
        // TODO: відкрити картку контрагента
    });

    // ── Колбек: новий контрагент ─────────────────────────────────────────────
    ui.on_counterparty_create_clicked(|| {
        tracing::info!("Натиснуто: Новий контрагент");
        // TODO: відкрити форму створення
    });

    // ── Колбек: фільтр статусу актів ─────────────────────────────────────────
    //
    // Індекс ComboBox: 0=Усі, 1=Чернетка, 2=Виставлено, 3=Підписано, 4=Оплачено
    let pool_acts_filter = pool.clone();
    let ui_weak_acts_filter = ui.as_weak();

    ui.on_act_status_filter_changed(move |filter_idx| {
        let pool = pool_acts_filter.clone();
        let ui_handle = ui_weak_acts_filter.clone();

        // Перетворюємо індекс ComboBox в Option<ActStatus>
        let status_filter = match filter_idx {
            1 => Some(ActStatus::Draft),
            2 => Some(ActStatus::Issued),
            3 => Some(ActStatus::Signed),
            4 => Some(ActStatus::Paid),
            _ => None,  // 0 = "Усі"
        };

        tokio::spawn(async move {
            match db::acts::list(&pool, status_filter).await {
                Ok(acts) => {
                    let data = to_acts_table_data(&acts);
                    ui_handle
                        .upgrade_in_event_loop(move |ui| {
                            let (rows, ids, statuses) = build_acts_models(data);
                            ui.set_act_rows(rows);
                            ui.set_act_row_ids(ids);
                            ui.set_act_row_statuses(statuses);
                        })
                        // unwrap безпечний: вікно живе поки tokio runtime активний.
                        .unwrap();
                }
                Err(e) => tracing::error!("Помилка фільтру актів: {e}"),
            }
        });
    });

    // ── Колбек: вибір акту ───────────────────────────────────────────────────
    ui.on_act_selected(|id| {
        tracing::debug!("Вибрано акт: {id}");
        // TODO: відкрити картку акту
    });

    // ── Колбек: новий акт ────────────────────────────────────────────────────
    ui.on_act_create_clicked(|| {
        tracing::info!("Натиснуто: Новий акт");
        // TODO: відкрити форму створення
    });

    // ── Колбек: наступний статус акту ────────────────────────────────────────
    let pool_acts_status = pool.clone();
    let ui_weak_acts_status = ui.as_weak();

    ui.on_act_advance_status_clicked(move |id| {
        let pool = pool_acts_status.clone();
        let ui_handle = ui_weak_acts_status.clone();
        let id_str = id.to_string();

        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID акту: {id_str}");
                return;
            };

            match db::acts::advance_status(&pool, uuid).await {
                Ok(Some(act)) => {
                    tracing::info!(
                        "Акт {} переведено до статусу '{}'.",
                        act.number,
                        act.status.label()
                    );
                    // Оновлюємо список після зміни статусу
                    if let Ok(acts) = db::acts::list(&pool, None).await {
                        let data = to_acts_table_data(&acts);
                        ui_handle
                            .upgrade_in_event_loop(move |ui| {
                                let (rows, ids, statuses) = build_acts_models(data);
                                ui.set_act_rows(rows);
                                ui.set_act_row_ids(ids);
                                ui.set_act_row_statuses(statuses);
                            })
                            // unwrap безпечний: вікно живе поки tokio runtime активний.
                            .unwrap();
                    }
                }
                Ok(None) => tracing::warn!("Акт {uuid} не знайдено."),
                Err(e)   => tracing::error!("Помилка зміни статусу: {e}"),
            }
        });
    });

    // ── Колбек: архівувати ───────────────────────────────────────────────────
    let pool_archive = pool.clone();
    let ui_weak_archive = ui.as_weak();

    ui.on_counterparty_archive_clicked(move |id| {
        let pool = pool_archive.clone();
        let ui_handle = ui_weak_archive.clone();
        let id_str = id.to_string();

        tokio::spawn(async move {
            // Перетворюємо рядок у UUID — let-else для чистого раннього виходу
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID: {id_str}");
                return;
            };

            match db::counterparties::archive(&pool, uuid).await {
                Ok(true) => {
                    tracing::info!("Контрагента {uuid} архівовано.");
                    if let Ok(cps) = db::counterparties::list(&pool).await {
                        let data = to_table_data(&cps);
                        ui_handle
                            .upgrade_in_event_loop(move |ui| {
                                let (rows, ids) = build_models(data);
                                ui.set_counterparty_rows(rows);
                                ui.set_counterparty_ids(ids);
                            })
                            // unwrap безпечний: аналогічно до пошуку — вікно живе
                            // поки tokio runtime активний (обидва зупиняються разом).
                            .unwrap();
                    }
                }
                Ok(false) => tracing::warn!("Контрагента {uuid} не знайдено."),
                Err(e) => tracing::error!("Помилка архівування: {e}"),
            }
        });
    });

    // run() блокує до закриття вікна
    ui.run()?;
    Ok(())
}

// ── Проміжний формат даних (Send) ────────────────────────────────────────────
//
// Чому не повертати ModelRc напряму?
// ModelRc = Rc<dyn Model> — не є Send (не можна передати між потоками).
// Ці прості Vec є Send і можна безпечно передати в upgrade_in_event_loop.
struct TableData {
    // Рядки таблиці: зовнішній Vec = рядки, внутрішній = комірки
    rows: Vec<Vec<SharedString>>,
    // Паралельний масив UUID — rows[i] відповідає ids[i]
    ids: Vec<SharedString>,
}

// Конвертуємо контрагентів у проміжний формат
fn to_table_data(cps: &[models::Counterparty]) -> TableData {
    let rows = cps
        .iter()
        .map(|cp| {
            vec![
                SharedString::from(cp.name.as_str()),
                SharedString::from(cp.edrpou.as_deref().unwrap_or("—")),
                SharedString::from(cp.iban.as_deref().unwrap_or("—")),
                SharedString::from(cp.phone.as_deref().unwrap_or("—")),
                SharedString::from(cp.email.as_deref().unwrap_or("—")),
            ]
        })
        .collect();

    let ids = cps
        .iter()
        .map(|cp| SharedString::from(cp.id.to_string().as_str()))
        .collect();

    TableData { rows, ids }
}

// ── Проміжний формат для актів (Send) ───────────────────────────────────────
struct ActsTableData {
    rows:     Vec<Vec<SharedString>>,
    ids:      Vec<SharedString>,
    // Сирі рядки статусів ("draft", "issued", ...) для логіки кнопки в Slint
    statuses: Vec<SharedString>,
}

fn to_acts_table_data(acts: &[models::ActListRow]) -> ActsTableData {
    let rows = acts
        .iter()
        .map(|a| {
            vec![
                SharedString::from(a.number.as_str()),
                // NaiveDate → "дд.мм.рррр" для відображення в таблиці
                SharedString::from(a.date.format("%d.%m.%Y").to_string().as_str()),
                SharedString::from(a.counterparty_name.as_str()),
                SharedString::from(format!("{:.2}", a.total_amount).as_str()),
                SharedString::from(a.status.label()),
            ]
        })
        .collect();

    let ids = acts
        .iter()
        .map(|a| SharedString::from(a.id.to_string().as_str()))
        .collect();

    let statuses = acts
        .iter()
        .map(|a| SharedString::from(a.status.as_str()))
        .collect();

    ActsTableData { rows, ids, statuses }
}

fn build_acts_models(
    data: ActsTableData,
) -> (
    ModelRc<ModelRc<StandardListViewItem>>,
    ModelRc<SharedString>,
    ModelRc<SharedString>,
) {
    let rows: Vec<ModelRc<StandardListViewItem>> = data
        .rows
        .into_iter()
        .map(|cells| {
            let items: Vec<StandardListViewItem> = cells
                .iter()
                .map(|s| StandardListViewItem::from(s.as_str()))
                .collect();
            ModelRc::new(VecModel::from(items))
        })
        .collect();

    (
        ModelRc::new(VecModel::from(rows)),
        ModelRc::new(VecModel::from(data.ids)),
        ModelRc::new(VecModel::from(data.statuses)),
    )
}

// Будуємо Slint моделі з TableData.
// Викликати ТІЛЬКИ з main thread (або з upgrade_in_event_loop).
//
// StandardListViewItem::from(&str) — офіційний спосіб побудови
// (struct non-exhaustive, тому { text: ... } не компілюється).
fn build_models(
    data: TableData,
) -> (ModelRc<ModelRc<StandardListViewItem>>, ModelRc<SharedString>) {
    // Кожен рядок → ModelRc<StandardListViewItem>
    let rows: Vec<ModelRc<StandardListViewItem>> = data
        .rows
        .into_iter()
        .map(|cells| {
            let items: Vec<StandardListViewItem> = cells
                .iter()
                .map(|s| StandardListViewItem::from(s.as_str()))
                .collect();
            ModelRc::new(VecModel::from(items))
        })
        .collect();

    (
        ModelRc::new(VecModel::from(rows)),
        ModelRc::new(VecModel::from(data.ids)),
    )
}
