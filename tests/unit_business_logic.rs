use std::sync::Arc;

use acta::import::bank_csv::{BankStatementParser, OschadbankCsvParser, SenseBankCsvParser};
use acta::models::payment::PaymentDirection;
use acta::notifications::reminder_loop;
use acta::pdf::generator::{amount_to_words, ensure_invoice_output_dir, ensure_output_dir};
use rust_decimal_macros::dec;
use sqlx::postgres::PgPoolOptions;
use tokio::time::Duration;

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
fn pdf_amount_to_words_handles_zero() {
    assert_eq!(amount_to_words(&dec!(0.00)), "нуль гривень 00 копійок");
}

#[test]
fn pdf_amount_to_words_handles_teens_and_feminine_forms() {
    assert_eq!(amount_to_words(&dec!(11.00)), "одинадцять гривень 00 копійок");
    assert_eq!(amount_to_words(&dec!(21.00)), "двадцять одна гривня 00 копійок");
}

#[test]
fn pdf_output_dir_sanitizes_unsafe_characters() {
    let path = ensure_output_dir("АКТ\\2026:001").unwrap();
    let name = path.file_name().unwrap().to_str().unwrap();

    assert!(!name.contains('\\'));
    assert!(!name.contains(':'));
}

#[test]
fn pdf_invoice_output_dir_uses_misc_for_non_standard_number() {
    let path = ensure_invoice_output_dir("INVOICE").unwrap();
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
