// Acta — програма управлінського обліку
//
// Підключаємо Rust типи, згенеровані з .slint файлів.
// Після цього доступний MainWindow (та інші export компоненти).
// ВАЖЛИВО: має бути на рівні модуля — не всередині функції.
slint::include_modules!();

mod db;
mod models;
mod pdf;

use anyhow::Result;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use slint::{Model, ModelRc, SharedString, StandardListViewItem, VecModel, Weak};
use sqlx::postgres::PgPoolOptions;

use models::{ActStatus, NewAct, NewActItem, UpdateAct};

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

    // ── Колбек: новий акт — відкрити форму ──────────────────────────────────
    //
    // Перед показом форми потрібно:
    //   1. Завантажити список контрагентів для ComboBox
    //   2. Згенерувати наступний номер акту
    // Обидві операції виконуються паралельно через tokio::join!
    let pool_create_act = pool.clone();
    let ui_weak_create_act = ui.as_weak();

    ui.on_act_create_clicked(move || {
        let pool = pool_create_act.clone();
        let ui_handle = ui_weak_create_act.clone();

        tokio::spawn(async move {
            // tokio::join! — запускає обидва futures паралельно (не послідовно)
            let (cp_result, num_result) = tokio::join!(
                db::acts::counterparties_for_select(&pool),
                db::acts::generate_next_number(&pool),
            );

            let counterparties = match cp_result {
                Ok(v)  => v,
                Err(e) => { tracing::error!("Помилка завантаження контрагентів: {e}"); return; }
            };
            let next_number = match num_result {
                Ok(n)  => n,
                Err(e) => { tracing::error!("Помилка генерації номеру: {e}"); return; }
            };

            // Розбиваємо Vec<(Uuid, String)> на два паралельних Vec<SharedString>
            let cp_names: Vec<SharedString> = counterparties
                .iter()
                .map(|(_, name)| SharedString::from(name.as_str()))
                .collect();
            let cp_ids: Vec<SharedString> = counterparties
                .iter()
                .map(|(id, _)| SharedString::from(id.to_string().as_str()))
                .collect();

            ui_handle
                .upgrade_in_event_loop(move |ui| {
                    ui.set_act_form_number(SharedString::from(next_number.as_str()));
                    ui.set_act_form_total(SharedString::from("0.00"));
                    ui.set_act_form_is_edit(false);
                    ui.set_act_form_cp_names(ModelRc::new(VecModel::from(cp_names)));
                    ui.set_act_form_cp_ids(ModelRc::new(VecModel::from(cp_ids)));
                    // Очищаємо позиції з попереднього відкриття форми
                    let empty: Vec<SharedString> = vec![];
                    ui.set_act_form_item_descriptions(ModelRc::new(VecModel::from(empty.clone())));
                    ui.set_act_form_item_quantities(ModelRc::new(VecModel::from(empty.clone())));
                    ui.set_act_form_item_units(ModelRc::new(VecModel::from(empty.clone())));
                    ui.set_act_form_item_prices(ModelRc::new(VecModel::from(empty.clone())));
                    ui.set_act_form_item_amounts(ModelRc::new(VecModel::from(empty)));
                    // Перемикаємо на форму
                    ui.set_show_act_form(true);
                })
                .ok();
        });
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

    // ── Колбек: відкрити акт для редагування ────────────────────────────────
    //
    // Паралельно завантажуємо акт з позиціями та список контрагентів,
    // потім заповнюємо всі поля форми та перемикаємось у edit-mode.
    let pool_edit = pool.clone();
    let ui_weak_edit = ui.as_weak();

    ui.on_act_edit_clicked(move |act_id| {
        let pool = pool_edit.clone();
        let ui_handle = ui_weak_edit.clone();
        let id_str = act_id.to_string();

        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID акту: {id_str}");
                return;
            };

            // tokio::join! — два незалежних запити паралельно (урок 2026-04-01)
            let (act_result, cp_result) = tokio::join!(
                db::acts::get_for_edit(&pool, uuid),
                db::acts::counterparties_for_select(&pool),
            );

            let act_opt = match act_result {
                Ok(v)  => v,
                Err(e) => { tracing::error!("Помилка завантаження акту: {e}"); return; }
            };
            let counterparties = match cp_result {
                Ok(v)  => v,
                Err(e) => { tracing::error!("Помилка завантаження контрагентів: {e}"); return; }
            };

            let Some((act, items)) = act_opt else {
                tracing::warn!("Акт {uuid} не знайдено.");
                return;
            };

            let cp_names: Vec<SharedString> = counterparties
                .iter()
                .map(|(_, n)| SharedString::from(n.as_str()))
                .collect();
            let cp_ids: Vec<SharedString> = counterparties
                .iter()
                .map(|(id, _)| SharedString::from(id.to_string().as_str()))
                .collect();

            // Шукаємо індекс контрагента акту у відсортованому списку
            let cp_index = counterparties
                .iter()
                .position(|(id, _)| *id == act.counterparty_id)
                .unwrap_or(0) as i32;

            let item_descs:   Vec<SharedString> = items.iter().map(|it| SharedString::from(it.description.as_str())).collect();
            let item_qtys:    Vec<SharedString> = items.iter().map(|it| SharedString::from(format!("{}", it.quantity).as_str())).collect();
            let item_units:   Vec<SharedString> = items.iter().map(|it| SharedString::from(it.unit.as_str())).collect();
            let item_prices:  Vec<SharedString> = items.iter().map(|it| SharedString::from(format!("{}", it.unit_price).as_str())).collect();
            let item_amounts: Vec<SharedString> = items.iter().map(|it| SharedString::from(format!("{:.2}", it.amount).as_str())).collect();

            let act_number  = act.number.clone();
            // Дата у форматі ДД.ММ.РРРР (урок 2026-04-01)
            let act_date    = act.date.format("%d.%m.%Y").to_string();
            let act_notes   = act.notes.clone().unwrap_or_default();
            let act_id_str  = act.id.to_string();
            let total_str   = format!("{:.2}", act.total_amount);

            ui_handle
                .upgrade_in_event_loop(move |ui| {
                    ui.set_act_form_number(SharedString::from(act_number.as_str()));
                    ui.set_act_form_date(SharedString::from(act_date.as_str()));
                    ui.set_act_form_notes(SharedString::from(act_notes.as_str()));
                    ui.set_act_form_cp_index(cp_index);
                    ui.set_act_form_edit_id(SharedString::from(act_id_str.as_str()));
                    ui.set_act_form_total(SharedString::from(total_str.as_str()));
                    ui.set_act_form_is_edit(true);
                    ui.set_act_form_cp_names(ModelRc::new(VecModel::from(cp_names)));
                    ui.set_act_form_cp_ids(ModelRc::new(VecModel::from(cp_ids)));
                    ui.set_act_form_item_descriptions(ModelRc::new(VecModel::from(item_descs)));
                    ui.set_act_form_item_quantities(  ModelRc::new(VecModel::from(item_qtys)));
                    ui.set_act_form_item_units(       ModelRc::new(VecModel::from(item_units)));
                    ui.set_act_form_item_prices(      ModelRc::new(VecModel::from(item_prices)));
                    ui.set_act_form_item_amounts(     ModelRc::new(VecModel::from(item_amounts)));
                    ui.set_show_act_form(true);
                })
                .ok();
        });
    });

    // ── Колбек: оновити акт з позиціями (режим редагування) ─────────────────
    //
    // Читаємо edit_id синхронно з UI (main thread),
    // потім spawn для async DB операції.
    let pool_update = pool.clone();
    let ui_weak_update = ui.as_weak();

    ui.on_act_form_update(move |number, date_str, cp_id_str, notes| {
        let Some(ui_ref) = ui_weak_update.upgrade() else { return; };
        // Читаємо edit_id та позиції поки ще в main thread
        let edit_id = ui_ref.get_act_form_edit_id().to_string();
        let items   = collect_form_items(&ui_ref);

        let pool      = pool_update.clone();
        let ui_weak   = ui_weak_update.clone();
        let number    = number.to_string();
        let date_str  = date_str.to_string();
        let cp_id_str = cp_id_str.to_string();
        let notes_opt = if notes.trim().is_empty() { None } else { Some(notes.to_string()) };

        tokio::spawn(async move {
            let Ok(uuid) = edit_id.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний edit_id: {edit_id}");
                return;
            };

            // Парсимо дату (урок 2026-04-01)
            let date = match NaiveDate::parse_from_str(&date_str, "%d.%m.%Y") {
                Ok(d)  => d,
                Err(_) => {
                    tracing::error!("Невірний формат дати: '{date_str}'");
                    return;
                }
            };

            let cp_uuid = match uuid::Uuid::parse_str(&cp_id_str) {
                Ok(id) => id,
                Err(_) => {
                    tracing::error!("Некоректний UUID контрагента: '{cp_id_str}'");
                    return;
                }
            };

            let update_data = UpdateAct {
                number:          number.clone(),
                counterparty_id: cp_uuid,
                contract_id:     None,
                date,
                notes:           notes_opt,
            };

            match db::acts::update_with_items(&pool, uuid, update_data, items).await {
                Ok(act) => {
                    tracing::info!("Акт '{}' оновлено (id={}).", act.number, act.id);
                    if let Ok(acts) = db::acts::list(&pool, None).await {
                        let data = to_acts_table_data(&acts);
                        ui_weak
                            .upgrade_in_event_loop(move |ui| {
                                let (rows, ids, statuses) = build_acts_models(data);
                                ui.set_act_rows(rows);
                                ui.set_act_row_ids(ids);
                                ui.set_act_row_statuses(statuses);
                                ui.set_show_act_form(false);
                            })
                            .ok();
                    }
                }
                Err(e) => tracing::error!("Помилка оновлення акту: {e}"),
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

    // ── Колбек: скасувати форму — повернутись до списку ─────────────────────
    //
    // Синхронний колбек (немає DB операцій) — просто скидаємо прапор.
    // Викликається без tokio::spawn: ми вже в main thread.
    let ui_weak_cancel = ui.as_weak();
    ui.on_act_form_cancel(move || {
        if let Some(ui) = ui_weak_cancel.upgrade() {
            ui.set_show_act_form(false);
        }
    });

    // ── Колбек: додати позицію до форми ─────────────────────────────────────
    //
    // Додає порожній рядок у кожен паралельний масив позицій.
    // Синхронно: оновлюємо ModelRc у main thread.
    // Редагування значень позиції — майбутня функція (TODO: edit-item колбек).
    let ui_weak_add = ui.as_weak();
    ui.on_act_form_add_item(move || {
        let Some(ui) = ui_weak_add.upgrade() else { return; };

        // Локальна функція (не closure) — не захоплює змінних, може бути вбудована
        fn append(model: ModelRc<SharedString>, val: &str) -> ModelRc<SharedString> {
            let mut v: Vec<SharedString> = (0..model.row_count())
                .filter_map(|i| model.row_data(i))
                .collect();
            v.push(SharedString::from(val));
            ModelRc::new(VecModel::from(v))
        }

        ui.set_act_form_item_descriptions(append(ui.get_act_form_item_descriptions(), "Нова послуга"));
        ui.set_act_form_item_quantities(  append(ui.get_act_form_item_quantities(),   "1"));
        ui.set_act_form_item_units(       append(ui.get_act_form_item_units(),        "шт"));
        ui.set_act_form_item_prices(      append(ui.get_act_form_item_prices(),       "0.00"));
        ui.set_act_form_item_amounts(     append(ui.get_act_form_item_amounts(),      "0.00"));

        // Перераховуємо total (після append amount = 0.00, тому total не змінюється)
        // Повноцінний перерахунок — після реалізації edit-item
    });

    // ── Колбек: видалити позицію з форми ────────────────────────────────────
    let ui_weak_remove = ui.as_weak();
    ui.on_act_form_remove_item(move |idx| {
        let Some(ui) = ui_weak_remove.upgrade() else { return; };
        let i = idx as usize;

        fn remove_at(model: ModelRc<SharedString>, i: usize) -> ModelRc<SharedString> {
            let mut v: Vec<SharedString> = (0..model.row_count())
                .filter_map(|j| model.row_data(j))
                .collect();
            if i < v.len() { v.remove(i); }
            ModelRc::new(VecModel::from(v))
        }

        ui.set_act_form_item_descriptions(remove_at(ui.get_act_form_item_descriptions(), i));
        ui.set_act_form_item_quantities(  remove_at(ui.get_act_form_item_quantities(),   i));
        ui.set_act_form_item_units(       remove_at(ui.get_act_form_item_units(),        i));
        ui.set_act_form_item_prices(      remove_at(ui.get_act_form_item_prices(),       i));
        ui.set_act_form_item_amounts(     remove_at(ui.get_act_form_item_amounts(),      i));
    });

    // ── Колбек: зберегти акт ("Зберегти") ───────────────────────────────────
    //
    // Читаємо поля форми + позиції синхронно (ми в main thread),
    // потім передаємо в tokio::spawn для async DB операції.
    let pool_save = pool.clone();
    let ui_weak_save = ui.as_weak();

    ui.on_act_form_save(move |number, date_str, cp_id_str, notes| {
        let Some(ui_ref) = ui_weak_save.upgrade() else { return; };
        let items = collect_form_items(&ui_ref);

        spawn_save_act(
            pool_save.clone(),
            ui_weak_save.clone(),
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.trim().is_empty() { None } else { Some(notes.to_string()) },
            items,
        );
    });

    // ── Колбек: зберегти як чернетку ("Чернетка") ───────────────────────────
    //
    // Наразі ідентичний до on_act_form_save — обидва створюють акт зі статусом Draft
    // (статус 'draft' є DEFAULT у БД, змінити можна лише через advance_status).
    // TODO: у майбутньому on_act_form_save може одразу переводити до Issued.
    let pool_draft = pool.clone();
    let ui_weak_draft = ui.as_weak();

    ui.on_act_form_save_draft(move |number, date_str, cp_id_str, notes| {
        let Some(ui_ref) = ui_weak_draft.upgrade() else { return; };
        let items = collect_form_items(&ui_ref);

        spawn_save_act(
            pool_draft.clone(),
            ui_weak_draft.clone(),
            number.to_string(),
            date_str.to_string(),
            cp_id_str.to_string(),
            if notes.trim().is_empty() { None } else { Some(notes.to_string()) },
            items,
        );
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

// ── Допоміжні функції для форми актів ────────────────────────────────────────

/// Зчитує поточний стан позицій з паралельних масивів форми.
///
/// Викликати ТІЛЬКИ з main thread (в синхронному колбеку Slint),
/// бо MainWindow не є Send і не може бути передана між потоками.
///
/// Позиції з невалідними числами (quantity або price) — мовчки пропускаються.
fn collect_form_items(ui: &MainWindow) -> Vec<NewActItem> {
    let descs  = ui.get_act_form_item_descriptions();
    let qtys   = ui.get_act_form_item_quantities();
    let units  = ui.get_act_form_item_units();
    let prices = ui.get_act_form_item_prices();

    (0..descs.row_count())
        .filter_map(|i| {
            let description = descs.row_data(i)?.to_string();
            let qty_str     = qtys.row_data(i)?;
            let unit        = units.row_data(i)?.to_string();
            let price_str   = prices.row_data(i)?;

            // parse::<Decimal>() — стандартний FromStr для rust_decimal
            // Якщо поле порожнє або не є числом — filter_map видаляє рядок
            let quantity   = qty_str.parse::<Decimal>().ok()?;
            let unit_price = price_str.parse::<Decimal>().ok()?;

            Some(NewActItem { description, quantity, unit, unit_price })
        })
        .collect()
}

/// Запускає tokio::spawn для збереження акту в БД.
///
/// Повертається відразу (non-blocking).
/// Після успішного збереження:
///   - Перезавантажує список актів
///   - Через upgrade_in_event_loop оновлює UI та ховає форму
///
/// `pool` — клонований PgPool (дешево: пул використовує Arc всередині).
fn spawn_save_act(
    pool:      sqlx::PgPool,
    ui_weak:   Weak<MainWindow>,
    number:    String,
    date_str:  String,
    cp_id_str: String,
    notes:     Option<String>,
    items:     Vec<NewActItem>,
) {
    tokio::spawn(async move {
        // Парсимо дату зі строки ДД.ММ.РРРР → chrono::NaiveDate
        let date = match NaiveDate::parse_from_str(&date_str, "%d.%m.%Y") {
            Ok(d)  => d,
            Err(_) => {
                tracing::error!("Невірний формат дати: '{date_str}'. Очікується ДД.ММ.РРРР");
                return;
            }
        };

        // UUID контрагента — якщо порожній або невалідний → помилка
        let cp_uuid = match uuid::Uuid::parse_str(&cp_id_str) {
            Ok(id) => id,
            Err(_) => {
                tracing::error!("Контрагент не вибраний або UUID некоректний: '{cp_id_str}'");
                return;
            }
        };

        let new_act = NewAct {
            number:          number.clone(),
            counterparty_id: cp_uuid,
            contract_id:     None,  // договір — майбутня функція
            date,
            notes,
            bas_id:          None,
            items,
        };

        match db::acts::create(&pool, &new_act).await {
            Ok(act) => {
                tracing::info!("Акт '{}' збережено (id={}).", act.number, act.id);

                // Оновлюємо список та повертаємось до нього
                if let Ok(acts) = db::acts::list(&pool, None).await {
                    let data = to_acts_table_data(&acts);
                    ui_weak
                        .upgrade_in_event_loop(move |ui| {
                            let (rows, ids, statuses) = build_acts_models(data);
                            ui.set_act_rows(rows);
                            ui.set_act_row_ids(ids);
                            ui.set_act_row_statuses(statuses);
                            ui.set_show_act_form(false);  // повертаємось до списку
                        })
                        .ok();
                }
            }
            Err(e) => tracing::error!("Помилка збереження акту: {e}"),
        }
    });
}
