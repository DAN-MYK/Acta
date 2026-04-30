use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::app_ctx::AppScreen;
use crate::db::search::{self, SearchResultItem};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    Navigate(AppScreen),
    OpenCounterparty(Uuid),
    OpenDocument(String),
    Create(PaletteCreateKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteCreateKind {
    Counterparty,
    Act,
    Invoice,
    Waybill,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteCreatePlan {
    OpenCounterpartyCreate,
    NavigateToCounterpartiesForSelection,
    CreateDocumentDraft {
        kind: PaletteDocumentKind,
        counterparty_id: Uuid,
    },
    InvalidSelectedCounterparty(String),
    UnsupportedKind(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteDocumentKind {
    Act,
    Invoice,
    Waybill,
}

impl PaletteDocumentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Act => "act",
            Self::Invoice => "invoice",
            Self::Waybill => "waybill",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteListItem {
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub shortcut: String,
    pub payload: String,
}

pub async fn search_palette_items(
    pool: &PgPool,
    company_id: Uuid,
    query: &str,
    selected_counterparty_id: &str,
) -> Result<Vec<PaletteListItem>> {
    let items = search::search(pool, company_id, query).await?;
    Ok(filter_palette_items(items, selected_counterparty_id))
}

pub fn parse_palette_action(payload: &str) -> Option<PaletteAction> {
    let (action, id) = payload.split_once(':')?;

    match action {
        "navigate" => app_screen_from_search_id(id).map(PaletteAction::Navigate),
        "open_cp" => Uuid::parse_str(id)
            .ok()
            .map(PaletteAction::OpenCounterparty),
        "open_doc" => Some(PaletteAction::OpenDocument(id.to_string())),
        "create" => Some(PaletteAction::Create(parse_create_kind(id))),
        _ => None,
    }
}

pub fn plan_palette_create(
    kind: &PaletteCreateKind,
    selected_counterparty_id: &str,
) -> PaletteCreatePlan {
    match kind {
        PaletteCreateKind::Counterparty => PaletteCreatePlan::OpenCounterpartyCreate,
        PaletteCreateKind::Act | PaletteCreateKind::Invoice | PaletteCreateKind::Waybill => {
            let selected_counterparty_id = selected_counterparty_id.trim();
            if selected_counterparty_id.is_empty() {
                return PaletteCreatePlan::NavigateToCounterpartiesForSelection;
            }

            match Uuid::parse_str(selected_counterparty_id) {
                Ok(counterparty_id) => PaletteCreatePlan::CreateDocumentDraft {
                    kind: document_kind_from_create_kind(kind),
                    counterparty_id,
                },
                Err(_) => PaletteCreatePlan::InvalidSelectedCounterparty(
                    selected_counterparty_id.to_string(),
                ),
            }
        }
        PaletteCreateKind::Other(kind) => PaletteCreatePlan::UnsupportedKind(kind.clone()),
    }
}

fn app_screen_from_search_id(id: &str) -> Option<AppScreen> {
    match id {
        "dashboard" => Some(AppScreen::Dashboard),
        "documents" | "acts" | "invoices" | "waybills" => Some(AppScreen::Documents),
        "counterparties" => Some(AppScreen::Counterparties),
        "payments" => Some(AppScreen::Payments),
        "reports" => Some(AppScreen::Reports),
        "tasks" => Some(AppScreen::Tasks),
        "settings" => Some(AppScreen::Settings),
        _ => None,
    }
}

fn parse_create_kind(kind: &str) -> PaletteCreateKind {
    match kind {
        "counterparty" => PaletteCreateKind::Counterparty,
        "act" => PaletteCreateKind::Act,
        "invoice" => PaletteCreateKind::Invoice,
        "waybill" => PaletteCreateKind::Waybill,
        other => PaletteCreateKind::Other(other.to_string()),
    }
}

fn document_kind_from_create_kind(kind: &PaletteCreateKind) -> PaletteDocumentKind {
    match kind {
        PaletteCreateKind::Act => PaletteDocumentKind::Act,
        PaletteCreateKind::Invoice => PaletteDocumentKind::Invoice,
        PaletteCreateKind::Waybill => PaletteDocumentKind::Waybill,
        PaletteCreateKind::Counterparty | PaletteCreateKind::Other(_) => {
            unreachable!("тільки документні create-kind мають потрапляти у document kind")
        }
    }
}

fn create_requires_selected_counterparty(kind: &str) -> bool {
    matches!(kind, "act" | "invoice" | "waybill")
}

fn filter_palette_items(
    items: Vec<SearchResultItem>,
    selected_counterparty_id: &str,
) -> Vec<PaletteListItem> {
    let has_selected_counterparty = !selected_counterparty_id.trim().is_empty();

    items
        .into_iter()
        .filter(|item| {
            if item.action != "create" {
                return true;
            }

            has_selected_counterparty || !create_requires_selected_counterparty(&item.id)
        })
        .map(|item| PaletteListItem {
            kind: item.kind,
            title: item.title,
            subtitle: item.subtitle,
            shortcut: item.shortcut,
            payload: if item.action.is_empty() {
                String::new()
            } else {
                format!("{}:{}", item.action, item.id)
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::search::SearchResultItem;

    use super::{
        filter_palette_items, parse_palette_action, plan_palette_create, PaletteAction,
        PaletteCreateKind, PaletteCreatePlan, PaletteDocumentKind,
    };

    #[test]
    fn parses_supported_palette_actions() {
        let counterparty_id = Uuid::new_v4();

        assert_eq!(
            parse_palette_action("navigate:documents"),
            Some(PaletteAction::Navigate(
                crate::app_ctx::AppScreen::Documents
            ))
        );
        assert_eq!(
            parse_palette_action(&format!("open_cp:{counterparty_id}")),
            Some(PaletteAction::OpenCounterparty(counterparty_id))
        );
        assert_eq!(
            parse_palette_action("open_doc:act:123"),
            Some(PaletteAction::OpenDocument("act:123".to_string()))
        );
        assert_eq!(
            parse_palette_action("create:invoice"),
            Some(PaletteAction::Create(PaletteCreateKind::Invoice))
        );
    }

    #[test]
    fn rejects_invalid_palette_actions() {
        assert_eq!(parse_palette_action("navigate:unknown"), None);
        assert_eq!(parse_palette_action("open_cp:not-a-uuid"), None);
        assert_eq!(parse_palette_action("broken"), None);
    }

    #[test]
    fn hides_document_create_items_without_selected_counterparty() {
        let items = vec![
            SearchResultItem {
                kind: "item".into(),
                action: "create".into(),
                id: "act".into(),
                title: "Новий акт".into(),
                subtitle: "".into(),
                shortcut: "".into(),
            },
            SearchResultItem {
                kind: "item".into(),
                action: "create".into(),
                id: "counterparty".into(),
                title: "Новий контрагент".into(),
                subtitle: "".into(),
                shortcut: "".into(),
            },
        ];

        let filtered = filter_palette_items(items, "");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "Новий контрагент");
    }

    #[test]
    fn document_create_plan_requires_selected_counterparty() {
        assert_eq!(
            plan_palette_create(&PaletteCreateKind::Invoice, ""),
            PaletteCreatePlan::NavigateToCounterpartiesForSelection
        );
    }

    #[test]
    fn document_create_plan_parses_selected_counterparty() {
        let counterparty_id = Uuid::new_v4();

        assert_eq!(
            plan_palette_create(&PaletteCreateKind::Act, &counterparty_id.to_string()),
            PaletteCreatePlan::CreateDocumentDraft {
                kind: PaletteDocumentKind::Act,
                counterparty_id,
            }
        );
    }
}
