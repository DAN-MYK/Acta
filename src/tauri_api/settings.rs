use std::path::PathBuf;

use anyhow::{anyhow, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs;
use tokio::process::Command;
use uuid::Uuid;

use crate::app_ctx::AppCtx;
use crate::config::AppConfig;
use crate::db;
use crate::models::company::{Company, UpdateCompany};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsCompanyDto {
    pub full_name: String,
    pub short_name: String,
    pub edrpou: String,
    pub ipn: String,
    pub address: String,
    pub director: String,
    pub iban: String,
    pub bank: String,
    pub vat_registered: bool,
    pub vat_cert: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsIntegrationDto {
    pub label: String,
    pub description: String,
    pub tag: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsTeamMemberDto {
    pub name: String,
    pub email: String,
    pub role: String,
    pub last_active: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsNumberingRowDto {
    pub doc_type: String,
    pub template: String,
    pub example: String,
    pub next_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPreferencesDto {
    pub dark_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsBackupDto {
    pub label: String,
    pub file: String,
    pub kind: String,
    pub note: String,
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsScreenDto {
    pub company: SettingsCompanyDto,
    pub integrations: Vec<SettingsIntegrationDto>,
    pub team: Vec<SettingsTeamMemberDto>,
    pub numbering: Vec<SettingsNumberingRowDto>,
    pub preferences: SettingsPreferencesDto,
    pub backup: SettingsBackupDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPreferencesRequest {
    pub dark_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSaveCompanyRequest {
    pub company: SettingsCompanyDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsIntegrationActionRequest {
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsScreenMutationResultDto {
    pub ok: bool,
    pub message: String,
    pub screen: SettingsScreenDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsActionResultDto {
    pub ok: bool,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct InviteDraft {
    name: String,
    email: String,
    role: String,
    last_active: String,
}

struct BackupInfo {
    label: String,
    file: String,
    kind: String,
    note: String,
    tone: String,
    path: Option<PathBuf>,
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn preferences_from_config(config: &AppConfig) -> SettingsPreferencesDto {
    SettingsPreferencesDto {
        dark_mode: config.dark_mode,
    }
}

fn company_to_dto(company: Option<&Company>) -> SettingsCompanyDto {
    let Some(company) = company else {
        return SettingsCompanyDto {
            full_name: String::new(),
            short_name: String::new(),
            edrpou: String::new(),
            ipn: String::new(),
            address: String::new(),
            director: String::new(),
            iban: String::new(),
            bank: String::new(),
            vat_registered: false,
            vat_cert: String::new(),
        };
    };

    SettingsCompanyDto {
        full_name: company.name.clone(),
        short_name: company.short_name.clone().unwrap_or_default(),
        edrpou: company.edrpou.clone().unwrap_or_default(),
        ipn: company.ipn.clone().unwrap_or_default(),
        address: company.legal_address.clone().unwrap_or_default(),
        director: company.director_name.clone().unwrap_or_default(),
        iban: company.iban.clone().unwrap_or_default(),
        bank: String::new(),
        vat_registered: company.is_vat_payer,
        vat_cert: if company.is_vat_payer {
            "Платник ПДВ".to_string()
        } else {
            "Без ПДВ".to_string()
        },
    }
}

fn numbering_rows() -> Vec<SettingsNumberingRowDto> {
    vec![
        SettingsNumberingRowDto {
            doc_type: "Акт".to_string(),
            template: "ACT-{YYYY}-{NNNN}".to_string(),
            example: "ACT-2026-0001".to_string(),
            next_number: "1".to_string(),
        },
        SettingsNumberingRowDto {
            doc_type: "Рахунок".to_string(),
            template: "INV-{YYYY}-{NNNN}".to_string(),
            example: "INV-2026-0001".to_string(),
            next_number: "1".to_string(),
        },
    ]
}

fn integrations_dir() -> PathBuf {
    PathBuf::from("storage/integrations")
}

fn integration_config_path(tag: &str) -> PathBuf {
    integrations_dir().join(format!("{tag}.json"))
}

fn team_invites_dir() -> PathBuf {
    PathBuf::from("storage/team/invites")
}

fn backups_dir() -> PathBuf {
    PathBuf::from("storage/backups")
}

async fn load_integrations() -> Result<Vec<SettingsIntegrationDto>> {
    fs::create_dir_all(integrations_dir()).await?;
    Ok(vec![
        SettingsIntegrationDto {
            label: "BAS".to_string(),
            description: "Імпорт документів та довідників".to_string(),
            tag: "bas".to_string(),
            enabled: fs::metadata(integration_config_path("bas")).await.is_ok(),
        },
        SettingsIntegrationDto {
            label: "Банк".to_string(),
            description: "Синхронізація виписок та звірка платежів".to_string(),
            tag: "bank".to_string(),
            enabled: fs::metadata(integration_config_path("bank")).await.is_ok(),
        },
    ])
}

async fn load_team(company: Option<&Company>) -> Result<Vec<SettingsTeamMemberDto>> {
    let mut members = Vec::new();

    if let Some(company) = company {
        members.push(SettingsTeamMemberDto {
            name: company
                .director_name
                .clone()
                .unwrap_or_else(|| "Власник компанії".to_string()),
            email: company
                .email
                .clone()
                .unwrap_or_else(|| "local-owner@acta".to_string()),
            role: "Owner".to_string(),
            last_active: "Локально".to_string(),
        });
    }

    fs::create_dir_all(team_invites_dir()).await?;
    let mut entries = fs::read_dir(team_invites_dir()).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let Ok(text) = fs::read_to_string(&path).await else {
            continue;
        };
        let Ok(invite) = serde_json::from_str::<InviteDraft>(&text) else {
            continue;
        };
        members.push(SettingsTeamMemberDto {
            name: invite.name,
            email: invite.email,
            role: invite.role,
            last_active: invite.last_active,
        });
    }

    Ok(members)
}

async fn load_backup_info() -> Result<BackupInfo> {
    fs::create_dir_all(backups_dir()).await?;
    let mut entries = fs::read_dir(backups_dir()).await?;
    let mut newest: Option<(std::time::SystemTime, PathBuf, u64)> = None;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;
        if !metadata.is_file() {
            continue;
        }

        let modified = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let len = metadata.len();
        match &newest {
            Some((current, _, _)) if modified <= *current => {}
            _ => newest = Some((modified, path, len)),
        }
    }

    let Some((modified, path, len)) = newest else {
        return Ok(BackupInfo {
            label: "Ще не створювався".to_string(),
            file: "Локальний backup ще не знайдено".to_string(),
            kind: "Немає резервної копії".to_string(),
            note: "Створіть першу локальну копію вручну.".to_string(),
            tone: "muted".to_string(),
            path: None,
        });
    };

    let modified: chrono::DateTime<Local> = modified.into();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    let (kind, note, tone) = if extension == "sql" {
        (
            "Повний backup БД".to_string(),
            "Створено через pg_dump, це повна локальна копія бази.".to_string(),
            "success".to_string(),
        )
    } else if extension == "json" {
        let text = fs::read_to_string(&path).await.unwrap_or_default();
        let mode = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|value| {
                value
                    .get("mode")
                    .and_then(|mode| mode.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "partial".to_string());

        if mode == "partial" {
            (
                "Partial metadata snapshot".to_string(),
                "Fallback без pg_dump: це не повний backup БД.".to_string(),
                "warning".to_string(),
            )
        } else {
            (
                "JSON snapshot".to_string(),
                "Локальний snapshot у JSON-форматі.".to_string(),
                "muted".to_string(),
            )
        }
    } else {
        (
            "Локальний файл backup".to_string(),
            "Тип файлу не класифіковано окремо, але копію можна відкрити локально.".to_string(),
            "muted".to_string(),
        )
    };

    Ok(BackupInfo {
        label: modified.format("%d.%m.%Y %H:%M").to_string(),
        file: format!(
            "{} · {:.1} KB",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default(),
            len as f64 / 1024.0
        ),
        kind,
        note,
        tone,
        path: Some(path),
    })
}

async fn write_integration_config(tag: &str) -> Result<PathBuf> {
    fs::create_dir_all(integrations_dir()).await?;

    let path = integration_config_path(tag);
    let template = match tag {
        "bas" => json!({
            "type": "bas",
            "input_dir": "./bas-export",
            "enabled": true
        }),
        "bank" => json!({
            "type": "bank",
            "import_dir": "./storage/import/bank",
            "enabled": true
        }),
        other => return Err(anyhow!("Невідома інтеграція: {other}")),
    };

    fs::write(&path, serde_json::to_string_pretty(&template)?).await?;
    Ok(path)
}

async fn create_invite_draft() -> Result<PathBuf> {
    fs::create_dir_all(team_invites_dir()).await?;
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let path = team_invites_dir().join(format!("invite-{stamp}-{}.json", Uuid::new_v4().simple()));
    let draft = InviteDraft {
        name: "Нове запрошення".to_string(),
        email: format!("pending-{stamp}@local"),
        role: "Спостерігач".to_string(),
        last_active: "Очікує надсилання".to_string(),
    };
    fs::write(&path, serde_json::to_string_pretty(&draft)?).await?;
    Ok(path)
}

async fn create_backup_snapshot(company_id: Uuid) -> Result<PathBuf> {
    fs::create_dir_all(backups_dir()).await?;
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let unique = Uuid::new_v4().simple().to_string();
    let sql_path = backups_dir().join(format!("acta-backup-{stamp}-{unique}.sql"));

    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        match Command::new("pg_dump")
            .arg("--dbname")
            .arg(&database_url)
            .arg("--file")
            .arg(&sql_path)
            .output()
            .await
        {
            Ok(output) if output.status.success() => return Ok(sql_path),
            Ok(output) => {
                tracing::warn!(
                    "settings: pg_dump fallback engaged: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(error) => {
                tracing::warn!("settings: pg_dump unavailable: {error}");
            }
        }
    }

    let json_path = backups_dir().join(format!("acta-backup-{stamp}-{unique}.json"));
    let payload = json!({
        "mode": "partial",
        "created_at": Local::now().to_rfc3339(),
        "company_id": company_id,
        "note": "pg_dump недоступний, тому створено локальний JSON snapshot."
    });
    fs::write(&json_path, serde_json::to_string_pretty(&payload)?).await?;
    Ok(json_path)
}

async fn open_backup(path: PathBuf) -> Result<()> {
    let open_path = path.clone();
    if let Ok(Err(error)) = tokio::task::spawn_blocking(move || open::that(open_path)).await {
        tracing::warn!("settings: failed to open backup: {error}");
    }
    Ok(())
}

async fn build_screen(ctx: &AppCtx) -> Result<SettingsScreenDto> {
    let company = db::companies::get_by_id(ctx.pool(), ctx.company_id()).await?;
    let config = AppConfig::load();
    let backup = load_backup_info().await?;

    Ok(SettingsScreenDto {
        company: company_to_dto(company.as_ref()),
        integrations: load_integrations().await?,
        team: load_team(company.as_ref()).await?,
        numbering: numbering_rows(),
        preferences: preferences_from_config(&config),
        backup: SettingsBackupDto {
            label: backup.label,
            file: backup.file,
            kind: backup.kind,
            note: backup.note,
            tone: backup.tone,
        },
    })
}

pub async fn settings_load(ctx: &AppCtx) -> Result<SettingsScreenDto> {
    build_screen(ctx).await
}

pub async fn settings_save_preferences(
    ctx: &AppCtx,
    request: SettingsPreferencesRequest,
) -> Result<SettingsScreenMutationResultDto> {
    let mut config = AppConfig::load();
    config.dark_mode = request.dark_mode;
    config.save();

    Ok(SettingsScreenMutationResultDto {
        ok: true,
        message: "Налаштування вигляду збережено".to_string(),
        screen: build_screen(ctx).await?,
    })
}

pub async fn settings_save_company(
    ctx: &AppCtx,
    request: SettingsSaveCompanyRequest,
) -> Result<SettingsScreenMutationResultDto> {
    let full_name = request.company.full_name.trim();
    if full_name.is_empty() {
        return Err(anyhow!("Назва компанії є обов'язковою"));
    }

    let payload = UpdateCompany {
        name: full_name.to_string(),
        short_name: optional_string(&request.company.short_name),
        edrpou: optional_string(&request.company.edrpou),
        iban: optional_string(&request.company.iban),
        legal_address: optional_string(&request.company.address),
        director_name: optional_string(&request.company.director),
        accountant_name: None,
        tax_system: None,
        is_vat_payer: request.company.vat_registered,
        logo_path: None,
    };

    let company = db::companies::update(ctx.pool(), ctx.company_id(), &payload)
        .await?
        .ok_or_else(|| anyhow!("Компанію не знайдено"))?;

    Ok(SettingsScreenMutationResultDto {
        ok: true,
        message: format!("Профіль компанії \"{}\" збережено", company.name),
        screen: build_screen(ctx).await?,
    })
}

pub async fn settings_configure_integration(
    ctx: &AppCtx,
    request: SettingsIntegrationActionRequest,
) -> Result<SettingsScreenMutationResultDto> {
    let path = write_integration_config(&request.tag.to_ascii_lowercase()).await?;
    Ok(SettingsScreenMutationResultDto {
        ok: true,
        message: format!("Конфіг інтеграції створено: {}", path.display()),
        screen: build_screen(ctx).await?,
    })
}

pub async fn settings_team_invite(ctx: &AppCtx) -> Result<SettingsScreenMutationResultDto> {
    let path = create_invite_draft().await?;
    Ok(SettingsScreenMutationResultDto {
        ok: true,
        message: format!("Чернетку запрошення створено: {}", path.display()),
        screen: build_screen(ctx).await?,
    })
}

pub async fn settings_backup_now(ctx: &AppCtx) -> Result<SettingsScreenMutationResultDto> {
    let path = create_backup_snapshot(ctx.company_id()).await?;
    Ok(SettingsScreenMutationResultDto {
        ok: true,
        message: format!("Резервну копію створено: {}", path.display()),
        screen: build_screen(ctx).await?,
    })
}

pub async fn settings_backup_open_latest(_ctx: &AppCtx) -> Result<SettingsActionResultDto> {
    let backup = load_backup_info().await?;
    let path = backup
        .path
        .ok_or_else(|| anyhow!("Ще немає жодної резервної копії"))?;
    open_backup(path.clone()).await?;

    Ok(SettingsActionResultDto {
        ok: true,
        message: "Резервну копію відкрито".to_string(),
        path: path.to_string_lossy().into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn company_to_dto_maps_optional_fields() {
        let company = Company {
            id: Uuid::new_v4(),
            name: "ТОВ Тест".to_string(),
            short_name: None,
            edrpou: Some("12345678".to_string()),
            ipn: None,
            iban: None,
            legal_address: None,
            actual_address: None,
            phone: None,
            email: None,
            director_name: None,
            accountant_name: None,
            tax_system: None,
            is_vat_payer: false,
            logo_path: None,
            notes: None,
            is_archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let dto = company_to_dto(Some(&company));
        assert_eq!(dto.full_name, "ТОВ Тест");
        assert_eq!(dto.edrpou, "12345678");
        assert_eq!(dto.short_name, "");
        assert_eq!(dto.vat_cert, "Без ПДВ");
    }
}
