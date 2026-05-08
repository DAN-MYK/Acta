use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app_ctx::AppCtx;
use crate::db;
use crate::models::counterparty::{is_valid_edrpou, is_valid_iban, is_valid_ipn};
use crate::models::{NewCounterparty, UpdateCounterparty};
use crate::tauri_api::documents::{DocumentItemDto, DocumentKindDto, DocumentStatusDto};
use crate::tauri_api::payments::{MutationResultDto, PaymentItemDto};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartiesScreenDto {
    pub items: Vec<CounterpartyItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyDetailScreenDto {
    pub info: CounterpartyDetailsDto,
    pub documents: Vec<DocumentItemDto>,
    pub payments: Vec<PaymentItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyEditorDto {
    pub form: CounterpartyDraftFormDto,
    pub show_editor: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartiesListRequest {
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartySaveRequest {
    pub form: CounterpartyDraftFormDto,
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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
    format!("{sign}{grouped},{frac}")
}

fn format_date(date: NaiveDate) -> String {
    date.format("%d.%m.%Y").to_string()
}

fn counterparty_item_from_model(
    counterparty: &crate::models::counterparty::Counterparty,
) -> CounterpartyItemDto {
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

fn counterparty_details_from_model(
    counterparty: &crate::models::counterparty::Counterparty,
    doc_count: i32,
) -> CounterpartyDetailsDto {
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
        client_since: format_date(counterparty.created_at.date_naive()),
        balance_str: "0".to_string(),
        balance_is_negative: false,
        doc_count,
        overdue_count: 0,
        overdue_amount_str: "0".to_string(),
        last_contact_days: 0,
        last_contact_date: String::new(),
    }
}

fn counterparty_editor_form(
    counterparty: Option<&crate::models::counterparty::Counterparty>,
) -> CounterpartyDraftFormDto {
    match counterparty {
        Some(counterparty) => CounterpartyDraftFormDto {
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
        },
        None => CounterpartyDraftFormDto {
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
        },
    }
}

fn validate_form(form: &CounterpartyDraftFormDto) -> Result<NewCounterparty> {
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

fn build_update_payload(form: &CounterpartyDraftFormDto) -> Result<UpdateCounterparty> {
    let payload = validate_form(form)?;
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

fn act_to_document_item(row: &crate::models::act::ActListRow) -> DocumentItemDto {
    DocumentItemDto {
        id: format!("act:{}", row.id),
        kind: DocumentKindDto::Act,
        number: row.number.clone(),
        date: format_date(row.date),
        counterparty: row.counterparty_name.clone(),
        amount_str: format_money_ua(row.total_amount),
        status: DocumentStatusDto::from_act_status(&row.status),
        status_label: row.status.label().to_string(),
        linked_id: String::new(),
        direction: row.direction.as_str().to_string(),
    }
}

fn invoice_to_document_item(row: &crate::models::invoice::InvoiceListRow) -> DocumentItemDto {
    DocumentItemDto {
        id: format!("inv:{}", row.id),
        kind: DocumentKindDto::Invoice,
        number: row.number.clone(),
        date: format_date(row.date),
        counterparty: row.counterparty_name.clone(),
        amount_str: format_money_ua(row.total_amount),
        status: DocumentStatusDto::from_invoice_status(&row.status),
        status_label: row.status.label().to_string(),
        linked_id: String::new(),
        direction: row.direction.as_str().to_string(),
    }
}

fn payment_to_item(row: &crate::models::payment::PaymentListRow) -> PaymentItemDto {
    PaymentItemDto {
        id: row.id.to_string(),
        date: row.date.clone(),
        counterparty_id: row
            .counterparty_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        counterparty: row.counterparty_name.clone().unwrap_or_default(),
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
        account: row.bank_name.clone().unwrap_or_default(),
    }
}

async fn load_detail(ctx: &AppCtx, counterparty_id: Uuid) -> Result<CounterpartyDetailScreenDto> {
    let company_id = ctx.company_id();
    let counterparty = db::counterparties::get_by_id(ctx.pool(), company_id, counterparty_id)
        .await?
        .ok_or_else(|| anyhow!("Контрагента не знайдено"))?;

    let today = chrono::Utc::now().date_naive();
    let (acts, invoices, payments) = tokio::join!(
        db::acts::list_filtered(
            ctx.pool(),
            company_id,
            None,
            None,
            None,
            Some(counterparty_id),
            None,
            None,
            None,
            None,
            false,
            today
        ),
        db::invoices::list_filtered(
            ctx.pool(),
            company_id,
            None,
            None,
            None,
            Some(counterparty_id),
            None,
            None,
            None,
            None,
            false,
            today
        ),
        db::payments::list_by_counterparty(ctx.pool(), company_id, counterparty_id)
    );

    let mut documents = Vec::new();
    let mut doc_count = 0;
    for row in acts? {
        documents.push((row.date, act_to_document_item(&row)));
        doc_count += 1;
    }
    for row in invoices? {
        documents.push((row.date, invoice_to_document_item(&row)));
        doc_count += 1;
    }
    documents.sort_by(|left, right| right.0.cmp(&left.0));

    Ok(CounterpartyDetailScreenDto {
        info: counterparty_details_from_model(&counterparty, doc_count),
        documents: documents.into_iter().map(|(_, item)| item).collect(),
        payments: payments?.iter().map(payment_to_item).collect(),
    })
}

pub async fn counterparties_list(
    ctx: &AppCtx,
    request: CounterpartiesListRequest,
) -> Result<CounterpartiesScreenDto> {
    let items = db::counterparties::list_filtered(
        ctx.pool(),
        ctx.company_id(),
        request.query.as_deref(),
        false,
    )
    .await?;

    Ok(CounterpartiesScreenDto {
        items: items.iter().map(counterparty_item_from_model).collect(),
    })
}

pub async fn counterparty_get(
    ctx: &AppCtx,
    counterparty_id: String,
) -> Result<CounterpartyDetailScreenDto> {
    let counterparty_id = Uuid::parse_str(&counterparty_id)
        .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
    load_detail(ctx, counterparty_id).await
}

pub async fn counterparty_open_editor(
    ctx: &AppCtx,
    counterparty_id: Option<String>,
) -> Result<CounterpartyEditorDto> {
    let Some(counterparty_id) = counterparty_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(CounterpartyEditorDto {
            form: counterparty_editor_form(None),
            show_editor: true,
        });
    };

    let counterparty_id = Uuid::parse_str(&counterparty_id)
        .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
    let counterparty = db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id)
        .await?
        .ok_or_else(|| anyhow!("Контрагента не знайдено"))?;

    Ok(CounterpartyEditorDto {
        form: counterparty_editor_form(Some(&counterparty)),
        show_editor: true,
    })
}

pub async fn counterparty_save(
    ctx: &AppCtx,
    request: CounterpartySaveRequest,
) -> Result<CounterpartySaveResultDto> {
    let saved_id = if let Some(counterparty_id) = optional_string(&request.form.id) {
        let counterparty_id = Uuid::parse_str(&counterparty_id)
            .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
        db::counterparties::update(
            ctx.pool(),
            counterparty_id,
            &build_update_payload(&request.form)?,
        )
        .await?
        .ok_or_else(|| anyhow!("Контрагента не знайдено"))?
        .id
    } else {
        db::counterparties::create(ctx.pool(), ctx.company_id(), &validate_form(&request.form)?)
            .await?
            .id
    };

    let updated_list = counterparties_list(ctx, CounterpartiesListRequest::default())
        .await?
        .items;
    let updated_detail = Some(load_detail(ctx, saved_id).await?);

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
) -> Result<MutationResultDto> {
    let counterparty_id = Uuid::parse_str(&counterparty_id)
        .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
    if !db::counterparties::archive(ctx.pool(), counterparty_id).await? {
        return Err(anyhow!("Контрагента не знайдено"));
    }

    Ok(MutationResultDto {
        ok: true,
        message: "Контрагента архівовано".to_string(),
    })
}

pub async fn counterparty_create_document_context(
    ctx: &AppCtx,
    counterparty_id: String,
) -> Result<CreateDocumentContextDto> {
    let counterparty_id = Uuid::parse_str(&counterparty_id)
        .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
    let counterparty = db::counterparties::get_by_id(ctx.pool(), ctx.company_id(), counterparty_id)
        .await?
        .ok_or_else(|| anyhow!("Контрагента не знайдено"))?;

    Ok(CreateDocumentContextDto {
        counterparty_id: counterparty.id.to_string(),
        counterparty_name: counterparty.name,
    })
}
