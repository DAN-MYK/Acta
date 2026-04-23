use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::Local;
use notify_rust::{Notification, Timeout};
use serde::{Deserialize, Serialize};
use serde_json::json;
use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use tokio::fs;
use tokio::process::Command;
use uuid::Uuid;

use acta::app_ctx::{AppCtx, AppScreen};
use acta::db;
use acta::models::company::{Company, UpdateCompany};

pub struct SettingsData {
    pub company_info: crate::CompanyInfo,
    pub integrations: Vec<crate::IntegrationItem>,
    pub team_members: Vec<crate::TeamMember>,
    pub numbering_rows: Vec<crate::NumberingRow>,
    pub last_backup_label: String,
    pub last_backup_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InviteDraft {
    name: String,
    email: String,
    role: String,
    last_active: String,
}

/// Перетворює `Company` з БД у `CompanyInfo` для Slint.
pub fn company_to_info(company: &Company) -> crate::CompanyInfo {
    crate::CompanyInfo {
        full_name: company.name.clone().into(),
        short_name: company.short_name.clone().unwrap_or_default().into(),
        edrpou: company.edrpou.clone().unwrap_or_default().into(),
        ipn: company.ipn.clone().unwrap_or_default().into(),
        address: company.legal_address.clone().unwrap_or_default().into(),
        director: company.director_name.clone().unwrap_or_default().into(),
        iban: company.iban.clone().unwrap_or_default().into(),
        bank: slint::SharedString::default(),
        vat_registered: company.is_vat_payer,
        vat_cert: if company.is_vat_payer {
            "Платник ПДВ".into()
        } else {
            "Без ПДВ".into()
        },
    }
}

fn info_to_update(info: &crate::CompanyInfo) -> UpdateCompany {
    fn opt(value: &slint::SharedString) -> Option<String> {
        let trimmed = value.as_str().trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    UpdateCompany {
        name: info.full_name.as_str().trim().to_string(),
        short_name: opt(&info.short_name),
        edrpou: opt(&info.edrpou),
        iban: opt(&info.iban),
        legal_address: opt(&info.address),
        director_name: opt(&info.director),
        accountant_name: None,
        tax_system: None,
        is_vat_payer: info.vat_registered,
        logo_path: None,
    }
}

fn default_company_info() -> crate::CompanyInfo {
    crate::CompanyInfo {
        full_name: slint::SharedString::default(),
        short_name: slint::SharedString::default(),
        edrpou: slint::SharedString::default(),
        ipn: slint::SharedString::default(),
        address: slint::SharedString::default(),
        director: slint::SharedString::default(),
        iban: slint::SharedString::default(),
        bank: slint::SharedString::default(),
        vat_registered: false,
        vat_cert: slint::SharedString::default(),
    }
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

fn default_numbering_rows() -> Vec<crate::NumberingRow> {
    vec![
        crate::NumberingRow {
            doc_type: "Акт".into(),
            template: "ACT-{YYYY}-{NNNN}".into(),
            example: "ACT-2026-0001".into(),
            next_number: 1,
        },
        crate::NumberingRow {
            doc_type: "Рахунок".into(),
            template: "INV-{YYYY}-{NNNN}".into(),
            example: "INV-2026-0001".into(),
            next_number: 1,
        },
    ]
}

fn notify_user(summary: &str, body: &str) {
    let _ = Notification::new()
        .appname("Acta")
        .summary(summary)
        .body(body)
        .timeout(Timeout::Milliseconds(6_000))
        .show();
}

async fn load_integrations() -> Vec<crate::IntegrationItem> {
    let _ = fs::create_dir_all(integrations_dir()).await;

    vec![
        crate::IntegrationItem {
            label: "BAS".into(),
            description: "Імпорт документів та довідників".into(),
            tag: "bas".into(),
            enabled: fs::metadata(integration_config_path("bas")).await.is_ok(),
        },
        crate::IntegrationItem {
            label: "Банк".into(),
            description: "Синхронізація виписок та звірка платежів".into(),
            tag: "bank".into(),
            enabled: fs::metadata(integration_config_path("bank")).await.is_ok(),
        },
    ]
}

async fn load_team_members(company: Option<&Company>) -> Vec<crate::TeamMember> {
    let mut members = Vec::new();

    if let Some(company) = company {
        members.push(crate::TeamMember {
            name: company
                .director_name
                .clone()
                .unwrap_or_else(|| "Власник компанії".to_string())
                .into(),
            email: company
                .email
                .clone()
                .unwrap_or_else(|| "local-owner@acta".to_string())
                .into(),
            role: "Owner".into(),
            last_active: "Локально".into(),
        });
    }

    let _ = fs::create_dir_all(team_invites_dir()).await;
    if let Ok(mut entries) = fs::read_dir(team_invites_dir()).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            if let Ok(text) = fs::read_to_string(&path).await {
                if let Ok(invite) = serde_json::from_str::<InviteDraft>(&text) {
                    members.push(crate::TeamMember {
                        name: invite.name.into(),
                        email: invite.email.into(),
                        role: invite.role.into(),
                        last_active: invite.last_active.into(),
                    });
                }
            }
        }
    }

    members
}

async fn last_backup_info() -> (String, String) {
    let _ = fs::create_dir_all(backups_dir()).await;
    let Ok(mut entries) = fs::read_dir(backups_dir()).await else {
        return (
            "Ще не створювався".to_string(),
            "Локальний бекап не знайдено".to_string(),
        );
    };

    let mut newest: Option<(std::time::SystemTime, PathBuf, u64)> = None;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let Ok(metadata) = entry.metadata().await else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }

        let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let len = metadata.len();
        match &newest {
            Some((current, _, _)) if modified <= *current => {}
            _ => newest = Some((modified, path, len)),
        }
    }

    if let Some((modified, path, len)) = newest {
        let modified: chrono::DateTime<Local> = modified.into();
        let label = modified.format("%d.%m.%Y %H:%M").to_string();
        let file = format!(
            "{} · {:.1} KB",
            path.file_name().and_then(|name| name.to_str()).unwrap_or_default(),
            len as f64 / 1024.0
        );
        (label, file)
    } else {
        (
            "Ще не створювався".to_string(),
            "Локальний бекап не знайдено".to_string(),
        )
    }
}

async fn write_integration_config(tag: &str) -> Result<PathBuf> {
    let dir = integrations_dir();
    fs::create_dir_all(&dir).await?;

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
        other => {
            return Err(anyhow!("Невідома інтеграція: {other}"));
        }
    };

    fs::write(&path, serde_json::to_string_pretty(&template)?).await?;
    Ok(path)
}

async fn create_invite_draft() -> Result<PathBuf> {
    let dir = team_invites_dir();
    fs::create_dir_all(&dir).await?;

    let now = Local::now();
    let stamp = now.format("%Y%m%d-%H%M%S").to_string();
    let path = dir.join(format!("invite-{stamp}.json"));
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
    let sql_path = backups_dir().join(format!("acta-backup-{stamp}.sql"));

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

    let json_path = backups_dir().join(format!("acta-backup-{stamp}.json"));
    let payload = json!({
        "mode": "partial",
        "created_at": Local::now().to_rfc3339(),
        "company_id": company_id,
        "note": "pg_dump недоступний, тому створено локальний JSON snapshot."
    });
    fs::write(&json_path, serde_json::to_string_pretty(&payload)?).await?;
    Ok(json_path)
}

async fn open_latest_backup() -> Result<PathBuf> {
    let (label, file) = last_backup_info().await;
    if label == "Ще не створювався" {
        return Err(anyhow!("Ще немає жодної резервної копії"));
    }

    let file_name = file
        .split('·')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Не вдалося визначити ім'я файлу резервної копії"))?;
    let path = backups_dir().join(file_name);

    let open_path = path.clone();
    let _ = tokio::task::spawn_blocking(move || open::that(open_path)).await;
    Ok(path)
}

pub async fn prepare_settings_data(pool: &PgPool, company_id: Uuid) -> SettingsData {
    let company = db::companies::get_by_id(pool, company_id)
        .await
        .ok()
        .flatten();
    let company_info = company
        .as_ref()
        .map(company_to_info)
        .unwrap_or_else(default_company_info);
    let integrations = load_integrations().await;
    let team_members = load_team_members(company.as_ref()).await;
    let (last_backup_label, last_backup_file) = last_backup_info().await;

    SettingsData {
        company_info,
        integrations,
        team_members,
        numbering_rows: default_numbering_rows(),
        last_backup_label,
        last_backup_file,
    }
}

pub fn apply_settings_to_ui(ui: &crate::AppWindow, data: SettingsData) {
    ui.set_settings_screen(crate::SettingsViewData {
        company_info: data.company_info,
        integrations: ModelRc::new(VecModel::from(data.integrations)),
        team_members: ModelRc::new(VecModel::from(data.team_members)),
        numbering_rows: ModelRc::new(VecModel::from(data.numbering_rows)),
        last_backup_label: data.last_backup_label.into(),
        last_backup_file: data.last_backup_file.into(),
    });
}

pub fn wire_settings_callbacks(ui: &crate::AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_settings_company_saved({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |info| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let update = info_to_update(&info);

            tokio::spawn(async move {
                let company_id = ctx.company_id();
                match db::companies::update(ctx.pool(), company_id, &update).await {
                    Ok(Some(company)) => {
                        tracing::info!("settings: company saved");
                        crate::bootstrap::refresh_screen(ui_weak.clone(), ctx.clone(), AppScreen::Settings).await;
                        notify_user(
                            "Налаштування компанії збережено",
                            &format!("Оновлено профіль '{}'", company.name),
                        );
                    }
                    Ok(None) => tracing::warn!("settings: company not found id={company_id}"),
                    Err(error) => {
                        tracing::error!("settings: save failed: {error}");
                        notify_user("Помилка збереження компанії", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_settings_section_changed(|section| {
        tracing::info!("settings: section_changed({section})");
    });
    ui.on_settings_dark_mode_toggled(|dark| {
        tracing::info!("settings: dark_mode_toggled({dark})");
    });
    ui.on_settings_density_changed(|density| {
        tracing::info!("settings: density_changed({density})");
    });

    ui.on_settings_integration_configure({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |integration| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let tag = integration.to_string().to_lowercase();
            tokio::spawn(async move {
                match write_integration_config(&tag).await {
                    Ok(path) => {
                        crate::bootstrap::refresh_screen(ui_weak.clone(), ctx.clone(), AppScreen::Settings).await;
                        notify_user(
                            "Інтеграцію налаштовано",
                            &format!("Створено конфіг: {}", path.display()),
                        );
                    }
                    Err(error) => {
                        tracing::error!("settings: integration configure failed: {error}");
                        notify_user("Помилка інтеграції", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_settings_team_invite({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move || {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                match create_invite_draft().await {
                    Ok(path) => {
                        crate::bootstrap::refresh_screen(ui_weak.clone(), ctx.clone(), AppScreen::Settings).await;
                        notify_user(
                            "Чернетку запрошення створено",
                            &format!("Відредагуйте файл {}", path.display()),
                        );
                    }
                    Err(error) => {
                        tracing::error!("settings: team invite failed: {error}");
                        notify_user("Помилка створення запрошення", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_settings_backup_now({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move || {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                match create_backup_snapshot(ctx.company_id()).await {
                    Ok(path) => {
                        crate::bootstrap::refresh_screen(ui_weak.clone(), ctx.clone(), AppScreen::Settings).await;
                        notify_user(
                            "Резервну копію створено",
                            &format!("Файл збережено: {}", path.display()),
                        );
                    }
                    Err(error) => {
                        tracing::error!("settings: backup failed: {error}");
                        notify_user("Помилка створення backup", &error.to_string());
                    }
                }
            });
        }
    });

    ui.on_settings_backup_download(|| {
        tokio::spawn(async move {
            match open_latest_backup().await {
                Ok(path) => {
                    notify_user("Відкрито резервну копію", &path.display().to_string());
                }
                Err(error) => {
                    tracing::error!("settings: backup open failed: {error}");
                    notify_user("Помилка відкриття backup", &error.to_string());
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn company_to_info_maps_optional_fields_to_empty_string() {
        let company = Company {
            id: Uuid::new_v4(),
            name: "ТОВ Тест".into(),
            short_name: None,
            edrpou: Some("12345678".into()),
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let info = company_to_info(&company);
        assert_eq!(info.full_name.as_str(), "ТОВ Тест");
        assert_eq!(info.edrpou.as_str(), "12345678");
        assert_eq!(info.short_name.as_str(), "");
    }
}
