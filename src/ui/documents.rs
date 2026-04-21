use std::sync::{Arc, Mutex};

use slint::{ComponentHandle, ModelRc, VecModel};
use sqlx::PgPool;
use uuid::Uuid;

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
    ui.set_docs_total_count(data.total);
    ui.set_all_documents(ModelRc::new(VecModel::from(data.items)));
}

pub fn wire_document_callbacks(
    ui: &crate::AppWindow,
    pool: &Arc<PgPool>,
    company_id: &Arc<Mutex<Uuid>>,
) {
    ui.on_doc_search_changed({
        let pool = pool.clone();
        let ui_weak = ui.as_weak();
        let company_id = company_id.clone();
        move |query| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let cid = *company_id.lock().unwrap();
            let q = query.to_string();
            tokio::spawn(async move {
                let search = if q.is_empty() { None } else { Some(q.as_str()) };
                let data = prepare_documents_data(&pool, cid, search, None).await;
                let _ = ui_weak.upgrade_in_event_loop(move |ui| apply_documents_to_ui(&ui, data));
            });
        }
    });

    ui.on_doc_tab_changed({
        let pool = pool.clone();
        let ui_weak = ui.as_weak();
        let company_id = company_id.clone();
        move |tab| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let cid = *company_id.lock().unwrap();
            let t = tab.to_string();
            tokio::spawn(async move {
                let tab_opt = if t == "all" { None } else { Some(t.as_str()) };
                let data = prepare_documents_data(&pool, cid, None, tab_opt).await;
                let _ = ui_weak.upgrade_in_event_loop(move |ui| apply_documents_to_ui(&ui, data));
            });
        }
    });

    ui.on_doc_send({
        let pool = pool.clone();
        let ui_weak = ui.as_weak();
        let company_id = company_id.clone();
        move |id| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let cid = *company_id.lock().unwrap();
            let id = id.to_string();
            tokio::spawn(async move {
                if let Some(uuid_str) = id.strip_prefix("act:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::acts::advance_status(&pool, uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("inv:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::invoices::advance_status(&pool, uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("wbl:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::waybills::advance_status(&pool, uuid).await;
                    }
                }
                let data = prepare_documents_data(&pool, cid, None, None).await;
                let _ = ui_weak.upgrade_in_event_loop(move |ui| apply_documents_to_ui(&ui, data));
            });
        }
    });

    ui.on_doc_delete({
        let pool = pool.clone();
        let ui_weak = ui.as_weak();
        let company_id = company_id.clone();
        move |id| {
            let pool = pool.clone();
            let ui_weak = ui_weak.clone();
            let cid = *company_id.lock().unwrap();
            let id = id.to_string();
            tokio::spawn(async move {
                if let Some(uuid_str) = id.strip_prefix("act:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::acts::delete(&pool, uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("inv:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::invoices::delete(&pool, uuid).await;
                    }
                } else if let Some(uuid_str) = id.strip_prefix("wbl:") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = db::waybills::delete(&pool, uuid).await;
                    }
                }
                let data = prepare_documents_data(&pool, cid, None, None).await;
                let _ = ui_weak.upgrade_in_event_loop(move |ui| apply_documents_to_ui(&ui, data));
            });
        }
    });

    ui.on_doc_new(|| {});
    ui.on_doc_open(|_| {});
    ui.on_doc_toggled(|_, _| {});
    ui.on_doc_page_changed(|_| {});
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_combined_documents_by_date_descending() {
        use chrono::NaiveDate;
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
}
