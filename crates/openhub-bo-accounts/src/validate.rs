//! Input rules for merchant accounts, as enforced by the sandbox.

use openhub_bo_core::validate::{iso_date_days, require_text};
use openhub_bo_core::{Error, Result};

/// NIT: digits only, 7..=20.
pub fn nit(value: &str) -> Result<()> {
    digits("nit", value, 7, 20)
}

/// Account numbers: digits only, 8..=20.
pub fn account_number(field: &str, value: &str) -> Result<()> {
    digits(field, value, 8, 20)
}

pub fn text_max(field: &str, value: &str, max: usize) -> Result<()> {
    require_text(field, value)?;
    if value.chars().count() > max {
        return Err(Error::validation(
            field,
            format!("at most {max} characters"),
        ));
    }
    Ok(())
}

/// Validates an inclusive `yyyy-mm-dd` range of at most `max_days` days
/// (difference between the dates, as the API counts it).
pub fn date_range(from: &str, to: &str, max_days: i64) -> Result<()> {
    let start = iso_date_days("date_from", from)?;
    let end = iso_date_days("date_to", to)?;
    if end < start {
        return Err(Error::validation("date_to", "must not be before date_from"));
    }
    if end - start > max_days {
        return Err(Error::validation(
            "date_to",
            format!("the range is at most {max_days} days"),
        ));
    }
    Ok(())
}

fn digits(field: &str, value: &str, min: usize, max: usize) -> Result<()> {
    if !(min..=max).contains(&value.len()) || !value.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::validation(
            field,
            format!("must be {min} to {max} digits"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_ranges_match_sandbox_limits() {
        assert!(date_range("2026-09-29", "2026-10-06", 7).is_ok());
        assert!(date_range("2026-09-28", "2026-10-06", 7).is_err());
        assert!(date_range("2026-09-05", "2026-10-06", 31).is_ok());
        assert!(date_range("2026-08-01", "2026-10-06", 31).is_err());
        assert!(date_range("2026-10-06", "2026-10-01", 31).is_err());
    }

    #[test]
    fn identifiers() {
        assert!(nit("1234567").is_ok());
        assert!(nit("123456").is_err());
        assert!(account_number("a", "7011234561").is_ok());
        assert!(account_number("a", "701-123").is_err());
    }
}
