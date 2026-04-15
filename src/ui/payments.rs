// ui/payments.rs — колбеки та дані для сторінки Платежі.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

use acta::app_ctx::AppCtx;
use crate::{
    ui::helpers::*,
    MainWindow, PaymentRow,
};
use acta::{db, models};

// ═══════════════════════════════════════════════════════════════════════════════
// ── Проміжні дані ──────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct PaymentsUiData {
    pub payment_rows: Vec<PaymentRow>,
    pub total_income: SharedString,
    pub total_expense: SharedString,
}

pub async fn prepare_payments_data(
    pool: &sqlx::PgPool,
    company_id: uuid::Uuid,
    direction: Option<models::payment::PaymentDirection>,
    query: &str,
) -> Result<PaymentsUiData> {
    let query_lower = query.trim().to_lowercase();
    let rows = db::payments::list(pool, company_id, direction)
        .await?
        .into_iter()
        .filter(|row| {
            if query_lower.is_empty() {
                return true;
            }
            let haystack = [
                row.date.as_str(),
                row.counterparty_name.as_deref().unwrap_or(""),
                row.description.as_deref().unwrap_or(""),
                row.bank_name.as_deref().unwrap_or(""),
            ]
            .join(" ")
            .to_lowercase();
            haystack.contains(query_lower.as_str())
        })
        .collect::<Vec<_>>();

    let mut total_income = Decimal::ZERO;
    let mut total_expense = Decimal::ZERO;
    for r in &rows {
        match r.direction {
            models::payment::PaymentDirection::Income => total_income += r.amount,
            models::payment::PaymentDirection::Expense => total_expense += r.amount,
        }
    }

    let payment_rows: Vec<PaymentRow> = rows
        .iter()
        .map(|r| PaymentRow {
            id: SharedString::from(r.id.to_string().as_str()),
            date: SharedString::from(r.date.as_str()),
            counterparty: SharedString::from(r.counterparty_name.as_deref().unwrap_or("")),
            description: SharedString::from(r.description.as_deref().unwrap_or("")),
            bank: SharedString::from(r.bank_name.as_deref().unwrap_or("")),
            amount: SharedString::from(format!("{:.2}", r.amount).as_str()),
            direction: SharedString::from(r.direction.label()),
            reconciled: r.is_reconciled,
        })
        .collect();

    Ok(PaymentsUiData {
        payment_rows,
        total_income: SharedString::from(format!("{:.2}", total_income).as_str()),
        total_expense: SharedString::from(format!("{:.2}", total_expense).as_str()),
    })
}

pub fn apply_payments_to_ui(ui: &MainWindow, d: PaymentsUiData) {
    ui.set_payment_rows(ModelRc::new(VecModel::from(d.payment_rows)));
    ui.set_payment_total_income(d.total_income);
    ui.set_payment_total_expense(d.total_expense);
    ui.set_payments_loading(false);
    ui.set_show_payment_form(false);
}

pub async fn reload_payments(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    direction: Option<models::payment::PaymentDirection>,
    query: &str,
) -> Result<()> {
    let d = prepare_payments_data(pool, company_id, direction, query).await?;
    ui_weak
        .upgrade_in_event_loop(move |ui| apply_payments_to_ui(&ui, d))
        .map_err(anyhow::Error::from)
}

#[derive(Clone)]
pub struct PaymentCpOptionsUiData {
    pub names: Vec<SharedString>,
    pub ids: Vec<SharedString>,
}

pub async fn prepare_payment_cp_options_data(
    pool: &sqlx::PgPool,
    company_id: uuid::Uuid,
) -> Result<PaymentCpOptionsUiData> {
    let counterparties = db::counterparties::list(pool, company_id).await?;
    let mut names = vec![SharedString::from("Без контрагента")];
    let mut ids = vec![SharedString::from("")];
    for cp in counterparties {
        names.push(SharedString::from(cp.name.as_str()));
        ids.push(SharedString::from(cp.id.to_string().as_str()));
    }
    Ok(PaymentCpOptionsUiData { names, ids })
}

pub async fn reload_payment_counterparty_options(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
) -> Result<()> {
    let d = prepare_payment_cp_options_data(pool, company_id).await?;
    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_payment_form_counterparty_names(ModelRc::new(VecModel::from(d.names)));
            ui.set_payment_form_counterparty_ids(ModelRc::new(VecModel::from(d.ids)));
            if !ui.get_payment_form_is_edit() {
                ui.set_payment_form_counterparty_index(0);
            }
        })
        .map_err(anyhow::Error::from)
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── setup — реєстрація колбеків ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn setup(ui: &MainWindow, ctx: Arc<AppCtx>) {
    let payment_state = ctx.payment_state.clone();

    // ── Фільтр напрямку ───────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = payment_state.clone();
    ui.on_payment_direction_filter_changed(move |index| {
        use models::payment::PaymentDirection;
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let direction: Option<PaymentDirection> = match index {
            1 => Some(PaymentDirection::Income),
            2 => Some(PaymentDirection::Expense),
            _ => None,
        };
        let query = {
            let mut s = state.lock().unwrap();
            s.direction_filter = direction.clone();
            s.query.clone()
        };
        tokio::spawn(async move {
            if let Err(e) = reload_payments(&pool, ui_weak, cid, direction, &query).await {
                tracing::error!("Помилка фільтрації платежів: {e}");
            }
        });
    });

    // ── Пошук ─────────────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = payment_state.clone();
    let search_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>> = Arc::new(Mutex::new(None));
    ui.on_payment_search_changed(move |query| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let (query, direction) = {
            let mut s = state.lock().unwrap();
            s.query = query.to_string();
            (s.query.clone(), s.direction_filter.clone())
        };
        let handle = tokio::spawn(async move {
            if let Err(e) = reload_payments(&pool, ui_weak, cid, direction, &query).await {
                tracing::error!("Помилка пошуку платежів: {e}");
            }
        });
        if let Some(old) = search_task.lock().unwrap().replace(handle) {
            old.abort();
        }
    });

    // ── Зведення платежу ─────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = payment_state.clone();
    ui.on_payment_reconcile_clicked(move |id_str| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let (query, direction) = {
            let s = state.lock().unwrap();
            (s.query.clone(), s.direction_filter.clone())
        };
        let id_s = id_str.to_string();
        tokio::spawn(async move {
            let Ok(uuid) = id_s.parse::<uuid::Uuid>() else {
                tracing::error!("Невалідний UUID платежу: {id_s}");
                return;
            };
            if let Err(e) = db::payments::mark_reconciled(&pool, uuid).await {
                tracing::error!("Помилка зведення платежу: {e}");
                return;
            }
            if let Err(e) = reload_payments(&pool, ui_weak, cid, direction, &query).await {
                tracing::error!("Помилка оновлення платежів: {e}");
            }
        });
    });

    // ── Новий платіж ──────────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_payment_create_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            reset_payment_form(&ui);
            ui.set_show_payment_form(true);
        }
    });

    // ── Редагувати платіж ─────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    ui.on_payment_edit_clicked(move |id| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let id_str = id.to_string();
        tokio::spawn(async move {
            let Ok(payment_id) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID платежу: {id_str}");
                return;
            };
            let counterparties = match db::counterparties::list(&pool, cid).await {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!("Помилка завантаження контрагентів для форми платежу: {e}");
                    return;
                }
            };
            match db::payments::get_by_id(&pool, payment_id).await {
                Ok(Some(payment)) => {
                    ui_weak
                        .upgrade_in_event_loop(move |ui| {
                            populate_payment_form(&ui, &counterparties, &payment);
                            ui.set_show_payment_form(true);
                        })
                        .warn_if_terminated();
                }
                Ok(None) => tracing::warn!("Платіж {payment_id} не знайдено."),
                Err(e) => {
                    tracing::error!("Помилка завантаження платежу для редагування: {e}")
                }
            }
        });
    });

    // ── Видалити платіж ───────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = payment_state.clone();
    ui.on_payment_delete_clicked(move |id| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let (query, direction) = {
            let s = state.lock().unwrap();
            (s.query.clone(), s.direction_filter.clone())
        };
        let id_str = id.to_string();
        tokio::spawn(async move {
            let Ok(payment_id) = id_str.parse::<uuid::Uuid>() else {
                tracing::error!("Некоректний UUID платежу: {id_str}");
                return;
            };
            match db::payments::delete(&pool, payment_id).await {
                Ok(()) => {
                    show_toast(ui_weak.clone(), "Платіж видалено".into(), false);
                    if let Err(e) = reload_payments(&pool, ui_weak, cid, direction, &query).await {
                        tracing::error!("Помилка оновлення списку платежів після видалення: {e}");
                    }
                }
                Err(e) => tracing::error!("Помилка видалення платежу: {e}"),
            }
        });
    });

    // ── Зберегти платіж ───────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = payment_state.clone();
    ui.on_payment_form_save(
        move |date, amount, direction, counterparty_id, bank_name, bank_ref, description| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let cid = *company_id_arc.lock().unwrap();
            let (query, direction_filter) = {
                let s = state.lock().unwrap();
                (s.query.clone(), s.direction_filter.clone())
            };
            let date = date.to_string();
            let amount = amount.to_string();
            let counterparty_id = counterparty_id.to_string();
            let bank_name = bank_name.to_string();
            let bank_ref = bank_ref.to_string();
            let description = description.to_string();
            tokio::spawn(async move {
                let date = match NaiveDate::parse_from_str(&date, "%d.%m.%Y") {
                    Ok(value) => value,
                    Err(e) => {
                        tracing::error!("Невірний формат дати платежу: {e}");
                        show_toast(ui_weak, "Невірний формат дати".into(), true);
                        return;
                    }
                };
                let amount = match amount.parse::<Decimal>() {
                    Ok(value) => value,
                    Err(e) => {
                        tracing::error!("Невірний формат суми платежу: {e}");
                        show_toast(ui_weak, "Невірний формат суми".into(), true);
                        return;
                    }
                };
                let data = models::payment::NewPayment {
                    company_id: cid,
                    date,
                    amount,
                    direction: if direction == 0 {
                        models::payment::PaymentDirection::Income
                    } else {
                        models::payment::PaymentDirection::Expense
                    },
                    counterparty_id: parse_optional_uuid(&counterparty_id),
                    bank_name: optional_text(&bank_name),
                    bank_ref: optional_text(&bank_ref),
                    description: optional_text(&description),
                };
                match db::payments::create(&pool, data).await {
                    Ok(payment) => {
                        show_toast(
                            ui_weak.clone(),
                            format!("Платіж на {:.2} збережено", payment.amount),
                            false,
                        );
                        if let Err(e) =
                            reload_payments(&pool, ui_weak, cid, direction_filter, &query).await
                        {
                            tracing::error!("Помилка оновлення платежів після створення: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Помилка створення платежу: {e}");
                        show_toast(ui_weak, format!("Помилка: {e}"), true);
                    }
                }
            });
        },
    );

    // ── Оновити платіж ────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = payment_state.clone();
    ui.on_payment_form_update(
        move |date, amount, direction, counterparty_id, bank_name, bank_ref, description| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let cid = *company_id_arc.lock().unwrap();
            let edit_id = ui_weak
                .upgrade()
                .map(|ui| ui.get_payment_form_edit_id().to_string())
                .unwrap_or_default();
            let (query, direction_filter) = {
                let s = state.lock().unwrap();
                (s.query.clone(), s.direction_filter.clone())
            };
            let date = date.to_string();
            let amount = amount.to_string();
            let counterparty_id = counterparty_id.to_string();
            let bank_name = bank_name.to_string();
            let bank_ref = bank_ref.to_string();
            let description = description.to_string();
            tokio::spawn(async move {
                let payment_id = match edit_id.parse::<uuid::Uuid>() {
                    Ok(value) => value,
                    Err(e) => {
                        tracing::error!("Некоректний UUID платежу для оновлення: {e}");
                        show_toast(ui_weak, "Не вдалося визначити платіж".into(), true);
                        return;
                    }
                };
                let date = match NaiveDate::parse_from_str(&date, "%d.%m.%Y") {
                    Ok(value) => value,
                    Err(e) => {
                        tracing::error!("Невірний формат дати платежу: {e}");
                        show_toast(ui_weak, "Невірний формат дати".into(), true);
                        return;
                    }
                };
                let amount = match amount.parse::<Decimal>() {
                    Ok(value) => value,
                    Err(e) => {
                        tracing::error!("Невірний формат суми платежу: {e}");
                        show_toast(ui_weak, "Невірний формат суми".into(), true);
                        return;
                    }
                };
                let data = models::payment::UpdatePayment {
                    date,
                    amount,
                    direction: if direction == 0 {
                        models::payment::PaymentDirection::Income
                    } else {
                        models::payment::PaymentDirection::Expense
                    },
                    counterparty_id: parse_optional_uuid(&counterparty_id),
                    bank_name: optional_text(&bank_name),
                    bank_ref: optional_text(&bank_ref),
                    description: optional_text(&description),
                };
                match db::payments::update(&pool, payment_id, data).await {
                    Ok(Some(payment)) => {
                        show_toast(
                            ui_weak.clone(),
                            format!("Платіж на {:.2} оновлено", payment.amount),
                            false,
                        );
                        if let Err(e) =
                            reload_payments(&pool, ui_weak, cid, direction_filter, &query).await
                        {
                            tracing::error!("Помилка оновлення платежів після редагування: {e}");
                        }
                    }
                    Ok(None) => {
                        tracing::warn!("Платіж {payment_id} не знайдено під час оновлення.");
                        show_toast(ui_weak, "Платіж не знайдено".into(), true);
                    }
                    Err(e) => {
                        tracing::error!("Помилка оновлення платежу: {e}");
                        show_toast(ui_weak, format!("Помилка: {e}"), true);
                    }
                }
            });
        },
    );

    // ── Скасувати форму платежу ───────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_payment_form_cancel(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_show_payment_form(false);
        }
    });
}
