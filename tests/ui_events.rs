// Epic 2: UI test safety net
//
// Headless-тести для callback-контракту канонічного Slint UI у `ui/`.
// Тут перевіряємо wiring і стабільність Slint contract.

slint::include_modules!();

mod shell_test_components {
    slint::slint! {
        import { Shell, CommandPalette } from "../ui/shell.slint";
        import { NavScreen as ShellScreen } from "../ui/types.slint";

        export component TestShellHost inherits Window {
            callback navigate-id(string);
            callback toggle-theme;
            callback open-cmd-palette;
            callback close-cmd-palette;

            shell := Shell {
                width: parent.width;
                height: parent.height;

                navigate(screen) => {
                    if (screen == ShellScreen.Dashboard) { root.navigate-id("dashboard"); }
                    else if (screen == ShellScreen.Documents) { root.navigate-id("documents"); }
                    else if (screen == ShellScreen.Counterparties) { root.navigate-id("counterparties"); }
                    else if (screen == ShellScreen.Payments) { root.navigate-id("payments"); }
                    else if (screen == ShellScreen.Reports) { root.navigate-id("reports"); }
                    else if (screen == ShellScreen.Tasks) { root.navigate-id("tasks"); }
                    else if (screen == ShellScreen.Settings) { root.navigate-id("settings"); }
                }
                toggle-theme => { root.toggle-theme(); }
                open-cmd-palette => { root.open-cmd-palette(); }
                close-cmd-palette => { root.close-cmd-palette(); }
            }
        }

        export component TestCommandPaletteHost inherits Window {
            in-out property <bool> open;
            in-out property <string> query;
            callback closed;
            callback navigated-id(string);
            callback query-changed(string);

            palette := CommandPalette {
                width: parent.width;
                height: parent.height;
                open: root.open;
                query: root.query;

                closed => { root.closed(); }
                navigated(screen) => {
                    if (screen == ShellScreen.Dashboard) { root.navigated-id("dashboard"); }
                    else if (screen == ShellScreen.Documents) { root.navigated-id("documents"); }
                    else if (screen == ShellScreen.Counterparties) { root.navigated-id("counterparties"); }
                    else if (screen == ShellScreen.Payments) { root.navigated-id("payments"); }
                    else if (screen == ShellScreen.Reports) { root.navigated-id("reports"); }
                    else if (screen == ShellScreen.Tasks) { root.navigated-id("tasks"); }
                    else if (screen == ShellScreen.Settings) { root.navigated-id("settings"); }
                }
                query-changed(value) => { root.query-changed(value); }
            }
        }
    }
}

mod helpers {
    use slint::SharedString;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    pub fn init_headless_ui() {
        i_slint_backend_testing::init_no_event_loop();
    }

    pub fn capture_string() -> Rc<RefCell<SharedString>> {
        Rc::new(RefCell::new(SharedString::default()))
    }

    pub fn capture_bool() -> Rc<Cell<bool>> {
        Rc::new(Cell::new(false))
    }

    pub fn capture_i32(initial: i32) -> Rc<Cell<i32>> {
        Rc::new(Cell::new(initial))
    }

    pub fn capture_nav(initial: super::NavScreen) -> Rc<Cell<super::NavScreen>> {
        Rc::new(Cell::new(initial))
    }

    pub fn new_window() -> super::AppWindow {
        super::AppWindow::new().expect("AppWindow має створюватися")
    }

    pub fn new_shell() -> super::shell_test_components::TestShellHost {
        super::shell_test_components::TestShellHost::new().expect("TestShellHost має створюватися")
    }

    pub fn new_palette() -> super::shell_test_components::TestCommandPaletteHost {
        super::shell_test_components::TestCommandPaletteHost::new()
            .expect("TestCommandPaletteHost має створюватися")
    }
}

mod app_window_contract {
    use super::*;
    use helpers::*;

    #[test]
    fn app_window_event_handlers() {
        init_headless_ui();

        navigation();
        inbox();
        documents();
        counterparties();
        payments();
        reports();
        tasks();
        settings();
        command_palette();
    }

    fn navigation() {
        let ui = new_window();
        let fired = capture_bool();
        let screen = capture_nav(NavScreen::Dashboard);

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

    fn inbox() {
        let ui = new_window();
        let fired = capture_bool();
        let doc_id = capture_string();
        let kind = capture_string();

        {
            let fired = fired.clone();
            let doc_id_capture = doc_id.clone();
            let kind_capture = kind.clone();
            ui.on_inbox_action(move |id, action| {
                fired.set(true);
                *doc_id_capture.borrow_mut() = id;
                *kind_capture.borrow_mut() = action;
            });
        }

        ui.invoke_inbox_action("act:inbox-001".into(), "open".into());

        assert!(fired.get(), "inbox: action");
        assert_eq!(doc_id.borrow().as_str(), "act:inbox-001");
        assert_eq!(kind.borrow().as_str(), "open");
    }

    fn documents() {
        let ui = new_window();

        let fired = capture_bool();
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

        let fired = capture_bool();
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

        let fired = capture_bool();
        let doc_id = capture_string();
        let selected = capture_bool();
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

        let fired = capture_bool();
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

        let fired = capture_bool();
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

        let fired = capture_bool();
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

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_doc_new(move || fired.set(true));
        }
        ui.invoke_doc_new();
        assert!(fired.get(), "doc: new");

        let fired = capture_bool();
        let page = capture_i32(0);
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

        let fired = capture_bool();
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

        let fired = capture_bool();
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
        let ui = new_window();

        let fired = capture_bool();
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

        let fired = capture_bool();
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

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_cp_new(move || fired.set(true));
        }
        ui.invoke_cp_new();
        assert!(fired.get(), "cp: new");

        let fired = capture_bool();
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

        let fired = capture_bool();
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
        let ui = new_window();

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_pay_import_csv(move || fired.set(true));
        }
        ui.invoke_pay_import_csv();
        assert!(fired.get(), "pay: import-csv");

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_pay_sync_bank(move || fired.set(true));
        }
        ui.invoke_pay_sync_bank();
        assert!(fired.get(), "pay: sync-bank");

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_pay_new(move || fired.set(true));
        }
        ui.invoke_pay_new();
        assert!(fired.get(), "pay: new");

        let fired = capture_bool();
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
        let ui = new_window();

        let fired = capture_bool();
        let period = capture_i32(-1);
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

        let fired = capture_bool();
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

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_rep_export_csv(move || fired.set(true));
        }
        ui.invoke_rep_export_csv();
        assert!(fired.get(), "rep: export-csv");

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_rep_export_pdf(move || fired.set(true));
        }
        ui.invoke_rep_export_pdf();
        assert!(fired.get(), "rep: export-pdf");
    }

    fn tasks() {
        let ui = new_window();

        let fired = capture_bool();
        let task_id = capture_string();
        let done = capture_bool();
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

        let fired = capture_bool();
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

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_task_new(move || fired.set(true));
        }
        ui.invoke_task_new();
        assert!(fired.get(), "task: new");

        let fired = capture_bool();
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
        let ui = new_window();

        let fired = capture_bool();
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

        let fired = capture_bool();
        let dark = capture_bool();
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

        let fired = capture_bool();
        let density = capture_i32(-1);
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

        let fired = capture_bool();
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

        let fired = capture_bool();
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

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_settings_team_invite(move || fired.set(true));
        }
        ui.invoke_settings_team_invite();
        assert!(fired.get(), "settings: team-invite");

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_settings_backup_now(move || fired.set(true));
        }
        ui.invoke_settings_backup_now();
        assert!(fired.get(), "settings: backup-now");

        let fired = capture_bool();
        {
            let fired = fired.clone();
            ui.on_settings_backup_download(move || fired.set(true));
        }
        ui.invoke_settings_backup_download();
        assert!(fired.get(), "settings: backup-download");
    }

    fn command_palette() {
        let ui = new_window();

        let fired = capture_bool();
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

        let fired = capture_bool();
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
}

mod shell_contract {
    use super::*;
    use helpers::*;

    #[test]
    fn shell_callback_contracts() {
        init_headless_ui();

        let shell = new_shell();

        let navigate_fired = capture_bool();
        let target = capture_string();
        {
            let navigate_fired = navigate_fired.clone();
            let target_capture = target.clone();
            shell.on_navigate_id(move |screen| {
                navigate_fired.set(true);
                *target_capture.borrow_mut() = screen;
            });
        }
        shell.invoke_navigate_id("settings".into());
        assert!(navigate_fired.get(), "shell: navigate");
        assert_eq!(target.borrow().as_str(), "settings");

        let theme_fired = capture_bool();
        {
            let theme_fired = theme_fired.clone();
            shell.on_toggle_theme(move || theme_fired.set(true));
        }
        shell.invoke_toggle_theme();
        assert!(theme_fired.get(), "shell: toggle-theme");

        let open_fired = capture_bool();
        {
            let open_fired = open_fired.clone();
            shell.on_open_cmd_palette(move || open_fired.set(true));
        }
        shell.invoke_open_cmd_palette();
        assert!(open_fired.get(), "shell: open-cmd-palette");

        let close_fired = capture_bool();
        {
            let close_fired = close_fired.clone();
            shell.on_close_cmd_palette(move || close_fired.set(true));
        }
        shell.invoke_close_cmd_palette();
        assert!(close_fired.get(), "shell: close-cmd-palette");
    }
}

mod keyboard_palette_regressions {
    use super::*;
    use helpers::*;

    #[test]
    fn command_palette_keeps_open_state_contract_stable() {
        init_headless_ui();

        let palette = new_palette();
        assert!(!palette.get_open(), "palette: closed by default");

        palette.set_open(true);
        assert!(palette.get_open(), "palette: open state should be mutable");

        palette.set_query("платежі".into());
        assert_eq!(palette.get_query().as_str(), "платежі");
    }

    #[test]
    fn command_palette_callback_contracts_cover_navigation_and_close() {
        init_headless_ui();

        let palette = new_palette();
        let closed = capture_bool();
        let navigated = capture_bool();
        let screen = capture_string();
        let query_changed = capture_bool();
        let query = capture_string();

        {
            let closed = closed.clone();
            palette.on_closed(move || closed.set(true));
        }
        {
            let navigated = navigated.clone();
            let screen_capture = screen.clone();
            palette.on_navigated_id(move |target| {
                navigated.set(true);
                *screen_capture.borrow_mut() = target;
            });
        }
        {
            let query_changed = query_changed.clone();
            let query_capture = query.clone();
            palette.on_query_changed(move |value| {
                query_changed.set(true);
                *query_capture.borrow_mut() = value;
            });
        }

        palette.invoke_query_changed("контрагент".into());
        palette.invoke_navigated_id("counterparties".into());
        palette.invoke_closed();

        assert!(query_changed.get(), "palette regression: query-changed");
        assert_eq!(query.borrow().as_str(), "контрагент");
        assert!(navigated.get(), "palette regression: navigated");
        assert_eq!(screen.borrow().as_str(), "counterparties");
        assert!(closed.get(), "palette regression: closed");
    }

    #[test]
    fn shell_navigation_shortcut_targets_remain_stable() {
        init_headless_ui();

        let shell = new_shell();
        let captured = capture_string();

        {
            let captured = captured.clone();
            shell.on_navigate_id(move |screen| *captured.borrow_mut() = screen);
        }

        shell.invoke_navigate_id("documents".into());
        assert_eq!(captured.borrow().as_str(), "documents");

        shell.invoke_navigate_id("payments".into());
        assert_eq!(captured.borrow().as_str(), "payments");

        shell.invoke_navigate_id("settings".into());
        assert_eq!(captured.borrow().as_str(), "settings");
    }
}
