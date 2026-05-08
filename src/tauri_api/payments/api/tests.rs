
use super::*;
use crate::services::payment_matching::{MatchCandidate, MatchScore};
use rust_decimal_macros::dec;

#[test]
fn parse_payment_date_dd_mm_yyyy() {
    let d = parse_payment_date("15.04.2026").unwrap();
    assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
}

#[test]
fn parse_payment_date_iso_format() {
    let d = parse_payment_date("2026-04-15").unwrap();
    assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
}

#[test]
fn parse_payment_date_trims_whitespace() {
    let d = parse_payment_date("  15.04.2026  ").unwrap();
    assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
}

#[test]
fn parse_payment_date_invalid_returns_err() {
    assert!(parse_payment_date("not-a-date").is_err());
    assert!(parse_payment_date("32.01.2026").is_err());
}

#[test]
fn parse_payment_amount_standard_dot() {
    assert_eq!(parse_payment_amount("1500.00").unwrap(), dec!(1500.00));
}

#[test]
fn parse_payment_amount_comma_separator() {
    assert_eq!(parse_payment_amount("1500,50").unwrap(), dec!(1500.50));
}

#[test]
fn parse_payment_amount_zero_returns_err() {
    assert!(parse_payment_amount("0").is_err());
    assert!(parse_payment_amount("0.00").is_err());
}

#[test]
fn parse_payment_amount_negative_returns_err() {
    assert!(parse_payment_amount("-100.00").is_err());
}

#[test]
fn parse_payment_amount_invalid_returns_err() {
    assert!(parse_payment_amount("abc").is_err());
}

#[test]
fn parse_payment_direction_income() {
    assert!(matches!(
        parse_payment_direction("income").unwrap(),
        PaymentDirection::Income
    ));
}

#[test]
fn parse_payment_direction_expense() {
    assert!(matches!(
        parse_payment_direction("expense").unwrap(),
        PaymentDirection::Expense
    ));
}

#[test]
fn parse_payment_direction_invalid_returns_err() {
    assert!(parse_payment_direction("credit").is_err());
    assert!(parse_payment_direction("in").is_err());
    assert!(parse_payment_direction("").is_err());
}

#[test]
fn parse_optional_counterparty_id_empty_is_none() {
    assert_eq!(parse_optional_counterparty_id("").unwrap(), None);
    assert_eq!(parse_optional_counterparty_id("  ").unwrap(), None);
}

#[test]
fn parse_optional_counterparty_id_valid_uuid() {
    let id = Uuid::new_v4();
    assert_eq!(
        parse_optional_counterparty_id(&id.to_string()).unwrap(),
        Some(id)
    );
}

#[test]
fn parse_optional_counterparty_id_invalid_returns_err() {
    assert!(parse_optional_counterparty_id("not-a-uuid").is_err());
}

#[test]
fn parse_optional_payment_id_empty_is_none() {
    assert_eq!(parse_optional_payment_id("").unwrap(), None);
}

#[test]
fn parse_optional_payment_id_valid_uuid() {
    let id = Uuid::new_v4();
    assert_eq!(
        parse_optional_payment_id(&id.to_string()).unwrap(),
        Some(id)
    );
}

#[test]
fn direction_to_str_maps_correctly() {
    assert_eq!(direction_to_str(&PaymentDirection::Income), "in");
    assert_eq!(direction_to_str(&PaymentDirection::Expense), "out");
}

#[test]
fn trimmed_option_empty_is_none() {
    assert_eq!(trimmed_option(""), None);
    assert_eq!(trimmed_option("   "), None);
}

#[test]
fn trimmed_option_non_empty_is_some() {
    assert_eq!(trimmed_option("  hello  "), Some("hello".to_string()));
}

#[test]
fn format_decimal_ua_basic() {
    assert_eq!(format_decimal_ua(dec!(1234.56)), "1\u{00a0}234,56");
}

#[test]
fn format_decimal_ua_small() {
    assert_eq!(format_decimal_ua(dec!(5.00)), "5,00");
}

#[test]
fn format_decimal_ua_negative() {
    assert_eq!(format_decimal_ua(dec!(-1234.56)), "-1\u{00a0}234,56");
}

#[test]
fn format_decimal_ua_zero() {
    assert_eq!(format_decimal_ua(dec!(0)), "0,00");
}

#[test]
fn payment_match_preview_helpers_map_exact_decision() {
    let decision = MatchDecision::Exact(ScoredMatchCandidate {
            candidate: MatchCandidate::act(
                Uuid::new_v4(),
                dec!(1250.00),
                Some("UA123".to_string()),
                "ТОВ Клієнт №42",
                "ACT-42",
                "Оплата акту №42",
                Some(
                    NaiveDate::from_ymd_opt(2026, 5, 1)
                        .expect("валідна дата"),
                ),
            ),
            score: MatchScore {
                total: 170,
                amount_fits: true,
                exact_amount: true,
                same_iban: true,
                reference_hit: true,
                text_hits: 2,
                days_distance: 0,
            },
        });

    let recommendation = exact_recommendation(&decision).expect(
            "exact decision має повертати recommendation",
        );

    assert_eq!(match_kind_to_str(decision.kind()), "exact");
    assert_eq!(recommendation.document_kind, "act");
    assert_eq!(
        recommendation.title,
        "ТОВ Клієнт №42"
    );
    assert_eq!(recommendation.amount_str, "1\u{00a0}250,00");
}

#[test]
fn payment_match_preview_helpers_map_candidate_scores() {
    let dto = scored_candidate_to_dto(ScoredMatchCandidate {
            candidate: MatchCandidate::invoice(
                Uuid::new_v4(),
                dec!(980.00),
                None,
                "Рахунок №7",
                "INV-7",
                "Оплата послуг",
                Some(
                    NaiveDate::from_ymd_opt(2026, 5, 3)
                        .expect("валідна дата"),
                ),
            ),
            score: MatchScore {
                total: 130,
                amount_fits: true,
                exact_amount: true,
                same_iban: false,
                reference_hit: false,
                text_hits: 1,
                days_distance: 2,
            },
        });

    assert_eq!(dto.document_kind, "invoice");
    assert_eq!(dto.open_amount_str, "980,00");
    assert_eq!(dto.total_score, 130);
    assert_eq!(dto.text_hits, 1);
    assert_eq!(dto.days_distance, 2);
}

#[test]
fn payment_match_preview_helpers_map_split_decision_kind() {
    let decision = MatchDecision::Split(vec![ScoredMatchCandidate {
            candidate: MatchCandidate::invoice(
                Uuid::new_v4(),
                dec!(1500.00),
                None,
                "Накладна INV-007",
                "INV-7",
                "Оплата накладної",
                Some(
                    NaiveDate::from_ymd_opt(2026, 5, 3)
                        .expect("валідна дата"),
                ),
            ),
            score: MatchScore {
                total: 88,
                amount_fits: true,
                exact_amount: false,
                same_iban: true,
                reference_hit: false,
                text_hits: 1,
                days_distance: 2,
            },
        }]);

    assert_eq!(match_kind_to_str(decision.kind()), "split");
    assert!(exact_recommendation(&decision).is_none());
}

#[test]
fn compile_check_public_function_signatures() {
    let _ = payments_list;
    let _ = payments_import_latest_csv;
    let _ = payments_sync_bank;
    let _ = payments_open_manual_template;
    let _ = payment_create_or_update;
    let _ = payment_reconcile;
    let _ = payment_reconcile_split;
    let _ = payment_unreconcile;
    let _ = payment_match_preview;
    let _ = payment_match_apply_auto;
}
