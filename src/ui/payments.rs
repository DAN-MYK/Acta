use slint::{ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use acta::db;
use crate::ui::helpers::payment_row_to_item;

pub struct PaymentsData {
    pub items: Vec<crate::PaymentItem>,
}

pub async fn prepare_payments_data(pool: &PgPool, company_id: Uuid) -> PaymentsData {
    let rows = db::payments::list(pool, company_id, None)
        .await
        .unwrap_or_default();
    PaymentsData {
        items: rows.iter().map(payment_row_to_item).collect(),
    }
}

pub fn apply_payments_to_ui(ui: &crate::AppWindow, data: PaymentsData) {
    ui.set_payments(ModelRc::new(VecModel::from(data.items)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payments_data_default_is_empty() {
        let data = PaymentsData { items: vec![] };
        assert!(data.items.is_empty());
    }
}
