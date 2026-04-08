// ui/companies.rs — колбеки та дані для компаній і сторінки Налаштувань.

use anyhow::Result;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

use crate::{
    app_ctx::AppCtx,
    ui::helpers::*,
    MainWindow, SettingsCategoryRow,
};
use acta::{config::AppConfig, db, models::{NewCompany, UpdateCompany}};

// ═══════════════════════════════════════════════════════════════════════════════
// ── Проміжні дані (Send-safe) ──────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct SettingsUiData {
    pub company: Option<Company>,
    pub category_rows: Vec<SettingsCategoryRow>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── prepare / apply / reload ───────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub async fn prepare_settings_data(
    pool: &sqlx::PgPool,
    company_id: uuid::Uuid,
) -> Result<SettingsUiData> {
    let (company_res, categories_res) = tokio::join!(
        db::companies::get_by_id(pool, company_id),
        db::categories::list(pool, company_id),
    );
    let company = company_res?;
    let categories = categories_res?;
    let category_rows = categories
        .iter()
        .map(|cat| SettingsCategoryRow {
            name: SharedString::from(cat.name.as_str()),
            kind: SharedString::from(cat.kind.as_str()),
            depth: if cat.parent_id.is_some() { 1 } else { 0 },
        })
        .collect();
    Ok(SettingsUiData { company, category_rows })
}

pub fn apply_settings_to_ui(ui: &MainWindow, d: SettingsUiData) {
    if let Some(company) = d.company {
        ui.set_settings_company_name(SharedString::from(company.name.as_str()));
        ui.set_settings_company_edrpou(SharedString::from(
            company.edrpou.as_deref().unwrap_or(""),
        ));
        ui.set_settings_company_iban(SharedString::from(
            company.iban.as_deref().unwrap_or(""),
        ));
        ui.set_settings_company_director(SharedString::from(
            company.director_name.as_deref().unwrap_or(""),
        ));
        ui.set_settings_company_address(SharedString::from(
            company.legal_address.as_deref().unwrap_or(""),
        ));
    }
    ui.set_settings_category_rows(ModelRc::new(VecModel::from(d.category_rows)));
}

pub async fn reload_settings(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
) -> Result<()> {
    let d = prepare_settings_data(pool, company_id).await?;
    ui_weak
        .upgrade_in_event_loop(move |ui| apply_settings_to_ui(&ui, d))
        .map_err(anyhow::Error::from)
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── reload_companies ───────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// Завантажити список компаній і оновити UI.
pub async fn reload_companies(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    active_company_id: uuid::Uuid,
) -> Result<()> {
    let companies = db::companies::list_with_summary(pool).await?;
    ui_weak.upgrade_in_event_loop(move |ui| {
        apply_company_rows(&ui, &companies, active_company_id);
    }).map_err(anyhow::Error::from)?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── setup ─────────────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn setup(ui: &MainWindow, ctx: std::sync::Arc<AppCtx>) {
    // ── Колбек: переключити активну компанію ─────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_switch_company(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_show_company_picker(true);
        }
    });

    // ── Колбек: обрати активну компанію ──────────────────────────────────────
    {
        let pool = ctx.pool.clone();
        let ui_weak = ui.as_weak();
        let ctx_c = ctx.clone();

        ui.on_company_select_clicked(move |id_str| {
            let pool = pool.clone();
            let ui_handle = ui_weak.clone();
            let ctx = ctx_c.clone();
            let id_s = id_str.to_string();

            tokio::spawn(async move {
                let Ok(uuid) = id_s.parse::<uuid::Uuid>() else {
                    tracing::error!("Некоректний UUID компанії: {id_s}");
                    return;
                };

                match db::companies::get_by_id(&pool, uuid).await {
                    Ok(Some(company)) => {
                        // Оновлюємо активну компанію
                        ctx.set_company_id(company.id);

                        // Зберігаємо вибір у конфігу
                        let mut cfg = AppConfig::load();
                        cfg.last_company_id = Some(company.id);
                        cfg.save();

                        let name = company_display_name(&company);
                        let subtitle = company_subtitle(&company);
                        let id_str = company.id.to_string();
                        let company_id = company.id;
                        let (cp_query, include_archived, cp_page) = {
                            let state = ctx.counterparty_state.lock().unwrap();
                            (state.query.clone(), state.include_archived, state.page)
                        };
                        let (act_query, status_filter) = {
                            let state = ctx.act_state.lock().unwrap();
                            (state.query.clone(), state.status_filter.clone())
                        };
                        {
                            let mut state = ctx.doc_state.lock().unwrap();
                            *state = DocListState::default();
                        }
                        {
                            let mut state = ctx.task_state.lock().unwrap();
                            *state = crate::app_ctx::TaskListState::default();
                        }
                        {
                            let mut state = ctx.payment_state.lock().unwrap();
                            *state = crate::app_ctx::PaymentListState::default();
                        }

                        ui_handle.upgrade_in_event_loop(move |ui| {
                            ui.set_active_company_name(SharedString::from(name.as_str()));
                            ui.set_active_company_id(SharedString::from(id_str.as_str()));
                            ui.set_active_company_subtitle(SharedString::from(subtitle.as_str()));
                            ui.set_show_company_picker(false);
                            ui.set_show_cp_form(false);
                            ui.set_show_act_form(false);
                            ui.set_show_task_form(false);
                            ui.set_show_payment_form(false);
                            ui.set_show_counterparty_card(false);
                            ui.set_doc_direction_index(0);
                            ui.set_doc_active_tab(0);
                            ui.set_doc_filter_cp_index(0);
                        }).warn_if_terminated();

                        if let Err(e) = crate::ui::counterparties::reload_counterparties(
                            &pool,
                            ui_handle.clone(),
                            company_id,
                            cp_query,
                            include_archived,
                            cp_page,
                            false,
                        )
                        .await
                        {
                            tracing::error!("Помилка оновлення контрагентів після вибору компанії: {e}");
                        }

                        if let Err(e) = crate::ui::acts::reload_acts(
                            &pool,
                            ui_handle.clone(),
                            company_id,
                            status_filter,
                            act_query,
                            false,
                        )
                        .await
                        {
                            tracing::error!("Помилка оновлення актів після вибору компанії: {e}");
                        }

                        if let Err(e) = crate::ui::payments::reload_payments(&pool, ui_handle.clone(), company_id, None, "").await {
                            tracing::error!("Помилка завантаження платежів після вибору компанії: {e}");
                        }

                        if let Err(e) = crate::ui::documents::reload_doc_cp_filter(&pool, ui_handle.clone(), company_id, &ctx.doc_cp_ids).await {
                            tracing::error!("Помилка оновлення фільтру контрагентів після вибору компанії: {e}");
                        }
                        if let Err(e) = crate::ui::documents::reload_documents(&pool, ui_handle.clone(), company_id, 0, "outgoing", "", None, None, None).await {
                            tracing::error!("Помилка завантаження документів після вибору компанії: {e}");
                        }
                        if let Err(e) = reload_settings(&pool, ui_handle.clone(), company_id).await {
                            tracing::error!("Помилка завантаження налаштувань після вибору компанії: {e}");
                        }
                        if let Err(e) = crate::ui::payments::reload_payment_counterparty_options(&pool, ui_handle.clone(), company_id).await {
                            tracing::error!("Помилка оновлення контрагентів для форми платежу після вибору компанії: {e}");
                        }
                    }
                    Ok(None) => tracing::warn!("Компанію {uuid} не знайдено."),
                    Err(e) => tracing::error!("Помилка вибору компанії: {e}"),
                }
            });
        });
    }

    // ── Колбек: додати нову компанію ─────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        ui.on_company_add_clicked(move || {
            if let Some(ui) = ui_weak.upgrade() {
                reset_company_form(&ui);
                ui.set_show_company_picker(false);
                ui.set_current_page(6);
                ui.set_show_company_form(true);
            }
        });
    }

    // ── Колбек: редагувати компанію ───────────────────────────────────────────
    {
        let pool = ctx.pool.clone();
        let ui_weak = ui.as_weak();

        ui.on_company_edit_clicked(move |id_str| {
            let pool = pool.clone();
            let ui_handle = ui_weak.clone();
            let id_s = id_str.to_string();

            tokio::spawn(async move {
                let Ok(uuid) = id_s.parse::<uuid::Uuid>() else {
                    tracing::error!("Некоректний UUID компанії: {id_s}");
                    return;
                };

                match db::companies::get_by_id(&pool, uuid).await {
                    Ok(Some(c)) => {
                        ui_handle.upgrade_in_event_loop(move |ui| {
                            ui.set_company_form_is_edit(true);
                            ui.set_company_form_edit_id(SharedString::from(c.id.to_string().as_str()));
                            ui.set_company_form_name(SharedString::from(c.name.as_str()));
                            ui.set_company_form_edrpou(SharedString::from(c.edrpou.as_deref().unwrap_or("")));
                            ui.set_company_form_iban(SharedString::from(c.iban.as_deref().unwrap_or("")));
                            ui.set_company_form_legal_address(SharedString::from(c.legal_address.as_deref().unwrap_or("")));
                            ui.set_company_form_director(SharedString::from(c.director_name.as_deref().unwrap_or("")));
                            ui.set_company_form_accountant(SharedString::from(c.accountant_name.as_deref().unwrap_or("")));
                            ui.set_company_form_is_vat(c.is_vat_payer);
                            ui.set_show_company_form(true);
                        }).warn_if_terminated();
                    }
                    Ok(None) => tracing::warn!("Компанію {uuid} не знайдено."),
                    Err(e) => tracing::error!("Помилка завантаження компанії: {e}"),
                }
            });
        });
    }

    // ── Колбек: редагувати активну компанію зі сторінки Налаштувань ──────────
    {
        let pool = ctx.pool.clone();
        let ui_weak = ui.as_weak();
        let ctx_c = ctx.clone();

        ui.on_settings_edit_company_clicked(move || {
            let pool = pool.clone();
            let ui_handle = ui_weak.clone();
            let company_id = ctx_c.company_id();

            tokio::spawn(async move {
                match db::companies::get_by_id(&pool, company_id).await {
                    Ok(Some(c)) => {
                        ui_handle
                            .upgrade_in_event_loop(move |ui| {
                                ui.set_company_form_is_edit(true);
                                ui.set_company_form_edit_id(SharedString::from(c.id.to_string().as_str()));
                                ui.set_company_form_name(SharedString::from(c.name.as_str()));
                                ui.set_company_form_edrpou(SharedString::from(c.edrpou.as_deref().unwrap_or("")));
                                ui.set_company_form_iban(SharedString::from(c.iban.as_deref().unwrap_or("")));
                                ui.set_company_form_legal_address(SharedString::from(c.legal_address.as_deref().unwrap_or("")));
                                ui.set_company_form_director(SharedString::from(c.director_name.as_deref().unwrap_or("")));
                                ui.set_company_form_accountant(SharedString::from(c.accountant_name.as_deref().unwrap_or("")));
                                ui.set_company_form_is_vat(c.is_vat_payer);
                                ui.set_current_page(6);
                                ui.set_show_company_form(true);
                            })
                            .warn_if_terminated();
                    }
                    Ok(None) => tracing::warn!("Активну компанію {company_id} не знайдено."),
                    Err(e) => tracing::error!("Помилка відкриття компанії з налаштувань: {e}"),
                }
            });
        });
    }

    // ── Колбек: архівувати компанію ───────────────────────────────────────────
    {
        let pool = ctx.pool.clone();
        let ui_weak = ui.as_weak();
        let ctx_c = ctx.clone();

        ui.on_company_archive_clicked(move |id_str| {
            let pool = pool.clone();
            let ui_handle = ui_weak.clone();
            let ctx = ctx_c.clone();
            let id_s = id_str.to_string();

            tokio::spawn(async move {
                let Ok(uuid) = id_s.parse::<uuid::Uuid>() else {
                    tracing::error!("Некоректний UUID компанії: {id_s}");
                    return;
                };

                match db::companies::archive(&pool, uuid).await {
                    Ok(true) => {
                        show_toast(ui_handle.clone(), "Компанію архівовано".to_string(), false);
                        let active_id = ctx.company_id();
                        if let Err(e) = reload_companies(&pool, ui_handle.clone(), active_id).await {
                            tracing::error!("Помилка оновлення списку компаній: {e}");
                        }

                        if ctx.company_id() == uuid {
                            match db::companies::list(&pool).await {
                                Ok(companies) if !companies.is_empty() => {
                                    let replacement = companies[0].clone();
                                    ctx.set_company_id(replacement.id);

                                    let mut cfg = AppConfig::load();
                                    cfg.last_company_id = Some(replacement.id);
                                    cfg.save();

                                    let name = company_display_name(&replacement);
                                    let subtitle = company_subtitle(&replacement);
                                    let replacement_id = replacement.id.to_string();

                                    ui_handle
                                        .upgrade_in_event_loop(move |ui| {
                                            ui.set_active_company_name(SharedString::from(name.as_str()));
                                            ui.set_active_company_id(SharedString::from(
                                                replacement_id.as_str(),
                                            ));
                                            ui.set_active_company_subtitle(SharedString::from(
                                                subtitle.as_str(),
                                            ));
                                        })
                                        .warn_if_terminated();
                                }
                                Ok(_) => {
                                    ui_handle
                                        .upgrade_in_event_loop(|ui| {
                                            ui.set_active_company_name(SharedString::from(
                                                "Оберіть компанію",
                                            ));
                                            ui.set_active_company_id(SharedString::from(""));
                                            ui.set_active_company_subtitle(SharedString::from(
                                                "Створіть першу компанію",
                                            ));
                                            ui.set_show_company_picker(false);
                                            ui.set_current_page(6);
                                            reset_company_form(&ui);
                                            ui.set_show_company_form(true);
                                        })
                                        .warn_if_terminated();
                                }
                                Err(e) => tracing::error!(
                                    "Помилка пошуку заміни активної компанії після архівації: {e}"
                                ),
                            }
                        }
                    }
                    Ok(false) => tracing::warn!("Компанію {uuid} не знайдено."),
                    Err(e) => tracing::error!("Помилка архівування компанії: {e}"),
                }
            });
        });
    }

    // ── Колбек: зберегти нову компанію ──────────────────────────────────────
    {
        let pool = ctx.pool.clone();
        let ui_weak = ui.as_weak();
        let ctx_c = ctx.clone();

        ui.on_company_form_save(move |name, edrpou, iban, address, director, _accountant, is_vat| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let ctx = ctx_c.clone();
            let data = NewCompany {
                name: name.to_string(),
                short_name: None,
                edrpou: if edrpou.trim().is_empty() { None } else { Some(edrpou.to_string()) },
                ipn: None,
                iban: if iban.trim().is_empty() { None } else { Some(iban.to_string()) },
                legal_address: if address.trim().is_empty() { None } else { Some(address.to_string()) },
                director_name: if director.trim().is_empty() { None } else { Some(director.to_string()) },
                tax_system: None,
                is_vat_payer: is_vat,
            };

            tokio::spawn(async move {
                if data.name.trim().is_empty() {
                    show_toast(ui_weak, "Введіть назву компанії".to_string(), true);
                    return;
                }
                match db::companies::create(&pool, &data).await {
                    Ok(c) => {
                        tracing::info!("Компанію '{}' створено (id={}).", c.name, c.id);
                        show_toast(ui_weak.clone(), format!("Компанію '{}' створено", c.name), false);
                        ctx.set_company_id(c.id);

                        // Заповнюємо стандартні категорії доходів/витрат
                        if let Err(e) = db::categories::seed_defaults(&pool, c.id).await {
                            tracing::warn!("Не вдалось заповнити категорії для нової компанії: {e}");
                        }

                        let mut cfg = AppConfig::load();
                        cfg.last_company_id = Some(c.id);
                        cfg.save();

                        let active_id = c.id;
                        if let Err(e) = reload_companies(&pool, ui_weak.clone(), active_id).await {
                            tracing::error!("Помилка оновлення списку компаній: {e}");
                        }
                        if let Err(e) = reload_settings(&pool, ui_weak.clone(), c.id).await {
                            tracing::error!("Помилка оновлення налаштувань компанії після створення: {e}");
                        }
                        if let Err(e) = crate::ui::payments::reload_payment_counterparty_options(&pool, ui_weak.clone(), c.id).await {
                            tracing::error!("Помилка оновлення контрагентів для форми платежу після створення компанії: {e}");
                        }

                        let (cp_query, include_archived, cp_page) = {
                            let state = ctx.counterparty_state.lock().unwrap();
                            (state.query.clone(), state.include_archived, state.page)
                        };
                        let (act_query, status_filter) = {
                            let state = ctx.act_state.lock().unwrap();
                            (state.query.clone(), state.status_filter.clone())
                        };

                        if let Err(e) = crate::ui::counterparties::reload_counterparties(
                            &pool,
                            ui_weak.clone(),
                            c.id,
                            cp_query,
                            include_archived,
                            cp_page,
                            false,
                        )
                        .await
                        {
                            tracing::error!("Помилка оновлення контрагентів після створення компанії: {e}");
                        }

                        if let Err(e) = crate::ui::acts::reload_acts(
                            &pool,
                            ui_weak.clone(),
                            c.id,
                            status_filter,
                            act_query,
                            false,
                        )
                        .await
                        {
                            tracing::error!("Помилка оновлення актів після створення компанії: {e}");
                        }

                        let name = company_display_name(&c);
                        let subtitle = company_subtitle(&c);
                        let id = c.id.to_string();
                        ui_weak
                            .upgrade_in_event_loop(move |ui| {
                                ui.set_active_company_name(SharedString::from(name.as_str()));
                                ui.set_active_company_id(SharedString::from(id.as_str()));
                                ui.set_active_company_subtitle(SharedString::from(subtitle.as_str()));
                                ui.set_show_company_picker(false);
                                ui.set_show_company_form(false);
                                ui.set_current_page(0);
                            })
                            .warn_if_terminated();
                    }
                    Err(e) => {
                        tracing::error!("Помилка створення компанії: {e}");
                        show_toast(ui_weak, format!("Помилка: {e}"), true);
                    }
                }
            });
        });
    }

    // ── Колбек: оновити компанію ─────────────────────────────────────────────
    {
        let pool = ctx.pool.clone();
        let ui_weak = ui.as_weak();
        let ctx_c = ctx.clone();

        ui.on_company_form_update(move |id, name, edrpou, iban, address, director, accountant, is_vat| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let ctx = ctx_c.clone();
            let edit_id = id.to_string();

            tokio::spawn(async move {
                let Ok(uuid) = edit_id.parse::<uuid::Uuid>() else {
                    tracing::error!("Некоректний edit_id компанії: {edit_id}");
                    return;
                };
                let data = UpdateCompany {
                    name: name.to_string(),
                    short_name: None,
                    edrpou: if edrpou.trim().is_empty() { None } else { Some(edrpou.to_string()) },
                    iban: if iban.trim().is_empty() { None } else { Some(iban.to_string()) },
                    legal_address: if address.trim().is_empty() { None } else { Some(address.to_string()) },
                    director_name: if director.trim().is_empty() { None } else { Some(director.to_string()) },
                    accountant_name: if accountant.trim().is_empty() { None } else { Some(accountant.to_string()) },
                    tax_system: None,
                    is_vat_payer: is_vat,
                    logo_path: None,
                };
                match db::companies::update(&pool, uuid, &data).await {
                    Ok(Some(c)) => {
                        tracing::info!("Компанію '{}' оновлено.", c.name);
                        show_toast(ui_weak.clone(), format!("Компанію '{}' оновлено", c.name), false);
                        let active_id = ctx.company_id();
                        if let Err(e) = reload_companies(&pool, ui_weak.clone(), active_id).await {
                            tracing::error!("Помилка оновлення списку компаній: {e}");
                        }
                        if ctx.company_id() == c.id {
                            if let Err(e) = reload_settings(&pool, ui_weak.clone(), c.id).await {
                                tracing::error!("Помилка оновлення налаштувань після редагування компанії: {e}");
                            }
                            if let Err(e) = crate::ui::payments::reload_payment_counterparty_options(&pool, ui_weak.clone(), c.id).await {
                                tracing::error!("Помилка оновлення контрагентів для форми платежу після редагування компанії: {e}");
                            }
                        }

                        if ctx.company_id() == c.id {
                            let name = company_display_name(&c);
                            let subtitle = company_subtitle(&c);
                            let id = c.id.to_string();
                            ui_weak
                                .upgrade_in_event_loop(move |ui| {
                                    ui.set_active_company_name(SharedString::from(name.as_str()));
                                    ui.set_active_company_id(SharedString::from(id.as_str()));
                                    ui.set_active_company_subtitle(SharedString::from(
                                        subtitle.as_str(),
                                    ));
                                    ui.set_show_company_form(false);
                                })
                                .warn_if_terminated();
                        } else {
                            ui_weak
                                .upgrade_in_event_loop(|ui| {
                                    ui.set_show_company_form(false);
                                })
                                .warn_if_terminated();
                        }
                    }
                    Ok(None) => tracing::warn!("Компанію {uuid} не знайдено."),
                    Err(e) => {
                        tracing::error!("Помилка оновлення компанії: {e}");
                        show_toast(ui_weak, format!("Помилка: {e}"), true);
                    }
                }
            });
        });
    }

    // ── Колбек: скасувати форму компанії ─────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        ui.on_company_form_cancel(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_show_company_form(false);
            }
        });
    }
}
