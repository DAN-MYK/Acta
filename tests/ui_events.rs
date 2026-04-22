// Epic 2: UI test safety net
//
// Headless-тести для callback-контракту канонічного `ui-redesign`.
// Перевіряємо лише те, що Slint події викликаються і передають очікувані значення.

slint::include_modules!();

use slint::SharedString;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn ui_event_handlers() {
    i_slint_backend_testing::init_no_event_loop();

    navigation();
    documents();
    counterparties();
    payments();
    reports();
    tasks();
    settings();
    command_palette();
}

fn create_window() -> AppWindow {
    AppWindow::new().expect("AppWindow має створюватись")
}

fn capture_string() -> Rc<RefCell<SharedString>> {
    Rc::new(RefCell::new(SharedString::default()))
}

fn navigation() {
    let ui = create_window();
    let fired = Rc::new(Cell::new(false));
    let screen = Rc::new(Cell::new(NavScreen::Dashboard));

    {
        let fired = fired.clone();
        let screen_capture = screen.clone();
        ui.on_nav_changed(move |value| {
            fired.set(true);
            screen_capture.set(value);
        });
    }

    ui.invoke_nav_changed(NavScreen::Reports);

    assert!(fired.get(), "nav: callback має викликатись");
    assert_eq!(screen.get(), NavScreen::Reports, "nav: екран має передаватись");
}

fn documents() {
    let ui = create_window();

    let fired = Rc::new(Cell::new(false));
    let query = capture_string();
    {
        let fired = fired.clone();
        let query_capture = query.clone();
        ui.on_doc_search_changed(move |value| {
            fired.set(true);
            *query_capture.borrow_mut() = value;
        });
    }
    ui.invoke_doc_search_changed("ТОВ Тест".into());
    assert!(fired.get(), "doc: search-changed");
    assert_eq!(query.borrow().as_str(), "ТОВ Тест");

    let fired = Rc::new(Cell::new(false));
    let tab = capture_string();
    {
        let fired = fired.clone();
        let tab_capture = tab.clone();
        ui.on_doc_tab_changed(move |value| {
            fired.set(true);
            *tab_capture.borrow_mut() = value;
        });
    }
    ui.invoke_doc_tab_changed("invoice".into());
    assert!(fired.get(), "doc: tab-changed");
    assert_eq!(tab.borrow().as_str(), "invoice");

    let fired = Rc::new(Cell::new(false));
    let doc_id = capture_string();
    let selected = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        let doc_id_capture = doc_id.clone();
        let selected_capture = selected.clone();
        ui.on_doc_toggled(move |id, is_selected| {
            fired.set(true);
            *doc_id_capture.borrow_mut() = id;
            selected_capture.set(is_selected);
        });
    }
    ui.invoke_doc_toggled("act:uuid-123".into(), true);
    assert!(fired.get(), "doc: toggled");
    assert_eq!(doc_id.borrow().as_str(), "act:uuid-123");
    assert!(selected.get());

    let fired = Rc::new(Cell::new(false));
    let doc_id = capture_string();
    {
        let fired = fired.clone();
        let doc_id_capture = doc_id.clone();
        ui.on_doc_open(move |id| {
            fired.set(true);
            *doc_id_capture.borrow_mut() = id;
        });
    }
    ui.invoke_doc_open("act:uuid-456".into());
    assert!(fired.get(), "doc: open");
    assert_eq!(doc_id.borrow().as_str(), "act:uuid-456");

    let fired = Rc::new(Cell::new(false));
    let doc_id = capture_string();
    {
        let fired = fired.clone();
        let doc_id_capture = doc_id.clone();
        ui.on_doc_send(move |id| {
            fired.set(true);
            *doc_id_capture.borrow_mut() = id;
        });
    }
    ui.invoke_doc_send("inv:uuid-789".into());
    assert!(fired.get(), "doc: send");
    assert_eq!(doc_id.borrow().as_str(), "inv:uuid-789");

    let fired = Rc::new(Cell::new(false));
    let doc_id = capture_string();
    {
        let fired = fired.clone();
        let doc_id_capture = doc_id.clone();
        ui.on_doc_delete(move |id| {
            fired.set(true);
            *doc_id_capture.borrow_mut() = id;
        });
    }
    ui.invoke_doc_delete("wbl:uuid-000".into());
    assert!(fired.get(), "doc: delete");
    assert_eq!(doc_id.borrow().as_str(), "wbl:uuid-000");

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_doc_new(move || fired.set(true));
    }
    ui.invoke_doc_new();
    assert!(fired.get(), "doc: new");

    let fired = Rc::new(Cell::new(false));
    let page = Rc::new(Cell::new(0i32));
    {
        let fired = fired.clone();
        let page_capture = page.clone();
        ui.on_doc_page_changed(move |value| {
            fired.set(true);
            page_capture.set(value);
        });
    }
    ui.invoke_doc_page_changed(3);
    assert!(fired.get(), "doc: page-changed");
    assert_eq!(page.get(), 3);

    let fired = Rc::new(Cell::new(false));
    let doc_id = capture_string();
    {
        let fired = fired.clone();
        let doc_id_capture = doc_id.clone();
        ui.on_doc_chain_load(move |id| {
            fired.set(true);
            *doc_id_capture.borrow_mut() = id;
        });
    }
    ui.invoke_doc_chain_load("act:uuid-chain".into());
    assert!(fired.get(), "doc: chain-load");
    assert_eq!(doc_id.borrow().as_str(), "act:uuid-chain");

    let fired = Rc::new(Cell::new(false));
    let doc_type = capture_string();
    let doc_id = capture_string();
    {
        let fired = fired.clone();
        let doc_type_capture = doc_type.clone();
        let doc_id_capture = doc_id.clone();
        ui.on_doc_chain_create(move |kind, id| {
            fired.set(true);
            *doc_type_capture.borrow_mut() = kind;
            *doc_id_capture.borrow_mut() = id;
        });
    }
    ui.invoke_doc_chain_create("act".into(), "inv:uuid-src".into());
    assert!(fired.get(), "doc: chain-create");
    assert_eq!(doc_type.borrow().as_str(), "act");
    assert_eq!(doc_id.borrow().as_str(), "inv:uuid-src");
}

fn counterparties() {
    let ui = create_window();

    let fired = Rc::new(Cell::new(false));
    let cp_id = capture_string();
    {
        let fired = fired.clone();
        let cp_id_capture = cp_id.clone();
        ui.on_cp_selected(move |id| {
            fired.set(true);
            *cp_id_capture.borrow_mut() = id;
        });
    }
    ui.invoke_cp_selected("cp-uuid-001".into());
    assert!(fired.get(), "cp: selected");
    assert_eq!(cp_id.borrow().as_str(), "cp-uuid-001");

    let fired = Rc::new(Cell::new(false));
    let query = capture_string();
    {
        let fired = fired.clone();
        let query_capture = query.clone();
        ui.on_cp_search_changed(move |value| {
            fired.set(true);
            *query_capture.borrow_mut() = value;
        });
    }
    ui.invoke_cp_search_changed("Ромашка".into());
    assert!(fired.get(), "cp: search-changed");
    assert_eq!(query.borrow().as_str(), "Ромашка");

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_cp_new(move || fired.set(true));
    }
    ui.invoke_cp_new();
    assert!(fired.get(), "cp: new");

    let fired = Rc::new(Cell::new(false));
    let cp_id = capture_string();
    {
        let fired = fired.clone();
        let cp_id_capture = cp_id.clone();
        ui.on_cp_create_doc(move |id| {
            fired.set(true);
            *cp_id_capture.borrow_mut() = id;
        });
    }
    ui.invoke_cp_create_doc("cp-uuid-doc".into());
    assert!(fired.get(), "cp: create-doc");
    assert_eq!(cp_id.borrow().as_str(), "cp-uuid-doc");

    let fired = Rc::new(Cell::new(false));
    let tab = capture_string();
    {
        let fired = fired.clone();
        let tab_capture = tab.clone();
        ui.on_cp_tab_changed(move |value| {
            fired.set(true);
            *tab_capture.borrow_mut() = value;
        });
    }
    ui.invoke_cp_tab_changed("payments".into());
    assert!(fired.get(), "cp: tab-changed");
    assert_eq!(tab.borrow().as_str(), "payments");
}

fn payments() {
    let ui = create_window();

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_pay_import_csv(move || fired.set(true));
    }
    ui.invoke_pay_import_csv();
    assert!(fired.get(), "pay: import-csv");

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_pay_sync_bank(move || fired.set(true));
    }
    ui.invoke_pay_sync_bank();
    assert!(fired.get(), "pay: sync-bank");

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_pay_new(move || fired.set(true));
    }
    ui.invoke_pay_new();
    assert!(fired.get(), "pay: new");

    let fired = Rc::new(Cell::new(false));
    let payment_id = capture_string();
    {
        let fired = fired.clone();
        let payment_id_capture = payment_id.clone();
        ui.on_pay_link(move |id| {
            fired.set(true);
            *payment_id_capture.borrow_mut() = id;
        });
    }
    ui.invoke_pay_link("pay-uuid-link".into());
    assert!(fired.get(), "pay: link");
    assert_eq!(payment_id.borrow().as_str(), "pay-uuid-link");
}

fn reports() {
    let ui = create_window();

    let fired = Rc::new(Cell::new(false));
    let period = Rc::new(Cell::new(-1i32));
    {
        let fired = fired.clone();
        let period_capture = period.clone();
        ui.on_rep_period_changed(move |value| {
            fired.set(true);
            period_capture.set(value);
        });
    }
    ui.invoke_rep_period_changed(2);
    assert!(fired.get(), "rep: period-changed");
    assert_eq!(period.get(), 2);

    let fired = Rc::new(Cell::new(false));
    let category = capture_string();
    {
        let fired = fired.clone();
        let category_capture = category.clone();
        ui.on_rep_category_drilled(move |value| {
            fired.set(true);
            *category_capture.borrow_mut() = value;
        });
    }
    ui.invoke_rep_category_drilled("Зарплата".into());
    assert!(fired.get(), "rep: category-drilled");
    assert_eq!(category.borrow().as_str(), "Зарплата");

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_rep_export_csv(move || fired.set(true));
    }
    ui.invoke_rep_export_csv();
    assert!(fired.get(), "rep: export-csv");

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_rep_export_pdf(move || fired.set(true));
    }
    ui.invoke_rep_export_pdf();
    assert!(fired.get(), "rep: export-pdf");
}

fn tasks() {
    let ui = create_window();

    let fired = Rc::new(Cell::new(false));
    let task_id = capture_string();
    let done = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        let task_id_capture = task_id.clone();
        let done_capture = done.clone();
        ui.on_task_toggled(move |id, is_done| {
            fired.set(true);
            *task_id_capture.borrow_mut() = id;
            done_capture.set(is_done);
        });
    }
    ui.invoke_task_toggled("task-uuid-001".into(), true);
    assert!(fired.get(), "task: toggled");
    assert_eq!(task_id.borrow().as_str(), "task-uuid-001");
    assert!(done.get());

    let fired = Rc::new(Cell::new(false));
    let task_id = capture_string();
    {
        let fired = fired.clone();
        let task_id_capture = task_id.clone();
        ui.on_task_more(move |id| {
            fired.set(true);
            *task_id_capture.borrow_mut() = id;
        });
    }
    ui.invoke_task_more("task-uuid-more".into());
    assert!(fired.get(), "task: more");
    assert_eq!(task_id.borrow().as_str(), "task-uuid-more");

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_task_new(move || fired.set(true));
    }
    ui.invoke_task_new();
    assert!(fired.get(), "task: new");

    let fired = Rc::new(Cell::new(false));
    let filter = capture_string();
    {
        let fired = fired.clone();
        let filter_capture = filter.clone();
        ui.on_task_filter_changed(move |value| {
            fired.set(true);
            *filter_capture.borrow_mut() = value;
        });
    }
    ui.invoke_task_filter_changed("done".into());
    assert!(fired.get(), "task: filter-changed");
    assert_eq!(filter.borrow().as_str(), "done");
}

fn settings() {
    let ui = create_window();

    let fired = Rc::new(Cell::new(false));
    let section = capture_string();
    {
        let fired = fired.clone();
        let section_capture = section.clone();
        ui.on_settings_section_changed(move |value| {
            fired.set(true);
            *section_capture.borrow_mut() = value;
        });
    }
    ui.invoke_settings_section_changed("appearance".into());
    assert!(fired.get(), "settings: section-changed");
    assert_eq!(section.borrow().as_str(), "appearance");

    let fired = Rc::new(Cell::new(false));
    let dark = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        let dark_capture = dark.clone();
        ui.on_settings_dark_mode_toggled(move |value| {
            fired.set(true);
            dark_capture.set(value);
        });
    }
    ui.invoke_settings_dark_mode_toggled(true);
    assert!(fired.get(), "settings: dark-mode-toggled");
    assert!(dark.get());

    let fired = Rc::new(Cell::new(false));
    let density = Rc::new(Cell::new(-1i32));
    {
        let fired = fired.clone();
        let density_capture = density.clone();
        ui.on_settings_density_changed(move |value| {
            fired.set(true);
            density_capture.set(value);
        });
    }
    ui.invoke_settings_density_changed(2);
    assert!(fired.get(), "settings: density-changed");
    assert_eq!(density.get(), 2);

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_settings_company_saved(move |_| fired.set(true));
    }
    ui.invoke_settings_company_saved(CompanyInfo {
        full_name: "ТОВ Тест".into(),
        short_name: "Тест".into(),
        edrpou: "12345678".into(),
        ipn: "".into(),
        address: "м. Київ".into(),
        director: "Іванов І.І.".into(),
        iban: "UA1234567890".into(),
        bank: "ПриватБанк".into(),
        vat_registered: false,
        vat_cert: "".into(),
    });
    assert!(fired.get(), "settings: company-saved");

    let fired = Rc::new(Cell::new(false));
    let integration = capture_string();
    {
        let fired = fired.clone();
        let integration_capture = integration.clone();
        ui.on_settings_integration_configure(move |value| {
            fired.set(true);
            *integration_capture.borrow_mut() = value;
        });
    }
    ui.invoke_settings_integration_configure("bas".into());
    assert!(fired.get(), "settings: integration-configure");
    assert_eq!(integration.borrow().as_str(), "bas");

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_settings_team_invite(move || fired.set(true));
    }
    ui.invoke_settings_team_invite();
    assert!(fired.get(), "settings: team-invite");

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_settings_backup_now(move || fired.set(true));
    }
    ui.invoke_settings_backup_now();
    assert!(fired.get(), "settings: backup-now");

    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        ui.on_settings_backup_download(move || fired.set(true));
    }
    ui.invoke_settings_backup_download();
    assert!(fired.get(), "settings: backup-download");
}

fn command_palette() {
    let ui = create_window();

    let fired = Rc::new(Cell::new(false));
    let query = capture_string();
    {
        let fired = fired.clone();
        let query_capture = query.clone();
        ui.on_palette_query_changed(move |value| {
            fired.set(true);
            *query_capture.borrow_mut() = value;
        });
    }
    ui.invoke_palette_query_changed("створити акт".into());
    assert!(fired.get(), "palette: query-changed");
    assert_eq!(query.borrow().as_str(), "створити акт");

    let fired = Rc::new(Cell::new(false));
    let item = capture_string();
    {
        let fired = fired.clone();
        let item_capture = item.clone();
        ui.on_palette_item_activated(move |value| {
            fired.set(true);
            *item_capture.borrow_mut() = value;
        });
    }
    ui.invoke_palette_item_activated("new-act".into());
    assert!(fired.get(), "palette: item-activated");
    assert_eq!(item.borrow().as_str(), "new-act");
}
