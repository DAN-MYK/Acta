use std::collections::BTreeSet;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use uuid::Uuid;

/// Тип документа, з яким може зіставлятися платіж.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchDocumentKind {
    Act,
    Invoice,
}

/// Вхідні дані для автозіставлення одного платежу.
#[derive(Debug, Clone, PartialEq)]
pub struct PaymentMatchInput {
    pub amount: Decimal,
    pub date: NaiveDate,
    pub counterparty_iban: Option<String>,
    pub description: String,
    pub bank_ref: Option<String>,
}

/// Кандидат на зіставлення з відкритого документа.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchCandidate {
    pub document_id: Uuid,
    pub document_kind: MatchDocumentKind,
    pub open_amount: Decimal,
    pub counterparty_iban: Option<String>,
    pub title: String,
    pub reference_text: Option<String>,
    pub match_text: Option<String>,
    pub match_date: Option<NaiveDate>,
}

impl MatchCandidate {
    pub fn act(
        document_id: Uuid,
        open_amount: Decimal,
        counterparty_iban: Option<String>,
        title: impl Into<String>,
        reference_text: impl Into<String>,
        match_text: impl Into<String>,
        match_date: Option<NaiveDate>,
    ) -> Self {
        Self {
            document_id,
            document_kind: MatchDocumentKind::Act,
            open_amount,
            counterparty_iban,
            title: title.into(),
            reference_text: optional_text(reference_text),
            match_text: optional_text(match_text),
            match_date,
        }
    }

    pub fn invoice(
        document_id: Uuid,
        open_amount: Decimal,
        counterparty_iban: Option<String>,
        title: impl Into<String>,
        reference_text: impl Into<String>,
        match_text: impl Into<String>,
        match_date: Option<NaiveDate>,
    ) -> Self {
        Self {
            document_id,
            document_kind: MatchDocumentKind::Invoice,
            open_amount,
            counterparty_iban,
            title: title.into(),
            reference_text: optional_text(reference_text),
            match_text: optional_text(match_text),
            match_date,
        }
    }
}

/// Деталізований скоринг для прозорого preview/review matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchScore {
    pub total: i32,
    pub amount_fits: bool,
    pub exact_amount: bool,
    pub same_iban: bool,
    pub reference_hit: bool,
    pub text_hits: usize,
    pub days_distance: i64,
}

/// Кандидат разом із деталями його скорингу.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoredMatchCandidate {
    pub candidate: MatchCandidate,
    pub score: MatchScore,
}

/// Підсумковий тип рішення автозіставлення.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    Exact,
    Ambiguous,
    Split,
    None,
}

/// Рішення сервісу matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchDecision {
    Exact(ScoredMatchCandidate),
    Ambiguous(Vec<ScoredMatchCandidate>),
    Split(Vec<ScoredMatchCandidate>),
    None,
}

impl MatchDecision {
    pub fn kind(&self) -> MatchKind {
        match self {
            MatchDecision::Exact(_) => MatchKind::Exact,
            MatchDecision::Ambiguous(_) => MatchKind::Ambiguous,
            MatchDecision::Split(_) => MatchKind::Split,
            MatchDecision::None => MatchKind::None,
        }
    }

    pub fn is_exact(&self) -> bool {
        matches!(self, MatchDecision::Exact(_))
    }

    pub fn best_match_id(&self) -> Option<Uuid> {
        match self {
            MatchDecision::Exact(scored) => Some(scored.candidate.document_id),
            MatchDecision::Ambiguous(_) | MatchDecision::Split(_) => None,
            MatchDecision::None => None,
        }
    }
}

/// Повернути відсортований список exact-amount кандидатів з деталями скорингу.
pub fn score_match_candidates(
    input: &PaymentMatchInput,
    candidates: &[MatchCandidate],
) -> Vec<ScoredMatchCandidate> {
    let mut scored: Vec<_> = candidates
        .iter()
        .cloned()
        .map(|candidate| score_candidate(input, candidate))
        .filter(|candidate| candidate.score.exact_amount)
        .collect();

    scored.sort_by(|left, right| {
        right
            .score
            .total
            .cmp(&left.score.total)
            .then_with(|| left.score.days_distance.cmp(&right.score.days_distance))
            .then_with(|| left.candidate.title.cmp(&right.candidate.title))
            .then_with(|| left.candidate.document_id.cmp(&right.candidate.document_id))
    });

    scored
}

/// Вибрати найкращий кандидат із прозорим result model для preview/auto-match.
pub fn choose_best_match(
    input: &PaymentMatchInput,
    candidates: &[MatchCandidate],
) -> MatchDecision {
    let scored: Vec<_> = score_match_candidates(input, candidates)
        .into_iter()
        .filter(|candidate| candidate.score.total > 100)
        .collect();

    if scored.is_empty() {
        return choose_split_match(input, candidates)
            .map(MatchDecision::Split)
            .unwrap_or(MatchDecision::None);
    }

    let top_score = scored[0].score.total;
    let tied: Vec<_> = scored
        .into_iter()
        .take_while(|candidate| candidate.score.total == top_score)
        .collect();

    if tied.len() == 1 {
        match tied.into_iter().next() {
            Some(candidate) => MatchDecision::Exact(candidate),
            None => MatchDecision::None,
        }
    } else {
        MatchDecision::Ambiguous(tied)
    }
}

pub fn choose_split_match(
    input: &PaymentMatchInput,
    candidates: &[MatchCandidate],
) -> Option<Vec<ScoredMatchCandidate>> {
    let mut scored: Vec<_> = candidates
        .iter()
        .cloned()
        .map(|candidate| score_candidate(input, candidate))
        .filter(|candidate| {
            candidate.score.amount_fits
                && !candidate.score.exact_amount
                && candidate.score.total > 0
        })
        .collect();

    scored.sort_by(|left, right| {
        right
            .score
            .total
            .cmp(&left.score.total)
            .then_with(|| left.score.days_distance.cmp(&right.score.days_distance))
            .then_with(|| left.candidate.title.cmp(&right.candidate.title))
            .then_with(|| left.candidate.document_id.cmp(&right.candidate.document_id))
    });

    let mut best: Option<(i32, usize, Vec<ScoredMatchCandidate>)> = None;
    let pool: Vec<_> = scored.into_iter().take(6).collect();
    let mut current = Vec::new();
    choose_split_match_recursive(&pool, 0, input.amount, 0, &mut current, &mut best);

    best.map(|(_, _, candidates)| candidates)
}

fn choose_split_match_recursive(
    pool: &[ScoredMatchCandidate],
    index: usize,
    remaining: Decimal,
    score_sum: i32,
    current: &mut Vec<ScoredMatchCandidate>,
    best: &mut Option<(i32, usize, Vec<ScoredMatchCandidate>)>,
) {
    if remaining.is_zero() {
        if current.len() < 2 {
            return;
        }

        let candidate = (score_sum, current.len(), current.clone());
        match best {
            Some((best_score, best_len, _))
                if *best_score > candidate.0
                    || (*best_score == candidate.0 && *best_len <= candidate.1) => {}
            _ => *best = Some(candidate),
        }
        return;
    }

    if index >= pool.len() || remaining.is_sign_negative() {
        return;
    }

    choose_split_match_recursive(pool, index + 1, remaining, score_sum, current, best);

    let candidate = &pool[index];
    if candidate.candidate.open_amount <= remaining {
        current.push(candidate.clone());
        choose_split_match_recursive(
            pool,
            index + 1,
            remaining - candidate.candidate.open_amount,
            score_sum + candidate.score.total,
            current,
            best,
        );
        current.pop();
    }
}

fn score_candidate(input: &PaymentMatchInput, candidate: MatchCandidate) -> ScoredMatchCandidate {
    let amount_fits = candidate.open_amount <= input.amount;
    let exact_amount = candidate.open_amount == input.amount;
    let same_iban = amount_fits
        && match (
            normalize_optional_iban(input.counterparty_iban.as_deref()),
            normalize_optional_iban(candidate.counterparty_iban.as_deref()),
        ) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        };

    let reference_hit = amount_fits
        && input
            .bank_ref
            .as_deref()
            .map(normalize_text)
            .filter(|reference| !reference.is_empty())
            .zip(
                candidate
                    .reference_text
                    .as_deref()
                    .map(normalize_text)
                    .filter(|text| !text.is_empty()),
            )
            .is_some_and(|(reference, candidate_reference)| {
                candidate_reference.contains(&reference)
            });

    let text_hits = if amount_fits {
        intersecting_tokens(
            &input.description,
            candidate.match_text.as_deref().unwrap_or(""),
        )
        .len()
    } else {
        0
    };

    let days_distance = candidate
        .match_date
        .map(|date| (date - input.date).num_days().abs())
        .unwrap_or(365);

    let total = if exact_amount {
        100 + if same_iban { 40 } else { 0 }
            + if reference_hit { 20 } else { 0 }
            + (text_hits.min(3) as i32 * 10)
            + (10 - days_distance.min(10) as i32)
    } else if amount_fits {
        40 + if same_iban { 25 } else { 0 }
            + if reference_hit { 15 } else { 0 }
            + (text_hits.min(3) as i32 * 8)
            + (8 - days_distance.min(8) as i32)
    } else {
        0
    };

    ScoredMatchCandidate {
        candidate,
        score: MatchScore {
            total,
            amount_fits,
            exact_amount,
            same_iban,
            reference_hit,
            text_hits,
            days_distance,
        },
    }
}

fn intersecting_tokens(left: &str, right: &str) -> BTreeSet<String> {
    let left_tokens = tokenize(left);
    let right_tokens = tokenize(right);

    left_tokens
        .intersection(&right_tokens)
        .cloned()
        .collect::<BTreeSet<_>>()
}

fn tokenize(value: &str) -> BTreeSet<String> {
    value
        .to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| token.chars().count() >= 2)
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_text(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|ch| ch.is_alphanumeric())
        .collect()
}

fn normalize_optional_iban(value: Option<&str>) -> Option<String> {
    value
        .map(|iban| {
            iban.chars()
                .filter(|ch| !ch.is_whitespace())
                .flat_map(char::to_uppercase)
                .collect::<String>()
        })
        .filter(|iban| !iban.is_empty())
}

fn optional_text(value: impl Into<String>) -> Option<String> {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn choose_best_match_returns_split_recommendation_for_covering_candidates() {
        let input = PaymentMatchInput {
            amount: dec!(3000.00),
            date: NaiveDate::from_ymd_opt(2026, 5, 2).expect("валідна дата"),
            counterparty_iban: Some("UA123456789012345678901234567".to_string()),
            description: "Оплата за акт ACT-009 і накладну INV-007".to_string(),
            bank_ref: Some("PAY-77".to_string()),
        };

        let decision = choose_best_match(
            &input,
            &[
                MatchCandidate::invoice(
                    Uuid::new_v4(),
                    dec!(1500.00),
                    Some("UA123456789012345678901234567".to_string()),
                    "Накладна INV-007",
                    "PAY-77 INV-007",
                    "Оплата накладної INV-007",
                    Some(NaiveDate::from_ymd_opt(2026, 5, 1).expect("валідна дата")),
                ),
                MatchCandidate::act(
                    Uuid::new_v4(),
                    dec!(1500.00),
                    Some("UA123456789012345678901234567".to_string()),
                    "Акт ACT-009",
                    "PAY-77 ACT-009",
                    "Оплата акту ACT-009",
                    Some(NaiveDate::from_ymd_opt(2026, 5, 3).expect("валідна дата")),
                ),
                MatchCandidate::invoice(
                    Uuid::new_v4(),
                    dec!(3000.00),
                    None,
                    "Нерелевантна накладна",
                    "",
                    "Інший документ",
                    Some(NaiveDate::from_ymd_opt(2026, 4, 1).expect("валідна дата")),
                ),
            ],
        );

        match decision {
            MatchDecision::Split(candidates) => {
                assert_eq!(candidates.len(), 2);
                assert_eq!(
                    candidates
                        .iter()
                        .map(|candidate| candidate.candidate.open_amount)
                        .sum::<Decimal>(),
                    dec!(3000.00)
                );
                assert!(candidates
                    .iter()
                    .all(|candidate| candidate.score.amount_fits));
                assert!(candidates
                    .iter()
                    .all(|candidate| !candidate.score.exact_amount));
            }
            other => panic!("очікували split-рекомендацію, отримали {other:?}"),
        }
    }
}
