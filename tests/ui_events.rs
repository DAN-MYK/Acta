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
        assert_eq!(got[3], "примітка",     "act-form: save notes");
        assert_eq!(got[4], "cat-uuid",     "act-form: save cat_id");
        assert_eq!(got[5], "con-uuid",     "act-form: save con_id");
        assert_eq!(got[6], "30.04.2026",   "act-form: save exp_date");
    }

    // update(number, date, cp_id, notes, cat_id, con_id, exp_date)
    let args: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
    let a = args.clone();
    ui.on_act_form_update(move |num, date, cp, notes, cat, con, exp| {
        *a.borrow_mut() = vec![
            num.into(), date.into(), cp.into(), notes.into(),
            cat.into(), con.into(), exp.into(),
        ];
    });
    ui.invoke_act_form_update(
        "АКТ-2026-003".into(), "03.04.2026".into(), "cp-update".into(),
        "оновлена примітка".into(), "cat-update".into(), "con-update".into(), "10.05.2026".into(),
    );
    {
        let got = args.borrow();
        assert_eq!(got[0], "АКТ-2026-003",      "act-form: update number");
        assert_eq!(got[1], "03.04.2026",        "act-form: update date");
        assert_eq!(got[2], "cp-update",         "act-form: update cp_id");
        assert_eq!(got[3], "оновлена примітка", "act-form: update notes");
        assert_eq!(got[4], "cat-update",        "act-form: update cat_id");
        assert_eq!(got[5], "con-update",        "act-form: update con_id");
        assert_eq!(got[6], "10.05.2026",        "act-form: update exp_date");
    }

    // save-draft
    let args: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
    let a = args.clone();
    ui.on_act_form_save_draft(move |num, date, cp, notes, cat, con, exp| {
        *a.borrow_mut() = vec![
            num.into(), date.into(), cp.into(), notes.into(),
            cat.into(), con.into(), exp.into(),
        ];
    });
    ui.invoke_act_form_save_draft(
        "АКТ-2026-002".into(), "02.04.2026".into(), "cp-draft".into(),
        "чернетка".into(), "cat-draft".into(), "con-draft".into(), "15.05.2026".into(),
    );
    {
        let got = args.borrow();
        assert_eq!(got[0], "АКТ-2026-002", "act-form: save-draft number");
        assert_eq!(got[1], "02.04.2026",   "act-form: save-draft date");
        assert_eq!(got[2], "cp-draft",     "act-form: save-draft cp_id");
        assert_eq!(got[3], "чернетка",     "act-form: save-draft notes");
        assert_eq!(got[4], "cat-draft",    "act-form: save-draft cat_id");
        assert_eq!(got[5], "con-draft",    "act-form: save-draft con_id");
        assert_eq!(got[6], "15.05.2026",   "act-form: save-draft exp_date");
    }

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

    // save(number, date, cp_id, notes, cat_id, con_id, exp_date)
    let args: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
    let a = args.clone();
    ui.on_invoice_form_save(move |num, date, cp, notes, cat, con, exp| {
        *a.borrow_mut() = vec![
            num.into(), date.into(), cp.into(), notes.into(),
            cat.into(), con.into(), exp.into(),
        ];
    });
    ui.invoke_invoice_form_save(
        "ВН-2026-001".into(), "05.04.2026".into(), "cp-uuid".into(),
        "коментар".into(), "cat-uuid".into(), "con-uuid".into(), "20.04.2026".into(),
    );
    {
        let got = args.borrow();
        assert_eq!(got[0], "ВН-2026-001", "invoice-form: save number");
        assert_eq!(got[1], "05.04.2026",  "invoice-form: save date");
        assert_eq!(got[2], "cp-uuid",     "invoice-form: save cp_id");
        assert_eq!(got[3], "коментар",    "invoice-form: save notes");
        assert_eq!(got[4], "cat-uuid",    "invoice-form: save cat_id");
        assert_eq!(got[5], "con-uuid",    "invoice-form: save con_id");
        assert_eq!(got[6], "20.04.2026",  "invoice-form: save exp_date");
    }

    // update(number, date, cp_id, notes, cat_id, con_id, exp_date)
    let args: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
    let a = args.clone();
    ui.on_invoice_form_update(move |num, date, cp, notes, cat, con, exp| {
        *a.borrow_mut() = vec![
            num.into(), date.into(), cp.into(), notes.into(),
            cat.into(), con.into(), exp.into(),
        ];
    });
    ui.invoke_invoice_form_update(
        "ВН-2026-002".into(), "06.04.2026".into(), "cp-update".into(),
        "оновлений коментар".into(), "cat-update".into(), "con-update".into(), "21.04.2026".into(),
    );
    {
        let got = args.borrow();
        assert_eq!(got[0], "ВН-2026-002",        "invoice-form: update number");
        assert_eq!(got[1], "06.04.2026",         "invoice-form: update date");
        assert_eq!(got[2], "cp-update",          "invoice-form: update cp_id");
        assert_eq!(got[3], "оновлений коментар", "invoice-form: update notes");
        assert_eq!(got[4], "cat-update",         "invoice-form: update cat_id");
        assert_eq!(got[5], "con-update",         "invoice-form: update con_id");
        assert_eq!(got[6], "21.04.2026",         "invoice-form: update exp_date");
    }

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
    let got = Rc::new(RefCell::new((
        String::new(),
        String::new(),
        -1i32,
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    )));
    let g = got.clone();
    ui.on_payment_form_save(move |date: SharedString, amount, dir, cp, bank, bank_ref, desc| {
        *g.borrow_mut() = (
            date.to_string(),
            amount.to_string(),
            dir,
            cp.to_string(),
            bank.to_string(),
            bank_ref.to_string(),
            desc.to_string(),
        );
    });
    ui.invoke_payment_form_save(
        "15.04.2026".into(), "5000.00".into(), 0,
        "cp-uuid".into(), "ПриватБанк".into(), "REF-001".into(), "оплата за акт".into(),
    );
    {
        let got = got.borrow();
        assert_eq!(got.0, "15.04.2026",    "payment-form: save date");
        assert_eq!(got.1, "5000.00",       "payment-form: save amount");
        assert_eq!(got.2, 0,               "payment-form: save direction");
        assert_eq!(got.3, "cp-uuid",       "payment-form: save cp_id");
        assert_eq!(got.4, "ПриватБанк",    "payment-form: save bank_name");
        assert_eq!(got.5, "REF-001",       "payment-form: save bank_ref");
        assert_eq!(got.6, "оплата за акт", "payment-form: save description");
    }

    // update
    let got = Rc::new(RefCell::new((
        String::new(),
        String::new(),
        -1i32,
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    )));
    let g = got.clone();
    ui.on_payment_form_update(move |date: SharedString, amount, dir, cp, bank, bank_ref, desc| {
        *g.borrow_mut() = (
            date.to_string(),
            amount.to_string(),
            dir,
            cp.to_string(),
            bank.to_string(),
            bank_ref.to_string(),
            desc.to_string(),
        );
    });
    ui.invoke_payment_form_update(
        "16.04.2026".into(), "7000.00".into(), 2,
        "cp-update".into(), "Монобанк".into(), "REF-777".into(), "оновлення платежу".into(),
    );
    {
        let got = got.borrow();
        assert_eq!(got.0, "16.04.2026",         "payment-form: update date");
        assert_eq!(got.1, "7000.00",            "payment-form: update amount");
        assert_eq!(got.2, 2,                    "payment-form: update direction");
        assert_eq!(got.3, "cp-update",          "payment-form: update cp_id");
        assert_eq!(got.4, "Монобанк",           "payment-form: update bank_name");
        assert_eq!(got.5, "REF-777",            "payment-form: update bank_ref");
        assert_eq!(got.6, "оновлення платежу",  "payment-form: update description");
    }
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

    // save(title, description, priority_idx, due_date, reminder_at)
    let got = Rc::new(RefCell::new((
        String::new(),
        String::new(),
        -1i32,
        String::new(),
        String::new(),
    )));
    let g = got.clone();
    ui.on_task_form_save(move |title: SharedString, description, prio, due, reminder| {
        *g.borrow_mut() = (
            title.to_string(),
            description.to_string(),
            prio,
            due.to_string(),
            reminder.to_string(),
        );
    });
    ui.invoke_task_form_save(
        "Підписати договір".into(), "терміново".into(),
        1, "30.04.2026".into(), "29.04.2026 10:00".into(),
    );
    {
        let got = got.borrow();
        assert_eq!(got.0, "Підписати договір", "task-form: save title");
        assert_eq!(got.1, "терміново",         "task-form: save description");
        assert_eq!(got.2, 1,                   "task-form: save priority");
        assert_eq!(got.3, "30.04.2026",        "task-form: save due_date");
        assert_eq!(got.4, "29.04.2026 10:00",  "task-form: save reminder_at");
    }

    // update(title, description, priority_idx, due_date, reminder_at)
    let got = Rc::new(RefCell::new((
        String::new(),
        String::new(),
        -1i32,
        String::new(),
        String::new(),
    )));
    let g = got.clone();
    ui.on_task_form_update(move |title: SharedString, description, prio, due, reminder| {
        *g.borrow_mut() = (
            title.to_string(),
            description.to_string(),
            prio,
            due.to_string(),
            reminder.to_string(),
        );
    });
    ui.invoke_task_form_update(
        "Оновити акт".into(), "зв'язатися з клієнтом".into(),
        2, "01.05.2026".into(), "30.04.2026 09:30".into(),
    );
    {
        let got = got.borrow();
        assert_eq!(got.0, "Оновити акт",           "task-form: update title");
        assert_eq!(got.1, "зв'язатися з клієнтом", "task-form: update description");
        assert_eq!(got.2, 2,                       "task-form: update priority");
        assert_eq!(got.3, "01.05.2026",            "task-form: update due_date");
        assert_eq!(got.4, "30.04.2026 09:30",      "task-form: update reminder_at");
    }
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
    let got = Rc::new(RefCell::new((
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    )));
    let g = got.clone();
    ui.on_cp_form_save(
        move |name, edrpou, ipn, iban, address, email, phone, notes| {
            *g.borrow_mut() = (
                name.to_string(),
                edrpou.to_string(),
                ipn.to_string(),
                iban.to_string(),
                address.to_string(),
                email.to_string(),
                phone.to_string(),
                notes.to_string(),
            );
        },
    );
    ui.invoke_cp_form_save(
        "ТОВ Ромашка".into(), "12345678".into(), "1234567890".into(),
        "UA12345678901234567890123456789".into(),
        "м. Київ".into(), "info@romashka.ua".into(), "+380441234567".into(), "нотатка".into(),
    );
    {
        let got = got.borrow();
        assert_eq!(got.0, "ТОВ Ромашка",                    "cp-form: save name");
        assert_eq!(got.1, "12345678",                       "cp-form: save edrpou");
        assert_eq!(got.2, "1234567890",                     "cp-form: save ipn");
        assert_eq!(got.3, "UA12345678901234567890123456789","cp-form: save iban");
        assert_eq!(got.4, "м. Київ",                        "cp-form: save address");
        assert_eq!(got.5, "info@romashka.ua",               "cp-form: save email");
        assert_eq!(got.6, "+380441234567",                  "cp-form: save phone");
        assert_eq!(got.7, "нотатка",                        "cp-form: save notes");
    }

    // update(name, edrpou, ipn, iban, phone, email, address, notes)
    let got = Rc::new(RefCell::new((
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    )));
    let g = got.clone();
    ui.on_cp_form_update(move |name, edrpou, ipn, iban, phone, email, address, notes| {
        *g.borrow_mut() = (
            name.to_string(),
            edrpou.to_string(),
            ipn.to_string(),
            iban.to_string(),
            phone.to_string(),
            email.to_string(),
            address.to_string(),
            notes.to_string(),
        );
    });
    ui.invoke_cp_form_update(
        "ТОВ Тест".into(),
        "12345678".into(),
        "1234567890".into(),
        "UA123456789".into(),
        "+380991234567".into(),
        "test@test.com".into(),
        "вул. Хрещатик 1".into(),
        "нотатка".into(),
    );
    {
        let got = got.borrow();
        assert_eq!(got.0, "ТОВ Тест",         "cp-form: update name");
        assert_eq!(got.1, "12345678",         "cp-form: update edrpou");
        assert_eq!(got.2, "1234567890",       "cp-form: update ipn");
        assert_eq!(got.3, "UA123456789",      "cp-form: update iban");
        assert_eq!(got.4, "+380991234567",    "cp-form: update phone");
        assert_eq!(got.5, "test@test.com",    "cp-form: update email");
        assert_eq!(got.6, "вул. Хрещатик 1",  "cp-form: update address");
        assert_eq!(got.7, "нотатка",          "cp-form: update notes");
    }
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
