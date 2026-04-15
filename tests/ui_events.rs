// Headless тести Slint event handlers.
//
// ВАЖЛИВО: Slint вимагає, щоб усі UI-операції відбувались на тому самому потоці,
// де ініціалізований backend. Тестовий раннер Rust за замовчуванням запускає
// тести в паралельних потоках, тому ВСІ Slint-тести вміщені в одну функцію
// `#[test] fn ui_event_handlers()`, яка виконується повністю в одному потоці.
//
// Структура:
//   act_*     — колбеки списку та форми актів
//   invoice_* — колбеки списку та форми накладних
//   payment_* — колбеки списку та форми платежів
//   task_*    — колбеки списку та форми задач
//   cp_*      — колбеки списку та форми контрагентів
//   dashboard_*
//   escape_*  — Escape → cancel через FocusScope

slint::include_modules!();

use slint::SharedString;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

// ───────────────────────────────────────────────────────────────────────────
// Єдиний тест, що запускає всі підтести послідовно в одному потоці.
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn ui_event_handlers() {
    i_slint_backend_testing::init_no_event_loop();

    act_list();
    act_form();
    invoice_list();
    invoice_form();
    payment_list();
    payment_form();
    task_list();
    task_form();
    cp_list();
    cp_form();
    dashboard();
    escape_key();
}

// ═══════════════════════════════════════════════════════════════════════════
// Акти — список
// ═══════════════════════════════════════════════════════════════════════════

fn act_list() {
    let ui = MainWindow::new().unwrap();

    // create-clicked
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_act_create_clicked(move || f.set(true));
    ui.invoke_act_create_clicked();
    assert!(fired.get(), "act: create-clicked");

    // status-filter-changed передає індекс вкладки
    let tab = Rc::new(Cell::new(-1i32));
    let t = tab.clone();
    ui.on_act_status_filter_changed(move |i| t.set(i));
    ui.invoke_act_status_filter_changed(2); // Виставлені
    assert_eq!(tab.get(), 2, "act: status-filter tab=2");
    ui.invoke_act_status_filter_changed(0); // Всі
    assert_eq!(tab.get(), 0, "act: status-filter tab=0");

    // row-selected передає id
    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_act_selected(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_act_selected("act-uuid-abc".into());
    assert_eq!(id.borrow().as_str(), "act-uuid-abc", "act: selected id");

    // edit-clicked передає id
    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_act_edit_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_act_edit_clicked("edit-act-id".into());
    assert_eq!(id.borrow().as_str(), "edit-act-id", "act: edit id");

    // advance-status-clicked передає id
    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_act_advance_status_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_act_advance_status_clicked("advance-act-id".into());
    assert_eq!(id.borrow().as_str(), "advance-act-id", "act: advance id");

    // pdf-clicked передає id
    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_act_pdf_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_act_pdf_clicked("pdf-act-id".into());
    assert_eq!(id.borrow().as_str(), "pdf-act-id", "act: pdf id");

    // search-changed передає запит
    let query = Rc::new(RefCell::new(String::new()));
    let q = query.clone();
    ui.on_act_search_changed(move |s: SharedString| *q.borrow_mut() = s.to_string());
    ui.invoke_act_search_changed("Рога та Копита".into());
    assert_eq!(query.borrow().as_str(), "Рога та Копита", "act: search query");
}

// ═══════════════════════════════════════════════════════════════════════════
// Акти — форма
// ═══════════════════════════════════════════════════════════════════════════

fn act_form() {
    let ui = MainWindow::new().unwrap();

    // cancel
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_act_form_cancel(move || f.set(true));
    ui.invoke_act_form_cancel();
    assert!(fired.get(), "act-form: cancel");

    // save(number, date, cp_id, notes, cat_id, con_id, exp_date)
    let args: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
    let a = args.clone();
    ui.on_act_form_save(move |num, date, cp, notes, cat, con, exp| {
        *a.borrow_mut() = vec![
            num.into(), date.into(), cp.into(), notes.into(),
            cat.into(), con.into(), exp.into(),
        ];
    });
    ui.invoke_act_form_save(
        "АКТ-2026-001".into(), "01.04.2026".into(), "cp-uuid".into(),
        "примітка".into(), "cat-uuid".into(), "con-uuid".into(), "30.04.2026".into(),
    );
    {
        let got = args.borrow();
        assert_eq!(got[0], "АКТ-2026-001", "act-form: save number");
        assert_eq!(got[1], "01.04.2026",   "act-form: save date");
        assert_eq!(got[2], "cp-uuid",      "act-form: save cp_id");
        assert_eq!(got[6], "30.04.2026",   "act-form: save exp_date");
    }

    // save-draft
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_act_form_save_draft(move |_, _, _, _, _, _, _| f.set(true));
    ui.invoke_act_form_save_draft(
        "АКТ-2026-002".into(), "02.04.2026".into(), "cp-id".into(),
        "".into(), "".into(), "".into(), "".into(),
    );
    assert!(fired.get(), "act-form: save-draft");

    // add-item
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_act_form_add_item(move || f.set(true));
    ui.invoke_act_form_add_item();
    assert!(fired.get(), "act-form: add-item");

    // remove-item передає індекс
    let idx = Rc::new(Cell::new(-1i32));
    let i = idx.clone();
    ui.on_act_form_remove_item(move |n| i.set(n));
    ui.invoke_act_form_remove_item(2);
    assert_eq!(idx.get(), 2, "act-form: remove-item idx");

    // item-changed передає (index, field, value)
    let got = Rc::new(RefCell::new((0i32, String::new(), String::new())));
    let g = got.clone();
    ui.on_act_form_item_changed(move |idx, field: SharedString, val: SharedString| {
        *g.borrow_mut() = (idx, field.to_string(), val.to_string());
    });
    ui.invoke_act_form_item_changed(1, "price".into(), "500.00".into());
    {
        let (idx, ref field, ref val) = *got.borrow();
        assert_eq!(idx, 1, "act-form: item-changed idx");
        assert_eq!(field, "price", "act-form: item-changed field");
        assert_eq!(val, "500.00", "act-form: item-changed value");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Накладні — список
// ═══════════════════════════════════════════════════════════════════════════

fn invoice_list() {
    let ui = MainWindow::new().unwrap();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_invoice_create_clicked(move || f.set(true));
    ui.invoke_invoice_create_clicked();
    assert!(fired.get(), "invoice: create-clicked");

    let tab = Rc::new(Cell::new(-1i32));
    let t = tab.clone();
    ui.on_invoice_status_filter_changed(move |i| t.set(i));
    ui.invoke_invoice_status_filter_changed(3);
    assert_eq!(tab.get(), 3, "invoice: status-filter");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_invoice_edit_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_invoice_edit_clicked("inv-id-xyz".into());
    assert_eq!(id.borrow().as_str(), "inv-id-xyz", "invoice: edit id");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_invoice_advance_status_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_invoice_advance_status_clicked("inv-advance-id".into());
    assert_eq!(id.borrow().as_str(), "inv-advance-id", "invoice: advance id");
}

// ═══════════════════════════════════════════════════════════════════════════
// Накладні — форма
// ═══════════════════════════════════════════════════════════════════════════

fn invoice_form() {
    let ui = MainWindow::new().unwrap();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_invoice_form_cancel(move || f.set(true));
    ui.invoke_invoice_form_cancel();
    assert!(fired.get(), "invoice-form: cancel");

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_invoice_form_add_item(move || f.set(true));
    ui.invoke_invoice_form_add_item();
    assert!(fired.get(), "invoice-form: add-item");

    let idx = Rc::new(Cell::new(-1i32));
    let i = idx.clone();
    ui.on_invoice_form_remove_item(move |n| i.set(n));
    ui.invoke_invoice_form_remove_item(0);
    assert_eq!(idx.get(), 0, "invoice-form: remove-item");

    let got = Rc::new(RefCell::new((0i32, String::new(), String::new())));
    let g = got.clone();
    ui.on_invoice_form_item_changed(move |idx, field: SharedString, val: SharedString| {
        *g.borrow_mut() = (idx, field.to_string(), val.to_string());
    });
    ui.invoke_invoice_form_item_changed(0, "qty".into(), "3.000".into());
    {
        let (idx, ref field, ref val) = *got.borrow();
        assert_eq!(idx, 0, "invoice-form: item-changed idx");
        assert_eq!(field, "qty");
        assert_eq!(val, "3.000");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Платежі — список
// ═══════════════════════════════════════════════════════════════════════════

fn payment_list() {
    let ui = MainWindow::new().unwrap();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_payment_create_clicked(move || f.set(true));
    ui.invoke_payment_create_clicked();
    assert!(fired.get(), "payment: create-clicked");

    // direction-filter: 0=Усі, 1=Доходи, 2=Витрати
    let dir = Rc::new(Cell::new(-1i32));
    let d = dir.clone();
    ui.on_payment_direction_filter_changed(move |i| d.set(i));
    ui.invoke_payment_direction_filter_changed(1);
    assert_eq!(dir.get(), 1, "payment: direction-filter income");
    ui.invoke_payment_direction_filter_changed(2);
    assert_eq!(dir.get(), 2, "payment: direction-filter expense");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_payment_edit_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_payment_edit_clicked("pay-id-123".into());
    assert_eq!(id.borrow().as_str(), "pay-id-123", "payment: edit id");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_payment_delete_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_payment_delete_clicked("pay-del-id".into());
    assert_eq!(id.borrow().as_str(), "pay-del-id", "payment: delete id");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_payment_reconcile_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_payment_reconcile_clicked("pay-rec-id".into());
    assert_eq!(id.borrow().as_str(), "pay-rec-id", "payment: reconcile id");

    let query = Rc::new(RefCell::new(String::new()));
    let q = query.clone();
    ui.on_payment_search_changed(move |s: SharedString| *q.borrow_mut() = s.to_string());
    ui.invoke_payment_search_changed("ПриватБанк".into());
    assert_eq!(query.borrow().as_str(), "ПриватБанк", "payment: search");
}

// ═══════════════════════════════════════════════════════════════════════════
// Платежі — форма
// ═══════════════════════════════════════════════════════════════════════════

fn payment_form() {
    let ui = MainWindow::new().unwrap();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_payment_form_cancel(move || f.set(true));
    ui.invoke_payment_form_cancel();
    assert!(fired.get(), "payment-form: cancel");

    // save(date, amount, direction_idx, cp_id, bank_name, bank_ref, description)
    let got_dir  = Rc::new(Cell::new(-1i32));
    let got_date = Rc::new(RefCell::new(String::new()));
    let d  = got_dir.clone();
    let dt = got_date.clone();
    ui.on_payment_form_save(move |date: SharedString, _amount, dir, _cp, _bank, _ref, _desc| {
        d.set(dir);
        *dt.borrow_mut() = date.to_string();
    });
    ui.invoke_payment_form_save(
        "15.04.2026".into(), "2500.00".into(), 1,
        "cp-id".into(), "ПриватБанк".into(), "REF123".into(), "оплата за послуги".into(),
    );
    assert_eq!(got_dir.get(), 1,             "payment-form: save direction");
    assert_eq!(got_date.borrow().as_str(), "15.04.2026", "payment-form: save date");

    // update
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_payment_form_update(move |_, _, _, _, _, _, _| f.set(true));
    ui.invoke_payment_form_update(
        "15.04.2026".into(), "1000.00".into(), 2,
        "cp-id".into(), "Монобанк".into(), "".into(), "".into(),
    );
    assert!(fired.get(), "payment-form: update");
}

// ═══════════════════════════════════════════════════════════════════════════
// Задачі — список
// ═══════════════════════════════════════════════════════════════════════════

fn task_list() {
    let ui = MainWindow::new().unwrap();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_task_create_clicked(move || f.set(true));
    ui.invoke_task_create_clicked();
    assert!(fired.get(), "task: create-clicked");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_task_edit_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_task_edit_clicked("task-id-99".into());
    assert_eq!(id.borrow().as_str(), "task-id-99", "task: edit id");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_task_toggle_status_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_task_toggle_status_clicked("task-toggle-id".into());
    assert_eq!(id.borrow().as_str(), "task-toggle-id", "task: toggle id");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_task_delete_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_task_delete_clicked("task-del-id".into());
    assert_eq!(id.borrow().as_str(), "task-del-id", "task: delete id");
}

// ═══════════════════════════════════════════════════════════════════════════
// Задачі — форма
// ═══════════════════════════════════════════════════════════════════════════

fn task_form() {
    let ui = MainWindow::new().unwrap();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_task_form_cancel(move || f.set(true));
    ui.invoke_task_form_cancel();
    assert!(fired.get(), "task-form: cancel");

    // save(title, due_date, priority_idx, act_id, notes)
    let got_title = Rc::new(RefCell::new(String::new()));
    let got_prio  = Rc::new(Cell::new(-1i32));
    let t = got_title.clone();
    let p = got_prio.clone();
    ui.on_task_form_save(move |title: SharedString, _due, prio, _act_id, _notes| {
        *t.borrow_mut() = title.to_string();
        p.set(prio);
    });
    ui.invoke_task_form_save(
        "Підписати договір".into(), "30.04.2026".into(),
        1, "act-id-xyz".into(), "терміново".into(),
    );
    assert_eq!(got_title.borrow().as_str(), "Підписати договір", "task-form: save title");
    assert_eq!(got_prio.get(), 1, "task-form: save priority");
}

// ═══════════════════════════════════════════════════════════════════════════
// Контрагенти — список
// ═══════════════════════════════════════════════════════════════════════════

fn cp_list() {
    let ui = MainWindow::new().unwrap();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_counterparty_create_clicked(move || f.set(true));
    ui.invoke_counterparty_create_clicked();
    assert!(fired.get(), "cp: create-clicked");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_counterparty_edit_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_counterparty_edit_clicked("cp-id-007".into());
    assert_eq!(id.borrow().as_str(), "cp-id-007", "cp: edit id");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_counterparty_archive_clicked(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_counterparty_archive_clicked("cp-archive-id".into());
    assert_eq!(id.borrow().as_str(), "cp-archive-id", "cp: archive id");

    let id = Rc::new(RefCell::new(String::new()));
    let i = id.clone();
    ui.on_counterparty_selected(move |s: SharedString| *i.borrow_mut() = s.to_string());
    ui.invoke_counterparty_selected("cp-selected-id".into());
    assert_eq!(id.borrow().as_str(), "cp-selected-id", "cp: selected id");

    let prev = Rc::new(Cell::new(false));
    let next = Rc::new(Cell::new(false));
    let p = prev.clone();
    let n = next.clone();
    ui.on_counterparty_prev_page_clicked(move || p.set(true));
    ui.on_counterparty_next_page_clicked(move || n.set(true));
    ui.invoke_counterparty_prev_page_clicked();
    ui.invoke_counterparty_next_page_clicked();
    assert!(prev.get(), "cp: prev-page");
    assert!(next.get(), "cp: next-page");
}

// ═══════════════════════════════════════════════════════════════════════════
// Контрагенти — форма
// ═══════════════════════════════════════════════════════════════════════════

fn cp_form() {
    let ui = MainWindow::new().unwrap();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_cp_form_cancel(move || f.set(true));
    ui.invoke_cp_form_cancel();
    assert!(fired.get(), "cp-form: cancel");

    // save(name, edrpou, ipn, iban, address, email, phone, notes)
    let got_name = Rc::new(RefCell::new(String::new()));
    let n = got_name.clone();
    ui.on_cp_form_save(
        move |name: SharedString, _edrp, _ipn, _iban, _addr, _email, _phone, _notes| {
            *n.borrow_mut() = name.to_string();
        },
    );
    ui.invoke_cp_form_save(
        "ТОВ Ромашка".into(), "12345678".into(), "".into(),
        "UA12345678901234567890123456789".into(),
        "м. Київ".into(), "info@romashka.ua".into(), "+380441234567".into(), "".into(),
    );
    assert_eq!(got_name.borrow().as_str(), "ТОВ Ромашка", "cp-form: save name");
}

// ═══════════════════════════════════════════════════════════════════════════
// Dashboard
// ═══════════════════════════════════════════════════════════════════════════

fn dashboard() {
    let ui = MainWindow::new().unwrap();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_dashboard_refresh(move || f.set(true));
    ui.invoke_dashboard_refresh();
    assert!(fired.get(), "dashboard: refresh");

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_dashboard_new_act_clicked(move || f.set(true));
    ui.invoke_dashboard_new_act_clicked();
    assert!(fired.get(), "dashboard: new-act-clicked");

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_dashboard_all_acts_clicked(move || f.set(true));
    ui.invoke_dashboard_all_acts_clicked();
    assert!(fired.get(), "dashboard: all-acts-clicked");
}

// ═══════════════════════════════════════════════════════════════════════════
// Escape → cancel через FocusScope
//
// Перевіряє що натискання Escape у PaymentForm вогнить payment-form-cancel.
// Потрібно: current-page=3, show-payment-form=true, show() щоб FocusScope
// отримав фокус, потім dispatch_event KeyPressed ESC.
// ═══════════════════════════════════════════════════════════════════════════

fn escape_key() {
    use slint::platform::WindowEvent;

    let ui = MainWindow::new().unwrap();
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    ui.on_payment_form_cancel(move || f.set(true));

    // Відкриваємо форму платежів
    ui.set_current_page(3);
    ui.set_show_payment_form(true);
    ui.show().unwrap();

    // Tab → фокус переходить на перший FocusScope (у PaymentForm).
    // З init_no_event_loop() жоден елемент не отримує фокус автоматично при show(),
    // тому Tab викликає focus_next_item() і встановлює focus_item на PaymentForm FocusScope.
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: SharedString::from("\u{0009}"), // Tab
    });

    // ESC = U+001B → доставляється до FocusScope, який вогнить cancel()
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: SharedString::from("\u{1B}"),
    });

    assert!(fired.get(), "escape → payment-form-cancel");
}
