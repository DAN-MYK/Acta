use serde::{Deserialize, Serialize};

use crate::models::act::ActStatus;
use crate::models::invoice::InvoiceStatus;
use crate::models::waybill::WaybillStatus;
use crate::models::DocumentDirection;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKindDto {
    Invoice,
    Act,
    Waybill,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatusDto {
    Draft,
    Issued,
    Signed,
    Paid,
    Delivered,
}

impl DocumentStatusDto {
    pub fn from_act_status(status: &ActStatus) -> Self {
        match status {
            ActStatus::Draft => Self::Draft,
            ActStatus::Issued => Self::Issued,
            ActStatus::Signed => Self::Signed,
            ActStatus::Paid => Self::Paid,
        }
    }

    pub fn from_invoice_status(status: &InvoiceStatus) -> Self {
        match status {
            InvoiceStatus::Draft => Self::Draft,
            InvoiceStatus::Issued => Self::Issued,
            InvoiceStatus::Signed => Self::Signed,
            InvoiceStatus::Paid => Self::Paid,
        }
    }

    pub fn from_waybill_status(status: &WaybillStatus) -> Self {
        match status {
            WaybillStatus::Draft => Self::Draft,
            WaybillStatus::Issued => Self::Issued,
            WaybillStatus::Signed => Self::Signed,
            WaybillStatus::Delivered => Self::Delivered,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentItemDto {
    pub id: String,
    pub kind: DocumentKindDto,
    pub number: String,
    pub date: String,
    pub counterparty: String,
    pub amount_str: String,
    pub status: DocumentStatusDto,
    pub status_label: String,
    pub linked_id: String,
    pub direction: String,  // "outgoing" | "incoming"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDraftFormDto {
    pub id: String,
    pub kind: String,
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub title: String,
    pub number: String,
    pub date: String,
    pub notes: String,
    pub direction: String,  // "outgoing" | "incoming"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDraftItemDto {
    pub description: String,
    pub unit: String,
    pub quantity: String,
    pub price: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChainStepDto {
    pub doc_type: String,
    pub doc_number: String,
    pub amount_str: String,
    pub status: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentsListDto {
    pub items: Vec<DocumentItemDto>,
    pub invoice_items: Vec<DocumentItemDto>,
    pub act_items: Vec<DocumentItemDto>,
    pub waybill_items: Vec<DocumentItemDto>,
    pub total_count: i32,
    pub page_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentEditorDto {
    pub form: DocumentDraftFormDto,
    pub items: Vec<DocumentDraftItemDto>,
    pub pdf: Option<DocumentPdfStateDto>,
    pub show_type_picker: bool,
    pub show_editor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPdfStateDto {
    pub file_path: String,
    pub page_count: usize,
    pub extracted_text: String,
    pub has_text_ops: bool,
    pub editable: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPdfActionResultDto {
    pub editor: DocumentEditorDto,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewDocumentContextDto {
    pub counterparty_id: String,
    pub counterparty_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentChainDto {
    pub source_id: String,
    pub steps: Vec<ChainStepDto>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DocumentsListRequest {
    pub query: Option<String>,
    pub direction: Option<DocumentDirection>,
    pub kind: Option<String>,
    pub counterparty_id: Option<String>,
    pub date_from: Option<chrono::NaiveDate>,
    pub date_to: Option<chrono::NaiveDate>,
    pub statuses: Option<Vec<String>>,
    pub amount_min: Option<rust_decimal::Decimal>,
    pub amount_max: Option<rust_decimal::Decimal>,
    pub overdue_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDocumentDraftRequest {
    pub counterparty_id: Option<String>,
    pub kind: String,
    pub direction: String,  // "outgoing" | "incoming"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateChainDraftRequest {
    pub source_id: String,
    pub target_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveDocumentRequest {
    pub form: DocumentDraftFormDto,
    pub items: Vec<DocumentDraftItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceDocumentPdfTextRequest {
    pub doc_id: String,
    pub find_text: String,
    pub replace_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BulkDocumentRequest {
    pub doc_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MutationResultDto {
    pub ok: bool,
    pub document_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveDocumentResponse {
    pub document_id: String,
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BulkMutationResultDto {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub message: String,
}
