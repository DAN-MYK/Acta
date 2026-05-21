use std::collections::HashSet;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::app_ctx::AppCtx;
use crate::db;
use crate::models::act::{Act, ActItem, ActStatus, NewAct, NewActItem, UpdateAct};
use crate::models::adjustment_act::{NewAdjustmentActItem, UpdateAdjustmentAct};
use crate::models::invoice::{
    Invoice, InvoiceItem, InvoiceStatus, NewInvoice, NewInvoiceItem, UpdateInvoice,
};
use crate::models::waybill::{
    NewWaybill, NewWaybillItem, UpdateWaybill, WaybillItem, WaybillStatus,
};
use crate::models::DocumentDirection;
use crate::pdf::reader::replace_pdf_text_with_report;

const CHAIN_PARENT_PREFIX: &str = "[chain-parent:";
const CHAIN_PARENT_SUFFIX: &str = "]";

use super::dto::*;
use super::pdf::*;

#[derive(Clone, Copy)]
pub(super) enum DocumentRef {
    Act(Uuid),
    Invoice(Uuid),
    Waybill(Uuid),
    AdjustmentAct(Uuid),
}

#[derive(Clone)]
struct DocumentSnapshot {
    ref_id: String,
    kind: String,
    number: String,
    counterparty_id: Uuid,
    counterparty_name: String,
    date: NaiveDate,
    total_amount: Decimal,
    status: String,
    notes: Option<String>,
    items: Vec<DocumentDraftItemDto>,
    direction: DocumentDirection,
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

    format!("{sign}{grouped},{frac}")
}

pub(super) fn parse_document_ref(id: &str) -> Option<DocumentRef> {
    if let Some(uuid) = id
        .strip_prefix("act:")
        .and_then(|value| Uuid::parse_str(value).ok())
    {
        return Some(DocumentRef::Act(uuid));
    }
    if let Some(uuid) = id
        .strip_prefix("inv:")
        .and_then(|value| Uuid::parse_str(value).ok())
    {
        return Some(DocumentRef::Invoice(uuid));
    }

    if let Some(uuid) = id
        .strip_prefix("wbl:")
        .and_then(|value| Uuid::parse_str(value).ok())
    {
        return Some(DocumentRef::Waybill(uuid));
    }

    id.strip_prefix("adj:")
        .and_then(|value| Uuid::parse_str(value).ok())
        .map(DocumentRef::AdjustmentAct)
}

fn document_ref_string(kind: &str, id: Uuid) -> String {
    match kind {
        "act" => format!("act:{id}"),
        "invoice" => format!("inv:{id}"),
        "waybill" => format!("wbl:{id}"),
        "adjustment_act" => format!("adj:{id}"),
        _ => id.to_string(),
    }
}

fn split_visible_notes_and_chain_parent(notes: Option<&str>) -> (String, Option<String>) {
    let mut visible_lines = Vec::new();
    let mut parent_ref = None;

    for line in notes.unwrap_or_default().lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(CHAIN_PARENT_PREFIX) && trimmed.ends_with(CHAIN_PARENT_SUFFIX) {
            let parent = trimmed
                .trim_start_matches(CHAIN_PARENT_PREFIX)
                .trim_end_matches(CHAIN_PARENT_SUFFIX)
                .trim();
            if !parent.is_empty() {
                parent_ref = Some(parent.to_string());
            }
            continue;
        }

        visible_lines.push(line);
    }

    (visible_lines.join("\n").trim().to_string(), parent_ref)
}

fn compose_notes_with_chain_parent(
    visible_notes: &str,
    parent_ref: Option<&str>,
) -> Option<String> {
    let visible = visible_notes.trim();
    let parent_line = parent_ref
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("{CHAIN_PARENT_PREFIX}{value}{CHAIN_PARENT_SUFFIX}"));

    match (visible.is_empty(), parent_line) {
        (true, None) => None,
        (false, None) => Some(visible.to_string()),
        (true, Some(parent)) => Some(parent),
        (false, Some(parent)) => Some(format!("{visible}\n\n{parent}")),
    }
}

fn parse_ui_date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%d.%m.%Y")
        .with_context(|| format!("Некоректна дата: {value}. Очікується формат дд.мм.рррр"))
}

fn parse_decimal_input(value: &str, field: &str) -> Result<Decimal> {
    let normalized = value.trim().replace(' ', "").replace(',', ".");
    Decimal::from_str_exact(&normalized)
        .with_context(|| format!("Некоректне число в полі \"{field}\": {value}"))
}

fn kind_title(kind: &str, is_new: bool) -> &'static str {
    match (kind, is_new) {
        ("act", true) => "Новий акт",
        ("invoice", true) => "Новий рахунок",
        ("waybill", true) => "Нова накладна",
        ("act", false) => "Редагування акта",
        ("invoice", false) => "Редагування рахунку",
        ("waybill", false) => "Редагування накладної",
        _ => "Документ",
    }
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_chain_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "act" => Some("act"),
        "invoice" | "inv" => Some("invoice"),
        "waybill" | "wbl" => Some("waybill"),
        _ => None,
    }
}

fn chain_kind_rank(kind: &str) -> Option<u8> {
    match normalize_chain_kind(kind) {
        Some("invoice") => Some(0),
        Some("act") => Some(1),
        Some("waybill") => Some(2),
        _ => None,
    }
}

fn can_create_chain_target(source_kind: &str, target_kind: &str) -> bool {
    match (chain_kind_rank(source_kind), chain_kind_rank(target_kind)) {
        (Some(source_rank), Some(target_rank)) => target_rank > source_rank,
        _ => false,
    }
}

fn act_items_to_draft(items: Vec<ActItem>) -> Vec<DocumentDraftItemDto> {
    items
        .into_iter()
        .map(|item| DocumentDraftItemDto {
            description: item.description,
            unit: item.unit,
            quantity: item.quantity.to_string(),
            price: item.unit_price.to_string(),
        })
        .collect()
}

fn invoice_items_to_draft(items: Vec<InvoiceItem>) -> Vec<DocumentDraftItemDto> {
    items
        .into_iter()
        .map(|item| DocumentDraftItemDto {
            description: item.description,
            unit: item.unit.unwrap_or_default(),
            quantity: item.quantity.to_string(),
            price: item.price.to_string(),
        })
        .collect()
}

fn waybill_items_to_draft(items: Vec<WaybillItem>) -> Vec<DocumentDraftItemDto> {
    items
        .into_iter()
        .map(|item| DocumentDraftItemDto {
            description: item.description,
            unit: item.unit.unwrap_or_default(),
            quantity: item.quantity.to_string(),
            price: item.price.to_string(),
        })
        .collect()
}

fn draft_items_to_new_act(items: Vec<DocumentDraftItemDto>) -> Result<Vec<NewActItem>> {
    items
        .into_iter()
        .map(|item| {
            Ok(NewActItem {
                description: item.description,
                quantity: parse_decimal_input(&item.quantity, "Кількість")?,
                unit: item.unit,
                unit_price: parse_decimal_input(&item.price, "Ціна")?,
            })
        })
        .collect()
}

fn draft_items_to_new_invoice(items: Vec<DocumentDraftItemDto>) -> Result<Vec<NewInvoiceItem>> {
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            Ok(NewInvoiceItem {
                position: (index + 1) as i16,
                description: item.description,
                unit: optional_string(&item.unit),
                quantity: parse_decimal_input(&item.quantity, "Кількість")?,
                price: parse_decimal_input(&item.price, "Ціна")?,
            })
        })
        .collect()
}

fn draft_items_to_new_waybill(items: Vec<DocumentDraftItemDto>) -> Result<Vec<NewWaybillItem>> {
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            Ok(NewWaybillItem {
                position: (index + 1) as i16,
                description: item.description,
                unit: optional_string(&item.unit),
                quantity: parse_decimal_input(&item.quantity, "Кількість")?,
                price: parse_decimal_input(&item.price, "Ціна")?,
            })
        })
        .collect()
}

fn document_status_from_act(status: &ActStatus) -> DocumentStatusDto {
    DocumentStatusDto::from_act_status(status)
}

fn document_status_from_invoice(status: &InvoiceStatus) -> DocumentStatusDto {
    DocumentStatusDto::from_invoice_status(status)
}

fn document_status_from_waybill(status: &WaybillStatus) -> DocumentStatusDto {
    DocumentStatusDto::from_waybill_status(status)
}

async fn load_counterparty_name(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
) -> Result<String> {
    let counterparty = db::counterparties::get_by_id(pool, company_id, counterparty_id)
        .await?
        .ok_or_else(|| anyhow!("Контрагента не знайдено"))?;
    Ok(counterparty.name)
}

fn parse_required_draft_counterparty_id(counterparty_id: Option<String>) -> Result<Uuid> {
    let counterparty_id = counterparty_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Оберіть контрагента перед створенням документа"))?;

    Uuid::parse_str(counterparty_id)
        .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))
}

async fn resolve_draft_counterparty(
    ctx: &AppCtx,
    counterparty_id: Option<String>,
) -> Result<(Uuid, String)> {
    let counterparty_uuid = parse_required_draft_counterparty_id(counterparty_id)?;
    let counterparty_name =
        load_counterparty_name(ctx.pool(), ctx.company_id(), counterparty_uuid).await?;
    Ok((counterparty_uuid, counterparty_name))
}

async fn load_document_snapshot(
    pool: &PgPool,
    company_id: Uuid,
    doc_ref: DocumentRef,
) -> Result<DocumentSnapshot> {
    match doc_ref {
        DocumentRef::Act(id) => {
            let (act, items) = db::acts::get_by_id_scoped(pool, company_id, id)
                .await?
                .ok_or_else(|| anyhow!("Акт не знайдено"))?;
            Ok(DocumentSnapshot {
                ref_id: document_ref_string("act", id),
                kind: "act".to_string(),
                number: act.number.clone(),
                counterparty_id: act.counterparty_id,
                counterparty_name: load_counterparty_name(pool, company_id, act.counterparty_id)
                    .await?,
                date: act.date,
                total_amount: act.total_amount,
                status: act.status.as_str().to_string(),
                notes: act.notes.clone(),
                items: act_items_to_draft(items),
                direction: act.direction,
            })
        }
        DocumentRef::Invoice(id) => {
            let (invoice, items) = db::invoices::get_by_id_scoped(pool, company_id, id)
                .await?
                .ok_or_else(|| anyhow!("Рахунок не знайдено"))?;
            Ok(DocumentSnapshot {
                ref_id: document_ref_string("invoice", id),
                kind: "invoice".to_string(),
                number: invoice.number.clone(),
                counterparty_id: invoice.counterparty_id,
                counterparty_name: load_counterparty_name(
                    pool,
                    company_id,
                    invoice.counterparty_id,
                )
                .await?,
                date: invoice.date,
                total_amount: invoice.total_amount,
                status: invoice.status.as_str().to_string(),
                notes: invoice.notes.clone(),
                items: invoice_items_to_draft(items),
                direction: invoice.direction,
            })
        }
        DocumentRef::Waybill(id) => {
            let (waybill, items) = db::waybills::get_by_id_scoped(pool, company_id, id)
                .await?
                .ok_or_else(|| anyhow!("Накладну не знайдено"))?;
            Ok(DocumentSnapshot {
                ref_id: document_ref_string("waybill", id),
                kind: "waybill".to_string(),
                number: waybill.number.clone(),
                counterparty_id: waybill.counterparty_id,
                counterparty_name: load_counterparty_name(
                    pool,
                    company_id,
                    waybill.counterparty_id,
                )
                .await?,
                date: waybill.date,
                total_amount: waybill.total_amount,
                status: waybill.status.as_str().to_string(),
                notes: waybill.notes.clone(),
                items: waybill_items_to_draft(items),
                direction: waybill.direction,
            })
        }
        DocumentRef::AdjustmentAct(_) => {
            Err(anyhow!("Акти коригування не беруть участі у ланцюжку документів"))
        }
    }
}

async fn snapshot_from_act(
    pool: &PgPool,
    company_id: Uuid,
    act: Act,
    items: Vec<ActItem>,
) -> Result<DocumentSnapshot> {
    Ok(DocumentSnapshot {
        ref_id: document_ref_string("act", act.id),
        kind: "act".to_string(),
        number: act.number,
        counterparty_id: act.counterparty_id,
        counterparty_name: load_counterparty_name(pool, company_id, act.counterparty_id).await?,
        date: act.date,
        total_amount: act.total_amount,
        status: act.status.as_str().to_string(),
        notes: act.notes,
        items: act_items_to_draft(items),
        direction: act.direction,
    })
}

async fn snapshot_from_invoice(
    pool: &PgPool,
    company_id: Uuid,
    invoice: Invoice,
    items: Vec<InvoiceItem>,
) -> Result<DocumentSnapshot> {
    Ok(DocumentSnapshot {
        ref_id: document_ref_string("invoice", invoice.id),
        kind: "invoice".to_string(),
        number: invoice.number,
        counterparty_id: invoice.counterparty_id,
        counterparty_name: load_counterparty_name(pool, company_id, invoice.counterparty_id)
            .await?,
        date: invoice.date,
        total_amount: invoice.total_amount,
        status: invoice.status.as_str().to_string(),
        notes: invoice.notes,
        items: invoice_items_to_draft(items),
        direction: invoice.direction,
    })
}

async fn snapshot_from_waybill(
    pool: &PgPool,
    company_id: Uuid,
    waybill: crate::models::waybill::Waybill,
    items: Vec<WaybillItem>,
) -> Result<DocumentSnapshot> {
    Ok(DocumentSnapshot {
        ref_id: document_ref_string("waybill", waybill.id),
        kind: "waybill".to_string(),
        number: waybill.number,
        counterparty_id: waybill.counterparty_id,
        counterparty_name: load_counterparty_name(pool, company_id, waybill.counterparty_id)
            .await?,
        date: waybill.date,
        total_amount: waybill.total_amount,
        status: waybill.status.as_str().to_string(),
        notes: waybill.notes,
        items: waybill_items_to_draft(items),
        direction: waybill.direction,
    })
}

async fn find_document_by_parent_ref(
    pool: &PgPool,
    company_id: Uuid,
    parent_ref: &str,
    target_kind: &str,
) -> Result<Option<DocumentSnapshot>> {
    let marker = format!("{CHAIN_PARENT_PREFIX}{parent_ref}{CHAIN_PARENT_SUFFIX}");

    match normalize_chain_kind(target_kind) {
        Some("act") => match db::acts::find_by_notes_marker(pool, company_id, &marker).await? {
            Some((act, items)) => Ok(Some(snapshot_from_act(pool, company_id, act, items).await?)),
            None => Ok(None),
        },
        Some("invoice") => {
            match db::invoices::find_by_notes_marker(pool, company_id, &marker).await? {
                Some((invoice, items)) => Ok(Some(
                    snapshot_from_invoice(pool, company_id, invoice, items).await?,
                )),
                None => Ok(None),
            }
        }
        Some("waybill") => {
            match db::waybills::find_by_notes_marker(pool, company_id, &marker).await? {
                Some((waybill, items)) => Ok(Some(
                    snapshot_from_waybill(pool, company_id, waybill, items).await?,
                )),
                None => Ok(None),
            }
        }
        _ => Err(anyhow!("Невідомий тип документа для пошуку: {target_kind}")),
    }
}

async fn build_existing_document_form(
    storage_dir: &Path,
    pool: &PgPool,
    company_id: Uuid,
    doc_ref: DocumentRef,
) -> Result<DocumentEditorDto> {
    match doc_ref {
        DocumentRef::Act(id) => {
            let (act, items) = db::acts::get_by_id_scoped(pool, company_id, id)
                .await?
                .ok_or_else(|| anyhow!("Акт не знайдено"))?;
            let counterparty_name =
                load_counterparty_name(pool, company_id, act.counterparty_id).await?;
            Ok(DocumentEditorDto {
                form: DocumentDraftFormDto {
                    id: format!("act:{id}"),
                    kind: "act".to_string(),
                    counterparty_id: act.counterparty_id.to_string(),
                    counterparty_name,
                    title: kind_title("act", false).to_string(),
                    number: act.number,
                    date: date_to_str(act.date),
                    notes: split_visible_notes_and_chain_parent(act.notes.as_deref()).0,
                    direction: act.direction.as_str().to_string(),
                    original_act_id: None,
                    original_act_number: None,
                },
                items: act_items_to_draft(items),
                pdf: None,
                show_type_picker: false,
                show_editor: true,
            })
        }
        DocumentRef::Invoice(id) => {
            let (invoice, items) = db::invoices::get_by_id_scoped(pool, company_id, id)
                .await?
                .ok_or_else(|| anyhow!("Рахунок не знайдено"))?;
            let counterparty_name =
                load_counterparty_name(pool, company_id, invoice.counterparty_id).await?;
            let pdf = match load_existing_pdf_path(storage_dir, pool, company_id, doc_ref).await? {
                Some(path) => Some(inspect_document_pdf_state(path).await),
                None => None,
            };
            Ok(DocumentEditorDto {
                form: DocumentDraftFormDto {
                    id: format!("inv:{id}"),
                    kind: "invoice".to_string(),
                    counterparty_id: invoice.counterparty_id.to_string(),
                    counterparty_name,
                    title: kind_title("invoice", false).to_string(),
                    number: invoice.number,
                    date: date_to_str(invoice.date),
                    notes: split_visible_notes_and_chain_parent(invoice.notes.as_deref()).0,
                    direction: invoice.direction.as_str().to_string(),
                    original_act_id: None,
                    original_act_number: None,
                },
                items: invoice_items_to_draft(items),
                pdf,
                show_type_picker: false,
                show_editor: true,
            })
        }
        DocumentRef::Waybill(id) => {
            let (waybill, items) = db::waybills::get_by_id_scoped(pool, company_id, id)
                .await?
                .ok_or_else(|| anyhow!("Накладну не знайдено"))?;
            let counterparty_name =
                load_counterparty_name(pool, company_id, waybill.counterparty_id).await?;
            let pdf = match load_existing_pdf_path(storage_dir, pool, company_id, doc_ref).await? {
                Some(path) => Some(inspect_document_pdf_state(path).await),
                None => None,
            };
            Ok(DocumentEditorDto {
                form: DocumentDraftFormDto {
                    id: format!("wbl:{id}"),
                    kind: "waybill".to_string(),
                    counterparty_id: waybill.counterparty_id.to_string(),
                    counterparty_name,
                    title: kind_title("waybill", false).to_string(),
                    number: waybill.number,
                    date: date_to_str(waybill.date),
                    notes: split_visible_notes_and_chain_parent(waybill.notes.as_deref()).0,
                    direction: waybill.direction.as_str().to_string(),
                    original_act_id: None,
                    original_act_number: None,
                },
                items: waybill_items_to_draft(items),
                pdf,
                show_type_picker: false,
                show_editor: true,
            })
        }
        DocumentRef::AdjustmentAct(id) => {
            let (adj, items) = db::adjustment_acts::get_full(pool, company_id, id)
                .await?
                .ok_or_else(|| anyhow!("Акт коригування не знайдено"))?;
            let counterparty_name =
                load_counterparty_name(pool, company_id, adj.counterparty_id).await?;
            let original_act_number: String = sqlx::query_scalar(
                "SELECT number FROM acts WHERE id = $1"
            )
            .bind(adj.original_act_id)
            .fetch_one(pool)
            .await?;

            Ok(DocumentEditorDto {
                form: DocumentDraftFormDto {
                    id: format!("adj:{id}"),
                    kind: "adjustment_act".to_string(),
                    counterparty_id: adj.counterparty_id.to_string(),
                    counterparty_name,
                    title: "Акт коригування".to_string(),
                    number: adj.number,
                    date: date_to_str(adj.date),
                    notes: adj.notes.unwrap_or_default(),
                    direction: adj.direction.as_str().to_string(),
                    original_act_id: Some(adj.original_act_id.to_string()),
                    original_act_number: Some(original_act_number),
                },
                items: items
                    .into_iter()
                    .map(|item| DocumentDraftItemDto {
                        description: item.description,
                        unit: String::new(),
                        quantity: item.quantity.to_string(),
                        price: item.unit_price.to_string(),
                    })
                    .collect(),
                pdf: None,
                show_type_picker: false,
                show_editor: true,
            })
        }
    }
}

async fn create_draft_form(
    pool: &PgPool,
    company_id: Uuid,
    counterparty_id: Uuid,
    counterparty_name: String,
    kind: &str,
    direction: DocumentDirection,
) -> Result<DocumentDraftFormDto> {
    let today = Utc::now().date_naive();

    match kind {
        "act" => {
            let number = db::acts::generate_next_number(pool, company_id).await?;
            let act = db::acts::create(
                pool,
                company_id,
                &NewAct {
                    number: number.clone(),
                    counterparty_id,
                    contract_id: None,
                    category_id: None,
                    direction,
                    date: today,
                    expected_payment_date: None,
                    status: ActStatus::Draft,
                    notes: None,
                    bas_id: None,
                    items: vec![],
                },
            )
            .await?;

            Ok(DocumentDraftFormDto {
                id: format!("act:{}", act.id),
                kind: "act".to_string(),
                counterparty_id: counterparty_id.to_string(),
                counterparty_name,
                title: kind_title("act", true).to_string(),
                number,
                date: date_to_str(today),
                notes: String::new(),
                direction: direction.as_str().to_string(),
                original_act_id: None,
                original_act_number: None,
            })
        }
        "invoice" => {
            let number = db::invoices::generate_next_number(pool, company_id).await?;
            let invoice = db::invoices::create(
                pool,
                company_id,
                &NewInvoice {
                    number: number.clone(),
                    counterparty_id,
                    contract_id: None,
                    category_id: None,
                    direction,
                    date: today,
                    expected_payment_date: None,
                    notes: None,
                    bas_id: None,
                    items: vec![],
                },
            )
            .await?;

            Ok(DocumentDraftFormDto {
                id: format!("inv:{}", invoice.id),
                kind: "invoice".to_string(),
                counterparty_id: counterparty_id.to_string(),
                counterparty_name,
                title: kind_title("invoice", true).to_string(),
                number,
                date: date_to_str(today),
                notes: String::new(),
                direction: direction.as_str().to_string(),
                original_act_id: None,
                original_act_number: None,
            })
        }
        "waybill" => {
            let number = db::waybills::generate_next_number(pool, company_id).await?;
            let waybill = db::waybills::create(
                pool,
                company_id,
                &NewWaybill {
                    number: number.clone(),
                    counterparty_id,
                    contract_id: None,
                    category_id: None,
                    direction,
                    date: today,
                    notes: None,
                    bas_id: None,
                    items: vec![],
                },
            )
            .await?;

            Ok(DocumentDraftFormDto {
                id: format!("wbl:{}", waybill.id),
                kind: "waybill".to_string(),
                counterparty_id: counterparty_id.to_string(),
                counterparty_name,
                title: kind_title("waybill", true).to_string(),
                number,
                date: date_to_str(today),
                notes: String::new(),
                direction: direction.as_str().to_string(),
                original_act_id: None,
                original_act_number: None,
            })
        }
        _ => Err(anyhow!("Невідомий тип документа: {kind}")),
    }
}

async fn load_document_chain(
    pool: &PgPool,
    company_id: Uuid,
    doc_ref: DocumentRef,
) -> Result<Vec<ChainStepDto>> {
    // For adjustment acts the chain concept doesn't apply — return single-step chain
    if let DocumentRef::AdjustmentAct(id) = doc_ref {
        let adj = db::adjustment_acts::get_full(pool, company_id, id)
            .await?
            .ok_or_else(|| anyhow!("Акт коригування не знайдено"))?
            .0;

        return Ok(vec![ChainStepDto {
            doc_type: "adjustment_act".to_string(),
            doc_number: adj.number,
            amount_str: format_money_ua(adj.total_amount),
            status: adj.status.as_str().to_string(),
            exists: true,
        }]);
    }

    let source = load_document_snapshot(pool, company_id, doc_ref).await?;
    let mut invoice = (source.kind == "invoice").then(|| source.clone());
    let mut act = (source.kind == "act").then(|| source.clone());
    let mut waybill = (source.kind == "waybill").then(|| source.clone());

    let mut current = source.clone();
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(current.ref_id.clone());

    while let Some(parent_ref) = split_visible_notes_and_chain_parent(current.notes.as_deref()).1 {
        let parent_doc_ref = parse_document_ref(&parent_ref)
            .ok_or_else(|| anyhow!("Некоректний parent link у документі {}", current.number))?;
        let parent = load_document_snapshot(pool, company_id, parent_doc_ref).await?;
        if visited.contains(&parent.ref_id) {
            tracing::warn!("chain: виявлено цикл при обробці {}", current.number);
            break;
        }
        visited.insert(parent.ref_id.clone());
        match parent.kind.as_str() {
            "invoice" if invoice.is_none() => invoice = Some(parent.clone()),
            "act" if act.is_none() => act = Some(parent.clone()),
            "waybill" if waybill.is_none() => waybill = Some(parent.clone()),
            _ => {}
        }
        current = parent;
    }

    if act.is_none() {
        if let Some(invoice_doc) = invoice.as_ref() {
            act = find_document_by_parent_ref(pool, company_id, &invoice_doc.ref_id, "act").await?;
        }
    }

    if waybill.is_none() {
        if let Some(act_doc) = act.as_ref() {
            waybill =
                find_document_by_parent_ref(pool, company_id, &act_doc.ref_id, "waybill").await?;
        } else if let Some(invoice_doc) = invoice.as_ref() {
            waybill = find_document_by_parent_ref(pool, company_id, &invoice_doc.ref_id, "waybill")
                .await?;
        }
    }

    Ok([("invoice", invoice), ("act", act), ("waybill", waybill)]
        .into_iter()
        .map(|(kind, document)| ChainStepDto {
            doc_type: kind.to_string(),
            doc_number: document
                .as_ref()
                .map(|item| item.number.clone())
                .unwrap_or_default(),
            amount_str: document
                .as_ref()
                .map(|item| format_money_ua(item.total_amount))
                .unwrap_or_default(),
            status: document
                .as_ref()
                .map(|item| item.status.clone())
                .unwrap_or_default(),
            exists: document.is_some(),
        })
        .collect())
}

pub async fn documents_list(
    ctx: &AppCtx,
    request: DocumentsListRequest,
) -> Result<DocumentsListDto> {
    let company_id = ctx.company_id();
    let search = request.query.as_deref();
    let direction_filter = request.direction;
    let counterparty_filter = request
        .counterparty_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            Uuid::parse_str(value)
                .with_context(|| format!("Некоректний фільтр контрагента: {value}"))
        })
        .transpose()?;

    let statuses_owned: Option<Vec<String>> = request.statuses.filter(|v| !v.is_empty());
    let statuses_slice: Option<&[String]> = statuses_owned.as_deref();

    let amount_min = request.amount_min;
    let amount_max = request.amount_max;
    let overdue_only = request.overdue_only.unwrap_or(false);
    let today = chrono::Utc::now().date_naive();

    let date_from = request.date_from;
    let date_to = request.date_to;

    // None = include all kinds; Some(k) skips the other two DB calls (cheaper than SQL filter)
    let include_acts = request.kind.as_deref().map_or(true, |k| k == "act");
    let include_invoices = request.kind.as_deref().map_or(true, |k| k == "invoice");
    let include_waybills =
        request.kind.as_deref().map_or(true, |k| k == "waybill") && !overdue_only;
    let include_adj_acts =
        request.kind.as_deref().map_or(true, |k| k == "adjustment_act") && !overdue_only;

    let (acts, invoices, waybills, adj_acts) = tokio::join!(
        async {
            if include_acts {
                db::acts::list_filtered(
                    ctx.pool(),
                    company_id,
                    statuses_slice,
                    direction_filter,
                    search,
                    counterparty_filter,
                    date_from,
                    date_to,
                    amount_min,
                    amount_max,
                    overdue_only,
                    today,
                )
                .await
            } else {
                Ok(vec![])
            }
        },
        async {
            if include_invoices {
                db::invoices::list_filtered(
                    ctx.pool(),
                    company_id,
                    statuses_slice,
                    direction_filter,
                    search,
                    counterparty_filter,
                    date_from,
                    date_to,
                    amount_min,
                    amount_max,
                    overdue_only,
                    today,
                )
                .await
            } else {
                Ok(vec![])
            }
        },
        async {
            if include_waybills {
                db::waybills::list_filtered(
                    ctx.pool(),
                    company_id,
                    statuses_slice,
                    direction_filter,
                    search,
                    counterparty_filter,
                    date_from,
                    date_to,
                    amount_min,
                    amount_max,
                )
                .await
            } else {
                Ok(vec![])
            }
        },
        async {
            if include_adj_acts {
                db::adjustment_acts::list_filtered(
                    ctx.pool(),
                    company_id,
                    statuses_slice,
                    direction_filter,
                    search,
                    counterparty_filter,
                    date_from,
                    date_to,
                    amount_min,
                    amount_max,
                )
                .await
            } else {
                Ok(vec![])
            }
        },
    );

    let mut combined: Vec<(NaiveDate, DocumentItemDto)> = Vec::new();

    for row in acts? {
        combined.push((
            row.date,
            DocumentItemDto {
                id: format!("act:{}", row.id),
                kind: DocumentKindDto::Act,
                number: row.number,
                date: date_to_str(row.date),
                counterparty: row.counterparty_name,
                amount_str: format_money_ua(row.total_amount),
                status: document_status_from_act(&row.status),
                status_label: row.status.label().to_string(),
                linked_id: String::new(),
                direction: row.direction.as_str().to_string(),
            },
        ));
    }

    for row in invoices? {
        combined.push((
            row.date,
            DocumentItemDto {
                id: format!("inv:{}", row.id),
                kind: DocumentKindDto::Invoice,
                number: row.number,
                date: date_to_str(row.date),
                counterparty: row.counterparty_name,
                amount_str: format_money_ua(row.total_amount),
                status: document_status_from_invoice(&row.status),
                status_label: row.status.label().to_string(),
                linked_id: String::new(),
                direction: row.direction.as_str().to_string(),
            },
        ));
    }

    for row in waybills? {
        combined.push((
            row.date,
            DocumentItemDto {
                id: format!("wbl:{}", row.id),
                kind: DocumentKindDto::Waybill,
                number: row.number,
                date: date_to_str(row.date),
                counterparty: row.counterparty_name,
                amount_str: format_money_ua(row.total_amount),
                status: document_status_from_waybill(&row.status),
                status_label: row.status.label().to_string(),
                linked_id: String::new(),
                direction: row.direction.as_str().to_string(),
            },
        ));
    }

    for row in adj_acts? {
        combined.push((
            row.date,
            DocumentItemDto {
                id: format!("adj:{}", row.id),
                kind: DocumentKindDto::AdjustmentAct,
                number: row.number,
                date: date_to_str(row.date),
                counterparty: row.counterparty_name,
                amount_str: format_money_ua(row.total_amount),
                status: DocumentStatusDto::from_adjustment_act_status(&row.status),
                status_label: row.status.label().to_string(),
                linked_id: row.original_act_id.to_string(),
                direction: row.direction.as_str().to_string(),
            },
        ));
    }

    combined.sort_by(|left, right| right.0.cmp(&left.0));
    let items = combined
        .into_iter()
        .map(|(_, item)| item)
        .collect::<Vec<_>>();
    let invoice_items = items
        .iter()
        .filter(|item| matches!(item.kind, DocumentKindDto::Invoice))
        .cloned()
        .collect::<Vec<_>>();
    let act_items = items
        .iter()
        .filter(|item| matches!(item.kind, DocumentKindDto::Act))
        .cloned()
        .collect::<Vec<_>>();
    let waybill_items = items
        .iter()
        .filter(|item| matches!(item.kind, DocumentKindDto::Waybill))
        .cloned()
        .collect::<Vec<_>>();
    let adjustment_act_items = items
        .iter()
        .filter(|item| matches!(item.kind, DocumentKindDto::AdjustmentAct))
        .cloned()
        .collect::<Vec<_>>();

    Ok(DocumentsListDto {
        total_count: items.len() as i32,
        page_count: 1,
        items,
        invoice_items,
        act_items,
        waybill_items,
        adjustment_act_items,
    })
}

pub async fn document_open(ctx: &AppCtx, doc_id: String) -> Result<DocumentEditorDto> {
    let doc_ref = parse_document_ref(&doc_id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документа"))?;
    build_existing_document_form(ctx.storage_dir(), ctx.pool(), ctx.company_id(), doc_ref).await
}

pub async fn document_pdf_attach_existing(
    ctx: &AppCtx,
    doc_id: String,
    source_path: String,
) -> Result<DocumentPdfActionResultDto> {
    let doc_ref = parse_document_ref(&doc_id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документа"))?;

    let doc_uuid = document_ref_uuid(doc_ref);
    let (kind, number) =
        load_document_kind_and_number(ctx.pool(), ctx.company_id(), doc_ref).await?;
    if !supports_existing_pdf_flow(&kind) {
        return Err(anyhow!(
            "Для документа типу {kind} прив’язка існуючого PDF поки не підтримується"
        ));
    }

    let safe_source = ensure_attach_source_safe(ctx.storage_dir(), Path::new(&source_path))?;

    let managed_path = attach_existing_pdf_copy(
        ctx.storage_dir().to_path_buf(),
        kind,
        doc_uuid,
        number,
        safe_source.to_string_lossy().into_owned(),
    )
    .await?;
    persist_existing_pdf_path(
        ctx.storage_dir(),
        ctx.pool(),
        ctx.company_id(),
        doc_ref,
        managed_path.clone(),
    )
    .await?;

    Ok(DocumentPdfActionResultDto {
        editor: document_open(ctx, doc_id).await?,
        message: format!("PDF прив’язано до документа: {managed_path}"),
    })
}

pub async fn document_pdf_apply_text_replace(
    ctx: &AppCtx,
    request: ReplaceDocumentPdfTextRequest,
) -> Result<DocumentPdfActionResultDto> {
    let doc_ref = parse_document_ref(&request.doc_id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документа"))?;
    let file_path =
        load_existing_pdf_path(ctx.storage_dir(), ctx.pool(), ctx.company_id(), doc_ref)
            .await?
            .ok_or_else(|| anyhow!("Спочатку прив’яжіть існуючий PDF до документа"))?;

    let doc_uuid = document_ref_uuid(doc_ref);
    let (kind, number) =
        load_document_kind_and_number(ctx.pool(), ctx.company_id(), doc_ref).await?;
    let safe_path = ensure_managed_pdf_path(
        ctx.storage_dir(),
        &kind,
        doc_uuid,
        &number,
        Path::new(&file_path),
    )?;

    let report = tokio::task::spawn_blocking({
        let safe_path = safe_path.clone();
        let find_text = request.find_text.clone();
        let replace_text = request.replace_text.clone();
        move || replace_pdf_text_with_report(&safe_path, &find_text, &replace_text)
    })
    .await
    .context("PDF replace thread error")??;

    let message = if report.changed {
        format!(
            "Текст у PDF оновлено. Знайдено до заміни: {}, залишилось після заміни: {}.",
            report.occurrences_before, report.occurrences_after
        )
    } else {
        report
            .warnings
            .first()
            .cloned()
            .unwrap_or_else(|| "Змін у PDF не внесено".to_string())
    };

    Ok(DocumentPdfActionResultDto {
        editor: document_open(ctx, request.doc_id).await?,
        message,
    })
}

pub async fn document_pdf_open_current(ctx: &AppCtx, doc_id: String) -> Result<MutationResultDto> {
    let doc_ref = parse_document_ref(&doc_id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документа"))?;
    let doc_uuid = document_ref_uuid(doc_ref);
    let (kind, number) =
        load_document_kind_and_number(ctx.pool(), ctx.company_id(), doc_ref).await?;
    let file_path =
        load_existing_pdf_path(ctx.storage_dir(), ctx.pool(), ctx.company_id(), doc_ref)
            .await?
            .ok_or_else(|| anyhow!("Для цього документа ще не прив’язано PDF"))?;

    let safe_path = ensure_managed_pdf_path(
        ctx.storage_dir(),
        &kind,
        doc_uuid,
        &number,
        Path::new(&file_path),
    )?;

    open_pdf_file(safe_path.display().to_string()).await?;

    Ok(MutationResultDto {
        ok: true,
        document_id: doc_id,
        message: format!("PDF відкрито: {file_path}"),
    })
}

pub async fn document_prepare_new(
    ctx: &AppCtx,
    counterparty_id: String,
) -> Result<NewDocumentContextDto> {
    let counterparty_uuid = Uuid::parse_str(&counterparty_id)
        .with_context(|| format!("Некоректний ідентифікатор контрагента: {counterparty_id}"))?;
    let counterparty_name =
        load_counterparty_name(ctx.pool(), ctx.company_id(), counterparty_uuid).await?;
    Ok(NewDocumentContextDto {
        counterparty_id,
        counterparty_name,
    })
}

pub async fn document_create_draft(
    ctx: &AppCtx,
    request: CreateDocumentDraftRequest,
) -> Result<DocumentEditorDto> {
    // Special path for adjustment acts — direction and counterparty come from the original act
    if request.kind == "adjustment_act" {
        let original_act_id_str = request
            .original_act_id
            .as_deref()
            .ok_or_else(|| anyhow!("original_act_id є обов'язковим для adjustment_act"))?;
        let original_act_id = Uuid::parse_str(original_act_id_str)
            .with_context(|| format!("Некоректний original_act_id: {original_act_id_str}"))?;

        let adj = db::adjustment_acts::create(ctx.pool(), ctx.company_id(), original_act_id)
            .await?;

        return build_existing_document_form(
            ctx.storage_dir(),
            ctx.pool(),
            ctx.company_id(),
            DocumentRef::AdjustmentAct(adj.id),
        )
        .await;
    }

    // Existing path for act / invoice / waybill
    let direction_str = request
        .direction
        .as_deref()
        .ok_or_else(|| anyhow!("direction є обов'язковим для документів цього типу"))?;
    let direction = DocumentDirection::try_from(direction_str.to_string())
        .map_err(|_| anyhow!("Невідома направленість документа"))?;
    let (counterparty_id, counterparty_name) =
        resolve_draft_counterparty(ctx, request.counterparty_id).await?;
    let form = create_draft_form(
        ctx.pool(),
        ctx.company_id(),
        counterparty_id,
        counterparty_name,
        &request.kind,
        direction,
    )
    .await?;

    Ok(DocumentEditorDto {
        form,
        items: Vec::new(),
        pdf: None,
        show_type_picker: false,
        show_editor: true,
    })
}

pub async fn document_save(
    ctx: &AppCtx,
    request: SaveDocumentRequest,
) -> Result<SaveDocumentResponse> {
    let doc_ref = parse_document_ref(&request.form.id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документа"))?;
    let date = parse_ui_date(&request.form.date)?;
    if request.items.is_empty() {
        return Err(anyhow!("Документ має містити хоча б одну позицію"));
    }

    match doc_ref {
        DocumentRef::Act(id) => {
            let (act, _) = db::acts::get_by_id_scoped(ctx.pool(), ctx.company_id(), id)
                .await?
                .ok_or_else(|| anyhow!("Акт не знайдено"))?;
            let parent_ref = split_visible_notes_and_chain_parent(act.notes.as_deref()).1;
            let direction = DocumentDirection::try_from(request.form.direction.clone())
                .map_err(|_| anyhow!("Невідома направленість документа"))?;
            db::acts::update_with_items_scoped(
                ctx.pool(),
                ctx.company_id(),
                id,
                UpdateAct {
                    number: request.form.number.clone(),
                    counterparty_id: act.counterparty_id,
                    contract_id: act.contract_id,
                    category_id: act.category_id,
                    direction,
                    date,
                    expected_payment_date: act.expected_payment_date,
                    notes: compose_notes_with_chain_parent(
                        &request.form.notes,
                        parent_ref.as_deref(),
                    ),
                },
                draft_items_to_new_act(request.items)?,
            )
            .await?
            .ok_or_else(|| anyhow!("РђРєС‚ РЅРµ Р·РЅР°Р№РґРµРЅРѕ"))?;

            Ok(SaveDocumentResponse {
                document_id: request.form.id,
                kind: "act".to_string(),
                message: "Акт збережено".to_string(),
            })
        }
        DocumentRef::Invoice(id) => {
            let (invoice, _) = db::invoices::get_by_id_scoped(ctx.pool(), ctx.company_id(), id)
                .await?
                .ok_or_else(|| anyhow!("Рахунок не знайдено"))?;
            let parent_ref = split_visible_notes_and_chain_parent(invoice.notes.as_deref()).1;
            let direction = DocumentDirection::try_from(request.form.direction.clone())
                .map_err(|_| anyhow!("Невідома направленість документа"))?;
            db::invoices::update_with_items_scoped(
                ctx.pool(),
                ctx.company_id(),
                id,
                UpdateInvoice {
                    number: request.form.number.clone(),
                    counterparty_id: invoice.counterparty_id,
                    contract_id: invoice.contract_id,
                    category_id: invoice.category_id,
                    direction,
                    date,
                    expected_payment_date: invoice.expected_payment_date,
                    notes: compose_notes_with_chain_parent(
                        &request.form.notes,
                        parent_ref.as_deref(),
                    ),
                },
                draft_items_to_new_invoice(request.items)?,
            )
            .await?
            .ok_or_else(|| anyhow!("Р Р°С…СѓРЅРѕРє РЅРµ Р·РЅР°Р№РґРµРЅРѕ"))?;

            Ok(SaveDocumentResponse {
                document_id: request.form.id,
                kind: "invoice".to_string(),
                message: "Рахунок збережено".to_string(),
            })
        }
        DocumentRef::Waybill(id) => {
            let (waybill, _) = db::waybills::get_by_id_scoped(ctx.pool(), ctx.company_id(), id)
                .await?
                .ok_or_else(|| anyhow!("Накладну не знайдено"))?;
            let parent_ref = split_visible_notes_and_chain_parent(waybill.notes.as_deref()).1;
            let direction = DocumentDirection::try_from(request.form.direction.clone())
                .map_err(|_| anyhow!("Невідома направленість документа"))?;
            db::waybills::update_with_items_scoped(
                ctx.pool(),
                ctx.company_id(),
                id,
                UpdateWaybill {
                    number: request.form.number.clone(),
                    counterparty_id: waybill.counterparty_id,
                    contract_id: waybill.contract_id,
                    category_id: waybill.category_id,
                    direction,
                    date,
                    notes: compose_notes_with_chain_parent(
                        &request.form.notes,
                        parent_ref.as_deref(),
                    ),
                },
                draft_items_to_new_waybill(request.items)?,
            )
            .await?
            .ok_or_else(|| anyhow!("Накладну не знайдено"))?;

            Ok(SaveDocumentResponse {
                document_id: request.form.id,
                kind: "waybill".to_string(),
                message: "Накладну збережено".to_string(),
            })
        }
        DocumentRef::AdjustmentAct(id) => {
            let update = UpdateAdjustmentAct {
                number: request.form.number.clone(),
                date,
                notes: optional_string(&request.form.notes),
            };
            let items: Vec<NewAdjustmentActItem> = request
                .items
                .into_iter()
                .map(|item| {
                    Ok(NewAdjustmentActItem {
                        description: item.description,
                        quantity: parse_decimal_input(&item.quantity, "Кількість")?,
                        unit_price: parse_decimal_input(&item.price, "Ціна")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            db::adjustment_acts::update_with_items_scoped(
                ctx.pool(),
                ctx.company_id(),
                id,
                update,
                items,
            )
            .await?
            .ok_or_else(|| anyhow!("Акт коригування не знайдено"))?;

            Ok(SaveDocumentResponse {
                document_id: request.form.id,
                kind: "adjustment_act".to_string(),
                message: "Акт коригування збережено".to_string(),
            })
        }
    }
}

pub async fn document_advance_status(ctx: &AppCtx, doc_id: String) -> Result<MutationResultDto> {
    let doc_ref = parse_document_ref(&doc_id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документа"))?;

    let message = match doc_ref {
        DocumentRef::Act(id) => {
            db::acts::advance_status_scoped(ctx.pool(), ctx.company_id(), id)
                .await?
                .ok_or_else(|| anyhow!("Акт не знайдено"))?;
            "Статус акта оновлено"
        }
        DocumentRef::Invoice(id) => {
            db::invoices::advance_status_scoped(ctx.pool(), ctx.company_id(), id)
                .await?
                .ok_or_else(|| anyhow!("Рахунок не знайдено"))?;
            "Статус рахунку оновлено"
        }
        DocumentRef::Waybill(id) => {
            db::waybills::advance_status_scoped(ctx.pool(), ctx.company_id(), id)
                .await?
                .ok_or_else(|| anyhow!("Накладну не знайдено"))?;
            "Статус накладної оновлено"
        }
        DocumentRef::AdjustmentAct(id) => {
            db::adjustment_acts::change_status_scoped(ctx.pool(), ctx.company_id(), id)
                .await?
                .ok_or_else(|| anyhow!("Акт коригування не знайдено"))?;
            "Статус акту коригування оновлено"
        }
    };

    Ok(MutationResultDto {
        ok: true,
        document_id: doc_id,
        message: message.to_string(),
    })
}

pub async fn document_delete(ctx: &AppCtx, doc_id: String) -> Result<MutationResultDto> {
    let doc_ref = parse_document_ref(&doc_id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документа"))?;

    match doc_ref {
        DocumentRef::Act(id) => {
            if !db::acts::delete_scoped(ctx.pool(), ctx.company_id(), id).await? {
                return Err(anyhow!("РђРєС‚ РЅРµ Р·РЅР°Р№РґРµРЅРѕ"));
            }
        }
        DocumentRef::Invoice(id) => {
            if !db::invoices::delete_scoped(ctx.pool(), ctx.company_id(), id).await? {
                return Err(anyhow!("Р Р°С…СѓРЅРѕРє РЅРµ Р·РЅР°Р№РґРµРЅРѕ"));
            }
        }
        DocumentRef::Waybill(id) => {
            if !db::waybills::delete_scoped(ctx.pool(), ctx.company_id(), id).await? {
                return Err(anyhow!("Накладну не знайдено"));
            }
        }
        DocumentRef::AdjustmentAct(id) => {
            if !db::adjustment_acts::delete_scoped(ctx.pool(), ctx.company_id(), id).await? {
                return Err(anyhow!("Акт коригування не знайдено"));
            }
        }
    }

    Ok(MutationResultDto {
        ok: true,
        document_id: doc_id,
        message: "Документ видалено".to_string(),
    })
}

pub async fn documents_bulk_advance_status(
    _ctx: &AppCtx,
    request: BulkDocumentRequest,
) -> Result<BulkMutationResultDto> {
    Err(anyhow!(
        "Bulk-операції для documents ще не перенесені у Tauri runtime. IDs: {}",
        request.doc_ids.join(", ")
    ))
}

pub async fn documents_bulk_delete(
    _ctx: &AppCtx,
    request: BulkDocumentRequest,
) -> Result<BulkMutationResultDto> {
    Err(anyhow!(
        "Bulk-видалення для documents ще не перенесене у Tauri runtime. IDs: {}",
        request.doc_ids.join(", ")
    ))
}

pub async fn document_chain_get(ctx: &AppCtx, doc_id: String) -> Result<DocumentChainDto> {
    let doc_ref = parse_document_ref(&doc_id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документа для ланцюжка"))?;
    let steps = load_document_chain(ctx.pool(), ctx.company_id(), doc_ref).await?;
    Ok(DocumentChainDto {
        source_id: doc_id,
        steps,
    })
}

pub async fn document_chain_create_draft(
    ctx: &AppCtx,
    request: CreateChainDraftRequest,
) -> Result<DocumentEditorDto> {
    let source_ref = parse_document_ref(&request.source_id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор джерельного документа"))?;
    let source = load_document_snapshot(ctx.pool(), ctx.company_id(), source_ref).await?;
    let target_kind = normalize_chain_kind(&request.target_kind).ok_or_else(|| {
        anyhow!(
            "Невідомий тип документа для ланцюжка: {}",
            request.target_kind
        )
    })?;

    if !can_create_chain_target(source.kind.as_str(), target_kind) {
        return Err(anyhow!(
            "Для документа типу {} не можна створити похідний документ типу {}",
            source.kind,
            target_kind
        ));
    }

    let chain_steps = load_document_chain(ctx.pool(), ctx.company_id(), source_ref).await?;
    if chain_steps
        .iter()
        .any(|step| step.doc_type == target_kind && step.exists)
    {
        return Err(anyhow!("Документ цього типу в ланцюжку вже існує"));
    }

    let draft_items = source.items.clone();
    let visible_notes = format!("Створено з {}", source.number);
    let stored_notes = compose_notes_with_chain_parent(&visible_notes, Some(&source.ref_id));

    let form = match target_kind {
        "act" => {
            let number = db::acts::generate_next_number(ctx.pool(), ctx.company_id()).await?;
            let act = db::acts::create(
                ctx.pool(),
                ctx.company_id(),
                &NewAct {
                    number: number.clone(),
                    counterparty_id: source.counterparty_id,
                    contract_id: None,
                    category_id: None,
                    direction: source.direction,
                    date: source.date,
                    expected_payment_date: None,
                    status: ActStatus::Draft,
                    notes: stored_notes,
                    bas_id: None,
                    items: draft_items_to_new_act(draft_items.clone())?,
                },
            )
            .await?;

            DocumentDraftFormDto {
                id: format!("act:{}", act.id),
                kind: "act".to_string(),
                counterparty_id: source.counterparty_id.to_string(),
                counterparty_name: source.counterparty_name.clone(),
                title: kind_title("act", true).to_string(),
                number,
                date: date_to_str(source.date),
                notes: visible_notes.clone(),
                direction: source.direction.as_str().to_string(),
                original_act_id: None,
                original_act_number: None,
            }
        }
        "invoice" => {
            let number = db::invoices::generate_next_number(ctx.pool(), ctx.company_id()).await?;
            let invoice = db::invoices::create(
                ctx.pool(),
                ctx.company_id(),
                &NewInvoice {
                    number: number.clone(),
                    counterparty_id: source.counterparty_id,
                    contract_id: None,
                    category_id: None,
                    direction: source.direction,
                    date: source.date,
                    expected_payment_date: None,
                    notes: stored_notes,
                    bas_id: None,
                    items: draft_items_to_new_invoice(draft_items.clone())?,
                },
            )
            .await?;

            DocumentDraftFormDto {
                id: format!("inv:{}", invoice.id),
                kind: "invoice".to_string(),
                counterparty_id: source.counterparty_id.to_string(),
                counterparty_name: source.counterparty_name.clone(),
                title: kind_title("invoice", true).to_string(),
                number,
                date: date_to_str(source.date),
                notes: visible_notes.clone(),
                direction: source.direction.as_str().to_string(),
                original_act_id: None,
                original_act_number: None,
            }
        }
        "waybill" => {
            let number = db::waybills::generate_next_number(ctx.pool(), ctx.company_id()).await?;
            let waybill = db::waybills::create(
                ctx.pool(),
                ctx.company_id(),
                &NewWaybill {
                    number: number.clone(),
                    counterparty_id: source.counterparty_id,
                    contract_id: None,
                    category_id: None,
                    direction: source.direction,
                    date: source.date,
                    notes: stored_notes,
                    bas_id: None,
                    items: draft_items_to_new_waybill(draft_items.clone())?,
                },
            )
            .await?;

            DocumentDraftFormDto {
                id: format!("wbl:{}", waybill.id),
                kind: "waybill".to_string(),
                counterparty_id: source.counterparty_id.to_string(),
                counterparty_name: source.counterparty_name.clone(),
                title: kind_title("waybill", true).to_string(),
                number,
                date: date_to_str(source.date),
                notes: visible_notes.clone(),
                direction: source.direction.as_str().to_string(),
                original_act_id: None,
                original_act_number: None,
            }
        }
        _ => return Err(anyhow!("Невідомий тип документа для ланцюжка")),
    };

    Ok(DocumentEditorDto {
        form,
        items: draft_items,
        pdf: None,
        show_type_picker: false,
        show_editor: true,
    })
}

fn document_word_form(count: usize) -> &'static str {
    let remainder_100 = count % 100;
    if (11..=14).contains(&remainder_100) {
        return "документів";
    }

    match count % 10 {
        1 => "документ",
        2..=4 => "документи",
        _ => "документів",
    }
}

pub async fn documents_bulk_delete_live(
    ctx: &AppCtx,
    request: BulkDocumentRequest,
) -> Result<BulkMutationResultDto> {
    let mut result = BulkMutationResultDto {
        total: request.doc_ids.len(),
        ..BulkMutationResultDto::default()
    };

    for doc_id in request.doc_ids {
        let delete_result = match parse_document_ref(&doc_id) {
            Some(DocumentRef::Act(id)) => db::acts::delete_scoped(ctx.pool(), ctx.company_id(), id)
                .await
                .and_then(|deleted| {
                    if deleted {
                        Ok(())
                    } else {
                        Err(anyhow!("РђРєС‚ РЅРµ Р·РЅР°Р№РґРµРЅРѕ"))
                    }
                }),
            Some(DocumentRef::Invoice(id)) => {
                db::invoices::delete_scoped(ctx.pool(), ctx.company_id(), id)
                    .await
                    .and_then(|deleted| {
                        if deleted {
                            Ok(())
                        } else {
                            Err(anyhow!("Р Р°С…СѓРЅРѕРє РЅРµ Р·РЅР°Р№РґРµРЅРѕ"))
                        }
                    })
            }
            Some(DocumentRef::Waybill(id)) => {
                db::waybills::delete_scoped(ctx.pool(), ctx.company_id(), id)
                    .await
                    .and_then(|deleted| {
                        if deleted {
                            Ok(())
                        } else {
                            Err(anyhow!("РќР°РєР»Р°РґРЅСѓ РЅРµ Р·РЅР°Р№РґРµРЅРѕ"))
                        }
                    })
            }
            Some(DocumentRef::AdjustmentAct(id)) => {
                db::adjustment_acts::delete_scoped(ctx.pool(), ctx.company_id(), id)
                    .await
                    .and_then(|deleted| {
                        if deleted {
                            Ok(())
                        } else {
                            Err(anyhow!("Акт коригування не знайдено"))
                        }
                    })
            }
            None => Err(anyhow!("Некоректний ідентифікатор документа: {doc_id}")),
        };

        match delete_result {
            Ok(_) => result.succeeded += 1,
            Err(error) => {
                result.failed += 1;
                result.errors.push(format!("{doc_id}: {error}"));
            }
        }
    }

    result.message = match (result.succeeded, result.failed) {
        (0, failed) if failed > 0 => {
            format!("Не вдалося видалити жодного документа ({failed} помилок)")
        }
        (succeeded, 0) => format!("Видалено {succeeded} {}", document_word_form(succeeded)),
        (succeeded, failed) => format!(
            "Видалено {succeeded} {}, {failed} помилок",
            document_word_form(succeeded)
        ),
    };

    Ok(result)
}

pub async fn document_change_counterparty(
    ctx: &AppCtx,
    doc_id: String,
    counterparty_id: String,
) -> Result<ChangeCounterpartyResultDto> {
    let doc_ref = parse_document_ref(&doc_id)
        .ok_or_else(|| anyhow!("Некоректний ідентифікатор документа: {}", doc_id))?;
    let cp_uuid = Uuid::parse_str(&counterparty_id)
        .map_err(|_| anyhow!("Некоректний UUID контрагента: {}", counterparty_id))?;
    let company_id = ctx.company_id();
    let pool = ctx.pool();

    match doc_ref {
        DocumentRef::Act(id) => {
            sqlx::query(
                "UPDATE acts SET counterparty_id = $1, updated_at = now() \
                 WHERE id = $2 AND company_id = $3",
            )
            .bind(cp_uuid)
            .bind(id)
            .bind(company_id)
            .execute(pool)
            .await?;
        }
        DocumentRef::Invoice(id) => {
            sqlx::query(
                "UPDATE invoices SET counterparty_id = $1, updated_at = now() \
                 WHERE id = $2 AND company_id = $3",
            )
            .bind(cp_uuid)
            .bind(id)
            .bind(company_id)
            .execute(pool)
            .await?;
        }
        DocumentRef::Waybill(id) => {
            sqlx::query(
                "UPDATE waybills SET counterparty_id = $1, updated_at = now() \
                 WHERE id = $2 AND company_id = $3",
            )
            .bind(cp_uuid)
            .bind(id)
            .bind(company_id)
            .execute(pool)
            .await?;
        }
        DocumentRef::AdjustmentAct(_) => {
            return Err(anyhow!(
                "Для актів коригування зміна контрагента не підтримується"
            ));
        }
    }

    let cp_name: String = sqlx::query_scalar(
        "SELECT name FROM counterparties WHERE id = $1 AND company_id = $2",
    )
    .bind(cp_uuid)
    .bind(company_id)
    .fetch_one(pool)
    .await?;

    Ok(ChangeCounterpartyResultDto {
        ok: true,
        counterparty_id,
        counterparty_name: cp_name,
    })
}

pub async fn documents_bulk_advance_status_live(
    ctx: &AppCtx,
    request: BulkDocumentRequest,
) -> Result<BulkMutationResultDto> {
    let mut result = BulkMutationResultDto {
        total: request.doc_ids.len(),
        ..BulkMutationResultDto::default()
    };

    for doc_id in request.doc_ids {
        let advance_result = match parse_document_ref(&doc_id) {
            Some(DocumentRef::Act(id)) => {
                db::acts::advance_status_scoped(ctx.pool(), ctx.company_id(), id)
                    .await
                    .map(|value| value.map(|_| ()))
            }
            Some(DocumentRef::Invoice(id)) => {
                db::invoices::advance_status_scoped(ctx.pool(), ctx.company_id(), id)
                    .await
                    .map(|value| value.map(|_| ()))
            }
            Some(DocumentRef::Waybill(id)) => {
                db::waybills::advance_status_scoped(ctx.pool(), ctx.company_id(), id)
                    .await
                    .map(|value| value.map(|_| ()))
            }
            Some(DocumentRef::AdjustmentAct(id)) => {
                db::adjustment_acts::change_status_scoped(ctx.pool(), ctx.company_id(), id)
                    .await
                    .map(|value| value.map(|_| ()))
            }
            None => Err(anyhow!("Некоректний ідентифікатор документа: {doc_id}")),
        };

        match advance_result {
            Ok(Some(())) => result.succeeded += 1,
            Ok(None) => {
                result.failed += 1;
                result
                    .errors
                    .push(format!("{doc_id}: документ не знайдено"));
            }
            Err(error) => {
                result.failed += 1;
                result.errors.push(format!("{doc_id}: {error}"));
            }
        }
    }

    result.message = match (result.succeeded, result.failed) {
        (0, failed) if failed > 0 => {
            format!("Не вдалося оновити статус жодного документа ({failed} помилок)")
        }
        (succeeded, 0) => {
            format!(
                "Оновлено статус для {succeeded} {}",
                document_word_form(succeeded)
            )
        }
        (succeeded, failed) => format!(
            "Оновлено статус для {succeeded} {}, {failed} помилок",
            document_word_form(succeeded)
        ),
    };

    Ok(result)
}

pub async fn act_adjustments_list(
    ctx: &AppCtx,
    original_act_id: String,
) -> Result<AdjustmentActsForActDto> {
    let act_id = Uuid::parse_str(&original_act_id)
        .with_context(|| format!("Некоректний UUID акту: {original_act_id}"))?;

    let rows = db::adjustment_acts::list_for_act(ctx.pool(), ctx.company_id(), act_id).await?;

    let items = rows
        .into_iter()
        .map(|row| AdjustmentActListItemDto {
            id: row.id.to_string(),
            number: row.number.clone(),
            date: row.date.format("%d.%m.%Y").to_string(),
            counterparty_id: row.counterparty_id.to_string(),
            counterparty_name: row.counterparty_name.clone(),
            amount_str: format!("{:.2}", row.total_amount),
            status: DocumentStatusDto::from_adjustment_act_status(&row.status),
            status_label: row.status.label().to_string(),
            direction: row.direction.as_str().to_string(),
            original_act_id: row.original_act_id.to_string(),
            original_act_number: row.original_act_number.clone(),
        })
        .collect();

    Ok(AdjustmentActsForActDto { items })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_required_draft_counterparty_id_rejects_empty_selection() {
        assert!(parse_required_draft_counterparty_id(None).is_err());
        assert!(parse_required_draft_counterparty_id(Some(String::new())).is_err());
        assert!(parse_required_draft_counterparty_id(Some("  ".to_string())).is_err());
    }
}
