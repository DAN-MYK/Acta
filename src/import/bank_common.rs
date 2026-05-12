//! Спільні утиліти для парсингу банківських виписок з різних форматів (CSV, XLSX).
//!
//! Цей модуль містить header-aliases, normalize helpers, decimal/date parsers
//! та `HeaderLayout`. Формат-специфічні модулі (`bank_csv`, `bank_xlsx`) лише
//! читають дані у вигляді рядків / комірок і викликають утиліти з цього модуля.

use anyhow::{anyhow, bail, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::models::payment::PaymentDirection;

/// Уніфікована модель одного рядка виписки після парсингу.
#[derive(Debug, Clone)]
pub struct ParsedBankRow {
    pub date: NaiveDate,
    pub amount: Decimal,
    pub direction: PaymentDirection,
    pub description: String,
    pub bank_ref: Option<String>,
    pub bank_name: String,
    pub counterparty_name: Option<String>,
    pub counterparty_iban: Option<String>,
    pub currency: Option<String>,
}

/// Маппінг індексів колонок у вихідній таблиці (CSV header або XLSX header row).
#[derive(Debug, Clone, Default)]
pub struct HeaderLayout {
    pub date_idx: Option<usize>,
    pub amount_idx: Option<usize>,
    pub description_idx: Option<usize>,
    pub direction_idx: Option<usize>,
    pub reference_idx: Option<usize>,
    pub counterparty_name_idx: Option<usize>,
    pub counterparty_iban_idx: Option<usize>,
    pub currency_idx: Option<usize>,
    pub debit_idx: Option<usize>,
    pub credit_idx: Option<usize>,
}

/// Розбирає десяткове число з банківських форматів: підтримує крапку/кому,
/// nbsp/звичайні пробіли як thousand separator, дужки як знак мінус,
/// trailing/leading мінус.
pub fn parse_decimal(raw: &str) -> Result<Decimal> {
    let trimmed = raw.trim().trim_matches('"').replace('\u{00a0}', " ");
    if trimmed.is_empty() {
        bail!("Порожнє числове поле");
    }

    let mut normalized = trimmed.replace(' ', "").replace(',', ".");
    let negative = normalized.starts_with('-')
        || normalized.ends_with('-')
        || (normalized.starts_with('(') && normalized.ends_with(')'));
    normalized = normalized
        .trim_start_matches('-')
        .trim_end_matches('-')
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim_start_matches('+')
        .to_string();

    let mut value = normalized.parse::<Decimal>()?;
    if negative {
        value = -value;
    }
    Ok(value)
}

/// Розбирає дату з рядка у різних поширених форматах виписок.
pub fn parse_date(raw: &str) -> Result<NaiveDate> {
    let trimmed = raw.trim().trim_matches('"');
    let date_only = trimmed.split(['T', ' ']).next().unwrap_or(trimmed).trim();

    for format in [
        "%d.%m.%Y", "%Y-%m-%d", "%d/%m/%Y", "%Y/%m/%d", "%Y%m%d", "%d-%m-%Y",
    ] {
        if let Ok(date) = NaiveDate::parse_from_str(date_only, format) {
            return Ok(date);
        }
    }

    Err(anyhow!("Не вдалося розібрати дату '{raw}'"))
}

/// Зрізає UTF-8 BOM та зайві символи на початку CSV-тексту.
pub fn preprocess_csv_text(csv_text: &str) -> &str {
    csv_text.trim_start_matches('\u{feff}')
}

/// Перетворює header у канонічну форму: lowercase, без пробілів і
/// розділових символів. Працює як для CSV, так і для XLSX.
pub fn normalize_header(raw: &str) -> String {
    raw.trim()
        .trim_matches('\u{feff}')
        .trim_matches('"')
        .to_lowercase()
        .chars()
        .filter(|ch| !matches!(ch, ' ' | '_' | '-' | '.' | ':' | '/' | '\\' | '(' | ')'))
        .collect()
}

/// Нормалізація IBAN: прибирає пробіли, переводить у uppercase, повертає `None` для порожнього.
pub fn normalize_iban(value: &str) -> Option<String> {
    let normalized = value
        .trim()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_uppercase();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Шукає індекс колонки у заголовку за списком aliases (рядки уже мають бути нормалізовані).
fn find_header_index_in<'a, I>(headers: I, aliases: &[&str]) -> Option<usize>
where
    I: IntoIterator<Item = &'a str>,
{
    headers
        .into_iter()
        .map(normalize_header)
        .enumerate()
        .find_map(|(idx, normalized)| {
            if aliases.iter().any(|alias| normalized == *alias) {
                Some(idx)
            } else {
                None
            }
        })
}

/// Будує `HeaderLayout` з масиву рядків заголовків.
///
/// Aliases дублюються між CSV та XLSX парсерами щоб обидва формати
/// видавали однакові індекси для однакових колонок.
pub fn header_layout_from_strs(headers: &[&str]) -> HeaderLayout {
    HeaderLayout {
        date_idx: find_header_index_in(
            headers.iter().copied(),
            &[
                "date",
                "operationdate",
                "documentdate",
                "valuedate",
                "posteddate",
                "transactiondate",
                "дата",
                "датаоперації",
                "документдата",
                "датавалютування",
                "датапроведення",
                "датадокументу",
            ],
        ),
        amount_idx: find_header_index_in(
            headers.iter().copied(),
            &[
                "amount",
                "sum",
                "total",
                "сума",
                "сумаоперації",
                "сумаудоговорі",
                "сумаугрн",
            ],
        ),
        description_idx: find_header_index_in(
            headers.iter().copied(),
            &[
                "description",
                "purpose",
                "details",
                "comment",
                "operationdescription",
                "transactiondescription",
                "назначениеплатежа",
                "призначенняплатежу",
                "опис",
                "описоперації",
                "коментар",
                "деталі",
                "детальплатежу",
            ],
        ),
        direction_idx: find_header_index_in(
            headers.iter().copied(),
            &[
                "direction",
                "type",
                "operationtype",
                "напрям",
                "напрямок",
                "тип",
                "типоперації",
            ],
        ),
        reference_idx: find_header_index_in(
            headers.iter().copied(),
            &[
                "reference",
                "ref",
                "bankref",
                "docno",
                "documentno",
                "operationid",
                "txnid",
                "референс",
                "номердокумента",
                "кодоперації",
                "ідплатежу",
                "ідентифікатороперації",
            ],
        ),
        counterparty_name_idx: find_header_index_in(
            headers.iter().copied(),
            &[
                "counterparty",
                "counterpartyname",
                "name",
                "receiver",
                "sender",
                "company",
                "payer",
                "payee",
                "контрагент",
                "назваконтрагента",
                "отримувач",
                "платник",
                "найменуванняконтрагента",
            ],
        ),
        counterparty_iban_idx: find_header_index_in(
            headers.iter().copied(),
            &[
                "iban",
                "counterpartyiban",
                "recipientiban",
                "senderiban",
                "ібан",
                "ибан",
                "контрагентібан",
                "рахунокконтрагента",
                "рахунокотримувача",
            ],
        ),
        currency_idx: find_header_index_in(
            headers.iter().copied(),
            &["currency", "curr", "currencycode", "валюта", "кодвалюти"],
        ),
        debit_idx: find_header_index_in(
            headers.iter().copied(),
            &[
                "debit",
                "debet",
                "видаток",
                "видатки",
                "списання",
                "списано",
                "withdrawal",
            ],
        ),
        credit_idx: find_header_index_in(
            headers.iter().copied(),
            &[
                "credit",
                "kredit",
                "надходження",
                "надходження(грн)",
                "зарахування",
                "зараховано",
                "deposit",
            ],
        ),
    }
}

/// Розпізнає текстове позначення напрямку платежу.
pub fn parse_direction_text(raw: &str) -> Option<PaymentDirection> {
    let normalized = raw.trim().trim_matches('"').to_lowercase();
    match normalized.as_str() {
        "income" | "in" | "credit" | "deposit" | "надходження" | "зарахування" | "прихід" => {
            Some(PaymentDirection::Income)
        }
        "expense" | "out" | "debit" | "withdrawal" | "витрата" | "списання" | "видаток" => {
            Some(PaymentDirection::Expense)
        }
        _ => None,
    }
}

/// Витягує суму та напрямок із набору рядкових полів за `HeaderLayout`.
///
/// Стратегія така ж, як у CSV: спочатку explicit direction-колонка, потім
/// signed amount, потім debit/credit pair. Кожне поле — `Option<&str>`
/// (None = колонки немає, Some("") = колонка є але порожня).
pub fn amount_and_direction_from_strings(
    direction: Option<&str>,
    amount: Option<&str>,
    debit: Option<&str>,
    credit: Option<&str>,
) -> Result<(Decimal, PaymentDirection)> {
    if let Some(direction_raw) = direction.map(str::trim) {
        if !direction_raw.is_empty() {
            let dir = parse_direction_text(direction_raw)
                .ok_or_else(|| anyhow!("Невідомий напрямок платежу: {direction_raw}"))?;
            let amount_value = parse_decimal(amount.unwrap_or(""))?;
            return Ok((amount_value.abs(), dir));
        }
    }

    if let Some(amount_raw) = amount.map(str::trim) {
        if !amount_raw.is_empty() {
            let parsed = parse_decimal(amount_raw)?;
            if parsed.is_sign_negative() {
                return Ok((parsed.abs(), PaymentDirection::Expense));
            }
            return Ok((parsed, PaymentDirection::Income));
        }
    }

    let debit_value = debit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(parse_decimal)
        .transpose()?;
    let credit_value = credit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(parse_decimal)
        .transpose()?;

    match (debit_value, credit_value) {
        (Some(debit), None) if !debit.is_zero() => Ok((debit.abs(), PaymentDirection::Expense)),
        (None, Some(credit)) if !credit.is_zero() => Ok((credit.abs(), PaymentDirection::Income)),
        (Some(debit), Some(credit)) if credit.is_zero() && !debit.is_zero() => {
            Ok((debit.abs(), PaymentDirection::Expense))
        }
        (Some(debit), Some(credit)) if debit.is_zero() && !credit.is_zero() => {
            Ok((credit.abs(), PaymentDirection::Income))
        }
        _ => bail!("Не вдалося визначити суму або напрямок платежу"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn header_layout_supports_uk_extended_aliases() {
        let headers = vec![
            "Дата проведення",
            "Сума операції",
            "Призначення платежу",
            "Тип операції",
            "Номер документа",
            "Контрагент",
            "Рахунок отримувача",
            "Валюта",
        ];
        let layout = header_layout_from_strs(&headers);
        assert_eq!(layout.date_idx, Some(0));
        assert_eq!(layout.amount_idx, Some(1));
        assert_eq!(layout.description_idx, Some(2));
        assert_eq!(layout.direction_idx, Some(3));
        assert_eq!(layout.reference_idx, Some(4));
        assert_eq!(layout.counterparty_name_idx, Some(5));
        assert_eq!(layout.counterparty_iban_idx, Some(6));
        assert_eq!(layout.currency_idx, Some(7));
    }

    #[test]
    fn parse_decimal_accepts_nbsp_thousands() {
        assert_eq!(parse_decimal("1\u{00a0}500,50").unwrap(), dec!(1500.50));
    }

    #[test]
    fn amount_and_direction_explicit_direction_wins_over_amount_sign() {
        let (amount, direction) =
            amount_and_direction_from_strings(Some("expense"), Some("100,00"), None, None)
                .expect("expected ok");
        assert_eq!(amount, dec!(100.00));
        assert_eq!(direction, PaymentDirection::Expense);
    }

    #[test]
    fn amount_and_direction_uses_debit_credit_pair() {
        let (amount, direction) =
            amount_and_direction_from_strings(None, None, Some("0"), Some("250,00"))
                .expect("expected ok");
        assert_eq!(amount, dec!(250.00));
        assert_eq!(direction, PaymentDirection::Income);
    }

    #[test]
    fn amount_and_direction_missing_data_returns_err() {
        assert!(amount_and_direction_from_strings(None, None, None, None).is_err());
    }

    #[test]
    fn normalize_iban_strips_spaces_and_uppercases() {
        assert_eq!(
            normalize_iban("ua 33 305299 00000 26002 0123 45678").as_deref(),
            Some("UA333052990000026002012345678")
        );
    }

    #[test]
    fn normalize_iban_empty_returns_none() {
        assert_eq!(normalize_iban("   "), None);
    }
}
