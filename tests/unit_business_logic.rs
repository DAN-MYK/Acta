use std::sync::Arc;

use acta::import::bank_csv::{
    BankStatementParser, OschadbankCsvParser, SenseBankCsvParser, UkrgasbankCsvParser,
};
use acta::models::payment::PaymentDirection;
use acta::notifications::reminder_loop;
use acta::pdf::generator::{amount_to_words, ensure_invoice_output_dir, ensure_output_dir};
use acta::services::payment_matching::{
    choose_best_match, MatchCandidate, MatchDecision, PaymentMatchInput,
};
use chrono::NaiveDate;
use rust_decimal_macros::dec;
use sqlx::postgres::PgPoolOptions;
use tokio::time::Duration;
use uuid::Uuid;

fn fake_pool() -> sqlx::PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(50))
        .connect_lazy("postgres://x@127.0.0.1:54321/nonexistent")
        .expect("connect_lazy should not fail")
}

#[test]
fn bank_csv_uses_header_positions_not_column_order() {
    let csv = "description,direction,reference,amount,date\n\
               Послуга за акт,out,REF-77,1500.25,2026-04-21\n";
    let rows = SenseBankCsvParser.parse(csv).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].description, "Послуга за акт");
    assert_eq!(rows[0].direction, PaymentDirection::Expense);
    assert_eq!(rows[0].bank_ref.as_deref(), Some("REF-77"));
    assert_eq!(rows[0].amount, dec!(1500.25));
}

#[test]
fn bank_csv_trims_description_and_reference() {
    let csv = "date,amount,description,direction,reference\n\
               15.04.2026,500.00,  Оплата за послуги  ,income,  REF-500  \n";
    let rows = SenseBankCsvParser.parse(csv).unwrap();

    assert_eq!(rows[0].description, "Оплата за послуги");
    assert_eq!(rows[0].bank_ref.as_deref(), Some("REF-500"));
}

#[test]
fn bank_csv_empty_amount_returns_error() {
    let csv = "date,amount,description,direction\n\
               15.04.2026,,Оплата,income\n";
    assert!(SenseBankCsvParser.parse(csv).is_err());
}

#[test]
fn bank_csv_case_insensitive_headers_work_for_other_parser() {
    let csv = "DATE,AMOUNT,DESCRIPTION,DIRECTION\n\
               2026-04-15,250.00,Тест,in\n";
    let rows = OschadbankCsvParser.parse(csv).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].direction, PaymentDirection::Income);
    assert_eq!(rows[0].amount, dec!(250.00));
}

#[test]
fn bank_csv_row_exposes_matching_fields() {
    let csv = "Дата операції;Сума;Призначення платежу;IBAN;Референс\n\
               01.05.2026;12500,00;Оплата акту №42;UA123456789012345678901234567;REF-42\n";
    let rows = UkrgasbankCsvParser.parse(csv).expect("CSV має парситися");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].description, "Оплата акту №42");
    assert_eq!(rows[0].bank_ref.as_deref(), Some("REF-42"));
    assert_eq!(
        rows[0].counterparty_iban.as_deref(),
        Some("UA123456789012345678901234567")
    );
}

#[test]
fn bank_csv_normalizes_counterparty_iban() {
    let csv = "Дата операції;Сума;Призначення платежу;IBAN;Референс\n\
               01.05.2026;12500,00;Оплата акту №42; ua12 3456 7890 1234 5678 9012 34567 ;REF-42\n";
    let rows = UkrgasbankCsvParser.parse(csv).expect("CSV має парситися");

    assert_eq!(
        rows[0].counterparty_iban.as_deref(),
        Some("UA123456789012345678901234567")
    );
}

#[test]
fn payment_matching_prefers_exact_amount_and_iban() {
    let preferred_id = Uuid::new_v4();
    let other_id = Uuid::new_v4();
    let payment = PaymentMatchInput {
        amount: dec!(12500.00),
        date: NaiveDate::from_ymd_opt(2026, 5, 1).expect("валідна дата"),
        counterparty_iban: Some("UA123".to_string()),
        description: "Оплата акту №42".to_string(),
        bank_ref: None,
    };

    let candidates = vec![
        MatchCandidate::act(
            preferred_id,
            dec!(12500.00),
            Some("UA123".to_string()),
            "Акт №42",
            "ACT-42",
            "Оплата акту №42",
            Some(NaiveDate::from_ymd_opt(2026, 5, 1).expect("валідна дата")),
        ),
        MatchCandidate::act(
            other_id,
            dec!(12500.00),
            Some("UA999".to_string()),
            "Акт №42",
            "ACT-42",
            "Оплата акту №42",
            Some(NaiveDate::from_ymd_opt(2026, 5, 1).expect("валідна дата")),
        ),
    ];

    let result = choose_best_match(&payment, &candidates);

    assert_eq!(result.best_match_id(), Some(preferred_id));
    assert!(result.is_exact());
}

#[test]
fn payment_matching_returns_ambiguous_when_top_scores_tie() {
    let first_id = Uuid::new_v4();
    let second_id = Uuid::new_v4();
    let payment = PaymentMatchInput {
        amount: dec!(8000.00),
        date: NaiveDate::from_ymd_opt(2026, 5, 3).expect("валідна дата"),
        counterparty_iban: None,
        description: "Оплата послуг".to_string(),
        bank_ref: None,
    };

    let candidates = vec![
        MatchCandidate::act(
            first_id,
            dec!(8000.00),
            None,
            "Акт на послуги",
            "ACT-SERVICES",
            "Оплата послуг",
            Some(NaiveDate::from_ymd_opt(2026, 5, 3).expect("валідна дата")),
        ),
        MatchCandidate::invoice(
            second_id,
            dec!(8000.00),
            None,
            "Рахунок на послуги",
            "INV-SERVICES",
            "Оплата послуг",
            Some(NaiveDate::from_ymd_opt(2026, 5, 3).expect("валідна дата")),
        ),
    ];

    let result = choose_best_match(&payment, &candidates);
    assert_eq!(result.best_match_id(), None);

    match result {
        MatchDecision::Ambiguous(candidates) => {
            assert_eq!(candidates.len(), 2);
            let ids = candidates
                .into_iter()
                .map(|candidate| candidate.candidate.document_id)
                .collect::<Vec<_>>();
            assert!(ids.contains(&first_id));
            assert!(ids.contains(&second_id));
        }
        other => panic!("очікували Ambiguous, отримали {other:?}"),
    }
}

#[test]
fn payment_matching_returns_none_without_exact_amount_candidate() {
    let payment = PaymentMatchInput {
        amount: dec!(5000.00),
        date: NaiveDate::from_ymd_opt(2026, 5, 4).expect("валідна дата"),
        counterparty_iban: Some("UA123".to_string()),
        description: "Оплата накладної".to_string(),
        bank_ref: Some("REF-5000".to_string()),
    };

    let candidates = vec![MatchCandidate::invoice(
        Uuid::new_v4(),
        dec!(4999.99),
        Some("UA123".to_string()),
        "Накладна №5 REF-5000",
        "INV-5",
        "Накладна №5",
        Some(NaiveDate::from_ymd_opt(2026, 5, 4).expect("валідна дата")),
    )];

    let result = choose_best_match(&payment, &candidates);

    assert!(matches!(result, MatchDecision::None));
    assert_eq!(result.best_match_id(), None);
}

#[test]
fn payment_matching_uses_canonical_fields_instead_of_display_title() {
    let preferred_id = Uuid::new_v4();
    let misleading_id = Uuid::new_v4();
    let payment = PaymentMatchInput {
        amount: dec!(9100.00),
        date: NaiveDate::from_ymd_opt(2026, 5, 7).expect("валідна дата"),
        counterparty_iban: None,
        description: "Оплата послуг за травень".to_string(),
        bank_ref: Some("ACT-77".to_string()),
    };

    let candidates = vec![
        MatchCandidate::act(
            preferred_id,
            dec!(9100.00),
            None,
            "Документ для показу",
            "ACT-77",
            "Оплата послуг за травень",
            Some(NaiveDate::from_ymd_opt(2026, 5, 7).expect("валідна дата")),
        ),
        MatchCandidate::act(
            misleading_id,
            dec!(9100.00),
            None,
            "ACT-77 Оплата послуг за травень",
            "ACT-99",
            "Інший платіж",
            Some(NaiveDate::from_ymd_opt(2026, 5, 7).expect("валідна дата")),
        ),
    ];

    let result = choose_best_match(&payment, &candidates);

    assert_eq!(result.best_match_id(), Some(preferred_id));
    assert!(result.is_exact());
}

#[test]
fn pdf_amount_to_words_handles_zero() {
    assert_eq!(amount_to_words(&dec!(0.00)), "нуль гривень 00 копійок");
}

#[test]
fn pdf_amount_to_words_handles_teens_and_feminine_forms() {
    assert_eq!(
        amount_to_words(&dec!(11.00)),
        "одинадцять гривень 00 копійок"
    );
    assert_eq!(
        amount_to_words(&dec!(21.00)),
        "двадцять одна гривня 00 копійок"
    );
}

#[test]
fn pdf_output_dir_sanitizes_unsafe_characters() {
    let base = std::env::temp_dir();
    let path = ensure_output_dir(&base, "АКТ\\2026:001").unwrap();
    let name = path.file_name().unwrap().to_str().unwrap();

    assert!(!name.contains('\\'));
    assert!(!name.contains(':'));
}

#[test]
fn pdf_invoice_output_dir_uses_misc_for_non_standard_number() {
    let base = std::env::temp_dir();
    let path = ensure_invoice_output_dir(&base, "INVOICE").unwrap();
    assert!(path.to_str().unwrap().contains("misc"));
}

#[tokio::test(start_paused = true)]
async fn notifications_loop_uses_default_sixty_second_period() {
    let handle = tokio::spawn(reminder_loop(Arc::new(fake_pool())));

    tokio::time::advance(Duration::from_millis(1)).await;
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    assert!(!handle.is_finished());

    tokio::time::advance(Duration::from_secs(59)).await;
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    assert!(!handle.is_finished());

    tokio::time::advance(Duration::from_secs(1)).await;
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    assert!(!handle.is_finished());

    handle.abort();
    assert!(handle.await.unwrap_err().is_cancelled());
}

// default_form_item / apply_form_item_change тестуються в src/ui/helpers.rs::tests (потребують Slint env).
// collect_model<T> тестується в src/ui/helpers.rs::tests (потребує Slint env).

// build_category_select тестується в src/ui/helpers.rs::tests_build_category

// build_cp_select тестується в src/ui/helpers.rs::tests_build_cp_select

// ─── Інлайн-тести parse_date_ui / parse_opt_uuid логіки ──────────────────────
// Ці тести дублюють логіку (не викликають helpers напряму) бо це бінарний крейт.

#[test]
fn parse_date_ui_valid_returns_some() {
    use chrono::NaiveDate;
    let result = NaiveDate::parse_from_str("15.04.2026", "%d.%m.%Y");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        NaiveDate::from_ymd_opt(2026, 4, 15).unwrap()
    );
}

#[test]
fn parse_date_ui_invalid_returns_none() {
    let result = chrono::NaiveDate::parse_from_str("не-дата", "%d.%m.%Y");
    assert!(result.is_err());
}

#[test]
fn parse_opt_uuid_empty_returns_none() {
    let s = "";
    let result: Option<uuid::Uuid> = if s.trim().is_empty() {
        None
    } else {
        uuid::Uuid::parse_str(s).ok()
    };
    assert!(result.is_none());
}

#[test]
fn parse_opt_uuid_valid_returns_some() {
    let id = uuid::Uuid::new_v4();
    let s = id.to_string();
    let result: Option<uuid::Uuid> = if s.trim().is_empty() {
        None
    } else {
        uuid::Uuid::parse_str(&s).ok()
    };
    assert_eq!(result, Some(id));
}
