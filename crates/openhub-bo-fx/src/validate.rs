//! Input rules for the FX family, as enforced by the sandbox.

use openhub_bo_core::Error;
use openhub_bo_core::Result;
use openhub_bo_core::WebhookTarget;
use openhub_bo_core::validate::{numeric_reference, require_text};

/// `branch_code|branch_name|category|description`, every part non-empty.
pub fn glosa(value: &str) -> Result<String> {
    let parts: Vec<&str> = value.split('|').map(str::trim).collect();
    if parts.len() != 4 || parts.iter().any(|p| p.is_empty()) {
        return Err(Error::validation(
            "glosa",
            "must be `branch_code|branch_name|category|description`",
        ));
    }
    Ok(parts.join("|"))
}

pub fn reference(value: &str, max_len: Option<usize>) -> Result<()> {
    numeric_reference("reference", value)?;
    if let Some(max) = max_len.filter(|max| value.len() > *max) {
        return Err(Error::validation(
            "reference",
            format!("at most {max} digits"),
        ));
    }
    Ok(())
}

pub fn channel(value: &str) -> Result<String> {
    require_text("channel", value)?;
    let channel = value.trim().to_ascii_uppercase();
    if !channel
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(Error::validation("channel", "letters, digits and `_` only"));
    }
    Ok(channel)
}

pub fn webhook(target: Option<&WebhookTarget>) -> Result<&WebhookTarget> {
    let target = target.ok_or_else(|| Error::validation("webhook", "is required by OpenHub"))?;
    target.validate()?;
    Ok(target)
}

/// `HH:MM:SS` within `1..=max_secs`.
pub fn hhmmss(field: &str, seconds: u64, max_secs: u64) -> Result<String> {
    if seconds == 0 || seconds > max_secs {
        return Err(Error::validation(
            field,
            format!("must be between 1 and {max_secs} seconds"),
        ));
    }
    Ok(format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    ))
}

pub fn cpf(value: &str) -> Result<()> {
    if value.len() != 11 || !value.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::validation("payer_cpf", "must be 11 digits"));
    }
    Ok(())
}

/// The sandbox requires `+` and 13 digits (14 characters).
pub fn phone(value: &str) -> Result<()> {
    let valid = value.len() == 14
        && value
            .strip_prefix('+')
            .is_some_and(|digits| digits.chars().all(|c| c.is_ascii_digit()));
    if !valid {
        return Err(Error::validation(
            "payer_phone",
            "must be `+` followed by 13 digits, e.g. +5511999999999",
        ));
    }
    Ok(())
}

pub fn email(value: &str) -> Result<()> {
    if value.len() > 80 || !value.contains('@') {
        return Err(Error::validation(
            "payer_email",
            "must be an email of at most 80 characters",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glosa_needs_four_parts() {
        assert_eq!(
            glosa(" 1 | Tienda |7011| Pago ").unwrap(),
            "1|Tienda|7011|Pago"
        );
        assert!(glosa("Pago de servicio").is_err());
        assert!(glosa("1||7011|Pago").is_err());
    }

    #[test]
    fn hhmmss_formats_and_bounds() {
        assert_eq!(hhmmss("expires_in", 300, 300).unwrap(), "00:05:00");
        assert_eq!(hhmmss("expires_in", 86399, 86399).unwrap(), "23:59:59");
        assert!(hhmmss("expires_in", 301, 300).is_err());
        assert!(hhmmss("expires_in", 0, 300).is_err());
    }

    #[test]
    fn payer_rules() {
        assert!(cpf("12345678901").is_ok());
        assert!(cpf("1234567890").is_err());
        assert!(phone("+5511999999999").is_ok());
        assert!(phone("+59171234567").is_err());
        assert!(phone("05511999999999").is_err());
    }

    #[test]
    fn reference_length() {
        assert!(reference("1234567890", Some(10)).is_ok());
        assert!(reference("12345678901", Some(10)).is_err());
        assert!(reference("12a", None).is_err());
    }
}
