// ui/dashboard.rs — Dashboard сторінка.

use std::sync::Arc;

use anyhow::Result;
use chrono::Datelike;
use chrono::Local;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

use crate::{
    app_ctx::AppCtx,
    ui::helpers::*,
    DashboardActRow, DashboardPaymentRow, MainWindow,
};
use acta::db;

pub async fn reload_dashboard(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
) -> Result<()> {
    let (kpi_res, bars_res, status_res, payments_res, recent_res) = tokio::join!(
        db::dashboard::get_kpi_summary(pool, company_id),
        db::dashboard::revenue_by_month(pool, company_id, 6),
        db::dashboard::acts_status_distribution(pool, company_id),
        db::dashboard::upcoming_payments(pool, company_id, 3),
        db::dashboard::get_recent_acts(pool, company_id, 5),
    );

    let kpi = kpi_res?;
    let bars = bars_res?;
    let statuses = status_res?;
    let payments = payments_res?;
    let recent = recent_res?;

    // ── KPI форматування ────────────────────────────────────────────────────
    let revenue_str = format_kpi_amount(kpi.revenue_this_month);
    let unpaid_str = format_kpi_amount(kpi.unpaid_total);
    let acts_str = kpi.acts_this_month.to_string();
    let cp_str = kpi.active_counterparties.to_string();

    // ── Бар-чарт ───────────────────────────────────────────────────────────
    let max_amount = bars.iter().map(|b| b.amount).max().unwrap_or(Decimal::ONE);
    let max_f = if max_amount.is_zero() { Decimal::ONE } else { max_amount };

    let bar_labels: Vec<SharedString> =
        bars.iter().map(|b| SharedString::from(b.month_label())).collect();
    let bar_val_lbls: Vec<SharedString> = bars
        .iter()
        .map(|b| {
            if b.amount.is_zero() {
                SharedString::from("")
            } else {
                SharedString::from(format_kpi_amount(b.amount))
            }
        })
        .collect();
    let bar_fractions: Vec<f32> = bars
        .iter()
        .map(|b| (b.amount / max_f).to_f64().unwrap_or(0.0) as f32)
        .collect();

    // ── Статуси ─────────────────────────────────────────────────────────────
    let get_count = |st: &str| -> i32 {
        statuses.iter().find(|s| s.status == st).map(|s| s.count as i32).unwrap_or(0)
    };
    let paid_count = get_count("paid");
    let issued_count = get_count("issued");
    let signed_count = get_count("signed");
    let draft_count = get_count("draft");

    // ── Останні акти ────────────────────────────────────────────────────────
    let recent_rows: Vec<DashboardActRow> = recent
        .iter()
        .map(|a| DashboardActRow {
            num: SharedString::from(a.num.as_str()),
            contractor: SharedString::from(a.contractor.as_str()),
            amount: SharedString::from(format_amount_ua(a.amount).as_str()),
            status: SharedString::from(a.status.as_str()),
            date: SharedString::from(a.date.as_str()),
        })
        .collect();

    // ── Очікувані платежі ────────────────────────────────────────────────────
    let payment_rows: Vec<DashboardPaymentRow> = payments
        .iter()
        .map(|p| DashboardPaymentRow {
            date_label: SharedString::from(p.date_label.as_str()),
            contractor: SharedString::from(p.contractor.as_str()),
            amount: SharedString::from(format_amount_ua(p.amount).as_str()),
            is_overdue: p.is_overdue,
        })
        .collect();

    // ── Підпис місяця ────────────────────────────────────────────────────────
    let now = Local::now();
    let month_ua = match now.month() {
        1 => "Січень",
        2 => "Лютий",
        3 => "Березень",
        4 => "Квітень",
        5 => "Травень",
        6 => "Червень",
        7 => "Липень",
        8 => "Серпень",
        9 => "Вересень",
        10 => "Жовтень",
        11 => "Листопад",
        12 => "Грудень",
        _ => "",
    };
    let month_label = format!("{} {}", month_ua, now.year());

    // ── Передача в UI ────────────────────────────────────────────────────────
    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_dashboard_kpi_revenue(SharedString::from(revenue_str.as_str()));
            ui.set_dashboard_kpi_unpaid(SharedString::from(unpaid_str.as_str()));
            ui.set_dashboard_kpi_acts_month(SharedString::from(acts_str.as_str()));
            ui.set_dashboard_kpi_counterparties(SharedString::from(cp_str.as_str()));
            ui.set_dashboard_month_label(SharedString::from(month_label.as_str()));

            ui.set_dashboard_chart_bar_labels(ModelRc::new(VecModel::from(bar_labels)));
            ui.set_dashboard_chart_bar_value_labels(ModelRc::new(VecModel::from(bar_val_lbls)));
            ui.set_dashboard_chart_bar_fractions(ModelRc::new(VecModel::from(bar_fractions)));

            ui.set_dashboard_status_paid(paid_count);
            ui.set_dashboard_status_issued(issued_count);
            ui.set_dashboard_status_signed(signed_count);
            ui.set_dashboard_status_draft(draft_count);

            ui.set_dashboard_recent_acts(ModelRc::new(VecModel::from(recent_rows)));
            ui.set_dashboard_upcoming_payments(ModelRc::new(VecModel::from(payment_rows)));
        })
        .map_err(anyhow::Error::from)?;

    Ok(())
}

pub fn setup(ui: &MainWindow, ctx: Arc<AppCtx>) {
    // Оновити дані Dashboard
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let cid_arc = ctx.active_company_id.clone();
    ui.on_dashboard_refresh(move || {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *cid_arc.lock().unwrap();
        tokio::spawn(async move {
            if let Err(e) = reload_dashboard(&pool, ui_weak, cid).await {
                tracing::error!("Dashboard refresh помилка: {e:#}");
            }
        });
    });

    // «+ Новий акт» на Dashboard
    let ui_weak = ui.as_weak();
    ui.on_dashboard_new_act_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_current_page(1);
            ui.invoke_act_create_clicked();
        }
    });

    // «Всі акти →» на Dashboard
    let ui_weak = ui.as_weak();
    ui.on_dashboard_all_acts_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_current_page(1);
        }
    });

    // «Відкрити →» To-Do
    let ui_weak = ui.as_weak();
    ui.on_dashboard_add_todo_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_current_page(5);
        }
    });
}
