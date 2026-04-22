use slint::{ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use acta::db;
use acta::db::payments::PaymentKpi;

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
