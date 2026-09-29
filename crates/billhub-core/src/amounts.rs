use crate::{Error, Result};

/// Parses a platform amount into exact cents. The signed result is useful for
/// diagnostics; normalized ledger events store a positive amount and encode
/// direction separately.
pub fn parse_cents(input: &str) -> Result<i64> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '¥' | '￥' | '$' | ',' | '\''))
        .collect();
    let negative = cleaned.starts_with('-');
    let digits = cleaned.trim_start_matches(['+', '-']);
    let (whole, fraction) = match digits.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (digits, ""),
    };
    if whole.is_empty() && fraction.is_empty() {
        return Err(Error::AmountParseFailed {
            row: 0,
            amount: input.to_owned(),
        });
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::AmountParseFailed {
            row: 0,
            amount: input.to_owned(),
        });
    }
    if fraction.len() > 2 {
        return Err(Error::AmountParseFailed {
            row: 0,
            amount: input.to_owned(),
        });
    }
    let whole_value: i64 = whole.parse().map_err(|_| Error::AmountParseFailed {
        row: 0,
        amount: input.to_owned(),
    })?;
    let fraction_value = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().unwrap() * 10,
        _ => fraction.parse::<i64>().unwrap(),
    };
    let result = whole_value
        .checked_mul(100)
        .and_then(|v| v.checked_add(fraction_value))
        .ok_or(Error::AmountParseFailed {
            row: 0,
            amount: input.to_owned(),
        })?;
    Ok(if negative { -result } else { result })
}

pub fn parse_positive_cents(input: &str, row: usize) -> Result<i64> {
    let parsed = parse_cents(input).map_err(|_| Error::AmountParseFailed {
        row,
        amount: input.to_owned(),
    })?;
    if parsed <= 0 {
        return Err(Error::AmountParseFailed {
            row,
            amount: input.to_owned(),
        });
    }
    Ok(parsed)
}

pub fn format_cents(amount: i64) -> String {
    format!("{}.{:02}", amount / 100, amount.rem_euclid(100))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exact_money_without_floats() {
        assert_eq!(parse_cents("80.8").unwrap(), 8_080);
        assert_eq!(parse_cents("¥ 1,234.56").unwrap(), 123_456);
        assert_eq!(parse_cents("0.01").unwrap(), 1);
        assert_eq!(parse_cents("-3.00").unwrap(), -300);
        assert_eq!(format_cents(-300), "-3.00");
    }

    #[test]
    fn rejects_lossy_or_invalid_amounts() {
        assert!(parse_cents("").is_err());
        assert!(parse_cents("1.234").is_err());
        assert!(parse_cents("abc").is_err());
        assert!(parse_positive_cents("0.00", 1).is_err());
    }
}
