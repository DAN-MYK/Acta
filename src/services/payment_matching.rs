use std::collections::BTreeSet;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::models::payment::PaymentDirection;

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
    pub direction: PaymentDirection,
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
    pub match_date: Option<NaiveDate>,
}

impl MatchCandidate {
    pub fn act(
        document_id: Uuid,
        open_amount: Decimal,
        counterparty_iban: Option<String>,
        title: impl Into<String>,
        match_date: Option<NaiveDate>,
    ) -> Self {
        Self {
            document_id,
            document_kind: MatchDocumentKind::Act,
            open_amount,
            counterparty_iban,
            title: title.into(),
            match_date,
        }
    }

    pub fn invoice(
        document_id: Uuid,
        open_amount: Decimal,
        counterparty_iban: Option<String>,
        title: impl Into<String>,
        match_date: Option<NaiveDate>,
    ) -> Self {
        Self {
            document_id,
            document_kind: MatchDocumentKind::Invoice,
            open_amount,
            counterparty_iban,
            title: title.into(),
            match_date,
        }
    }
}

/// Деталізований скоринг для прозорого preview/review matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchScore {
    pub total: i32,
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
    None,
}

/// Рішення сервісу matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchDecision {
    Exact(ScoredMatchCandidate),
    Ambiguous(Vec<ScoredMatchCandidate>),
    None,
}

impl MatchDecision {
    pub fn kind(&self) -> MatchKind {
        match self {
            MatchDecision::Exact(_) => MatchKind::Exact,
            MatchDecision::Ambiguous(_) => MatchKind::Ambiguous,
            MatchDecision::None => MatchKind::None,
        }
    }

    pub fn is_exact(&self) -> bool {
        matches!(self, MatchDecision::Exact(_))
    }

    pub fn best_match_id(&self) -> Option<Uuid> {
        match self {
            MatchDecision::Exact(scored) => Some(scored.candidate.document_id),
            MatchDecision::Ambiguous(candidates) => {
                candidates.first().map(|scored| scored.candidate.document_id)
            }
            MatchDecision::None => None,
        }
    }
}

/// Вибрати найкращий кандидат із прозорим result model для preview/auto-match.
pub fn choose_best_match(
    input: &PaymentMatchInput,
    candidates: &[MatchCandidate],
) -> MatchDecision {
    let scored = score_match_candidates(input, candidates);

    if scored.is_empty() {
        return MatchDecision::None;
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

/// Повернути exact-amount кандидатів із прозорим скорингом у стабільному порядку.
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

fn score_candidate(input: &PaymentMatchInput, candidate: MatchCandidate) -> ScoredMatchCandidate {
    let exact_amount = candidate.open_amount == input.amount;
    let same_iban = exact_amount
        && match (
            normalize_optional_iban(input.counterparty_iban.as_deref()),
            normalize_optional_iban(candidate.counterparty_iban.as_deref()),
        ) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        };

    let normalized_title = normalize_text(&candidate.title);
    let reference_hit = exact_amount
        && input
            .bank_ref
            .as_deref()
            .map(normalize_text)
            .filter(|reference| !reference.is_empty())
            .is_some_and(|reference| normalized_title.contains(&reference));

    let text_hits = if exact_amount {
        intersecting_tokens(&input.description, &candidate.title).len()
    } else {
        0
    };

    let days_distance = candidate
        .match_date
        .map(|date| (date - input.date).num_days().abs())
        .unwrap_or(365);

    let total = if exact_amount {
        100
            + if same_iban { 40 } else { 0 }
            + if reference_hit { 20 } else { 0 }
            + (text_hits.min(3) as i32 * 10)
            + (10 - days_distance.min(10) as i32)
    } else {
        0
    };

    ScoredMatchCandidate {
        candidate,
        score: MatchScore {
            total,
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
    value.map(|iban| {
        iban.chars()
            .filter(|ch| !ch.is_whitespace())
            .flat_map(char::to_uppercase)
            .collect::<String>()
    })
    .filter(|iban| !iban.is_empty())
}
