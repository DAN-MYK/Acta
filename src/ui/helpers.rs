// ui/helpers.rs — допоміжні функції та типи для UI-модулів.
//
// Всі функції тут є синхронними (не async). Вони використовуються як
// у callbacks (main thread), так і в async prepare_*/apply_* функціях.

use crate::MainWindow;
use acta::models;
use anyhow::Result;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use slint::{Model, ModelRc, SharedString, StandardListViewItem, VecModel, Weak};
pub use crate::app_ctx::{ActListState, DocListState, InvoiceListState, PaymentListState, TaskListState};
pub use acta::models::{
    ActStatus as ModelActStatus, Company, CompanySummary, NewActItem, NewInvoiceItem,
    Task, TaskPriority,
};

// ── Slint-generated types ──────────────────────────────────────────────────────
// Ці типи генеруються через slint::include_modules!() у main.rs.
// Отримуємо їх через crate:: шлях (бінарний крейт = main.rs).
use crate::{ActRow, ActStatus, CompanyItem, FormItemRow, TaskRow};

// ═══════════════════════════════════════════════════════════════════════════════
// ── TableData ──────────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// Проміжний формат даних (Send).
///
/// Чому не повертати ModelRc напряму?
/// ModelRc = Rc<dyn Model> — не є Send (не можна передати між потоками).
/// Ці прості Vec є Send і можна безпечно передати в upgrade_in_event_loop.
#[derive(Clone)]
pub struct TableData {
    /// Рядки таблиці: зовнішній Vec = рядки, внутрішній = комірки
    pub rows: Vec<Vec<SharedString>>,
    /// Паралельний масив UUID — rows[i] відповідає ids[i]
    pub ids: Vec<SharedString>,
    /// Паралельний масив архівованості — true якщо контрагент в архіві
    pub archived: Vec<bool>,
}

/// Конвертуємо контрагентів у проміжний формат.
/// Колонки: Назва, ЄДРПОУ, IBAN, Телефон.
pub fn to_table_data(cps: &[models::Counterparty]) -> TableData {
    let rows = cps
        .iter()
        .map(|cp| {
            vec![
                SharedString::from(cp.name.as_str()),
                SharedString::from(cp.edrpou.as_deref().unwrap_or("—")),
                SharedString::from(cp.iban.as_deref().unwrap_or("—")),
                SharedString::from(cp.phone.as_deref().unwrap_or("—")),
            ]
        })
        .collect();

    let ids = cps
        .iter()
        .map(|cp| SharedString::from(cp.id.to_string().as_str()))
        .collect();

    let archived = cps.iter().map(|cp| cp.is_archived).collect();

    TableData { rows, ids, archived }
}

/// Будуємо Slint моделі з TableData.
/// Викликати ТІЛЬКИ з main thread (або з upgrade_in_event_loop).
pub fn build_models(
    data: TableData,
) -> (
    ModelRc<ModelRc<StandardListViewItem>>,
    ModelRc<SharedString>,
    ModelRc<bool>,
) {
    let rows: Vec<ModelRc<StandardListViewItem>> = data
        .rows
        .into_iter()
        .map(|cells| {
            let items: Vec<StandardListViewItem> =
                cells.iter().map(|s| StandardListViewItem::from(s.as_str())).collect();
            ModelRc::new(VecModel::from(items))
        })
        .collect();

    (
        ModelRc::new(VecModel::from(rows)),
        ModelRc::new(VecModel::from(data.ids)),
        ModelRc::new(VecModel::from(data.archived)),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Акти ──────────────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// Перетворити список актів у Vec<ActRow> для Slint.
pub fn to_act_rows(acts: &[models::ActListRow]) -> Vec<ActRow> {
    acts.iter()
        .map(|a| ActRow {
            id: SharedString::from(a.id.to_string().as_str()),
            num: SharedString::from(a.number.as_str()),
            date: SharedString::from(a.date.format("%d.%m.%Y").to_string().as_str()),
            counterparty: SharedString::from(a.counterparty_name.as_str()),
            amount: SharedString::from(format_amount_ua(a.total_amount).as_str()),
            status_label: SharedString::from(a.status.label()),
            status: match a.status {
                ModelActStatus::Draft => ActStatus::Draft,
                ModelActStatus::Issued => ActStatus::Issued,
                ModelActStatus::Signed => ActStatus::Signed,
                ModelActStatus::Paid => ActStatus::Paid,
            },
        })
        .collect()
}

pub fn act_status_from_ui(status: ActStatus) -> ModelActStatus {
    match status {
        ActStatus::Draft => ModelActStatus::Draft,
        ActStatus::Issued => ModelActStatus::Issued,
        ActStatus::Signed => ModelActStatus::Signed,
        ActStatus::Paid => ModelActStatus::Paid,
    }
}

/// Зчитує поточний стан позицій з форми акту.
/// Викликати ТІЛЬКИ з main thread.
pub fn collect_form_items(ui: &MainWindow) -> Vec<NewActItem> {
    let items_model = ui.get_act_form_items();
    (0..items_model.row_count())
        .filter_map(|i| {
            let item = items_model.row_data(i)?;
            let quantity = item.quantity.parse::<Decimal>().ok()?;
            let unit_price = item.price.parse::<Decimal>().ok()?;
            Some(NewActItem {
                description: item.description.to_string(),
                quantity,
                unit: item.unit.to_string(),
                unit_price,
            })
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Накладні ──────────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// Зібрати позиції накладної з UI-форми у Vec<NewInvoiceItem>.
pub fn collect_invoice_items_from_ui(ui_weak: &Weak<MainWindow>) -> Vec<NewInvoiceItem> {
    let Some(ui) = ui_weak.upgrade() else {
        return vec![];
    };
    let model = ui.get_invoice_form_items();
    let count = model.row_count();
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let row = model.row_data(i).unwrap_or_default();
        let quantity = row.quantity.to_string().parse::<Decimal>().unwrap_or(Decimal::ONE);
        let price = row.price.to_string().parse::<Decimal>().unwrap_or(Decimal::ZERO);
        let unit = row.unit.to_string();
        items.push(NewInvoiceItem {
            position: (i + 1) as i16,
            description: row.description.to_string(),
            unit: if unit.is_empty() { None } else { Some(unit) },
            quantity,
            price,
        });
    }
    items
}

/// Перерахувати total-amount у формі накладної на основі позицій.
pub fn recalculate_invoice_total(ui: &MainWindow) {
    let model = ui.get_invoice_form_items();
    let mut items: Vec<FormItemRow> =
        (0..model.row_count()).filter_map(|i| model.row_data(i)).collect();
    let mut total = Decimal::ZERO;
    for item in &mut items {
        let qty = item.quantity.to_string().parse::<Decimal>().unwrap_or(Decimal::ZERO);
        let price = item.price.to_string().parse::<Decimal>().unwrap_or(Decimal::ZERO);
        let amount = (qty * price).round_dp(2);
        item.amount = SharedString::from(format!("{:.2}", amount).as_str());
        total += amount;
    }
    ui.set_invoice_form_items(ModelRc::new(VecModel::from(items)));
    ui.set_invoice_form_total(SharedString::from(format!("{:.2}", total).as_str()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Компанії ──────────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// Перетворити Vec<CompanySummary> у ModelRc і встановити у UI.
pub fn apply_company_rows(
    ui: &MainWindow,
    companies: &[CompanySummary],
    active_company_id: uuid::Uuid,
) {
    let items: Vec<CompanyItem> = companies
        .iter()
        .map(|c| CompanyItem {
            id: SharedString::from(c.id.to_string().as_str()),
            name: SharedString::from(c.name.as_str()),
            short_name: SharedString::from(c.short_name.as_deref().unwrap_or("")),
            edrpou: SharedString::from(c.edrpou.as_deref().unwrap_or("")),
            is_vat: c.is_vat_payer,
            act_count: c.act_count as i32,
            total_amount: SharedString::from(format_company_total(&c.total_amount).as_str()),
            is_current: c.id == active_company_id,
            initials: SharedString::from(company_initials(c).as_str()),
        })
        .collect();
    ui.set_company_rows(ModelRc::new(VecModel::from(items)));
}

pub fn company_display_name(company: &Company) -> String {
    company.short_name.clone().unwrap_or_else(|| company.name.clone())
}

pub fn company_subtitle(company: &Company) -> String {
    company
        .edrpou
        .as_ref()
        .map(|edrpou| format!("ЄДРПОУ: {edrpou}"))
        .unwrap_or_else(|| "ЄДРПОУ не вказано".to_string())
}

pub fn company_initials(company: &CompanySummary) -> String {
    let source = company
        .short_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(company.name.as_str());

    let mut letters = source
        .split(|c: char| c.is_whitespace() || c == '-' || c == '—')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();

    if letters.is_empty() {
        letters = "К".to_string();
    }

    letters
}

pub fn reset_company_form(ui: &MainWindow) {
    ui.set_company_form_is_edit(false);
    ui.set_company_form_edit_id(SharedString::from(""));
    ui.set_company_form_name(SharedString::from(""));
    ui.set_company_form_edrpou(SharedString::from(""));
    ui.set_company_form_iban(SharedString::from(""));
    ui.set_company_form_legal_address(SharedString::from(""));
    ui.set_company_form_director(SharedString::from(""));
    ui.set_company_form_accountant(SharedString::from(""));
    ui.set_company_form_is_vat(false);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Платежі ──────────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn reset_payment_form(ui: &MainWindow) {
    ui.set_payment_form_is_edit(false);
    ui.set_payment_form_edit_id(SharedString::from(""));
    ui.set_payment_form_date(SharedString::from(
        Local::now().format("%d.%m.%Y").to_string(),
    ));
    ui.set_payment_form_amount(SharedString::from(""));
    ui.set_payment_form_direction_index(0);
    ui.set_payment_form_counterparty_index(0);
    ui.set_payment_form_bank_name(SharedString::from(""));
    ui.set_payment_form_bank_ref(SharedString::from(""));
    ui.set_payment_form_description(SharedString::from(""));
}

pub fn populate_payment_form(
    ui: &MainWindow,
    counterparties: &[models::counterparty::Counterparty],
    payment: &models::payment::Payment,
) {
    let mut names = vec![SharedString::from("Без контрагента")];
    let mut ids = vec![SharedString::from("")];
    let mut selected_index = 0_i32;

    for (index, cp) in counterparties.iter().enumerate() {
        names.push(SharedString::from(cp.name.as_str()));
        ids.push(SharedString::from(cp.id.to_string().as_str()));
        if payment.counterparty_id == Some(cp.id) {
            selected_index = index as i32 + 1;
        }
    }

    ui.set_payment_form_counterparty_names(ModelRc::new(VecModel::from(names)));
    ui.set_payment_form_counterparty_ids(ModelRc::new(VecModel::from(ids)));
    ui.set_payment_form_is_edit(true);
    ui.set_payment_form_edit_id(SharedString::from(payment.id.to_string().as_str()));
    ui.set_payment_form_date(SharedString::from(
        payment.date.format("%d.%m.%Y").to_string(),
    ));
    ui.set_payment_form_amount(SharedString::from(format!("{:.2}", payment.amount)));
    ui.set_payment_form_direction_index(match payment.direction {
        models::payment::PaymentDirection::Income => 0,
        models::payment::PaymentDirection::Expense => 1,
    });
    ui.set_payment_form_counterparty_index(selected_index);
    ui.set_payment_form_bank_name(SharedString::from(
        payment.bank_name.as_deref().unwrap_or(""),
    ));
    ui.set_payment_form_bank_ref(SharedString::from(
        payment.bank_ref.as_deref().unwrap_or(""),
    ));
    ui.set_payment_form_description(SharedString::from(
        payment.description.as_deref().unwrap_or(""),
    ));
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Документи ─────────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn doc_direction_from_index(index: i32) -> &'static str {
    if index == 1 { "incoming" } else { "outgoing" }
}

pub fn doc_direction_index(direction: &str) -> i32 {
    if direction == "incoming" { 1 } else { 0 }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Задачі ─────────────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn task_priority_from_index(index: i32) -> TaskPriority {
    match index {
        0 => TaskPriority::Low,
        1 => TaskPriority::Normal,
        2 => TaskPriority::High,
        _ => TaskPriority::Critical,
    }
}

pub fn task_priority_index(priority: &TaskPriority) -> i32 {
    match priority {
        TaskPriority::Low => 0,
        TaskPriority::Normal => 1,
        TaskPriority::High => 2,
        TaskPriority::Critical => 3,
    }
}

pub fn format_task_datetime(value: Option<DateTime<Utc>>) -> SharedString {
    value
        .map(|dt| SharedString::from(dt.format("%d.%m.%Y %H:%M").to_string().as_str()))
        .unwrap_or_else(|| SharedString::from("—"))
}

pub fn parse_task_datetime(input: &str) -> Result<Option<DateTime<Utc>>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let naive = NaiveDateTime::parse_from_str(trimmed, "%d.%m.%Y %H:%M").map_err(|_| {
        anyhow::anyhow!(
            "Невірний формат дати/часу: '{trimmed}'. Очікується ДД.ММ.РРРР ГГ:ХХ"
        )
    })?;

    Ok(Some(Utc.from_utc_datetime(&naive)))
}

pub fn task_matches_query(task: &Task, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };

    let needle = query.to_lowercase();
    task.title.to_lowercase().contains(&needle)
        || task
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(&needle)
}

pub fn to_task_rows(tasks: &[Task]) -> Vec<TaskRow> {
    tasks
        .iter()
        .map(|task| TaskRow {
            id: SharedString::from(task.id.to_string().as_str()),
            title: SharedString::from(task.title.as_str()),
            priority_label: SharedString::from(task.priority.label()),
            due_date: format_task_datetime(task.due_date),
            reminder: format_task_datetime(task.reminder_at),
            status_label: SharedString::from(task.status.label()),
            status: SharedString::from(task.status.as_str()),
            priority: SharedString::from(task.priority.as_str()),
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Форматування ──────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// Форматує суму в українському вигляді: "78\u{00A0}000,00 ₴".
pub fn format_amount_ua(amount: Decimal) -> String {
    let s = format!("{:.2}", amount.abs());
    let (int_part, dec_part) = s.split_once('.').unwrap_or((&s, "00"));
    let len = int_part.len();
    let mut result = String::with_capacity(len + len / 3 + 8);
    for (i, ch) in int_part.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('\u{00A0}');
        }
        result.push(ch);
    }
    format!("{},{} ₴", result, dec_part)
}

/// Форматує суму для KPI-картки: "78\u{00A0}000 ₴".
pub fn format_kpi_amount(amount: Decimal) -> String {
    let s = amount.round().abs().to_string();
    let len = s.len();
    let mut result = String::with_capacity(len + len / 3 + 2);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('\u{00A0}');
        }
        result.push(ch);
    }
    format!("{} ₴", result)
}

pub fn format_company_total(amount: &Decimal) -> String {
    format!("{} грн", amount.round_dp(2))
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Утилітарні функції ─────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

pub fn parse_optional_uuid(value: &str) -> Option<uuid::Uuid> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { uuid::Uuid::parse_str(trimmed).ok() }
}

pub fn normalized_query(query: &str) -> Option<&str> {
    let trimmed = query.trim();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

pub fn total_filtered_pages(total_items: usize) -> usize {
    let pages = total_items.div_ceil(crate::COUNTERPARTY_PAGE_SIZE);
    pages.max(1)
}

/// Показує toast-сповіщення на 3 секунди, потім автоматично прибирає.
pub fn show_toast(ui_weak: Weak<MainWindow>, message: String, is_error: bool) {
    let msg = SharedString::from(message.as_str());
    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_toast_message(msg);
            ui.set_toast_is_error(is_error);
        })
        .ok();

    let clear_handle = ui_weak.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        clear_handle
            .upgrade_in_event_loop(|ui| {
                ui.set_toast_message(SharedString::from(""));
            })
            .ok();
    });
}
