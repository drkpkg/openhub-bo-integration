//! Payment status shared by every product family. Each family maps its own
//! wire vocabulary through a [`StatusVocabulary`]; callers keep the original
//! value in `raw_status` for auditing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Processing,
    Paid,
    Cancelled,
    Expired,
    Rejected,
    Reversed,
    Error,
    /// Not in any documented vocabulary; see `raw_status`.
    Unknown,
}

impl PaymentStatus {
    /// No further transitions are expected from this state.
    pub fn is_final(self) -> bool {
        !matches!(
            self,
            PaymentStatus::Pending | PaymentStatus::Processing | PaymentStatus::Unknown
        )
    }
}

/// Maps a product family's raw status strings to [`PaymentStatus`].
pub trait StatusVocabulary {
    fn map(raw: &str) -> PaymentStatus;
}

/// QR Simple / MLD-BCB `estado`.
pub struct SpanishQr;

/// PIX / virtual assets / Binance `codigoRespuesta`.
pub struct EnglishCodes;

/// Synchronous and asynchronous payouts `estado`.
pub struct Payout;

fn normalize(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

impl StatusVocabulary for SpanishQr {
    fn map(raw: &str) -> PaymentStatus {
        match normalize(raw).as_str() {
            "PENDIENTE" => PaymentStatus::Pending,
            "PAGADO" => PaymentStatus::Paid,
            "CANCELADO" => PaymentStatus::Cancelled,
            "EXPIRADO" => PaymentStatus::Expired,
            "ERROR" => PaymentStatus::Error,
            _ => PaymentStatus::Unknown,
        }
    }
}

impl StatusVocabulary for EnglishCodes {
    fn map(raw: &str) -> PaymentStatus {
        match normalize(raw).as_str() {
            "PENDING" => PaymentStatus::Pending,
            "PAID" | "SUCCESS" => PaymentStatus::Paid,
            "CANCELLED" => PaymentStatus::Cancelled,
            "EXPIRED" => PaymentStatus::Expired,
            "ERROR" => PaymentStatus::Error,
            _ => PaymentStatus::Unknown,
        }
    }
}

impl StatusVocabulary for Payout {
    fn map(raw: &str) -> PaymentStatus {
        match normalize(raw).as_str() {
            "PENDIENTE" | "PENDIENTE_PAGO" | "INICIALIZADO" => PaymentStatus::Pending,
            "EN_PROCESO" | "PROCESO" | "ENVIADO" | "TRANSITO" | "PENDIENTE_CONFIRMACION" => {
                PaymentStatus::Processing
            }
            "APROBADA" | "PAGADO" | "COMPLETADO" => PaymentStatus::Paid,
            "RECHAZADA" | "RECHAZADO" => PaymentStatus::Rejected,
            "REVERTIDO" => PaymentStatus::Reversed,
            "CANCELADO" => PaymentStatus::Cancelled,
            "ERROR" => PaymentStatus::Error,
            _ => PaymentStatus::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_vocabulary() {
        assert_eq!(SpanishQr::map(" pagado "), PaymentStatus::Paid);
        assert_eq!(SpanishQr::map("NUEVO"), PaymentStatus::Unknown);
        assert_eq!(EnglishCodes::map("SUCCESS"), PaymentStatus::Paid);
        assert_eq!(EnglishCodes::map("CANCELLED"), PaymentStatus::Cancelled);
        assert_eq!(
            Payout::map("PENDIENTE_CONFIRMACION"),
            PaymentStatus::Processing
        );
        assert_eq!(Payout::map("APROBADA"), PaymentStatus::Paid);
        assert_eq!(Payout::map("REVERTIDO"), PaymentStatus::Reversed);
    }

    #[test]
    fn finality() {
        assert!(PaymentStatus::Paid.is_final());
        assert!(!PaymentStatus::Processing.is_final());
        assert!(!PaymentStatus::Unknown.is_final());
    }
}
