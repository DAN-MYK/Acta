use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use notify_rust::{Notification, Timeout};
use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use tokio::fs;
use uuid::Uuid;

use acta::app_ctx::{AppCtx, AppScreen};
use acta::db;
use acta::db::payments::PaymentKpi;
use acta::import::bank_csv::{
    BankStatementParser, OschadbankCsvParser, ParsedBankRow, SenseBankCsvParser,
    UkrgasbankCsvParser,
};
use acta::models::payment::NewPayment;

use crate::ui::helpers::{format_money_round, payment_row_to_item};

pub struct PaymentsData {
    pub items: Vec<crate::PaymentItem>,
    pub kpi: PaymentKpi,
}

pub async fn prepare_payments_data(pool: &PgPool, company_id: Uuid) -> PaymentsData {
    let (rows_res, kpi_res) = tokio::join!(
        db::payments::list(pool, company_id, None),
        db::payments::payment_kpi(pool, company_id),
    );

    let items = rows_res
        .unwrap_or_default()
        .iter()
        .map(payment_row_to_item)
        .collect();

    let kpi = kpi_res.unwrap_or(PaymentKpi {
        incoming_month: rust_decimal::Decimal::ZERO,
        outgoing_month: rust_decimal::Decimal::ZERO,
        unmatched_count: 0,
    });

    PaymentsData { items, kpi }
}

pub fn apply_payments_to_ui(ui: &crate::AppWindow, data: PaymentsData) {
    let net = data.kpi.incoming_month - data.kpi.outgoing_month;

    ui.set_payments(ModelRc::new(VecModel::from(data.items)));
    ui.set_pay_incoming_str(format_money_round(data.kpi.incoming_month).into());
    ui.set_pay_outgoing_str(format_money_round(data.kpi.outgoing_month).into());
    ui.set_pay_net_str(format_money_round(net).into());
    ui.set_pay_unmatched_count(data.kpi.unmatched_count as i32);
    ui.set_pay_unmatched_str(data.kpi.unmatched_count.to_string().into());
    ui.set_pay_incoming_sub("поточний місяць".into());
    ui.set_pay_outgoing_sub("поточний місяць".into());
}

fn notify_user(summary: &str, body: &str) {
    let _ = Notification::new()
        .appname("Acta")
        .summary(summary)
        .body(body)
        .timeout(Timeout::Milliseconds(6_000))
        .show();
}

async fn reload_payments(ui_weak: slint::Weak<crate::AppWindow>, ctx: Arc<AppCtx>) {
    crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Payments).await;
}

fn bank_import_dir() -> PathBuf {
    PathBuf::from("storage/import/bank")
}

async fn newest_csv_path() -> Result<PathBuf> {
    let dir = bank_import_dir();
    fs::create_dir_all(&dir).await?;

    let mut entries = fs::read_dir(&dir).await?;
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !ext.eq_ignore_ascii_case("csv") {
            continue;
        }

        let modified = entry
            .metadata()
            .await
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        match &newest {
            Some((current, _)) if modified <= *current => {}
            _ => newest = Some((modified, path)),
        }
    }

    newest
        .map(|(_, path)| path)
        .ok_or_else(|| anyhow!("У `storage/import/bank/` не знайдено CSV для імпорту"))
}

fn parser_candidates(path: &Path) -> Vec<Box<dyn BankStatementParser>> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase();

    if file_name.contains("oschad") || file_name.contains("ощад") {
        return vec![Box::new(OschadbankCsvParser)];
    }
    if file_name.contains("sense") {
        return vec![Box::new(SenseBankCsvParser)];
    }
    if file_name.contains("ukrgas") || file_name.contains("укргаз") {
        return vec![Box::new(UkrgasbankCsvParser)];
    }

    vec![
        Box::new(UkrgasbankCsvParser),
        Box::new(OschadbankCsvParser),
        Box::new(SenseBankCsvParser),
    ]
}

fn parse_bank_rows(path: &Path, csv_text: &str) -> Result<Vec<ParsedBankRow>> {
    let mut last_error: Option<anyhow::Error> = None;

    for parser in parser_candidates(path) {
        match parser.parse(csv_text) {
            Ok(rows) if !rows.is_empty() => return Ok(rows),
            Ok(_) => last_error = Some(anyhow!("CSV не містить жодного рядка")),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Не вдалося розпізнати формат банківського CSV")))
}

async fn import_bank_rows(pool: &PgPool, company_id: Uuid, rows: Vec<ParsedBankRow>) -> Result<usize> {
    let mut imported = 0usize;

    for row in rows {
        let already_exists = db::payments::exists_imported_row(
            pool,
            company_id,
            row.date,
            row.amount,
            row.direction.clone(),
            row.bank_ref.as_deref(),
            &row.description,
        )
        .await?;

        if already_exists {
            continue;
        }

        db::payments::create(
            pool,
            NewPayment {
                company_id,
                date: row.date,
                amount: row.amount,
                direction: row.direction,
                counterparty_id: None,
                bank_name: Some(row.bank_name),
                bank_ref: row.bank_ref,
                description: Some(row.description),
            },
        )
        .await?;

        imported += 1;
    }

    Ok(imported)
}

async fn import_latest_bank_csv(pool: &PgPool, company_id: Uuid) -> Result<(usize, PathBuf)> {
    let csv_path = newest_csv_path().await?;
    let csv_text = fs::read_to_string(&csv_path)
        .await
        .with_context(|| format!("Не вдалося прочитати {}", csv_path.display()))?;
    let rows = parse_bank_rows(&csv_path, &csv_text)?;
    let imported = import_bank_rows(pool, company_id, rows).await?;
    Ok((imported, csv_path))
}

async fn ensure_manual_import_template() -> Result<PathBuf> {
    let dir = bank_import_dir();
    fs::create_dir_all(&dir).await?;

    let path = dir.join("manual-payment-template.csv");
    if fs::metadata(&path).await.is_err() {
        let template = concat!(
            "date,amount,description,direction,reference\n",
            "2026-04-22,1500.00,Ручне надходження,income,MANUAL-001\n",
        );
        fs::write(&path, template).await?;
    }

    Ok(path)
}

pub fn wire_payment_callbacks(ui: &crate::AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_pay_import_csv({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move || {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                match import_latest_bank_csv(ctx.pool(), ctx.company_id()).await {
                    Ok((imported, path)) => {
                        reload_payments(ui_weak, ctx).await;
                        notify_user(
                            "Імпорт платежів завершено",
                            &format!("Імпортовано {imported} рядків з {}", path.display()),
                        );
                    }
                    Err(error) => {
                        tracing::error!("payments: import failed: {error}");
                        notify_user("Помилка імпорту платежів", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_pay_sync_bank({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move || {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                match import_latest_bank_csv(ctx.pool(), ctx.company_id()).await {
                    Ok((imported, path)) => {
                        reload_payments(ui_weak, ctx).await;
                        notify_user(
                            "Синхронізація банку завершена",
                            &format!("Оброблено файл {}. Нових платежів: {imported}", path.display()),
                        );
                    }
                    Err(error) => {
                        tracing::error!("payments: sync failed: {error}");
                        notify_user("Помилка синхронізації банку", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_pay_new(|| {
        tokio::spawn(async move {
            match ensure_manual_import_template().await {
                Ok(path) => {
                    let open_path = path.clone();
                    let _ = tokio::task::spawn_blocking(move || open::that(open_path)).await;
                    notify_user(
                        "Підготовлено шаблон платежу",
                        &format!("Відкрито шаблон CSV: {}", path.display()),
                    );
                }
                Err(error) => {
                    tracing::error!("payments: new template failed: {error}");
                    notify_user("Помилка підготовки шаблону", &error.to_string());
                }
            }
        });
    });

    ui.on_pay_link({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                let Ok(payment_id) = Uuid::parse_str(&id) else {
                    notify_user("Помилка звірки", "Невалідний ідентифікатор платежу");
                    return;
                };

                match db::payments::mark_reconciled(ctx.pool(), payment_id).await {
                    Ok(()) => {
                        reload_payments(ui_weak, ctx).await;
                        notify_user("Платіж звірено", "Платіж позначено як звірений");
                    }
                    Err(error) => {
                        tracing::error!("payments: reconcile failed: {error}");
                        notify_user("Помилка звірки", &error.to_string());
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payments_data_default_is_empty() {
        let data = PaymentsData {
            items: vec![],
            kpi: PaymentKpi {
                incoming_month: rust_decimal::Decimal::ZERO,
                outgoing_month: rust_decimal::Decimal::ZERO,
                unmatched_count: 0,
            },
        };
        assert!(data.items.is_empty());
    }
}
