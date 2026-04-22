use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use notify_rust::{Notification, Timeout};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use tokio::fs;
use tokio::process::Command;
use uuid::Uuid;

use acta::app_ctx::AppCtx;
use acta::db;
use acta::models::dashboard::{CategoryRevenue, MonthRevenue};

use crate::ui::helpers::format_money_round;

pub struct ReportsData {
    pub metrics: crate::ReportMetrics,
    pub chart_bars: Vec<crate::ChartBar>,
    pub categories: Vec<crate::ExpenseCategory>,
    pub drill_rows: Vec<crate::DrillRow>,
}

#[derive(Clone, Default)]
struct ReportsUiState {
    period: i32,
    drill_category: String,
}

fn notify_user(summary: &str, body: &str) {
    let _ = Notification::new()
        .appname("Acta")
        .summary(summary)
        .body(body)
        .timeout(Timeout::Milliseconds(6_000))
        .show();
}

fn reports_dir() -> PathBuf {
    PathBuf::from("storage/reports")
}

fn period_label(period: i32) -> &'static str {
    match period {
        0 => "Місяць",
        1 => "Квартал",
        2 => "Рік",
        3 => "Власний",
        _ => "Квартал",
    }
}

fn report_stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn typst_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn period_to_months(period: i32) -> u32 {
    match period {
        0 => 1,
        1 => 3,
        2 => 12,
        3 => 3,
        _ => 3,
    }
}

fn build_chart_bars(revenue: &[MonthRevenue], expenses: &[MonthRevenue]) -> Vec<crate::ChartBar> {
    let max_revenue = revenue
        .iter()
        .filter_map(|row| row.amount.to_f64())
        .fold(0.0_f64, f64::max);
    let max_expenses = expenses
        .iter()
        .filter_map(|row| row.amount.to_f64())
        .fold(0.0_f64, f64::max);
    let max_value = max_revenue.max(max_expenses);

    revenue
        .iter()
        .enumerate()
        .map(|(index, revenue_row)| {
            let expense_amount = expenses
                .get(index)
                .map(|row| row.amount)
                .unwrap_or(Decimal::ZERO);

            let rev_h = if max_value > 0.0 {
                (revenue_row.amount.to_f64().unwrap_or(0.0) / max_value) as f32
            } else {
                0.0
            };
            let exp_h = if max_value > 0.0 {
                (expense_amount.to_f64().unwrap_or(0.0) / max_value) as f32
            } else {
                0.0
            };

            crate::ChartBar {
                rev_h,
                exp_h,
                month: revenue_row.month_label().into(),
            }
        })
        .collect()
}

fn build_expense_categories(categories: &[CategoryRevenue]) -> Vec<crate::ExpenseCategory> {
    let total: Decimal = categories.iter().map(|category| category.amount).sum();

    categories
        .iter()
        .map(|category| {
            let percent = if total > Decimal::ZERO {
                ((category.amount / total) * Decimal::from(100))
                    .round()
                    .to_i32()
                    .unwrap_or(0)
            } else {
                0
            };

            crate::ExpenseCategory {
                label: category.label.clone().into(),
                amount_str: format_money_round(category.amount).into(),
                percent,
            }
        })
        .collect()
}

async fn load_drill_rows(
    pool: &PgPool,
    company_id: Uuid,
    months: u32,
    category: Option<&str>,
) -> Vec<crate::DrillRow> {
    let Some(category) = category.filter(|value| !value.is_empty()) else {
        return Vec::new();
    };

    struct Row {
        date: String,
        operation: String,
        counterparty: String,
        doc_id: String,
        amount: Decimal,
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Row {
        fn from_row(r: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            use sqlx::Row as _;
            Ok(Self {
                date: r.try_get("date")?,
                operation: r.try_get("operation")?,
                counterparty: r.try_get("counterparty")?,
                doc_id: r.try_get("doc_id")?,
                amount: r.try_get("amount")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        WITH docs AS (
            SELECT
                TO_CHAR(a.date, 'DD.MM.YYYY') AS date,
                'Акт'::text AS operation,
                c.name AS counterparty,
                a.number AS doc_id,
                a.total_amount AS amount
            FROM acts a
            JOIN counterparties c ON c.id = a.counterparty_id
            JOIN categories cat ON cat.id = a.category_id
            WHERE a.company_id = $1
              AND cat.name = $2
              AND a.date >= date_trunc('month', CURRENT_DATE) - ($3::int - 1) * INTERVAL '1 month'

            UNION ALL

            SELECT
                TO_CHAR(i.date, 'DD.MM.YYYY') AS date,
                'Рахунок'::text AS operation,
                c.name AS counterparty,
                i.number AS doc_id,
                i.total_amount AS amount
            FROM invoices i
            JOIN counterparties c ON c.id = i.counterparty_id
            JOIN categories cat ON cat.id = i.category_id
            WHERE i.company_id = $1
              AND cat.name = $2
              AND i.date >= date_trunc('month', CURRENT_DATE) - ($3::int - 1) * INTERVAL '1 month'
        )
        SELECT date, operation, counterparty, doc_id, amount
        FROM docs
        ORDER BY date DESC, amount DESC
        LIMIT 50
        "#,
    )
    .bind(company_id)
    .bind(category)
    .bind(months as i32)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|row| crate::DrillRow {
            date: row.date.into(),
            operation: row.operation.into(),
            counterparty: row.counterparty.into(),
            doc_id: row.doc_id.into(),
            amount_str: format_money_round(row.amount).into(),
        })
        .collect()
}

pub async fn prepare_reports_data(
    pool: &PgPool,
    company_id: Uuid,
    period: i32,
    drill_category: Option<&str>,
) -> ReportsData {
    let months = period_to_months(period);
    let (revenue_res, expenses_res, categories_res, drill_rows) = tokio::join!(
        db::dashboard::revenue_by_month(pool, company_id, months),
        db::dashboard::expenses_by_month(pool, company_id, months),
        db::dashboard::category_breakdown(pool, company_id, months),
        load_drill_rows(pool, company_id, months, drill_category),
    );

    let revenue = revenue_res.unwrap_or_default();
    let expenses = expenses_res.unwrap_or_default();
    let categories = categories_res.unwrap_or_default();

    let revenue_total: Decimal = revenue.iter().map(|row| row.amount).sum();
    let expense_total: Decimal = expenses.iter().map(|row| row.amount).sum();
    let profit = revenue_total - expense_total;
    let margin = if revenue_total > Decimal::ZERO {
        ((profit / revenue_total) * Decimal::from(100))
            .round_dp(1)
            .to_string()
    } else {
        "0".to_string()
    };

    ReportsData {
        metrics: crate::ReportMetrics {
            revenue: format_money_round(revenue_total).into(),
            expenses: format_money_round(expense_total).into(),
            profit: format_money_round(profit).into(),
            margin: margin.into(),
            delta_revenue: slint::SharedString::default(),
            delta_expenses: slint::SharedString::default(),
            delta_profit: slint::SharedString::default(),
            delta_margin: slint::SharedString::default(),
        },
        chart_bars: build_chart_bars(&revenue, &expenses),
        categories: build_expense_categories(&categories),
        drill_rows,
    }
}

pub fn apply_reports_to_ui(ui: &crate::AppWindow, data: ReportsData) {
    ui.set_rep_metrics(data.metrics);
    ui.set_rep_chart_bars(ModelRc::new(VecModel::from(data.chart_bars)));
    ui.set_rep_categories(ModelRc::new(VecModel::from(data.categories)));
    ui.set_rep_drill_rows(ModelRc::new(VecModel::from(data.drill_rows)));
}

async fn export_reports_csv(
    pool: &PgPool,
    company_id: Uuid,
    period: i32,
    drill_category: Option<&str>,
) -> Result<PathBuf> {
    fs::create_dir_all(reports_dir()).await?;

    let data = prepare_reports_data(pool, company_id, period, drill_category).await;
    let path = reports_dir().join(format!("report-{}.csv", report_stamp()));

    let mut csv = String::from("section,key,value\n");
    csv.push_str(&format!("metrics,period,{}\n", period_label(period)));
    csv.push_str(&format!("metrics,revenue,{}\n", data.metrics.revenue));
    csv.push_str(&format!("metrics,expenses,{}\n", data.metrics.expenses));
    csv.push_str(&format!("metrics,profit,{}\n", data.metrics.profit));
    csv.push_str(&format!("metrics,margin,{}\n", data.metrics.margin));
    csv.push_str("\nsection,label,amount,percent\n");
    for category in &data.categories {
        csv.push_str(&format!(
            "categories,{},{},{}\n",
            category.label, category.amount_str, category.percent
        ));
    }
    csv.push_str("\nsection,date,operation,counterparty,document,amount\n");
    for row in &data.drill_rows {
        csv.push_str(&format!(
            "drill,{},{},{},{},{}\n",
            row.date, row.operation, row.counterparty, row.doc_id, row.amount_str
        ));
    }

    fs::write(&path, csv).await?;
    Ok(path)
}

async fn export_reports_pdf(
    pool: &PgPool,
    company_id: Uuid,
    period: i32,
    drill_category: Option<&str>,
) -> Result<PathBuf> {
    fs::create_dir_all(reports_dir()).await?;

    let data = prepare_reports_data(pool, company_id, period, drill_category).await;
    let stamp = report_stamp();
    let typ_path = reports_dir().join(format!("report-{stamp}.typ"));
    let pdf_path = reports_dir().join(format!("report-{stamp}.pdf"));

    let mut document = String::new();
    document.push_str("#set page(margin: 18mm)\n");
    document.push_str("#set text(size: 11pt)\n");
    document.push_str("= Звіт Acta\n");
    document.push_str(&format!("Період: {}\n\n", period_label(period)));
    document.push_str("== KPI\n");
    document.push_str(&format!("Дохід: {}\n", typst_escape(data.metrics.revenue.as_str())));
    document.push_str(&format!("Витрати: {}\n", typst_escape(data.metrics.expenses.as_str())));
    document.push_str(&format!("Прибуток: {}\n", typst_escape(data.metrics.profit.as_str())));
    document.push_str(&format!("Рентабельність: {}\n\n", typst_escape(data.metrics.margin.as_str())));
    document.push_str("== Структура витрат\n");
    for category in &data.categories {
        document.push_str(&format!(
            "- {} — {} ({}%)\n",
            typst_escape(category.label.as_str()),
            typst_escape(category.amount_str.as_str()),
            category.percent
        ));
    }
    if !data.drill_rows.is_empty() {
        document.push_str("\n== Деталізація\n");
        for row in &data.drill_rows {
            document.push_str(&format!(
                "- {} · {} · {} · {} · {}\n",
                typst_escape(row.date.as_str()),
                typst_escape(row.operation.as_str()),
                typst_escape(row.counterparty.as_str()),
                typst_escape(row.doc_id.as_str()),
                typst_escape(row.amount_str.as_str()),
            ));
        }
    }

    fs::write(&typ_path, document).await?;

    let output = Command::new("typst")
        .arg("compile")
        .arg(&typ_path)
        .arg(&pdf_path)
        .output()
        .await
        .context("Не вдалося запустити typst для PDF-звіту")?;

    if !output.status.success() {
        return Err(anyhow!(
            "typst завершився з помилкою: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(pdf_path)
}

pub fn wire_reports_callbacks(ui: &crate::AppWindow, ctx: &Arc<AppCtx>) {
    let state = Arc::new(Mutex::new(ReportsUiState {
        period: 1,
        drill_category: String::new(),
    }));

    ui.on_rep_period_changed({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        let state = state.clone();
        move |period| {
            {
                let mut current = state.lock().unwrap_or_else(|error| error.into_inner());
                current.period = period;
                current.drill_category.clear();
            }

            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                let data = prepare_reports_data(ctx.pool(), ctx.company_id(), period, None).await;
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    apply_reports_to_ui(&ui, data);
                });
            });
        }
    });

    ui.on_rep_category_drilled({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        let state = state.clone();
        move |category| {
            let (period, category_string) = {
                let mut current = state.lock().unwrap_or_else(|error| error.into_inner());
                current.drill_category = category.to_string();
                (current.period, current.drill_category.clone())
            };

            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                let selected = if category_string.is_empty() {
                    None
                } else {
                    Some(category_string.as_str())
                };
                let data = prepare_reports_data(ctx.pool(), ctx.company_id(), period, selected).await;
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    apply_reports_to_ui(&ui, data);
                });
            });
        }
    });

    ui.on_rep_export_csv({
        let ctx = ctx.clone();
        let state = state.clone();
        move || {
            let ctx = ctx.clone();
            let state = state.clone();
            tokio::spawn(async move {
                let (period, drill) = {
                    let current = state.lock().unwrap_or_else(|error| error.into_inner());
                    (current.period, current.drill_category.clone())
                };
                let drill = if drill.is_empty() { None } else { Some(drill.as_str()) };
                match export_reports_csv(ctx.pool(), ctx.company_id(), period, drill).await {
                    Ok(path) => notify_user("CSV-звіт збережено", &path.display().to_string()),
                    Err(error) => {
                        tracing::error!("reports: csv export failed: {error}");
                        notify_user("Помилка експорту CSV", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_rep_export_pdf({
        let ctx = ctx.clone();
        let state = state.clone();
        move || {
            let ctx = ctx.clone();
            let state = state.clone();
            tokio::spawn(async move {
                let (period, drill) = {
                    let current = state.lock().unwrap_or_else(|error| error.into_inner());
                    (current.period, current.drill_category.clone())
                };
                let drill = if drill.is_empty() { None } else { Some(drill.as_str()) };
                match export_reports_pdf(ctx.pool(), ctx.company_id(), period, drill).await {
                    Ok(path) => notify_user("PDF-звіт збережено", &path.display().to_string()),
                    Err(error) => {
                        tracing::error!("reports: pdf export failed: {error}");
                        notify_user("Помилка експорту PDF", &error.to_string());
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn period_to_months_maps_known_values() {
        assert_eq!(period_to_months(0), 1);
        assert_eq!(period_to_months(1), 3);
        assert_eq!(period_to_months(2), 12);
        assert_eq!(period_to_months(3), 3);
    }

    #[test]
    fn build_expense_categories_calculates_percent() {
        let categories = vec![
            CategoryRevenue {
                label: "Оренда".into(),
                amount: dec!(750),
            },
            CategoryRevenue {
                label: "Зарплата".into(),
                amount: dec!(250),
            },
        ];

        let result = build_expense_categories(&categories);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].percent, 75);
        assert_eq!(result[1].percent, 25);
    }
}
