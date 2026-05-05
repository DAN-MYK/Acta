use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use rust_decimal::Decimal;
use uuid::Uuid;

use super::{
    format_decimal_ua, MutationResultDto, PaymentCalendarDayDto, PaymentCalendarEventDto,
    PaymentCalendarMonthDto, PaymentCalendarMonthRequest, PaymentScheduleCompleteRequest,
};
use crate::app_ctx::AppCtx;
use crate::db;

pub(super) fn parse_calendar_month(value: &str) -> Result<NaiveDate> {
    let month_value = format!("{}-01", value.trim());
    NaiveDate::parse_from_str(&month_value, "%Y-%m-%d").map_err(|_| {
        anyhow!("Невірний місяць. Використовуйте формат yyyy-mm")
    })
}

pub(super) fn parse_calendar_date(value: &str, field: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| anyhow!("Невірна дата у полі {field}. Використовуйте формат yyyy-mm-dd"))
}

pub(super) fn format_date_iso(value: NaiveDate) -> String {
    value.format("%Y-%m-%d").to_string()
}

fn weekday_short_label(value: Weekday) -> &'static str {
    match value {
        Weekday::Mon => "Пн",
        Weekday::Tue => "Вт",
        Weekday::Wed => "Ср",
        Weekday::Thu => "Чт",
        Weekday::Fri => "Пт",
        Weekday::Sat => "Сб",
        Weekday::Sun => "Нд",
    }
}

fn month_label_uk(value: NaiveDate) -> String {
    let month = match value.month() {
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
        _ => "Місяць",
    };
    format!("{month} {}", value.year())
}

pub(super) fn recurrence_label(value: &crate::models::payment::ScheduleRecurrence) -> &'static str {
    match value {
        crate::models::payment::ScheduleRecurrence::None => "Разово",
        crate::models::payment::ScheduleRecurrence::Weekly => "Щотижня",
        crate::models::payment::ScheduleRecurrence::Monthly => "Щомісяця",
        crate::models::payment::ScheduleRecurrence::Quarterly => "Щокварталу",
        crate::models::payment::ScheduleRecurrence::Yearly => "Щороку",
    }
}

pub(super) fn schedule_status_label(is_completed: bool) -> &'static str {
    if is_completed {
        "Виконано"
    } else {
        "Заплановано"
    }
}

pub(super) fn link_label_if_present(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "Без прив'язки"
    } else {
        trimmed
    }
}

fn calendar_event_sort_key(event: &PaymentCalendarEventDto) -> (u8, String, String) {
    let kind_weight = match event.kind.as_str() {
        "schedule" => 0,
        "task" => 1,
        _ => 9,
    };
    (kind_weight, event.title.clone(), event.id.clone())
}

fn parse_event_amount(value: &str) -> Decimal {
    let normalized = value
        .replace('\u{00a0}', "")
        .replace(' ', "")
        .replace(',', ".");
    normalized.parse::<Decimal>().unwrap_or(Decimal::ZERO)
}

fn calendar_grid_bounds(anchor: NaiveDate) -> Result<(NaiveDate, NaiveDate)> {
    let month_start = anchor.with_day(1).ok_or_else(|| {
        anyhow!("Не вдалося визначити початок місяця")
    })?;
    let next_month = if month_start.month() == 12 {
        NaiveDate::from_ymd_opt(month_start.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(month_start.year(), month_start.month() + 1, 1)
    }
    .ok_or_else(|| {
        anyhow!("Не вдалося визначити наступний місяць")
    })?;
    let month_end = next_month - Duration::days(1);
    let grid_start =
        month_start - Duration::days(month_start.weekday().num_days_from_monday() as i64);
    let grid_end =
        month_end + Duration::days((6 - month_end.weekday().num_days_from_monday()) as i64);
    Ok((grid_start, grid_end))
}

pub(super) fn build_calendar_month(
    anchor: NaiveDate,
    selected: NaiveDate,
    events: Vec<PaymentCalendarEventDto>,
) -> Result<PaymentCalendarMonthDto> {
    let (grid_start, grid_end) = calendar_grid_bounds(anchor)?;
    let today = Local::now().date_naive();
    let month_start = anchor.with_day(1).ok_or_else(|| {
        anyhow!("Не вдалося визначити початок місяця")
    })?;

    let mut events_by_date: BTreeMap<String, Vec<PaymentCalendarEventDto>> = BTreeMap::new();
    for event in events {
        events_by_date
            .entry(event.date.clone())
            .or_default()
            .push(event);
    }

    let mut days = Vec::new();
    let mut cursor = grid_start;
    while cursor <= grid_end {
        let key = format_date_iso(cursor);
        let mut day_events = events_by_date.remove(&key).unwrap_or_default();
        day_events.sort_by_key(calendar_event_sort_key);

        let income_total = day_events
            .iter()
            .filter(|event| event.kind == "schedule" && event.direction == "income")
            .fold(Decimal::ZERO, |sum, event| {
                sum + parse_event_amount(&event.amount_str)
            });
        let expense_total = day_events
            .iter()
            .filter(|event| event.kind == "schedule" && event.direction == "expense")
            .fold(Decimal::ZERO, |sum, event| {
                sum + parse_event_amount(&event.amount_str)
            });

        days.push(PaymentCalendarDayDto {
            date: key,
            day_number: cursor.day(),
            weekday_short: weekday_short_label(cursor.weekday()).to_string(),
            in_current_month: cursor.month() == month_start.month()
                && cursor.year() == month_start.year(),
            today: cursor == today,
            selected: cursor == selected,
            has_overdue: day_events.iter().any(|event| event.overdue),
            income_total_str: if income_total > Decimal::ZERO {
                format_decimal_ua(income_total)
            } else {
                String::new()
            },
            expense_total_str: if expense_total > Decimal::ZERO {
                format_decimal_ua(expense_total)
            } else {
                String::new()
            },
            event_count: day_events.len(),
            events: day_events,
        });

        cursor += Duration::days(1);
    }

    Ok(PaymentCalendarMonthDto {
        month: month_start.format("%Y-%m").to_string(),
        month_label: month_label_uk(month_start),
        selected_date: format_date_iso(selected),
        today: format_date_iso(today),
        days,
    })
}

pub async fn payments_calendar_load(
    ctx: &AppCtx,
    request: PaymentCalendarMonthRequest,
) -> Result<PaymentCalendarMonthDto> {
    let anchor = parse_calendar_month(&request.month)?;
    let month_start = anchor.with_day(1).ok_or_else(|| {
        anyhow!("Не вдалося визначити початок місяця")
    })?;
    let (grid_start, grid_end) = calendar_grid_bounds(anchor)?;
    let today = Local::now().date_naive();
    let selected = match request.selected_date.as_deref() {
        Some(value) => parse_calendar_date(value, "selectedDate")?,
        None if today.month() == month_start.month() && today.year() == month_start.year() => today,
        None => month_start,
    };

    let schedules =
        db::payments::list_schedule_in_range(ctx.pool(), ctx.company_id(), grid_start, grid_end)
            .await?;
    let tasks = db::tasks::list_all(ctx.pool(), ctx.company_id()).await?;

    let mut events = Vec::new();

    for schedule in schedules {
        let (counterparty_id, counterparty_name) = match schedule.counterparty_id {
            Some(counterparty_id) => {
                db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id)
                    .await?
                    .map(|item| (item.id.to_string(), item.name))
                    .unwrap_or_default()
            }
            None => (String::new(), String::new()),
        };

        events.push(PaymentCalendarEventDto {
            id: schedule.id.to_string(),
            kind: "schedule".to_string(),
            title: schedule.title,
            subtitle: if counterparty_name.is_empty() {
                "Р СџР В»Р В°Р Р…Р С•Р Р†Р С‘Р в„– Р С—Р В»Р В°РЎвЂљРЎвЂ“Р В¶".to_string()
            } else {
                counterparty_name.clone()
            },
            date: format_date_iso(schedule.scheduled_date),
            amount_str: schedule.amount.map(format_decimal_ua).unwrap_or_default(),
            direction: schedule.direction.as_str().to_string(),
            status_label: schedule_status_label(schedule.is_completed).to_string(),
            recurrence_label: recurrence_label(&schedule.recurrence).to_string(),
            counterparty_id,
            counterparty_name,
            link_kind: "schedule".to_string(),
            link_id: schedule.id.to_string(),
            note: schedule.notes.unwrap_or_default(),
            actionable: !schedule.is_completed,
            overdue: !schedule.is_completed && schedule.scheduled_date < today,
            done: schedule.is_completed,
        });
    }

    for task in tasks {
        let Some(due_date) = task
            .due_date
            .map(|value| value.with_timezone(&Local).date_naive())
        else {
            continue;
        };
        if due_date < grid_start || due_date > grid_end {
            continue;
        }

        let (link_kind, link_label) =
            crate::tauri_api::tasks::resolve_link_label(ctx, &task).await?;
        let (counterparty_id, counterparty_name) = match task.counterparty_id {
            Some(counterparty_id) => {
                db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id)
                    .await?
                    .map(|item| (item.id.to_string(), item.name))
                    .unwrap_or_default()
            }
            None => (String::new(), String::new()),
        };

        let is_done = matches!(
            task.status,
            crate::models::task::TaskStatus::Done | crate::models::task::TaskStatus::Cancelled
        );

        events.push(PaymentCalendarEventDto {
            id: task.id.to_string(),
            kind: "task".to_string(),
            title: task.title.clone(),
            subtitle: format!(
                "{} Р’В· {}",
                task.priority.label(),
                link_label_if_present(&link_label)
            ),
            date: format_date_iso(due_date),
            amount_str: String::new(),
            direction: String::new(),
            status_label: task.status.label().to_string(),
            recurrence_label: String::new(),
            counterparty_id,
            counterparty_name,
            link_kind: if link_kind.is_empty() {
                "task".to_string()
            } else {
                link_kind
            },
            link_id: task.id.to_string(),
            note: task.description.unwrap_or_default(),
            actionable: !is_done,
            overdue: !is_done && due_date < today,
            done: is_done,
        });
    }

    build_calendar_month(anchor, selected, events)
}

pub async fn payment_schedule_complete(
    ctx: &AppCtx,
    request: PaymentScheduleCompleteRequest,
) -> Result<MutationResultDto> {
    let schedule_id = Uuid::parse_str(request.schedule_id.trim())
        .map_err(|_| anyhow!("Невалідний ідентифікатор запланованого платежу"))?;
    let updated =
        db::payments::complete_schedule_scoped(ctx.pool(), ctx.company_id(), schedule_id).await?;
    anyhow::ensure!(
        updated,
        "Запланований платіж не знайдено в межах активної компанії"
    );

    Ok(MutationResultDto {
        ok: true,
        message: "Запланований платіж позначено як виконаний"
            .to_string(),
    })
}
