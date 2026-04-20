// ui/command_palette.rs — Command Palette (Ctrl+K): пошук та обробка вибору
//
// reload_palette — запускає пошук у БД та відправляє результати в UI.
// handle_item_selected — обробляє вибір елемента (навігація, створити, відкрити).

use std::sync::{Arc, Mutex};

use slint::{ModelRc, SharedString, VecModel, Weak};
use uuid::Uuid;

use acta::db;
use crate::{CommandPaletteItem, MainWindow};

// ── Завантаження результатів ──────────────────────────────────────────────────

pub fn reload_palette(
    ui: Weak<MainWindow>,
    pool: Arc<sqlx::PgPool>,
    company_id: Uuid,
    query: String,
) {
    tokio::spawn(async move {
        let items = db::search::search(&pool, company_id, &query)
            .await
            .unwrap_or_default();

        let slint_items: Vec<CommandPaletteItem> = items
            .into_iter()
            .map(|r| CommandPaletteItem {
                kind:     SharedString::from(r.kind),
                action:   SharedString::from(r.action),
                id:       SharedString::from(r.id),
                title:    SharedString::from(r.title),
                subtitle: SharedString::from(r.subtitle),
                shortcut: SharedString::from(r.shortcut),
            })
            .collect();

        let _ = ui.upgrade_in_event_loop(move |ui| {
            ui.set_command_palette_items(
                ModelRc::new(VecModel::from(slint_items))
            );
        });
    });
}

// ── Обробка вибору елемента ───────────────────────────────────────────────────
// Ця функція викликається вже всередині Slint event loop (з on_command_palette_item_selected),
// тому ui — вже живий, і invoke_navigate_to викликається напряму.

pub fn handle_item_selected(
    ui: &MainWindow,
    pool: Arc<sqlx::PgPool>,
    active_company_id: Arc<Mutex<Uuid>>,
    action: &str,
    id: &str,
) {
    match action {
        "navigate" => {
            ui.invoke_navigate_to(SharedString::from(id));
        }
        "create" => {
            handle_create(ui, pool, active_company_id, id);
        }
        "open_doc" => {
            ui.invoke_navigate_to(SharedString::from("documents"));
        }
        "open_cp" => {
            ui.invoke_navigate_to(SharedString::from("counterparties"));
        }
        _ => {}
    }
}

fn handle_create(
    ui: &MainWindow,
    _pool: Arc<sqlx::PgPool>,
    _active_company_id: Arc<Mutex<Uuid>>,
    doc_type: &str,
) {
    match doc_type {
        "act" => {
            ui.invoke_navigate_to(SharedString::from("acts"));
            ui.invoke_act_create_clicked();
        }
        "invoice" => {
            ui.invoke_navigate_to(SharedString::from("invoices"));
            ui.invoke_invoice_create_clicked();
        }
        "waybill" => {
            ui.invoke_navigate_to(SharedString::from("waybills"));
            ui.invoke_waybill_create_clicked();
        }
        "counterparty" => {
            ui.invoke_navigate_to(SharedString::from("counterparties"));
            ui.invoke_counterparty_create_clicked();
        }
        _ => {}
    }
}
