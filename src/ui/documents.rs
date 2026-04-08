// ui/documents.rs — колбеки та дані для сторінки Документи (акти + накладні).

use std::sync::Arc;

use anyhow::Result;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

use crate::{
    app_ctx::AppCtx,
    ui::helpers::*,
    DocRow, MainWindow,
};
use acta::{db, models};

// ═══════════════════════════════════════════════════════════════════════════════
// ── Проміжні дані ──────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct DocumentsUiData {
    pub doc_rows: Vec<DocRow>,
    pub tab: i32,
    pub direction_index: i32,
}

pub async fn prepare_documents_data(
    pool: &sqlx::PgPool,
    company_id: uuid::Uuid,
    tab: i32,
    direction: &str,
    query: &str,
    counterparty_id: Option<uuid::Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
) -> Result<DocumentsUiData> {
    let search = if query.trim().is_empty() { None } else { Some(query) };

    let (acts_res, invs_res) = tokio::join!(
        async {
            if tab != 2 {
                db::acts::list_filtered(
                    pool,
                    company_id,
                    None,
                    Some(direction),
                    search,
                    counterparty_id,
                    date_from,
                    date_to,
                )
                .await
            } else {
                Ok(vec![])
            }
        },
        async {
            if tab != 1 {
                db::invoices::list_filtered(
                    pool,
                    company_id,
                    None,
                    Some(direction),
                    search,
                    counterparty_id,
                    date_from,
                    date_to,
                )
                .await
            } else {
                Ok(vec![])
            }
        }
    );
    let acts = acts_res?;
    let invs = invs_res?;

    let mut combined: Vec<(chrono::NaiveDate, DocRow)> =
        Vec::with_capacity(acts.len() + invs.len());
    for a in &acts {
        combined.push((
            a.date,
            DocRow {
                id: SharedString::from(format!("act:{}", a.id)),
                doc_type: SharedString::from("АКТ"),
                number: SharedString::from(a.number.as_str()),
                counterparty: SharedString::from(a.counterparty_name.as_str()),
                amount: SharedString::from(format!(
                    "{} ₴",
                    format_amount_ua(a.total_amount)
                )),
                date: SharedString::from(a.date.format("%d.%m.%Y").to_string()),
                status: SharedString::from(match a.status {
                    models::ActStatus::Draft => "Чернетка",
                    models::ActStatus::Issued => "Виставлений",
                    models::ActStatus::Signed => "Підписаний",
                    models::ActStatus::Paid => "Оплачений",
                }),
            },
        ));
    }
    for i in &invs {
        combined.push((
            i.date,
            DocRow {
                id: SharedString::from(format!("inv:{}", i.id)),
                doc_type: SharedString::from("НАК"),
                number: SharedString::from(i.number.as_str()),
                counterparty: SharedString::from(i.counterparty_name.as_str()),
                amount: SharedString::from(format!(
                    "{} ₴",
                    format_amount_ua(i.total_amount)
                )),
                date: SharedString::from(i.date.format("%d.%m.%Y").to_string()),
                status: SharedString::from(match i.status {
                    models::InvoiceStatus::Draft => "Чернетка",
                    models::InvoiceStatus::Issued => "Виставлений",
                    models::InvoiceStatus::Signed => "Підписаний",
                    models::InvoiceStatus::Paid => "Оплачений",
                }),
            },
        ));
    }
    combined.sort_by(|(da, _), (db, _)| db.cmp(da));
    let doc_rows: Vec<DocRow> = combined.into_iter().map(|(_, r)| r).collect();

    Ok(DocumentsUiData {
        doc_rows,
        tab,
        direction_index: doc_direction_index(direction),
    })
}

pub fn apply_documents_to_ui(ui: &MainWindow, d: DocumentsUiData) {
    ui.set_document_rows(ModelRc::new(VecModel::from(d.doc_rows)));
    ui.set_doc_active_tab(d.tab);
    ui.set_doc_direction_index(d.direction_index);
    ui.set_documents_loading(false);
}

pub async fn reload_documents(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    tab: i32,
    direction: &str,
    query: &str,
    counterparty_id: Option<uuid::Uuid>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
) -> Result<()> {
    let d = prepare_documents_data(
        pool,
        company_id,
        tab,
        direction,
        query,
        counterparty_id,
        date_from,
        date_to,
    )
    .await?;
    ui_weak
        .upgrade_in_event_loop(move |ui| apply_documents_to_ui(&ui, d))
        .map_err(anyhow::Error::from)
}

/// Дані фільтру контрагентів для списку документів.
#[derive(Clone)]
pub struct DocCpFilterData {
    /// UUID-и контрагентів (без елемента "Всі контрагенти" на позиції 0).
    pub cp_ids: Vec<uuid::Uuid>,
    /// Назви з "Всі контрагенти" на позиції 0.
    pub names: Vec<SharedString>,
}

pub async fn fetch_doc_cp_filter_data(
    pool: &sqlx::PgPool,
    company_id: uuid::Uuid,
) -> Result<DocCpFilterData> {
    let cps = db::acts::counterparties_for_select(pool, company_id).await?;
    let cp_ids: Vec<uuid::Uuid> = cps.iter().map(|(id, _)| *id).collect();
    let mut names: Vec<SharedString> = vec![SharedString::from("Всі контрагенти")];
    names.extend(cps.iter().map(|(_, n)| SharedString::from(n.as_str())));
    Ok(DocCpFilterData { cp_ids, names })
}

pub async fn reload_doc_cp_filter(
    pool: &sqlx::PgPool,
    ui_weak: Weak<MainWindow>,
    company_id: uuid::Uuid,
    doc_cp_ids: &std::sync::Mutex<Vec<uuid::Uuid>>,
) -> Result<()> {
    let data = fetch_doc_cp_filter_data(pool, company_id).await?;
    {
        let mut ids = doc_cp_ids.lock().unwrap();
        *ids = data.cp_ids;
    }
    let names = data.names;
    ui_weak
        .upgrade_in_event_loop(move |ui| {
            ui.set_doc_filter_cp_names(ModelRc::new(VecModel::from(names)));
        })
        .map_err(anyhow::Error::from)
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── setup — реєстрація колбеків ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

pub fn setup(ui: &MainWindow, ctx: Arc<AppCtx>) {
    // ── Зміна таба ────────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = ctx.doc_state.clone();
    ui.on_doc_tab_changed(move |tab| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let (query, direction, cp_id, df, dt) = {
            let mut s = state.lock().unwrap();
            s.tab = tab;
            (s.query.clone(), s.direction.clone(), s.counterparty_id, s.date_from, s.date_to)
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_doc_active_tab(tab);
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) =
                reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt)
                    .await
            {
                tracing::error!("Помилка фільтру документів за табом: {e}");
            }
        });
    });

    // ── Зміна напрямку ────────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = ctx.doc_state.clone();
    ui.on_doc_direction_changed(move |index| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let (tab, direction, query, cp_id, df, dt) = {
            let mut s = state.lock().unwrap();
            s.direction = doc_direction_from_index(index).to_string();
            (
                s.tab,
                s.direction.clone(),
                s.query.clone(),
                s.counterparty_id,
                s.date_from,
                s.date_to,
            )
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_doc_direction_index(index);
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) =
                reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt)
                    .await
            {
                tracing::error!("Помилка фільтру документів за напрямком: {e}");
            }
        });
    });

    // ── Текстовий пошук ───────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = ctx.doc_state.clone();
    ui.on_doc_search_changed(move |q| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let (tab, direction, query, cp_id, df, dt) = {
            let mut s = state.lock().unwrap();
            s.query = q.to_string();
            (
                s.tab,
                s.direction.clone(),
                s.query.clone(),
                s.counterparty_id,
                s.date_from,
                s.date_to,
            )
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) =
                reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt)
                    .await
            {
                tracing::error!("Помилка пошуку документів: {e}");
            }
        });
    });

    // ── Новий акт ─────────────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_doc_new_act_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.invoke_act_create_clicked();
        }
    });

    // ── Нова накладна ─────────────────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_doc_new_invoice_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.invoke_invoice_create_clicked();
        }
    });

    // ── Фільтр за контрагентом ────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = ctx.doc_state.clone();
    let doc_cp_ids = ctx.doc_cp_ids.clone();
    ui.on_doc_cp_filter_changed(move |idx| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let cp_id = if idx <= 0 {
            None
        } else {
            let ids = doc_cp_ids.lock().unwrap();
            ids.get(idx as usize - 1).copied()
        };
        let (tab, direction, query, df, dt) = {
            let mut s = state.lock().unwrap();
            s.counterparty_id = cp_id;
            s.counterparty_index = idx;
            (
                s.tab,
                s.direction.clone(),
                s.query.clone(),
                s.date_from,
                s.date_to,
            )
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_doc_filter_cp_index(idx);
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) =
                reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt)
                    .await
            {
                tracing::error!("Помилка фільтру документів за контрагентом: {e}");
            }
        });
    });

    // ── Фільтр за датою від ───────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = ctx.doc_state.clone();
    ui.on_doc_date_from_changed(move |text| {
        let df = if text.len() == 10 {
            chrono::NaiveDate::parse_from_str(text.as_str(), "%d.%m.%Y").ok()
        } else if text.is_empty() {
            None
        } else {
            return;
        };
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let (tab, direction, query, cp_id, dt) = {
            let mut s = state.lock().unwrap();
            s.date_from = df;
            (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_to)
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) =
                reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt)
                    .await
            {
                tracing::error!("Помилка фільтру документів за датою від: {e}");
            }
        });
    });

    // ── Фільтр за датою до ────────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = ctx.doc_state.clone();
    ui.on_doc_date_to_changed(move |text| {
        let dt = if text.len() == 10 {
            chrono::NaiveDate::parse_from_str(text.as_str(), "%d.%m.%Y").ok()
        } else if text.is_empty() {
            None
        } else {
            return;
        };
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let (tab, direction, query, cp_id, df) = {
            let mut s = state.lock().unwrap();
            s.date_to = dt;
            (s.tab, s.direction.clone(), s.query.clone(), s.counterparty_id, s.date_from)
        };
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_documents_loading(true);
        }
        tokio::spawn(async move {
            if let Err(e) =
                reload_documents(&pool, ui_weak, cid, tab, &direction, &query, cp_id, df, dt)
                    .await
            {
                tracing::error!("Помилка фільтру документів за датою до: {e}");
            }
        });
    });

    // ── Відкрити документ для редагування ────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_doc_open_clicked(move |id| {
        let id_s = id.to_string();
        if let Some(ui) = ui_weak.upgrade() {
            if let Some(act_uuid) = id_s.strip_prefix("act:") {
                ui.invoke_act_edit_clicked(SharedString::from(act_uuid));
            } else if let Some(inv_uuid) = id_s.strip_prefix("inv:") {
                ui.invoke_invoice_edit_clicked(SharedString::from(inv_uuid));
            } else {
                tracing::warn!("doc-open-clicked: невідомий префікс id='{id_s}'");
            }
        }
    });

    // ── Генерація PDF документу ───────────────────────────────────────────────
    let ui_weak = ui.as_weak();
    ui.on_doc_pdf_clicked(move |id| {
        let id_s = id.to_string();
        if let Some(ui) = ui_weak.upgrade() {
            if let Some(act_uuid) = id_s.strip_prefix("act:") {
                ui.invoke_act_pdf_clicked(SharedString::from(act_uuid));
            } else {
                tracing::info!("PDF для накладних ще не реалізовано (id='{id_s}')");
            }
        }
    });

    // ── Видалення документу ───────────────────────────────────────────────────
    let pool = ctx.pool.clone();
    let ui_weak = ui.as_weak();
    let company_id_arc = ctx.active_company_id.clone();
    let state = ctx.doc_state.clone();
    ui.on_doc_delete_clicked(move |id| {
        let pool = pool.clone();
        let ui_weak = ui_weak.clone();
        let cid = *company_id_arc.lock().unwrap();
        let id_s = id.to_string();
        let (tab, direction, query, cp_id, df, dt) = {
            let s = state.lock().unwrap();
            (
                s.tab,
                s.direction.clone(),
                s.query.clone(),
                s.counterparty_id,
                s.date_from,
                s.date_to,
            )
        };
        tokio::spawn(async move {
            let result = if let Some(act_uuid_s) = id_s.strip_prefix("act:") {
                let Ok(uuid) = act_uuid_s.parse::<uuid::Uuid>() else {
                    tracing::error!("Невалідний UUID акту: {act_uuid_s}");
                    return;
                };
                db::acts::delete(&pool, uuid).await
            } else if let Some(inv_uuid_s) = id_s.strip_prefix("inv:") {
                let Ok(uuid) = inv_uuid_s.parse::<uuid::Uuid>() else {
                    tracing::error!("Невалідний UUID накладної: {inv_uuid_s}");
                    return;
                };
                db::invoices::delete(&pool, uuid).await
            } else {
                tracing::warn!("doc-delete-clicked: невідомий префікс id='{id_s}'");
                return;
            };
            match result {
                Ok(_) => {
                    tracing::info!("Документ '{id_s}' видалено.");
                    if let Err(e) = reload_documents(
                        &pool,
                        ui_weak,
                        cid,
                        tab,
                        &direction,
                        &query,
                        cp_id,
                        df,
                        dt,
                    )
                    .await
                    {
                        tracing::error!("Помилка оновлення документів після видалення: {e}");
                    }
                }
                Err(e) => tracing::error!("Помилка видалення документу '{id_s}': {e}"),
            }
        });
    });
}
