use slint::{ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use acta::db;
use acta::models::dashboard::{KpiSummary, MonthRevenue, RecentAct};

use crate::ui::helpers::{
    format_money, format_money_round, max_chart_value, normalize_chart_value, task_to_item,
};

pub struct DashboardData {
    pub revenue_str: String,
    pub expenses_str: String,
    pub net_str: String,
    pub outstanding_str: String,
    pub overdue_str: String,
    pub accounts_total: String,
    pub delta_revenue_str: String,
    pub delta_expenses_str: String,
    pub delta_net_str: String,
    pub ytd_total_str: String,
    pub ytd_revenue_str: String,
    pub ytd_expenses_str: String,
    pub spark_revenue: Vec<f32>,
    pub spark_expenses: Vec<f32>,
    pub journal: Vec<crate::JournalRow>,
    pub tasks: Vec<crate::TaskItem>,
    pub inbox: Vec<crate::InboxItem>,
    pub chart_bars: Vec<crate::ChartBar>,
}

pub fn recent_act_to_journal_row(a: &RecentAct) -> crate::JournalRow {
    crate::JournalRow {
        date: a.date.clone().into(),
        id: a.num.clone().into(),
        operation: "Акт".into(),
        counterparty: a.contractor.clone().into(),
        debit_str: "".into(),
        credit_str: format_money(a.amount).into(),
        is_credit: true,
        status_label: a.status.clone().into(),
        status_tone: slint::SharedString::default(),
    }
}

/// Перетворює рядок InboxRow у Slint InboxItem.
pub fn inbox_item_from_row(
    doc_id: &str,
    doc_number: &str,
    counterparty: &str,
    amount: rust_decimal::Decimal,
    age_days: i32,
    kind: &str,
    action_label: &str,
) -> crate::InboxItem {
    crate::InboxItem {
        kind: kind.into(),
        doc_id: doc_id.into(),
        doc_number: doc_number.into(),
        counterparty: counterparty.into(),
        amount_str: format_money(amount).into(),
        age_days,
        action_label: action_label.into(),
    }
}

pub fn revenue_months_to_chart_bars(months: &[MonthRevenue]) -> Vec<crate::ChartBar> {
    let max = max_chart_value(months.iter().map(|month| &month.amount));

    months
        .iter()
        .map(|m| crate::ChartBar {
            rev_h: normalize_chart_value(m.amount, max),
            exp_h: 0.0,
            month: m.month_label().into(),
        })
        .collect()
}

pub async fn prepare_dashboard_data(pool: &PgPool, company_id: Uuid) -> DashboardData {
    let (kpi_res, recent_res, tasks_res, inbox_res, rev_months_res) = tokio::join!(
        db::dashboard::get_kpi_summary(pool, company_id),
        db::dashboard::get_recent_acts(pool, company_id, 20),
        db::tasks::list_open(pool, company_id),
        db::dashboard::inbox_items(pool, company_id),
        db::dashboard::revenue_by_month(pool, company_id, 6),
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
    let inbox_rows = inbox_res.unwrap_or_default();
    let rev_months = rev_months_res.unwrap_or_default();

    let journal: Vec<crate::JournalRow> = recent.iter().map(recent_act_to_journal_row).collect();
    let task_items: Vec<crate::TaskItem> = tasks.iter().map(task_to_item).collect();
    let inbox: Vec<crate::InboxItem> = inbox_rows
        .iter()
        .map(|r| {
            inbox_item_from_row(
                &r.doc_id,
                &r.doc_number,
                &r.counterparty,
                r.amount,
                r.age_days,
                &r.kind,
                &r.action_label,
            )
        })
        .collect();
    let chart_bars = revenue_months_to_chart_bars(&rev_months);

    DashboardData {
        revenue_str: format_money_round(kpi.revenue_this_month),
        expenses_str: "0".to_string(),
        net_str: format_money_round(kpi.revenue_this_month),
        outstanding_str: format_money_round(kpi.unpaid_total),
        overdue_str: "0".to_string(),
        accounts_total: "0".to_string(),
        delta_revenue_str: String::new(),
        delta_expenses_str: String::new(),
        delta_net_str: String::new(),
        ytd_total_str: "0".to_string(),
        ytd_revenue_str: "0".to_string(),
        ytd_expenses_str: "0".to_string(),
        spark_revenue: vec![],
        spark_expenses: vec![],
        journal,
        tasks: task_items,
        inbox,
        chart_bars,
    }
}

pub fn apply_dashboard_to_ui(ui: &crate::AppWindow, data: DashboardData) {
    let metrics = crate::DashboardMetrics {
        revenue_month: data.revenue_str.clone().into(),
        expenses_month: data.expenses_str.clone().into(),
        net_month: data.net_str.clone().into(),
        outstanding: data.outstanding_str.clone().into(),
        overdue: data.overdue_str.clone().into(),
        delta_revenue: data.delta_revenue_str.clone().into(),
        delta_expenses: data.delta_expenses_str.clone().into(),
        delta_net: data.delta_net_str.clone().into(),
    };

    ui.set_dashboard(crate::DashboardViewData {
        metrics,
        chart_bars: ModelRc::new(VecModel::from(data.chart_bars)),
        journal: ModelRc::new(VecModel::from(data.journal)),
        accounts: ModelRc::new(VecModel::<crate::AccountItem>::default()),
        accounts_total: data.accounts_total.into(),
        tasks: ModelRc::new(VecModel::from(data.tasks)),
        revenue_str: data.revenue_str.into(),
        expenses_str: data.expenses_str.into(),
        net_str: data.net_str.into(),
        outstanding_str: data.outstanding_str.into(),
        overdue_str: data.overdue_str.into(),
        delta_revenue_str: data.delta_revenue_str.into(),
        delta_expenses_str: data.delta_expenses_str.into(),
        delta_net_str: data.delta_net_str.into(),
        ytd_total_str: data.ytd_total_str.into(),
        ytd_revenue_str: data.ytd_revenue_str.into(),
        ytd_expenses_str: data.ytd_expenses_str.into(),
        spark_revenue: ModelRc::new(VecModel::from(data.spark_revenue)),
        spark_expenses: ModelRc::new(VecModel::from(data.spark_expenses)),
        inbox: ModelRc::new(VecModel::from(data.inbox)),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn inbox_item_from_row_maps_kind_and_action() {
        let item = inbox_item_from_row(
            "act:abc123",
            "АКТ-2026-001",
            "ТОВ Тест",
            dec!(15000),
            45,
            "overdue",
            "Нагадати",
        );
        assert_eq!(item.kind.as_str(), "overdue");
        assert_eq!(item.doc_number.as_str(), "АКТ-2026-001");
        assert_eq!(item.counterparty.as_str(), "ТОВ Тест");
        assert_eq!(item.age_days, 45);
        assert_eq!(item.action_label.as_str(), "Нагадати");
        assert!(
            item.amount_str.contains("15 000"),
            "amount: {}",
            item.amount_str
        );
    }

    #[test]
    fn chart_bars_normalized_to_max_revenue() {
        use acta::models::dashboard::MonthRevenue;
        use rust_decimal_macros::dec;

        let months = vec![
            MonthRevenue {
                month_num: 1,
                year: 2026,
                amount: dec!(1000),
            },
            MonthRevenue {
                month_num: 2,
                year: 2026,
                amount: dec!(2000),
            },
            MonthRevenue {
                month_num: 3,
                year: 2026,
                amount: dec!(0),
            },
        ];
        let bars = revenue_months_to_chart_bars(&months);
        assert_eq!(bars.len(), 3);
        assert!((bars[1].rev_h - 1.0).abs() < 0.001, "max bar should be 1.0");
        assert!((bars[0].rev_h - 0.5).abs() < 0.01);
        assert!((bars[2].rev_h).abs() < 0.001);
        assert_eq!(bars[0].month.as_str(), "Січ");
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
        assert_eq!(row.credit_str.as_str(), "1 000");
        assert!(row.is_credit);
    }
}
