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
use std::sync::{Arc, Mutex};

use models::{ActStatus, NewAct, NewActItem, NewCounterparty, UpdateAct, UpdateCounterparty};

#[derive(Clone, Default)]
struct CounterpartyListState {
    query: String,
    include_archived: bool,
}

#[derive(Clone, Default)]
struct ActListState {
    query: String,
    status_filter: Option<ActStatus>,
}

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
    ui.set_counterparty_show_archived(false);

    let counterparty_state = Arc::new(Mutex::new(CounterpartyListState::default()));
    let act_state = Arc::new(Mutex::new(ActListState::default()));

    // ── Початкове завантаження ───────────────────────────────────────────────
    // Тут ми ще в main thread (до ui.run()), тому ModelRc будувати безпечно.
    reload_counterparties(&pool, ui.as_weak(), String::new(), false, false).await?;

    // ── Початкове завантаження актів ─────────────────────────────────────────
    reload_acts(&pool, ui.as_weak(), None, String::new(), false).await?;

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
    let counterparty_state_search = counterparty_state.clone();

    ui.on_counterparty_search_changed(move |query| {
        let pool = pool_search.clone();
        let ui_handle = ui_weak.clone();
        let (query_str, include_archived) = {
            let mut state = counterparty_state_search.lock().unwrap();
            state.query = query.to_string();
            (state.query.clone(), state.include_archived)
        };

        tokio::spawn(async move {
            if let Err(e) =
                reload_counterparties(&pool, ui_handle, query_str, include_archived, false).await
            {
                tracing::error!("Помилка пошуку: {e}");
            }
        });
    });

    // ── Колбек: вибір контрагента — відкрити форму редагування ─────────────
    let pool_cp_select = pool.clone();
    let ui_weak_cp_select = ui.as_weak();

    ui.on_counterparty_selected(move |id| {
        let pool     = pool_cp_select.clone();
        let ui_handle = ui_weak_cp_select.clone();
        let id_str   = id.to_string();

        tokio::spawn(async move {
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID контрагента: {id_str}");
                return;
            };

            match db::counterparties::get_by_id(&pool, uuid).await {
                Ok(Some(cp)) => {
                    ui_handle
                        .upgrade_in_event_loop(move |ui| {
                            ui.set_cp_form_name(SharedString::from(cp.name.as_str()));
                            ui.set_cp_form_edrpou(SharedString::from(cp.edrpou.as_deref().unwrap_or("")));
                            ui.set_cp_form_iban(SharedString::from(cp.iban.as_deref().unwrap_or("")));
                            ui.set_cp_form_phone(SharedString::from(cp.phone.as_deref().unwrap_or("")));
                            ui.set_cp_form_email(SharedString::from(cp.email.as_deref().unwrap_or("")));
                            ui.set_cp_form_address(SharedString::from(cp.address.as_deref().unwrap_or("")));
                            ui.set_cp_form_notes(SharedString::from(cp.notes.as_deref().unwrap_or("")));
                            ui.set_cp_form_edit_id(SharedString::from(cp.id.to_string().as_str()));
                            ui.set_cp_form_is_edit(true);
                            ui.set_show_cp_form(true);
                        })
                        .ok();
                }
                Ok(None) => tracing::warn!("Контрагента {uuid} не знайдено."),
                Err(e)   => tracing::error!("Помилка завантаження контрагента: {e}"),
            }
        });
    });

    // ── Колбек: новий контрагент — відкрити порожню форму ───────────────────
    let ui_weak_cp_create = ui.as_weak();

    ui.on_counterparty_create_clicked(move || {
        if let Some(ui) = ui_weak_cp_create.upgrade() {
            ui.set_cp_form_name(SharedString::from(""));
            ui.set_cp_form_edrpou(SharedString::from(""));
            ui.set_cp_form_iban(SharedString::from(""));
            ui.set_cp_form_phone(SharedString::from(""));
            ui.set_cp_form_email(SharedString::from(""));
            ui.set_cp_form_address(SharedString::from(""));
            ui.set_cp_form_notes(SharedString::from(""));
            ui.set_cp_form_edit_id(SharedString::from(""));
            ui.set_cp_form_is_edit(false);
            ui.set_show_cp_form(true);
        }
    });

    // ── Колбек: фільтр контрагентів ──────────────────────────────────────────
    let pool_cp_filter = pool.clone();
    let ui_weak_cp_filter = ui.as_weak();
    let counterparty_state_filter = counterparty_state.clone();

    ui.on_counterparty_filter_clicked(move || {
        let pool = pool_cp_filter.clone();
        let ui_handle = ui_weak_cp_filter.clone();
        let (query, include_archived) = {
            let mut state = counterparty_state_filter.lock().unwrap();
            state.include_archived = !state.include_archived;
            (state.query.clone(), state.include_archived)
        };

        tokio::spawn(async move {
            if let Err(e) = reload_counterparties(&pool, ui_handle, query, include_archived, false).await {
                tracing::error!("Помилка фільтра контрагентів: {e}");
            }
        });
    });

    // ── Колбек: фільтр статусу актів ─────────────────────────────────────────
    //
    // Індекс ComboBox: 0=Усі, 1=Чернетка, 2=Виставлено, 3=Підписано, 4=Оплачено
    let pool_acts_filter = pool.clone();
    let ui_weak_acts_filter = ui.as_weak();
    let act_state_filter = act_state.clone();

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
        let query = {
            let mut state = act_state_filter.lock().unwrap();
            state.status_filter = status_filter.clone();
            state.query.clone()
        };

        tokio::spawn(async move {
            if let Err(e) = reload_acts(&pool, ui_handle, status_filter, query, false).await {
                tracing::error!("Помилка фільтру актів: {e}");
            }
        });
    });

    let pool_acts_search = pool.clone();
    let ui_weak_acts_search = ui.as_weak();
    let act_state_search = act_state.clone();

    ui.on_act_search_changed(move |query| {
        let pool = pool_acts_search.clone();
        let ui_handle = ui_weak_acts_search.clone();
        let (query, status_filter) = {
            let mut state = act_state_search.lock().unwrap();
            state.query = query.to_string();
            (state.query.clone(), state.status_filter.clone())
        };

        tokio::spawn(async move {
            if let Err(e) = reload_acts(&pool, ui_handle, status_filter, query, false).await {
                tracing::error!("Помилка пошуку актів: {e}");
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

            // Сьогоднішня дата у форматі ДД.ММ.РРРР — обчислюємо до closure (sync)
            let today = chrono::Local::now()
                .date_naive()
                .format("%d.%m.%Y")
                .to_string();

            ui_handle
                .upgrade_in_event_loop(move |ui| {
                    ui.set_act_form_number(SharedString::from(next_number.as_str()));
                    ui.set_act_form_date(SharedString::from(today.as_str()));
                    ui.set_act_form_notes(SharedString::from(""));
                    ui.set_act_form_total(SharedString::from("0.00"));
                    ui.set_act_form_cp_index(0);
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
    let act_state_advance = act_state.clone();

    ui.on_act_advance_status_clicked(move |id| {
        let pool = pool_acts_status.clone();
        let ui_handle = ui_weak_acts_status.clone();
        let id_str = id.to_string();
        let act_state = act_state_advance.clone();

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
                    show_toast(
                        ui_handle.clone(),
                        format!("Акт '{}' → {}", act.number, act.status.label()),
                        false,
                    );
                    let (query, status_filter) = {
                        let state = act_state.lock().unwrap();
                        (state.query.clone(), state.status_filter.clone())
                    };
                    if let Err(e) = reload_acts(&pool, ui_handle.clone(), status_filter, query, false).await {
                        tracing::error!("Помилка оновлення списку актів після зміни статусу: {e}");
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
    let act_state_update = act_state.clone();

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
        let act_state = act_state_update.clone();

        tokio::spawn(async move {
            let Ok(uuid) = edit_id.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний edit_id: {edit_id}");
                return;
            };

            // Валідація обов'язкових полів форми
            if number.trim().is_empty() {
                tracing::error!("Номер акту не може бути порожнім");
                return;
            }
            if date_str.trim().is_empty() {
                tracing::error!("Дата акту не може бути порожньою");
                return;
            }
            if cp_id_str.trim().is_empty() {
                tracing::error!("Контрагент не вибраний");
                return;
            }

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
                    show_toast(ui_weak.clone(), format!("Акт '{}' оновлено", act.number), false);
                    let (query, status_filter) = {
                        let state = act_state.lock().unwrap();
                        (state.query.clone(), state.status_filter.clone())
                    };
                    if let Err(e) = reload_acts(&pool, ui_weak.clone(), status_filter, query, true).await {
                        tracing::error!("Помилка оновлення списку актів після редагування: {e}");
                    }
                }
                Err(e) => tracing::error!("Помилка оновлення акту: {e}"),
            }
        });
    });

    // ── Колбек: скасувати форму контрагента ─────────────────────────────────
    let ui_weak_cp_cancel = ui.as_weak();
    ui.on_cp_form_cancel(move || {
        if let Some(ui) = ui_weak_cp_cancel.upgrade() {
            ui.set_show_cp_form(false);
        }
    });

    // ── Колбек: зберегти нового контрагента ──────────────────────────────────
    let pool_cp_save  = pool.clone();
    let ui_weak_cp_save = ui.as_weak();
    let counterparty_state_save = counterparty_state.clone();

    ui.on_cp_form_save(move |name, edrpou, iban, phone, email, address, notes| {
        let pool     = pool_cp_save.clone();
        let ui_weak  = ui_weak_cp_save.clone();
        let name_s   = name.to_string();
        let edrpou_s = edrpou.to_string();
        let iban_s   = iban.to_string();
        let phone_s  = phone.to_string();
        let email_s  = email.to_string();
        let address_s = address.to_string();
        let notes_s  = notes.to_string();
        let counterparty_state = counterparty_state_save.clone();

        tokio::spawn(async move {
            if name_s.trim().is_empty() {
                tracing::error!("Назва контрагента не може бути порожньою");
                show_toast(ui_weak, "Введіть назву контрагента".to_string(), true);
                return;
            }

            let data = NewCounterparty {
                name:    name_s.clone(),
                edrpou:  if edrpou_s.trim().is_empty()  { None } else { Some(edrpou_s) },
                iban:    if iban_s.trim().is_empty()     { None } else { Some(iban_s) },
                phone:   if phone_s.trim().is_empty()    { None } else { Some(phone_s) },
                email:   if email_s.trim().is_empty()    { None } else { Some(email_s) },
                address: if address_s.trim().is_empty()  { None } else { Some(address_s) },
                notes:   if notes_s.trim().is_empty()    { None } else { Some(notes_s) },
                bas_id:  None,
            };

            match db::counterparties::create(&pool, &data).await {
                Ok(cp) => {
                    tracing::info!("Контрагента '{}' створено (id={}).", cp.name, cp.id);
                    show_toast(ui_weak.clone(), format!("Контрагента '{}' створено", cp.name), false);
                    let (query, include_archived) = {
                        let state = counterparty_state.lock().unwrap();
                        (state.query.clone(), state.include_archived)
                    };
                    if let Err(e) = reload_counterparties(&pool, ui_weak, query, include_archived, true).await {
                        tracing::error!("Помилка оновлення списку контрагентів після створення: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("Помилка створення контрагента: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });

    // ── Колбек: оновити контрагента (режим редагування) ──────────────────────
    let pool_cp_update  = pool.clone();
    let ui_weak_cp_update = ui.as_weak();
    let counterparty_state_update = counterparty_state.clone();

    ui.on_cp_form_update(move |name, edrpou, iban, phone, email, address, notes| {
        let Some(ui_ref) = ui_weak_cp_update.upgrade() else { return; };
        let edit_id = ui_ref.get_cp_form_edit_id().to_string();

        let pool      = pool_cp_update.clone();
        let ui_weak   = ui_weak_cp_update.clone();
        let name_s    = name.to_string();
        let edrpou_s  = edrpou.to_string();
        let iban_s    = iban.to_string();
        let phone_s   = phone.to_string();
        let email_s   = email.to_string();
        let address_s = address.to_string();
        let notes_s   = notes.to_string();
        let counterparty_state = counterparty_state_update.clone();

        tokio::spawn(async move {
            let Ok(uuid) = edit_id.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний edit_id: {edit_id}");
                return;
            };

            if name_s.trim().is_empty() {
                show_toast(ui_weak, "Введіть назву контрагента".to_string(), true);
                return;
            }

            let data = UpdateCounterparty {
                name:    name_s,
                edrpou:  if edrpou_s.trim().is_empty()  { None } else { Some(edrpou_s) },
                iban:    if iban_s.trim().is_empty()     { None } else { Some(iban_s) },
                phone:   if phone_s.trim().is_empty()    { None } else { Some(phone_s) },
                email:   if email_s.trim().is_empty()    { None } else { Some(email_s) },
                address: if address_s.trim().is_empty()  { None } else { Some(address_s) },
                notes:   if notes_s.trim().is_empty()    { None } else { Some(notes_s) },
            };

            match db::counterparties::update(&pool, uuid, &data).await {
                Ok(Some(cp)) => {
                    tracing::info!("Контрагента '{}' оновлено (id={}).", cp.name, cp.id);
                    show_toast(ui_weak.clone(), format!("Контрагента '{}' оновлено", cp.name), false);
                    let (query, include_archived) = {
                        let state = counterparty_state.lock().unwrap();
                        (state.query.clone(), state.include_archived)
                    };
                    if let Err(e) = reload_counterparties(&pool, ui_weak, query, include_archived, true).await {
                        tracing::error!("Помилка оновлення списку контрагентів після редагування: {e}");
                    }
                }
                Ok(None) => tracing::warn!("Контрагента {uuid} не знайдено."),
                Err(e) => {
                    tracing::error!("Помилка оновлення контрагента: {e}");
                    show_toast(ui_weak, format!("Помилка: {e}"), true);
                }
            }
        });
    });

    // ── Колбек: архівувати ───────────────────────────────────────────────────
    let pool_archive = pool.clone();
    let ui_weak_archive = ui.as_weak();
    let counterparty_state_archive = counterparty_state.clone();

    ui.on_counterparty_archive_clicked(move |id| {
        let pool = pool_archive.clone();
        let ui_handle = ui_weak_archive.clone();
        let id_str = id.to_string();
        let counterparty_state = counterparty_state_archive.clone();

        tokio::spawn(async move {
            // Перетворюємо рядок у UUID — let-else для чистого раннього виходу
            let Ok(uuid) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID: {id_str}");
                return;
            };

            match db::counterparties::archive(&pool, uuid).await {
                Ok(true) => {
                    tracing::info!("Контрагента {uuid} архівовано.");
                    show_toast(ui_handle.clone(), "Контрагента архівовано".to_string(), false);
                    let (query, include_archived) = {
                        let state = counterparty_state.lock().unwrap();
                        (state.query.clone(), state.include_archived)
                    };
                    if let Err(e) = reload_counterparties(&pool, ui_handle, query, include_archived, false).await {
                        tracing::error!("Помилка оновлення списку контрагентів після архівування: {e}");
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

    // ── Колбек: редагування поля позиції акту ───────────────────────────────
    //
    // Синхронний колбек (немає DB) — лише перебудовуємо ModelRc.
    // При зміні qty або price — перераховуємо amounts та total.
    //
    // Чому не оновлюємо qty/price через set_row_data: ModelRc не дає
    // доступу до внутрішнього VecModel після побудови. Замість цього
    // створюємо новий ModelRc — Slint порівнює значення і не скидає
    // фокус LineEdit якщо значення не змінилось.
    let ui_weak_item = ui.as_weak();

    ui.on_act_form_item_changed(move |idx, field, value| {
        let Some(ui) = ui_weak_item.upgrade() else { return; };
        let i = idx as usize;
        let val = value.to_string();

        // Перебудувати ModelRc з одним зміненим елементом
        fn set_at(model: ModelRc<SharedString>, i: usize, val: &str) -> ModelRc<SharedString> {
            let mut v: Vec<SharedString> = (0..model.row_count())
                .filter_map(|j| model.row_data(j))
                .collect();
            if i < v.len() { v[i] = SharedString::from(val); }
            ModelRc::new(VecModel::from(v))
        }

        match field.as_str() {
            "desc"  => ui.set_act_form_item_descriptions(set_at(ui.get_act_form_item_descriptions(), i, &val)),
            "qty"   => ui.set_act_form_item_quantities(  set_at(ui.get_act_form_item_quantities(),   i, &val)),
            "unit"  => ui.set_act_form_item_units(       set_at(ui.get_act_form_item_units(),        i, &val)),
            "price" => ui.set_act_form_item_prices(      set_at(ui.get_act_form_item_prices(),       i, &val)),
            _ => return,
        }

        // Перераховуємо суми рядків та total лише при зміні qty або price
        if field == "qty" || field == "price" {
            let qtys   = ui.get_act_form_item_quantities();
            let prices = ui.get_act_form_item_prices();
            let n      = qtys.row_count();

            let mut new_amounts: Vec<SharedString> = Vec::with_capacity(n);
            let mut total = Decimal::ZERO;

            for j in 0..n {
                let qty   = qtys.row_data(j).unwrap_or_default()
                    .parse::<Decimal>().unwrap_or_default();
                let price = prices.row_data(j).unwrap_or_default()
                    .parse::<Decimal>().unwrap_or_default();
                let amt   = qty * price;
                total += amt;
                new_amounts.push(SharedString::from(format!("{:.2}", amt).as_str()));
            }

            ui.set_act_form_item_amounts(ModelRc::new(VecModel::from(new_amounts)));
            ui.set_act_form_total(SharedString::from(format!("{:.2}", total).as_str()));
        }
    });

    // ── Колбек: зберегти акт ("Зберегти") ───────────────────────────────────
    //
    // Читаємо поля форми + позиції синхронно (ми в main thread),
    // потім передаємо в tokio::spawn для async DB операції.
    let pool_save = pool.clone();
    let ui_weak_save = ui.as_weak();
    let act_state_save = act_state.clone();

    ui.on_act_form_save(move |number, date_str, cp_id_str, notes| {
        let Some(ui_ref) = ui_weak_save.upgrade() else { return; };
        let items = collect_form_items(&ui_ref);

        spawn_save_act(
            pool_save.clone(),
            ui_weak_save.clone(),
            act_state_save.clone(),
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
    let act_state_draft = act_state.clone();

    ui.on_act_form_save_draft(move |number, date_str, cp_id_str, notes| {
        let Some(ui_ref) = ui_weak_draft.upgrade() else { return; };
        let items = collect_form_items(&ui_ref);

        spawn_save_act(
            pool_draft.clone(),
            ui_weak_draft.clone(),
            act_state_draft.clone(),
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
    // Паралельний масив архівованості — true якщо контрагент в архіві
    archived: Vec<bool>,
}

// Конвертуємо контрагентів у проміжний формат.
// Колонки: Назва, ЄДРПОУ, IBAN, Телефон (email не відображається в таблиці).
fn to_table_data(cps: &[models::Counterparty]) -> TableData {
    let rows = cps
        .iter()
        .map(|cp| {
            vec![
                SharedString::from(cp.name.as_str()),
                SharedString::from(cp.edrpou.as_deref().unwrap_or("—")),
                SharedString::from(cp.iban.as_deref().unwrap_or("—")),
                SharedString::from(cp.phone.as_deref().unwrap_or("—")),
            ]
        })
        .collect();

    let ids = cps
        .iter()
        .map(|cp| SharedString::from(cp.id.to_string().as_str()))
        .collect();

    let archived = cps.iter().map(|cp| cp.is_archived).collect();

    TableData { rows, ids, archived }
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
) -> (
    ModelRc<ModelRc<StandardListViewItem>>,
    ModelRc<SharedString>,
    ModelRc<bool>,
) {
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
        ModelRc::new(VecModel::from(data.archived)),
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

async fn reload_counterparties(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    query: String,
    include_archived: bool,
    close_form: bool,
) -> Result<()> {
    let counterparties =
        db::counterparties::list_filtered(pool, normalized_query(&query), include_archived).await?;
    let archived_cnt = db::counterparties::count_archived(pool).await.unwrap_or(0) as i32;
    let data = to_table_data(&counterparties);
    let total = data.ids.len() as i32;
    let active = data.archived.iter().filter(|archived| !**archived).count() as i32;
    let pagination = SharedString::from(format!("Показано {} контрагентів", total).as_str());

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            let (rows, ids, archived) = build_models(data);
            ui.set_counterparty_rows(rows);
            ui.set_counterparty_ids(ids);
            ui.set_counterparty_archived(archived);
            ui.set_counterparty_total_count(total);
            ui.set_counterparty_active_count(active);
            ui.set_counterparty_archived_count(archived_cnt);
            ui.set_counterparty_pagination_text(pagination);
            ui.set_counterparty_show_archived(include_archived);
            if close_form {
                ui.set_show_cp_form(false);
            }
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

async fn reload_acts(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    status_filter: Option<ActStatus>,
    query: String,
    close_form: bool,
) -> Result<()> {
    let acts = db::acts::list_filtered(pool, status_filter, normalized_query(&query)).await?;
    let data = to_acts_table_data(&acts);

    ui_weak
        .upgrade_in_event_loop(move |ui| {
            let (rows, ids, statuses) = build_acts_models(data);
            ui.set_act_rows(rows);
            ui.set_act_row_ids(ids);
            ui.set_act_row_statuses(statuses);
            if close_form {
                ui.set_show_act_form(false);
            }
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

fn normalized_query(query: &str) -> Option<&str> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
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
    act_state: Arc<Mutex<ActListState>>,
    number:    String,
    date_str:  String,
    cp_id_str: String,
    notes:     Option<String>,
    items:     Vec<NewActItem>,
) {
    tokio::spawn(async move {
        // Валідація обов'язкових полів форми
        if number.trim().is_empty() {
            tracing::error!("Номер акту не може бути порожнім");
            return;
        }
        if date_str.trim().is_empty() {
            tracing::error!("Дата акту не може бути порожньою");
            return;
        }
        if cp_id_str.trim().is_empty() {
            tracing::error!("Контрагент не вибраний");
            return;
        }

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
                show_toast(ui_weak.clone(), format!("Акт '{}' збережено", act.number), false);

                // Оновлюємо список та повертаємось до нього
                let (query, status_filter) = {
                    let state = act_state.lock().unwrap();
                    (state.query.clone(), state.status_filter.clone())
                };
                if let Err(e) = reload_acts(&pool, ui_weak.clone(), status_filter, query, true).await {
                    tracing::error!("Помилка оновлення списку актів після збереження: {e}");
                }
            }
            Err(e) => {
                tracing::error!("Помилка збереження акту: {e}");
                show_toast(ui_weak.clone(), format!("Помилка: {e}"), true);
            }
        }
    });
}

/// Показує toast-сповіщення на 3 секунди, потім автоматично прибирає.
fn show_toast(ui_weak: Weak<MainWindow>, message: String, is_error: bool) {
    let msg = SharedString::from(message.as_str());
    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_toast_message(msg);
            ui.set_toast_is_error(is_error);
        })
        .ok();

    let clear_handle = ui_weak.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        clear_handle
            .upgrade_in_event_loop(|ui| {
                ui.set_toast_message(SharedString::from(""));
            })
            .ok();
    });
}
