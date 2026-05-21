use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::palette::{
    parse_palette_action, plan_palette_create, search_palette_items, PaletteAction,
    PaletteCreatePlan,
};
use crate::app_ctx::{AppCtx, AppScreen};
use crate::config::AppConfig;
use crate::db::companies;
use crate::models::company::Company;
use crate::tauri_api::documents::{self, CreateDocumentDraftRequest, DocumentEditorDto};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellChromeDto {
    pub company_name: String,
    pub user_name: String,
    pub user_initials: String,
    pub user_role: String,
    pub documents_badge: i32,
    pub tasks_badge: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompanySwitcherItemDto {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub initials: String,
    pub badge: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaletteItemDto {
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub shortcut: String,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellStateDto {
    pub chrome: ShellChromeDto,
    pub company_items: Vec<CompanySwitcherItemDto>,
    pub active_company_id: String,
    pub is_dark: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaletteSearchRequestDto {
    pub query: String,
    pub selected_counterparty_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaletteSearchResultDto {
    pub items: Vec<PaletteItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaletteActivationKindDto {
    Navigate,
    OpenDocument,
    OpenCounterparty,
    CreateDocumentDraft,
    OpenCounterpartyCreate,
    NavigateForCounterpartySelection,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaletteActivationResultDto {
    pub kind: PaletteActivationKindDto,
    pub screen: Option<String>,
    pub document_id: Option<String>,
    pub counterparty_id: Option<String>,
    pub document_editor: Option<DocumentEditorDto>,
    pub message: Option<String>,
}

fn app_screen_id(screen: AppScreen) -> String {
    match screen {
        AppScreen::Dashboard => "dashboard",
        AppScreen::Documents => "documents",
        AppScreen::Counterparties => "counterparties",
        AppScreen::Payments => "payments",
        AppScreen::Reports => "reports",
        AppScreen::Tasks => "tasks",
        AppScreen::Settings => "settings",
    }
    .to_string()
}

fn company_display_name(company: &Company) -> String {
    company
        .short_name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| company.name.clone())
}

fn company_switcher_initials(company: &Company) -> String {
    let display_name = company_display_name(company);
    let mut initials = display_name
        .split(|ch: char| ch.is_whitespace() || ch == '«' || ch == '»' || ch == '"' || ch == '\'')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();

    if initials.is_empty() {
        initials = "A".to_string();
    }

    initials
}

fn company_switcher_subtitle(company: &Company) -> String {
    let mut parts = Vec::new();
    if let Some(edrpou) = company
        .edrpou
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("ЄДРПОУ {edrpou}"));
    }
    if company.is_vat_payer {
        parts.push("ПДВ".to_string());
    }

    if parts.is_empty() {
        "Без додаткових реквізитів".to_string()
    } else {
        parts.join(" · ")
    }
}

pub async fn shell_load(ctx: &AppCtx) -> Result<ShellStateDto> {
    let companies = companies::list(ctx.pool()).await?;
    let active_company_id = ctx.company_id();
    let config = AppConfig::load();

    let company_items = companies
        .iter()
        .map(|company| CompanySwitcherItemDto {
            id: company.id.to_string(),
            name: company_display_name(company),
            subtitle: company_switcher_subtitle(company),
            initials: company_switcher_initials(company),
            badge: if company.id == active_company_id {
                "Активна".to_string()
            } else {
                String::new()
            },
            active: company.id == active_company_id,
        })
        .collect::<Vec<_>>();

    let company_name = companies
        .iter()
        .find(|company| company.id == active_company_id)
        .map(company_display_name)
        .or_else(|| companies.first().map(company_display_name))
        .unwrap_or_else(|| "Acta".to_string());

    Ok(ShellStateDto {
        chrome: ShellChromeDto {
            company_name,
            user_name: "Адміністратор".to_string(),
            user_initials: "АД".to_string(),
            user_role: "Адміністратор".to_string(),
            documents_badge: 0,
            tasks_badge: 0,
        },
        company_items,
        active_company_id: active_company_id.to_string(),
        is_dark: config.dark_mode,
    })
}

pub async fn shell_set_active_company(ctx: &AppCtx, company_id: String) -> Result<ShellStateDto> {
    let parsed = Uuid::parse_str(&company_id)
        .with_context(|| format!("Некоректний UUID компанії: {company_id}"))?;

    if companies::get_by_id(ctx.pool(), parsed).await?.is_none() {
        return Err(anyhow!("Компанію не знайдено"));
    }

    ctx.set_company_id(parsed);
    let mut config = AppConfig::load();
    config.last_company_id = Some(parsed);
    config.save();
    shell_load(ctx).await
}

pub async fn shell_palette_search(
    ctx: &AppCtx,
    request: PaletteSearchRequestDto,
) -> Result<PaletteSearchResultDto> {
    let selected_counterparty_id = request.selected_counterparty_id.unwrap_or_default();
    let items = search_palette_items(
        ctx.pool(),
        ctx.company_id(),
        &request.query,
        &selected_counterparty_id,
    )
    .await?;

    Ok(PaletteSearchResultDto {
        items: items
            .into_iter()
            .map(|item| PaletteItemDto {
                kind: item.kind,
                title: item.title,
                subtitle: item.subtitle,
                shortcut: item.shortcut,
                payload: item.payload,
            })
            .collect(),
    })
}

pub async fn shell_palette_activate(
    ctx: &AppCtx,
    payload: String,
    selected_counterparty_id: Option<String>,
) -> Result<PaletteActivationResultDto> {
    match parse_palette_action(&payload) {
        Some(PaletteAction::Navigate(screen)) => Ok(PaletteActivationResultDto {
            kind: PaletteActivationKindDto::Navigate,
            screen: Some(app_screen_id(screen)),
            document_id: None,
            counterparty_id: None,
            document_editor: None,
            message: None,
        }),
        Some(PaletteAction::OpenCounterparty(counterparty_id)) => Ok(PaletteActivationResultDto {
            kind: PaletteActivationKindDto::OpenCounterparty,
            screen: Some(app_screen_id(AppScreen::Counterparties)),
            document_id: None,
            counterparty_id: Some(counterparty_id.to_string()),
            document_editor: None,
            message: None,
        }),
        Some(PaletteAction::OpenDocument(document_id)) => Ok(PaletteActivationResultDto {
            kind: PaletteActivationKindDto::OpenDocument,
            screen: Some(app_screen_id(AppScreen::Documents)),
            document_id: Some(document_id),
            counterparty_id: None,
            document_editor: None,
            message: None,
        }),
        Some(PaletteAction::Create(kind)) => {
            let selected_counterparty_id = selected_counterparty_id.unwrap_or_default();
            match plan_palette_create(&kind, &selected_counterparty_id) {
                PaletteCreatePlan::OpenCounterpartyCreate => Ok(PaletteActivationResultDto {
                    kind: PaletteActivationKindDto::OpenCounterpartyCreate,
                    screen: Some(app_screen_id(AppScreen::Counterparties)),
                    document_id: None,
                    counterparty_id: None,
                    document_editor: None,
                    message: Some("Відкрити створення контрагента".to_string()),
                }),
                PaletteCreatePlan::NavigateToCounterpartiesForSelection => {
                    Ok(PaletteActivationResultDto {
                        kind: PaletteActivationKindDto::NavigateForCounterpartySelection,
                        screen: Some(app_screen_id(AppScreen::Counterparties)),
                        document_id: None,
                        counterparty_id: None,
                        document_editor: None,
                        message: Some("Спершу виберіть контрагента".to_string()),
                    })
                }
                PaletteCreatePlan::CreateDocumentDraft {
                    kind,
                    counterparty_id,
                } => {
                    // Command palette has no tab context — always creates outgoing (most common case)
                    let document_editor = documents::document_create_draft(
                        ctx,
                        CreateDocumentDraftRequest {
                            counterparty_id: Some(counterparty_id.to_string()),
                            kind: kind.as_str().to_string(),
                            direction: Some("outgoing".to_string()),
                            original_act_id: None,
                        },
                    )
                    .await?;

                    Ok(PaletteActivationResultDto {
                        kind: PaletteActivationKindDto::CreateDocumentDraft,
                        screen: Some(app_screen_id(AppScreen::Documents)),
                        document_id: Some(document_editor.form.id.clone()),
                        counterparty_id: Some(counterparty_id.to_string()),
                        document_editor: Some(document_editor),
                        message: None,
                    })
                }
                PaletteCreatePlan::InvalidSelectedCounterparty(counterparty_id) => Err(anyhow!(
                    "Некоректний вибраний контрагент для create flow: {counterparty_id}"
                )),
                PaletteCreatePlan::UnsupportedKind(kind) => Ok(PaletteActivationResultDto {
                    kind: PaletteActivationKindDto::Unsupported,
                    screen: None,
                    document_id: None,
                    counterparty_id: None,
                    document_editor: None,
                    message: Some(format!("Непідтриманий create-kind: {kind}")),
                }),
            }
        }
        None => Err(anyhow!(
            "Некоректний або непідтриманий payload палітри: {payload}"
        )),
    }
}
