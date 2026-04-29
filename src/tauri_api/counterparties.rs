use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app_ctx::AppCtx;
use crate::db;
use crate::models::counterparty::{is_valid_edrpou, is_valid_iban, is_valid_ipn};
use crate::models::{NewCounterparty, UpdateCounterparty};
use crate::tauri_api::{documents, payments};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyItemDto {
    pub id: String,
    pub name: String,
    pub edrpou: String,
    pub kind: String,
    pub balance_str: String,
    pub doc_count: i32,
    pub overdue_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyDetailsDto {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub edrpou: String,
    pub ipn: String,
    pub vat: String,
    pub iban: String,
    pub bank: String,
    pub address: String,
    pub director: String,
    pub phone: String,
    pub email: String,
    pub client_since: String,
    pub balance_str: String,
    pub balance_is_negative: bool,
    pub doc_count: i32,
    pub overdue_count: i32,
    pub overdue_amount_str: String,
    pub last_contact_days: i32,
    pub last_contact_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyDraftFormDto {
    pub id: String,
    pub title: String,
    pub name: String,
    pub edrpou: String,
    pub ipn: String,
    pub iban: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartiesListRequest {
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartiesScreenDto {
    pub items: Vec<CounterpartyItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyDetailScreenDto {
    pub info: CounterpartyDetailsDto,
    pub documents: Vec<documents::DocumentItemDto>,
    pub payments: Vec<payments::PaymentItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyEditorDto {
    pub form: CounterpartyDraftFormDto,
    pub show_editor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartySaveRequest {
    pub form: CounterpartyDraftFormDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartySaveResultDto {
    pub ok: bool,
    pub saved_id: String,
    pub message: String,
    pub updated_list: Vec<CounterpartyItemDto>,
    pub updated_detail: Option<CounterpartyDetailScreenDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDocumentContextDto {
    pub counterparty_id: String,
    pub counterparty_name: String,
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn date_to_str(date: NaiveDate) -> String {
    date.format("%d.%m.%Y").to_string()
}

fn format_money_ua(value: Decimal) -> String {
    let normalized = format!("{:.2}", value.round_dp(2)).replace('.', ",");
    let (sign, digits) = normalized
        .strip_prefix('-')
        .map_or(("", normalized.as_str()), |rest| ("-", rest));
    let (whole, frac) = digits.split_once(',').unwrap_or((digits, "00"));
    let grouped = whole
        .chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .rev()
        .collect::<String>();

    format!("{sign}{grouped},{frac} ₴")
}

fn empty_counterparty_form() -> CounterpartyDraftFormDto {
    CounterpartyDraftFormDto {
        id: String::new(),
        title: "Новий контрагент".to_string(),
        name: String::new(),
        edrpou: String::new(),
        ipn: String::new(),
        iban: String::new(),
        address: String::new(),
        phone: String::new(),
        email: String::new(),
        notes: String::new(),
    }
}

fn edit_counterparty_form(counterparty: &crate::models::Counterparty) -> CounterpartyDraftFormDto {
    CounterpartyDraftFormDto {
        id: counterparty.id.to_string(),
        title: "Редагування контрагента".to_string(),
        name: counterparty.name.clone(),
        edrpou: counterparty.edrpou.clone().unwrap_or_default(),
        ipn: counterparty.ipn.clone().unwrap_or_default(),
        iban: counterparty.iban.clone().unwrap_or_default(),
        address: counterparty.address.clone().unwrap_or_default(),
        phone: counterparty.phone.clone().unwrap_or_default(),
        email: counterparty.email.clone().unwrap_or_default(),
        notes: counterparty.notes.clone().unwrap_or_default(),
    }
}

fn validate_counterparty_form(form: &CounterpartyDraftFormDto) -> Result<NewCounterparty> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(anyhow!("Назва контрагента є обов'язковою"));
    }

    let edrpou = optional_string(&form.edrpou);
    if let Some(value) = edrpou.as_deref() {
        if !is_valid_edrpou(value) {
            return Err(anyhow!("ЄДРПОУ має містити рівно 8 цифр"));
        }
    }

    let ipn = optional_string(&form.ipn);
    if let Some(value) = ipn.as_deref() {
        if !is_valid_ipn(value) {
            return Err(anyhow!("ІПН має містити рівно 10 цифр"));
        }
    }

    let iban = optional_string(&form.iban);
    if let Some(value) = iban.as_deref() {
        if !is_valid_iban(value) {
            return Err(anyhow!("IBAN має починатися з UA і містити 29 символів"));
        }
    }

    Ok(NewCounterparty {
        name: name.to_string(),
        edrpou,
        ipn,
        iban,
        address: optional_string(&form.address),
        phone: optional_string(&form.phone),
        email: optional_string(&form.email),
        notes: optional_string(&form.notes),
        bas_id: None,
    })
}

fn update_payload_from_form(form: &CounterpartyDraftFormDto) -> Result<UpdateCounterparty> {
    let payload = validate_counterparty_form(form)?;
    Ok(UpdateCounterparty {
        name: payload.name,
        edrpou: payload.edrpou,
        ipn: payload.ipn,
        iban: payload.iban,
        address: payload.address,
        phone: payload.phone,
        email: payload.email,
        notes: payload.notes,
    })
}

fn counterparty_to_item(counterparty: &crate::models::Counterparty) -> CounterpartyItemDto {
    CounterpartyItemDto {
        id: counterparty.id.to_string(),
        name: counterparty.name.clone(),
        edrpou: counterparty.edrpou.clone().unwrap_or_default(),
        kind: String::new(),
        balance_str: "0".to_string(),
        doc_count: 0,
        overdue_count: 0,
    }
}

fn counterparty_to_details(counterparty: &crate::models::Counterparty) -> CounterpartyDetailsDto {
    CounterpartyDetailsDto {
        id: counterparty.id.to_string(),
        name: counterparty.name.clone(),
        kind: String::new(),
        edrpou: counterparty.edrpou.clone().unwrap_or_default(),
        ipn: counterparty.ipn.clone().unwrap_or_default(),
        vat: String::new(),
        iban: counterparty.iban.clone().unwrap_or_default(),
        bank: String::new(),
        address: counterparty.address.clone().unwrap_or_default(),
        director: String::new(),
        phone: counterparty.phone.clone().unwrap_or_default(),
        email: counterparty.email.clone().unwrap_or_default(),
        client_since: String::new(),
        balance_str: "0".to_string(),
        balance_is_negative: false,
        doc_count: 0,
        overdue_count: 0,
        overdue_amount_str: "0".to_string(),
        last_contact_days: 0,
        last_contact_date: String::new(),
    }
}

async fn load_counterparties_items(
    ctx: &AppCtx,
    query: Option<&str>,
) -> Result<Vec<CounterpartyItemDto>> {
    let rows =
        db::counterparties::list_filtered(ctx.pool(), ctx.company_id(), query, false).await?;
    Ok(rows.iter().map(counterparty_to_item).collect())
}

async fn load_counterparty_detail(
    ctx: &AppCtx,
    counterparty_id: Uuid,
) -> Result<CounterpartyDetailScreenDto> {
    let counterparty = db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id)
        .await?
        .ok_or_else(|| anyhow!("Контрагента не знайдено"))?;

    let (acts, invoices, waybills, payment_rows) = tokio::join!(
        db::acts::list_filtered(
            ctx.pool(),
            ctx.company_id(),
            None,
            None,
            None,
            Some(counterparty_id),
            None,
            None,
        ),
        db::invoices::list_filtered(
            ctx.pool(),
            ctx.company_id(),
            None,
            None,
            None,
            Some(counterparty_id),
            None,
            None,
        ),
        db::waybills::list_filtered(
            ctx.pool(),
            ctx.company_id(),
            None,
            None,
            None,
            Some(counterparty_id),
            None,
            None,
        ),
        db::payments::list_by_counterparty(ctx.pool(), ctx.company_id(), counterparty_id),
    );

    let mut documents_rows: Vec<(NaiveDate, documents::DocumentItemDto)> = Vec::new();

    for row in acts? {
        documents_rows.push((
            row.date,
            documents::DocumentItemDto {
                id: format!("act:{}", row.id),
                kind: documents::DocumentKindDto::Act,
                number: row.number,
                date: date_to_str(row.date),
                counterparty: row.counterparty_name,
                amount_str: format_money_ua(row.total_amount),
                status: documents::DocumentStatusDto::from_act_status(&row.status),
                status_label: row.status.label().to_string(),
                linked_id: String::new(),
            },
        ));
    }

    for row in invoices? {
        documents_rows.push((
            row.date,
            documents::DocumentItemDto {
                id: format!("inv:{}", row.id),
                kind: documents::DocumentKindDto::Invoice,
                number: row.number,
                date: date_to_str(row.date),
                counterparty: row.counterparty_name,
                amount_str: format_money_ua(row.total_amount),
                status: documents::DocumentStatusDto::from_invoice_status(&row.status),
                status_label: row.status.label().to_string(),
                linked_id: String::new(),
            },
        ));
    }

    for row in waybills? {
        documents_rows.push((
            row.date,
            documents::DocumentItemDto {
                id: format!("wbl:{}", row.id),
                kind: documents::DocumentKindDto::Waybill,
                number: row.number,
                date: date_to_str(row.date),
                counterparty: row.counterparty_name,
                amount_str: format_money_ua(row.total_amount),
                status: documents::DocumentStatusDto::from_waybill_status(&row.status),
                status_label: row.status.label().to_string(),
                linked_id: String::new(),
            },
        ));
    }

    documents_rows.sort_by(|left, right| right.0.cmp(&left.0));
    let documents = documents_rows
        .into_iter()
        .map(|(_, item)| item)
        .collect::<Vec<_>>();

    let payments = payment_rows?
        .into_iter()
        .map(|row| payments::PaymentItemDto {
            id: row.id.to_string(),
            date: row.date,
            counterparty: row.counterparty_name.unwrap_or_default(),
            amount_str: format_money_ua(row.amount),
            direction: match row.direction {
                crate::models::payment::PaymentDirection::Income => "in".to_string(),
                crate::models::payment::PaymentDirection::Expense => "out".to_string(),
            },
            matched_doc: if row.is_reconciled {
                "Звірено".to_string()
            } else {
                String::new()
            },
            account: row.bank_name.unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    Ok(CounterpartyDetailScreenDto {
        info: counterparty_to_details(&counterparty),
        documents,
        payments,
    })
}

pub async fn counterparties_list(
    ctx: &AppCtx,
    request: CounterpartiesListRequest,
) -> Result<CounterpartiesScreenDto> {
    let items = load_counterparties_items(ctx, request.query.as_deref()).await?;
    Ok(CounterpartiesScreenDto { items })
}

pub async fn counterparty_get(
    ctx: &AppCtx,
    counterparty_id: String,
) -> Result<CounterpartyDetailScreenDto> {
    let parsed = Uuid::parse_str(&counterparty_id)
        .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
    load_counterparty_detail(ctx, parsed).await
}

pub async fn counterparty_open_editor(
    ctx: &AppCtx,
    counterparty_id: Option<String>,
) -> Result<CounterpartyEditorDto> {
    let form = if let Some(counterparty_id) =
        counterparty_id.filter(|value| !value.trim().is_empty())
    {
        let parsed = Uuid::parse_str(&counterparty_id)
            .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
        let counterparty = db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), parsed)
            .await?
            .ok_or_else(|| anyhow!("Контрагента не знайдено"))?;
        edit_counterparty_form(&counterparty)
    } else {
        empty_counterparty_form()
    };

    Ok(CounterpartyEditorDto {
        form,
        show_editor: true,
    })
}

pub async fn counterparty_save(
    ctx: &AppCtx,
    request: CounterpartySaveRequest,
) -> Result<CounterpartySaveResultDto> {
    let maybe_id = optional_string(&request.form.id);
    let saved_id = if let Some(counterparty_id) = maybe_id {
        let parsed = Uuid::parse_str(&counterparty_id)
            .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
        db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), parsed)
            .await?
            .ok_or_else(|| anyhow!("Контрагента не знайдено"))?;
        let payload = update_payload_from_form(&request.form)?;
        db::counterparties::update(ctx.pool(), parsed, &payload)
            .await?
            .ok_or_else(|| anyhow!("Контрагента не знайдено"))?
            .id
    } else {
        let payload = validate_counterparty_form(&request.form)?;
        db::counterparties::create(ctx.pool(), ctx.company_id(), &payload)
            .await?
            .id
    };

    let updated_list = load_counterparties_items(ctx, None).await?;
    let updated_detail = Some(load_counterparty_detail(ctx, saved_id).await?);

    Ok(CounterpartySaveResultDto {
        ok: true,
        saved_id: saved_id.to_string(),
        message: "Контрагента збережено".to_string(),
        updated_list,
        updated_detail,
    })
}

pub async fn counterparty_archive(
    ctx: &AppCtx,
    counterparty_id: String,
) -> Result<payments::MutationResultDto> {
    let parsed = Uuid::parse_str(&counterparty_id)
        .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
    db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), parsed)
        .await?
        .ok_or_else(|| anyhow!("Контрагента не знайдено"))?;
    let archived = db::counterparties::archive(ctx.pool(), parsed).await?;
    if !archived {
        return Err(anyhow!("Контрагента не знайдено"));
    }

    Ok(payments::MutationResultDto {
        ok: true,
        message: "Контрагента архівовано".to_string(),
    })
}

pub async fn counterparty_create_document_context(
    ctx: &AppCtx,
    counterparty_id: String,
) -> Result<CreateDocumentContextDto> {
    let parsed = Uuid::parse_str(&counterparty_id)
        .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
    let counterparty = db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), parsed)
        .await?
        .ok_or_else(|| anyhow!("Контрагента не знайдено"))?;

    Ok(CreateDocumentContextDto {
        counterparty_id,
        counterparty_name: counterparty.name,
    })
}
