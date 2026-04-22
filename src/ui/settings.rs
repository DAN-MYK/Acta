use std::sync::Arc;

use slint::{ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use acta::app_ctx::AppCtx;
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

fn default_integrations() -> Vec<crate::IntegrationItem> {
    vec![
        crate::IntegrationItem {
            label: "BAS".into(),
            description: "Імпорт документів та довідників".into(),
            tag: "bas".into(),
            enabled: false,
        },
        crate::IntegrationItem {
            label: "Банк".into(),
            description: "Синхронізація виписок та звірка платежів".into(),
            tag: "bank".into(),
            enabled: false,
        },
    ]
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

pub async fn prepare_settings_data(pool: &PgPool, company_id: Uuid) -> SettingsData {
    let company_info = db::companies::get_by_id(pool, company_id)
        .await
        .ok()
        .flatten()
        .map(|company| company_to_info(&company))
        .unwrap_or_else(default_company_info);

    SettingsData {
        company_info,
        integrations: default_integrations(),
        team_members: vec![],
        numbering_rows: default_numbering_rows(),
        last_backup_label: "Ще не створювався".to_string(),
        last_backup_file: "Локальний бекап не знайдено".to_string(),
    }
}

pub fn apply_settings_to_ui(ui: &crate::AppWindow, data: SettingsData) {
    ui.set_company_info(data.company_info);
    ui.set_integrations(ModelRc::new(VecModel::from(data.integrations)));
    ui.set_team_members(ModelRc::new(VecModel::from(data.team_members)));
    ui.set_numbering_rows(ModelRc::new(VecModel::from(data.numbering_rows)));
    ui.set_last_backup_label(data.last_backup_label.into());
    ui.set_last_backup_file(data.last_backup_file.into());
}

pub fn wire_settings_callbacks(ui: &crate::AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_settings_company_saved({
        let ctx = ctx.clone();
        move |info| {
            let ctx = ctx.clone();
            let update = info_to_update(&info);

            tokio::spawn(async move {
                let company_id = ctx.company_id();
                match db::companies::update(ctx.pool(), company_id, &update).await {
                    Ok(Some(_)) => tracing::info!("settings: company saved"),
                    Ok(None) => tracing::warn!("settings: company not found id={company_id}"),
                    Err(error) => tracing::error!("settings: save failed: {error}"),
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
    ui.on_settings_integration_configure(|integration| {
        tracing::info!("TODO: settings_integration_configure({integration})");
    });
    ui.on_settings_team_invite(|| tracing::info!("TODO: settings_team_invite"));
    ui.on_settings_backup_now(|| tracing::info!("TODO: settings_backup_now"));
    ui.on_settings_backup_download(|| tracing::info!("TODO: settings_backup_download"));
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
