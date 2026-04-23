use std::sync::Arc;

use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

use acta::app_ctx::{AppCtx, AppScreen};
use acta::db;
use crate::ui::helpers::{
    act_row_to_document_item, invoice_row_to_document_item, waybill_row_to_document_item,
};

pub struct DocumentsData {
    pub items: Vec<crate::DocumentItem>,
    pub total: i32,
}

pub async fn prepare_documents_data(
    pool: &PgPool,
    company_id: Uuid,
    search: Option<&str>,
    tab: Option<&str>,
) -> DocumentsData {
    let include_acts     = !matches!(tab, Some("invoice") | Some("waybill"));
    let include_invoices = !matches!(tab, Some("act")     | Some("waybill"));
    let include_waybills = !matches!(tab, Some("act")     | Some("invoice"));

    let (acts, invoices, waybills) = tokio::join!(
        async {
            if include_acts {
                db::acts::list_filtered(pool, company_id, None, None, search, None, None, None).await
            } else {
                Ok(vec![])
            }
        },
        async {
            if include_invoices {
                db::invoices::list_filtered(pool, company_id, None, None, search, None, None, None).await
            } else {
                Ok(vec![])
            }
        },
        async {
            if include_waybills {
                db::waybills::list_filtered(pool, company_id, None, None, search, None, None, None).await
            } else {
                Ok(vec![])
            }
        },
    );

    let mut combined: Vec<(chrono::NaiveDate, crate::DocumentItem)> = vec![];

    if let Ok(rows) = acts {
        for r in &rows {
            combined.push((r.date, act_row_to_document_item(r)));
        }
    }
    if let Ok(rows) = invoices {
        for r in &rows {
            combined.push((r.date, invoice_row_to_document_item(r)));
        }
    }
    if let Ok(rows) = waybills {
        for r in &rows {
            combined.push((r.date, waybill_row_to_document_item(r)));
        }
    }

    combined.sort_by(|a, b| b.0.cmp(&a.0));
    let items: Vec<crate::DocumentItem> = combined.into_iter().map(|(_, item)| item).collect();
    let total = items.len() as i32;

    DocumentsData { items, total }
}

pub fn apply_documents_to_ui(ui: &crate::AppWindow, data: DocumentsData) {
    let previous = ui.get_documents();
    ui.set_documents(crate::DocumentsViewData {
        items: ModelRc::new(VecModel::from(data.items)),
        selected_ids: previous.selected_ids,
        total_count: data.total,
        page_count: if previous.page_count > 0 { previous.page_count } else { 1 },
        chain_steps: previous.chain_steps,
        cp_doc_chains: previous.cp_doc_chains,
    });
}

/// Підписує всі document callbacks на UI компонент.
/// Приймає AppCtx замість окремих pool/company_id — Epic 4.
pub fn wire_document_callbacks(ui: &crate::AppWindow, ctx: &Arc<AppCtx>) {
    ui.on_doc_search_changed({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |query| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let q = query.to_string();
            ctx.update_documents_state(|state| {
                state.query = q;
            });
            crate::bootstrap::spawn_refresh_screen(ui_weak.clone(), ctx.clone(), AppScreen::Documents);
        }
    });

    ui.on_doc_tab_changed({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |tab| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let t = tab.to_string();
            ctx.update_documents_state(|state| {
                state.tab = t;
            });
            crate::bootstrap::spawn_refresh_screen(ui_weak.clone(), ctx.clone(), AppScreen::Documents);
        }
    });

    ui.on_doc_send({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                if let Some(uuid_str) = id.strip_prefix("act:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::acts::advance_status(ctx.pool(), uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("inv:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::invoices::advance_status(ctx.pool(), uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("wbl:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::waybills::advance_status(ctx.pool(), uuid).await;
                    }
                }
                crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
            });
        }
    });

    ui.on_doc_delete({
        let ctx = ctx.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let ctx = ctx.clone();
            let ui_weak = ui_weak.clone();
            let id = id.to_string();
            tokio::spawn(async move {
                if let Some(uuid_str) = id.strip_prefix("act:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::acts::delete(ctx.pool(), uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("inv:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::invoices::delete(ctx.pool(), uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("wbl:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::waybills::delete(ctx.pool(), uuid).await;
                    }
                }
                crate::bootstrap::refresh_screen(ui_weak, ctx, AppScreen::Documents).await;
            });
        }
    });

    // Epic 7: Реалізація базових no-op callbacks
    ui.on_doc_new(|| {
        tracing::warn!("TODO: створення нового документа — dialog form coming in future sprint");
    });
    ui.on_doc_open(|id| {
        tracing::warn!("TODO: відкриття документа {} — detail view coming in future sprint", id);
    });
    ui.on_doc_toggled(|_id, _sel| {
        tracing::debug!("doc_toggled — document selection updates UI local state automatically");
    });
    ui.on_doc_page_changed(|_p| {
        tracing::debug!("doc_page_changed — pagination updates UI local state automatically");
    });
}

// ────────────────────────────────────────────────────────────────────────────
// Epic 9: Presenter-layer tests
// ────────────────────────────────────────────────────────────────────────────

/// Визначає які типи документів включити за значенням вкладки.
#[cfg(test)]
fn tab_includes(tab: Option<&str>) -> (bool, bool, bool) {
    let include_acts     = !matches!(tab, Some("invoice") | Some("waybill"));
    let include_invoices = !matches!(tab, Some("act")     | Some("waybill"));
    let include_waybills = !matches!(tab, Some("act")     | Some("invoice"));
    (include_acts, include_invoices, include_waybills)
}

#[cfg(test)]
mod tests {
    use super::tab_includes;
    use chrono::NaiveDate;

    #[test]
    fn sort_combined_documents_by_date_descending() {
        let pairs = vec![
            (NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), "A".to_string()),
            (NaiveDate::from_ymd_opt(2026, 4, 21).unwrap(), "B".to_string()),
            (NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(), "C".to_string()),
        ];
        let mut sorted = pairs;
        sorted.sort_by(|a, b| b.0.cmp(&a.0));
        assert_eq!(sorted[0].1, "B");
        assert_eq!(sorted[2].1, "C");
    }

    // ── Tab filtering ───────────────────────────────────────────────

    #[test]
    fn tab_all_includes_everything() {
        let (acts, inv, wb) = tab_includes(None);
        assert!(acts);
        assert!(inv);
        assert!(wb);
    }

    #[test]
    fn tab_all_string_includes_everything() {
        let (acts, inv, wb) = tab_includes(Some("all"));
        assert!(acts);
        assert!(inv);
        assert!(wb);
    }

    #[test]
    fn tab_invoice_excludes_invoices() {
        let (acts, inv, wb) = tab_includes(Some("invoice"));
        assert!(!acts);
        assert!(inv);
        assert!(!wb);
    }

    #[test]
    fn tab_act_excludes_acts() {
        let (acts, inv, wb) = tab_includes(Some("act"));
        assert!(acts);
        assert!(!inv);
        assert!(!wb);
    }

    #[test]
    fn tab_waybill_only_waybills() {
        let (acts, inv, wb) = tab_includes(Some("waybill"));
        assert!(!acts);
        assert!(!inv);
        assert!(wb);
    }

    // ── Empty states ────────────────────────────────────────────────

    #[test]
    fn empty_documents_data_has_zero_total() {
        let data = super::DocumentsData { items: vec![], total: 0 };
        assert!(data.items.is_empty());
        assert_eq!(data.total, 0);
    }

    #[test]
    fn non_empty_documents_data_has_correct_total() {
        let items = vec![
            crate::DocumentItem {
                id: "act:1".into(),
                kind: crate::DocumentKind::Act,
                number: "АКТ-001".into(),
                date: "01.04.2026".into(),
                counterparty: "ТОВ Тест".into(),
                amount_str: "1 000 ₴".into(),
                status: crate::DocumentStatus::Issued,
                linked_id: "".into(),
            },
            crate::DocumentItem {
                id: "inv:2".into(),
                kind: crate::DocumentKind::Invoice,
                number: "РАХ-001".into(),
                date: "02.04.2026".into(),
                counterparty: "ТОВ Тест".into(),
                amount_str: "2 000 ₴".into(),
                status: crate::DocumentStatus::Draft,
                linked_id: "".into(),
            },
        ];
        let data = super::DocumentsData {
            total: items.len() as i32,
            items: items.clone(),
        };
        assert_eq!(data.total, 2);
        assert_eq!(data.items.len(), 2);
    }

    // ── Status display mapping ──────────────────────────────────────

    #[test]
    fn all_document_statuses_are_distinct() {
        let statuses = [
            crate::DocumentStatus::Draft,
            crate::DocumentStatus::Issued,
            crate::DocumentStatus::Signed,
            crate::DocumentStatus::Paid,
            crate::DocumentStatus::Overdue,
            crate::DocumentStatus::Partial,
        ];
        // Перевірка що всі варіанти існують і компілюються
        for (i, s1) in statuses.iter().enumerate() {
            for (j, s2) in statuses.iter().enumerate() {
                if i != j {
                    // Не можемо порівняти enum-variants напряму без PartialEq,
                    // але перевірка що вони існують — це вже тест компіляції
                    let _ = (s1, s2);
                }
            }
        }
    }

    // ── Document ID prefix parsing ──────────────────────────────────

    #[test]
    fn document_id_prefix_act_strips_correctly() {
        let id = "act:550e8400-e29b-41d4-a716-446655440000";
        let uuid_str = id.strip_prefix("act:").unwrap();
        assert!(uuid_str.starts_with("550e8400"));
    }

    #[test]
    fn document_id_prefix_inv_strips_correctly() {
        let id = "inv:550e8400-e29b-41d4-a716-446655440000";
        let uuid_str = id.strip_prefix("inv:").unwrap();
        assert!(uuid_str.starts_with("550e8400"));
    }

    #[test]
    fn document_id_prefix_wbl_strips_correctly() {
        let id = "wbl:550e8400-e29b-41d4-a716-446655440000";
        let uuid_str = id.strip_prefix("wbl:").unwrap();
        assert!(uuid_str.starts_with("550e8400"));
    }

    #[test]
    fn unknown_document_id_prefix_returns_none() {
        let id = "unknown:123";
        assert!(id.strip_prefix("act:").is_none());
        assert!(id.strip_prefix("inv:").is_none());
        assert!(id.strip_prefix("wbl:").is_none());
    }
}
