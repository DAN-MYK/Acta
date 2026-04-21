// Dashboard модуль — підготовка та відображення даних головного екрану.
//
// Розділено на:
//  - `prepare_dashboard_data` — async, збирає дані з БД (токіо-задача)
//  - `apply_dashboard_to_ui`  — sync, записує дані у Slint властивості

use slint::{ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use acta::db;
use acta::models::dashboard::{KpiSummary, RecentAct};

use crate::ui::helpers::{decimal_to_f32, task_to_item};

/// Усі дані Dashboard, готові для передачі у Slint (Send-safe).
pub struct DashboardData {
    pub metrics: crate::DashboardMetrics,
    pub revenue_str: String,
    pub outstanding_str: String,
    pub journal: Vec<crate::JournalRow>,
    pub tasks: Vec<crate::TaskItem>,
}

/// Перетворює `KpiSummary` у `DashboardMetrics` для Slint.
///
/// Поля expenses/net/delta заповнюються нулями — дані для них
/// будуть додані окремим запитом у наступних ітераціях.
pub fn kpi_to_metrics(kpi: &KpiSummary) -> crate::DashboardMetrics {
    crate::DashboardMetrics {
        revenue_month: decimal_to_f32(kpi.revenue_this_month),
        expenses_month: 0.0,
        net_month: decimal_to_f32(kpi.revenue_this_month),
        outstanding: decimal_to_f32(kpi.unpaid_total),
        overdue: 0.0,
        delta_revenue: 0.0,
        delta_expenses: 0.0,
        delta_net: 0.0,
    }
}

/// Перетворює `RecentAct` у рядок журналу `JournalRow`.
///
/// Акти завжди відображаються на кредитній стороні (надходження).
pub fn recent_act_to_journal_row(a: &RecentAct) -> crate::JournalRow {
    crate::JournalRow {
        date: a.date.clone().into(),
        id: a.num.clone().into(),
        operation: "Акт".into(),
        counterparty: a.contractor.clone().into(),
        debit: 0.0,
        credit: decimal_to_f32(a.amount),
        is_credit: true,
        status_label: a.status.clone().into(),
        status_tone: slint::SharedString::default(),
    }
}

/// Паралельно завантажує KPI, нещодавні акти та відкриті задачі з БД.
///
/// Використовує `tokio::join!` — три запити виконуються одночасно.
/// При помилці будь-якого запиту — логує і повертає порожні дані.
pub async fn prepare_dashboard_data(pool: &PgPool, company_id: Uuid) -> DashboardData {
    let (kpi_res, recent_res, tasks_res) = tokio::join!(
        db::dashboard::get_kpi_summary(pool, company_id),
        db::dashboard::get_recent_acts(pool, company_id, 20),
        db::tasks::list_open(pool),
    );

    let kpi = kpi_res.unwrap_or_else(|e| {
        tracing::error!("dashboard kpi failed: {e}");
        KpiSummary {
            revenue_this_month: rust_decimal::Decimal::ZERO,
            unpaid_total: rust_decimal::Decimal::ZERO,
            acts_this_month: 0,
            active_counterparties: 0,
        }
    });

    let recent = recent_res.unwrap_or_default();
    let tasks = tasks_res.unwrap_or_default();

    let journal: Vec<crate::JournalRow> = recent.iter().map(recent_act_to_journal_row).collect();
    let task_items: Vec<crate::TaskItem> = tasks.iter().map(task_to_item).collect();

    DashboardData {
        revenue_str: format!("{:.0}", kpi.revenue_this_month),
        outstanding_str: format!("{:.0}", kpi.unpaid_total),
        metrics: kpi_to_metrics(&kpi),
        journal,
        tasks: task_items,
    }
}

/// Записує підготовлені дані Dashboard у властивості Slint вікна.
///
/// Викликається з головного потоку (або через `upgrade_in_event_loop`).
pub fn apply_dashboard_to_ui(ui: &crate::AppWindow, data: DashboardData) {
    ui.set_dash_metrics(data.metrics);
    ui.set_dash_revenue_str(data.revenue_str.into());
    ui.set_dash_outstanding_str(data.outstanding_str.into());
    ui.set_dash_journal(ModelRc::new(VecModel::from(data.journal)));
    ui.set_dash_tasks(ModelRc::new(VecModel::from(data.tasks)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use acta::models::dashboard::KpiSummary;
    use rust_decimal_macros::dec;

    #[test]
    fn kpi_summary_converts_to_dashboard_metrics() {
        let kpi = KpiSummary {
            revenue_this_month: dec!(50000.00),
            unpaid_total: dec!(12000.00),
            acts_this_month: 5,
            active_counterparties: 10,
        };
        let metrics = kpi_to_metrics(&kpi);
        assert!((metrics.revenue_month - 50000.0_f32).abs() < 1.0);
        assert!((metrics.outstanding - 12000.0_f32).abs() < 1.0);
    }

    #[test]
    fn recent_act_converts_to_journal_row() {
        use acta::models::dashboard::RecentAct;
        let act = RecentAct {
            num: "АКТ-001".to_string(),
            contractor: "ТОВ Тест".to_string(),
            amount: dec!(1000.00),
            status: "Видано".to_string(),
            date: "21.04.2026".to_string(),
        };
        let row = recent_act_to_journal_row(&act);
        assert_eq!(row.id.as_str(), "АКТ-001");
        assert_eq!(row.counterparty.as_str(), "ТОВ Тест");
        assert!((row.credit - 1000.0_f32).abs() < 0.01);
        assert!(row.is_credit);
    }
}
