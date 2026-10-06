//! Input checks shared by product families.

use crate::error::{Error, Result};

pub fn require_text(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(Error::validation(field, "must not be empty"));
    }
    Ok(())
}

/// OpenHub rejects non-numeric references (`INVALID_FORMAT`), and ATC's own
/// references are numeric; this also keeps URL paths safe.
pub fn numeric_reference(field: &str, value: &str) -> Result<()> {
    require_text(field, value)?;
    if !value.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::validation(field, "must contain only digits"));
    }
    Ok(())
}

pub fn positive(field: &str, value: u64) -> Result<()> {
    if value == 0 {
        return Err(Error::validation(field, "must be greater than zero"));
    }
    Ok(())
}

/// Days since 1970-01-01 for a strict `yyyy-mm-dd` date.
pub fn iso_date_days(field: &str, value: &str) -> Result<i64> {
    let invalid = || Error::validation(field, "must be a date as yyyy-mm-dd");
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(invalid());
    }
    let num = |range: std::ops::Range<usize>| value[range].parse::<i64>().map_err(|_| invalid());
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&m) || d < 1 || d > month_days[(m - 1) as usize] {
        return Err(invalid());
    }
    // Howard Hinnant's days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146_097 + doe - 719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_dates() {
        assert_eq!(iso_date_days("d", "1970-01-01").unwrap(), 0);
        assert_eq!(
            iso_date_days("d", "2024-03-01").unwrap() - iso_date_days("d", "2024-02-28").unwrap(),
            2
        );
        for bad in [
            "2026-02-30",
            "2026-13-01",
            "2026/10/06",
            "06-10-2026",
            "2026-1-6",
        ] {
            assert!(iso_date_days("d", bad).is_err(), "{bad}");
        }
    }
}
