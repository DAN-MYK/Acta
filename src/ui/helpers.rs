use chrono::NaiveDate;
use rust_decimal::Decimal;
use slint::SharedString;

use acta::models::act::{ActListRow, ActStatus};
use acta::models::invoice::{InvoiceListRow, InvoiceStatus};
use acta::models::payment::{PaymentDirection, PaymentListRow};
use acta::models::task::{Task, TaskPriority, TaskStatus};
use acta::models::waybill::{WaybillListRow, WaybillStatus};

// ---------------------------------------------------------------------------
// Formatter — грошові суми форматуємо ТІЛЬКИ в Rust, в Slint передаємо string
// ---------------------------------------------------------------------------

/// Форматує Decimal у рядок для відображення: "1 234,56".
/// Використовує український формат: пробіл як роздільник тисяч, кома — десяткові.
pub fn format_money(d: Decimal) -> String {
    // Перетворюємо на string з 2 знаками після коми
    let s = d.to_string();

    // Перевіряємо чи є десяткова частина
    let parts: Vec<&str> = s.split('.').collect();
    let int_part = parts[0];
    let dec_part = if parts.len() > 1 { parts[1] } else { "00" };

    // Pad decimal to 2 digits
    let dec_padded: String = match dec_part.len() {
        0 => "00".to_string(),
        1 => {
            let mut p = dec_part.to_string();
            p.push('0');
            p
        }
        n if n > 2 => dec_part[..2].to_string(),
        _ => dec_part.to_string(),
    };

    // Додаємо пробіли як роздільник тисяч
    let with_sep = format_thousands(int_part, true);

    if dec_padded == "00" {
        with_sep
    } else {
        format!("{},{}", with_sep, dec_padded)
    }
}

/// Форматує ціле число з роздільником тисяч.
/// `allow_sign` — чи дозволяти мінус на початку.
fn format_thousands(s: &str, allow_sign: bool) -> String {
    if s.is_empty() {
        return "0".to_string();
    }

    let (sign, digits) = if allow_sign && s.starts_with('-') {
        ("-", &s[1..])
    } else {
        ("", s)
    };

    if digits.is_empty() {
        return format!("{}0", sign);
    }

    let mut result = String::new();
    let len = digits.len();

    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(' ');
        }
        result.push(c);
    }

    format!("{}{}", sign, result)
}

/// Форматує Decimal у рядок для KPI: "50 000" (без копійок).
pub fn format_money_round(d: Decimal) -> String {
    format_thousands(&d.to_string(), true)
}

/// Форматує Decimal із знаком ₴: "50 000 ₴".
pub fn format_money_ua(d: Decimal) -> String {
    format!("{} ₴", format_money(d))
}

pub fn date_to_str(d: NaiveDate) -> SharedString {
    d.format("%d.%m.%Y").to_string().into()
}

pub fn act_status_to_slint(s: &ActStatus) -> crate::DocumentStatus {
    match s {
        ActStatus::Draft => crate::DocumentStatus::Draft,
        ActStatus::Issued => crate::DocumentStatus::Issued,
        ActStatus::Signed => crate::DocumentStatus::Signed,
        ActStatus::Paid => crate::DocumentStatus::Paid,
    }
}

pub fn invoice_status_to_slint(s: &InvoiceStatus) -> crate::DocumentStatus {
    match s {
        InvoiceStatus::Draft => crate::DocumentStatus::Draft,
        InvoiceStatus::Issued => crate::DocumentStatus::Issued,
        InvoiceStatus::Signed => crate::DocumentStatus::Signed,
        InvoiceStatus::Paid => crate::DocumentStatus::Paid,
    }
}

pub fn waybill_status_to_slint(s: &WaybillStatus) -> crate::DocumentStatus {
    match s {
        WaybillStatus::Draft => crate::DocumentStatus::Draft,
        WaybillStatus::Issued => crate::DocumentStatus::Issued,
        WaybillStatus::Signed => crate::DocumentStatus::Signed,
        WaybillStatus::Delivered => crate::DocumentStatus::Paid,
    }
}

pub fn act_row_to_document_item(r: &ActListRow) -> crate::DocumentItem {
    crate::DocumentItem {
        id: format!("act:{}", r.id).into(),
        kind: crate::DocumentKind::Act,
        number: r.number.clone().into(),
        date: date_to_str(r.date),
        counterparty: r.counterparty_name.clone().into(),
        amount_str: format_money_ua(r.total_amount).into(),
        status: act_status_to_slint(&r.status),
        linked_id: SharedString::default(),
    }
}

pub fn invoice_row_to_document_item(r: &InvoiceListRow) -> crate::DocumentItem {
    crate::DocumentItem {
        id: format!("inv:{}", r.id).into(),
        kind: crate::DocumentKind::Invoice,
        number: r.number.clone().into(),
        date: date_to_str(r.date),
        counterparty: r.counterparty_name.clone().into(),
        amount_str: format_money_ua(r.total_amount).into(),
        status: invoice_status_to_slint(&r.status),
        linked_id: SharedString::default(),
    }
}

pub fn waybill_row_to_document_item(r: &WaybillListRow) -> crate::DocumentItem {
    crate::DocumentItem {
        id: format!("wbl:{}", r.id).into(),
        kind: crate::DocumentKind::Waybill,
        number: r.number.clone().into(),
        date: date_to_str(r.date),
        counterparty: r.counterparty_name.clone().into(),
        amount_str: format_money_ua(r.total_amount).into(),
        status: waybill_status_to_slint(&r.status),
        linked_id: SharedString::default(),
    }
}

pub fn counterparty_to_item(c: &acta::models::counterparty::Counterparty) -> crate::CounterpartyItem {
    crate::CounterpartyItem {
        id: c.id.to_string().into(),
        name: c.name.clone().into(),
        edrpou: c.edrpou.clone().unwrap_or_default().into(),
        kind: SharedString::default(),
        balance_str: "0".into(),
        doc_count: 0,
        overdue_count: 0,
    }
}

pub fn counterparty_to_details(c: &acta::models::counterparty::Counterparty) -> crate::CounterpartyDetails {
    crate::CounterpartyDetails {
        id: c.id.to_string().into(),
        name: c.name.clone().into(),
        kind: SharedString::default(),
        edrpou: c.edrpou.clone().unwrap_or_default().into(),
        ipn: c.ipn.clone().unwrap_or_default().into(),
        vat: SharedString::default(),
        iban: c.iban.clone().unwrap_or_default().into(),
        bank: SharedString::default(),
        address: c.address.clone().unwrap_or_default().into(),
        director: SharedString::default(),
        phone: c.phone.clone().unwrap_or_default().into(),
        email: c.email.clone().unwrap_or_default().into(),
        client_since: SharedString::default(),
        balance_str: "0".into(),
        balance_is_negative: false,
        doc_count: 0,
        overdue_count: 0,
        overdue_amount_str: "0".into(),
        last_contact_days: 0,
        last_contact_date: SharedString::default(),
    }
}

pub fn payment_row_to_item(r: &PaymentListRow) -> crate::PaymentItem {
    crate::PaymentItem {
        id: r.id.to_string().into(),
        date: r.date.clone().into(),
        counterparty: r.counterparty_name.clone().unwrap_or_default().into(),
        amount_str: format_money_ua(r.amount).into(),
        direction: match r.direction {
            PaymentDirection::Income => crate::Direction::In,
            PaymentDirection::Expense => crate::Direction::Out,
        },
        matched_doc: if r.is_reconciled {
            "Звірено".into()
        } else {
            SharedString::default()
        },
        account: r.bank_name.clone().unwrap_or_default().into(),
    }
}

pub fn task_to_item(t: &Task) -> crate::TaskItem {
    crate::TaskItem {
        id: t.id.to_string().into(),
        title: t.title.clone().into(),
        due_date: t
            .due_date
            .map(|d| d.with_timezone(&chrono::Local).date_naive().format("%d.%m.%Y").to_string())
            .unwrap_or_default()
            .into(),
        done: t.status == TaskStatus::Done || t.status == TaskStatus::Cancelled,
        priority: match t.priority {
            TaskPriority::High | TaskPriority::Critical => crate::Priority::High,
            TaskPriority::Normal => crate::Priority::Medium,
            TaskPriority::Low => crate::Priority::Low,
        },
        linked_doc: SharedString::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use acta::models::act::{ActListRow, ActStatus};
    use acta::models::invoice::{InvoiceListRow, InvoiceStatus};
    use acta::models::payment::{PaymentListRow, PaymentDirection};
    use uuid::Uuid;

    fn sample_act_row() -> ActListRow {
        ActListRow {
            id: Uuid::nil(),
            number: "АКТ-2026-001".to_string(),
            direction: "out".to_string(),
            date: NaiveDate::from_ymd_opt(2026, 4, 21).unwrap(),
            counterparty_name: "ТОВ Тест".to_string(),
            total_amount: dec!(1234.56),
            status: ActStatus::Issued,
        }
    }

    #[test]
    fn format_money_zero() {
        assert_eq!(format_money(dec!(0)), "0");
    }

    #[test]
    fn format_money_whole() {
        assert_eq!(format_money(dec!(50000)), "50 000");
    }

    #[test]
    fn format_money_with_cents() {
        assert_eq!(format_money(dec!(1234.56)), "1 234,56");
    }

    #[test]
    fn format_money_one_digit_cents() {
        assert_eq!(format_money(dec!(100.5)), "100,50");
    }

    #[test]
    fn format_money_large() {
        assert_eq!(format_money(dec!(1234567.89)), "1 234 567,89");
    }

    #[test]
    fn format_money_round_whole() {
        assert_eq!(format_money_round(dec!(50000)), "50 000");
    }

    #[test]
    fn format_money_ua_has_currency() {
        let s = format_money_ua(dec!(100));
        assert!(s.contains("₴"), "має місти символ ₴: {}", s);
        assert!(s.contains("100"), "має місти число: {}", s);
    }

    #[test]
    fn date_to_str_formats_dd_mm_yyyy() {
        let d = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        assert_eq!(date_to_str(d).as_str(), "21.04.2026");
    }

    #[test]
    fn act_status_to_slint_all_variants() {
        assert_eq!(act_status_to_slint(&ActStatus::Draft), crate::DocumentStatus::Draft);
        assert_eq!(act_status_to_slint(&ActStatus::Issued), crate::DocumentStatus::Issued);
        assert_eq!(act_status_to_slint(&ActStatus::Signed), crate::DocumentStatus::Signed);
        assert_eq!(act_status_to_slint(&ActStatus::Paid), crate::DocumentStatus::Paid);
    }

    #[test]
    fn act_row_converts_to_document_item_with_act_prefix() {
        let row = sample_act_row();
        let item = act_row_to_document_item(&row);
        assert!(item.id.as_str().starts_with("act:"));
        assert_eq!(item.number.as_str(), "АКТ-2026-001");
        assert_eq!(item.kind, crate::DocumentKind::Act);
        assert_eq!(item.amount_str.as_str(), "1 234,56 ₴");
        assert_eq!(item.status, crate::DocumentStatus::Issued);
        assert_eq!(item.date.as_str(), "21.04.2026");
    }

    #[test]
    fn invoice_row_converts_with_inv_prefix() {
        let row = InvoiceListRow {
            id: Uuid::nil(),
            number: "РАХ-001".to_string(),
            direction: "in".to_string(),
            date: NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            counterparty_name: "ФОП Іванов".to_string(),
            total_amount: dec!(500.00),
            status: InvoiceStatus::Draft,
        };
        let item = invoice_row_to_document_item(&row);
        assert!(item.id.as_str().starts_with("inv:"));
        assert_eq!(item.kind, crate::DocumentKind::Invoice);
        assert_eq!(item.amount_str.as_str(), "500 ₴");
    }

    #[test]
    fn payment_row_direction_maps_correctly() {
        let row = PaymentListRow {
            id: Uuid::nil(),
            date: "21.04.2026".to_string(),
            amount: dec!(100.00),
            direction: PaymentDirection::Income,
            counterparty_id: None,
            counterparty_name: None,
            bank_name: Some("Monobank".to_string()),
            description: None,
            is_reconciled: false,
        };
        let item = payment_row_to_item(&row);
        assert_eq!(item.direction, crate::Direction::In);
        assert_eq!(item.amount_str.as_str(), "100 ₴");
    }

    #[test]
    fn waybill_status_delivered_maps_to_paid() {
        use acta::models::waybill::WaybillStatus;
        assert_eq!(waybill_status_to_slint(&WaybillStatus::Draft), crate::DocumentStatus::Draft);
        assert_eq!(waybill_status_to_slint(&WaybillStatus::Delivered), crate::DocumentStatus::Paid);
    }
}
