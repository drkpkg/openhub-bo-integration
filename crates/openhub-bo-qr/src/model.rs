use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use openhub_bo_core::amount;
use openhub_bo_core::status::PaymentStatus;
use openhub_bo_core::webhook::WebhookTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrKind {
    /// Interbank "QR Simple" (ASOBAN standard).
    #[default]
    Simple,
    /// QR under the Banco Central de Bolivia interoperability standard (MLD).
    Mld,
}

impl QrKind {
    pub(crate) fn path(self) -> &'static str {
        match self {
            QrKind::Simple => "qr/simple/v2",
            QrKind::Mld => "qr/mld/v2",
        }
    }
}

/// Input of [`crate::Generate`] (English names; mapped to the Spanish wire format).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateQr {
    #[serde(default)]
    pub kind: QrKind,
    /// `glosa`: description shown to the payer.
    pub description: String,
    /// `monto`: at most two decimals, in BOB.
    #[serde(
        deserialize_with = "amount::deserialize",
        serialize_with = "amount::serialize"
    )]
    pub amount: Decimal,
    /// `numeroReferencia`: the integrator's own reference, digits only.
    pub reference: String,
    /// `vigencia`: validity in seconds.
    pub expires_in: u64,
    /// `idEstablecimiento`: establishment id registered with ATC.
    pub establishment_id: u64,
    /// `nombreEstablecimiento`.
    pub establishment_name: String,
    /// Required by OpenHub (`REQUIRED_FIELD` otherwise). Optional in the type
    /// so a missing value surfaces as a validation error.
    #[serde(default)]
    pub webhook: Option<WebhookTarget>,
}

/// Input of [`crate::Verify`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyQr {
    #[serde(default)]
    pub kind: QrKind,
    /// ATC's `numeroReferencia` returned by generation.
    pub reference: String,
}

/// Input of [`crate::Cancel`] (QR Simple only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelQr {
    /// ATC's `numeroReferencia` returned by generation.
    pub reference: String,
}

/// Result of a successful generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GeneratedQr {
    /// Simple and MLD-BCB references live in separate spaces (a MLD reference
    /// is not found through the Simple endpoints): store it with the reference.
    pub kind: QrKind,
    /// ATC's `numeroReferencia`. Use it for status queries and to match webhooks.
    pub reference: String,
    /// `numeroReferenciaOriginante`: the reference you sent.
    pub merchant_reference: Option<String>,
    pub status: PaymentStatus,
    pub raw_status: String,
    /// `fechaExpiracion`, ISO 8601 without offset, Bolivia time (UTC-4).
    pub expires_at: Option<String>,
    pub currency: String,
    #[serde(serialize_with = "amount::serialize")]
    pub amount: Decimal,
    /// PNG image, base64 encoded.
    pub qr_image_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Payer {
    pub name: Option<String>,
    pub account_number: Option<String>,
    /// CI or NIT.
    pub document_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct PayerBank {
    pub bank_code: Option<String>,
    pub bank_name: Option<String>,
    pub ach_order_number: Option<String>,
    pub transaction_date: Option<String>,
}

/// Result of a status query or a cancellation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QrStatusInfo {
    pub kind: QrKind,
    pub reference: String,
    pub merchant_reference: Option<String>,
    pub status: PaymentStatus,
    pub raw_status: String,
    pub message: Option<String>,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub amount: Option<Decimal>,
    pub currency: Option<String>,
    pub payer: Option<Payer>,
    pub payer_bank: Option<PayerBank>,
}

/// Payment notification POSTed by ATC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaymentNotification {
    /// ATC's `numeroReferencia` (the one returned when generating the QR).
    pub reference: String,
    #[serde(serialize_with = "amount::serialize")]
    pub amount: Decimal,
    pub currency: Option<String>,
    pub status: PaymentStatus,
    /// `codigoRespuesta` or `estado`, as sent.
    pub response_code: String,
    pub response_detail: Option<String>,
    /// `fechaHoraTransaccion`, ISO 8601 without offset, Bolivia time.
    pub transaction_at: Option<String>,
    pub payer: Option<Payer>,
    pub payer_bank: Option<PayerBank>,
    /// Shortcut for `status == Paid`.
    pub success: bool,
}
