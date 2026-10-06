//! Input rules for payouts (docs + sandbox).

use openhub_core::validate::{iso_date_days, require_text};
use openhub_core::{Error, Result};

pub const CITIES: [&str; 9] = [
    "CBB", "COB", "LPZ", "ORU", "POT", "SCZ", "SUC", "TJA", "TRI",
];

pub fn digits(field: &str, value: &str, min: usize, max: usize) -> Result<()> {
    if !(min..=max).contains(&value.len()) || !value.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::validation(
            field,
            format!("must be {min} to {max} digits"),
        ));
    }
    Ok(())
}

/// Letters, digits, `-` and `_`, within `min..=max` characters.
pub fn identifier(field: &str, value: &str, min: usize, max: usize) -> Result<()> {
    let ok = (min..=max).contains(&value.len())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(Error::validation(
            field,
            format!("{min} to {max} letters, digits, `-` or `_`"),
        ));
    }
    Ok(())
}

pub fn text(field: &str, value: &str, min: usize, max: usize) -> Result<()> {
    require_text(field, value)?;
    if !(min..=max).contains(&value.trim().chars().count()) {
        return Err(Error::validation(
            field,
            format!("{min} to {max} characters"),
        ));
    }
    Ok(())
}

pub fn uuid(field: &str, value: &str) -> Result<()> {
    let groups: Vec<&str> = value.split('-').collect();
    let ok = groups.len() == 5
        && groups
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(g, len)| g.len() == len && g.chars().all(|c| c.is_ascii_hexdigit()));
    if !ok {
        return Err(Error::validation(field, "must be a UUID (36 characters)"));
    }
    Ok(())
}

pub fn date(field: &str, value: &str) -> Result<()> {
    iso_date_days(field, value).map(|_| ())
}

/// The token travels in the URL, so keep it URL-safe; require some length.
pub fn webhook(url: &str, token: &str) -> Result<String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(Error::validation("webhook_url", "must be an http(s) URL"));
    }
    let safe = token
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~'));
    if token.len() < 16 || !safe {
        return Err(Error::validation(
            "webhook_token",
            "at least 16 URL-safe characters (letters, digits, - _ . ~)",
        ));
    }
    let separator = if url.contains('?') { '&' } else { '?' };
    let full = format!("{url}{separator}token={token}");
    if full.len() > 255 {
        return Err(Error::validation(
            "webhook_url",
            "with the token, at most 255 characters",
        ));
    }
    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules() {
        assert!(uuid("p", "66ec5b3d-61ea-4254-b366-7104545aa3c6").is_ok());
        assert!(uuid("p", "4554645646446").is_err());
        assert!(identifier("t", "REQ-TEST07", 1, 32).is_ok());
        assert!(identifier("t", "a b", 1, 32).is_err());
        assert_eq!(
            webhook("https://a.bo/hook?x=1", "0123456789abcdef").unwrap(),
            "https://a.bo/hook?x=1&token=0123456789abcdef"
        );
        assert!(webhook("https://a.bo/hook", "short").is_err());
        assert!(webhook("https://a.bo/hook", "has space 0123456789").is_err());
    }
}
