//! Exact money handling. OpenHub sends and expects amounts as JSON numbers;
//! internally they are always `Decimal` so no float rounding leaks out.

use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serializer};

use crate::error::{Error, Result};

pub const MAX_SCALE: u32 = 2;

pub fn validate(field: &str, amount: Decimal) -> Result<Decimal> {
    if amount <= Decimal::ZERO {
        return Err(Error::validation(field, "must be greater than zero"));
    }
    let normalized = amount.normalize();
    if normalized.scale() > MAX_SCALE {
        return Err(Error::validation(
            field,
            format!("at most {MAX_SCALE} decimal places are allowed"),
        ));
    }
    Ok(normalized)
}

/// Converts to a JSON number written exactly as the decimal (serde_json's
/// `arbitrary_precision`), so no float rounding is involved.
pub fn to_json_number(amount: Decimal) -> Result<serde_json::Value> {
    amount
        .normalize()
        .to_string()
        .parse::<serde_json::Number>()
        .map(serde_json::Value::Number)
        .map_err(|e| Error::validation("amount", format!("not representable as JSON: {e}")))
}

/// Accepts JSON numbers or strings without going through f64 arithmetic.
pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Decimal, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    from_value(&value).map_err(serde::de::Error::custom)
}

pub fn deserialize_opt<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Decimal>, D::Error> {
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => from_value(&v).map(Some).map_err(serde::de::Error::custom),
    }
}

/// Amounts cross the FFI boundary as strings to stay exact in every language.
pub fn serialize<S: Serializer>(
    amount: &Decimal,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_str(&amount.normalize().to_string())
}

pub fn serialize_opt<S: Serializer>(
    amount: &Option<Decimal>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    match amount {
        Some(a) => serialize(a, serializer),
        None => serializer.serialize_none(),
    }
}

fn from_value(value: &serde_json::Value) -> std::result::Result<Decimal, String> {
    let text = match value {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.trim().to_owned(),
        other => return Err(format!("expected a number, got {other}")),
    };
    Decimal::from_str(&text)
        .or_else(|_| Decimal::from_scientific(&text))
        .map_err(|e| format!("invalid amount `{text}`: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn rejects_more_than_two_decimals() {
        assert!(validate("amount", dec("10.505")).is_err());
        assert_eq!(validate("amount", dec("10.500")).unwrap(), dec("10.5"));
    }

    #[test]
    fn rejects_non_positive() {
        assert!(validate("amount", dec("0")).is_err());
        assert!(validate("amount", dec("-1")).is_err());
    }

    #[test]
    fn json_number_is_exact() {
        for (input, wire) in [
            ("10.5", "10.5"),
            ("0.1", "0.1"),
            ("0.07", "0.07"),
            ("99999.99", "99999.99"),
            ("10.00", "10"),
            ("12345678901234567.89", "12345678901234567.89"),
        ] {
            let number = to_json_number(dec(input)).unwrap();
            assert_eq!(number.to_string(), wire);
            assert_eq!(from_value(&number).unwrap(), dec(input));
        }
    }
}
