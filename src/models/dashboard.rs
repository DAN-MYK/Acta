// Моделі даних для Dashboard сторінки
// KpiSummary, MonthRevenue, StatusSlice, UpcomingPayment, RecentAct

use rust_decimal::Decimal;

/// Агрегований KPI для Dashboard — один SQL запит.
pub struct KpiSummary {
    /// Сума оплачених актів поточного місяця.
    pub revenue_this_month: Decimal,
    /// Загальна сума виставлених і підписаних актів (ще не оплачених).
    pub unpaid_total: Decimal,
    /// Кількість актів створених у поточному місяці.
    pub acts_this_month: i64,
    /// Кількість активних (не архівованих) контрагентів.
    pub active_counterparties: i64,
}

/// Виручка за один місяць для бар-чарту.
pub struct MonthRevenue {
    /// Номер місяця 1–12.
    pub month_num: u32,
    /// Рік.
    pub year: i32,
    /// Сума оплачених актів за місяць.
    pub amount: Decimal,
}

impl MonthRevenue {
    /// Скорочена назва місяця українською (3 літери).
    pub fn month_label(&self) -> &'static str {
        match self.month_num {
            1  => "Січ",
            2  => "Лют",
            3  => "Бер",
            4  => "Кві",
            5  => "Тра",
            6  => "Чер",
            7  => "Лип",
            8  => "Сер",
            9  => "Вер",
            10 => "Жов",
            11 => "Лис",
            12 => "Гру",
            _  => "???",
        }
    }
}

/// Кількість актів за статусом (для donut-chart).
pub struct StatusSlice {
    /// Значення статусу: "draft" | "issued" | "signed" | "paid".
    pub status: String,
    pub count: i64,
}

/// Рядок очікуваного платежу (акт або накладна з expected_payment_date).
pub struct UpcomingPayment {
    /// Форматована дата: "05 Кві".
    pub date_label: String,
    /// Назва контрагента.
    pub contractor: String,
    /// Сума.
    pub amount: Decimal,
    /// true якщо expected_payment_date <= сьогодні.
    pub is_overdue: bool,
}

/// Рядок нещодавнього акту для таблиці на Dashboard.
pub struct RecentAct {
    pub num: String,
    pub contractor: String,
    pub amount: Decimal,
    /// "draft" | "issued" | "signed" | "paid"
    pub status: String,
    /// "ДД.ММ.РРРР"
    pub date: String,
}
