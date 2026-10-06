use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use openhub_core::amount;
use openhub_core::{PaymentStatus, WebhookTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Currency {
    #[default]
    #[serde(rename = "BOB")]
    Bob,
    #[serde(rename = "USD")]
    Usd,
}

impl Currency {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Currency::Bob => "BOB",
            Currency::Usd => "USD",
        }
    }
}

/// Settlement asset for Koibanx QRs (`activoVirtual`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VirtualAsset {
    /// `UT`
    Usdt,
    /// `UP`
    Usdc,
    /// `BK`
    Bk,
}

impl VirtualAsset {
    pub(crate) fn code(self) -> &'static str {
        match self {
            VirtualAsset::Usdt => "UT",
            VirtualAsset::Usdc => "UP",
            VirtualAsset::Bk => "BK",
        }
    }
}

fn default_channel() -> String {
    "WEB".to_owned()
}

/// Input of [`crate::PixGenerate`]. The payer pays in BRL; you receive BOB/USD.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixQr {
    /// Your reference, digits only.
    pub reference: String,
    /// `branch_code|branch_name|category|description` (required by the sandbox).
    pub glosa: String,
    #[serde(
        deserialize_with = "amount::deserialize",
        serialize_with = "amount::serialize"
    )]
    pub amount: Decimal,
    #[serde(default)]
    pub currency: Currency,
    /// Groups transactions in reports, e.g. `WEB`, `MOVIL`.
    #[serde(default = "default_channel")]
    pub channel: String,
    /// Validity in seconds (`tiempoQr`); OpenHub defaults to 100 s. Max 86399.
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// Brazilian CPF of the payer, 11 digits.
    pub payer_cpf: String,
    /// `+` followed by 13 digits (14 characters), e.g. `+5511999999999`.
    pub payer_phone: String,
    #[serde(default)]
    pub payer_email: Option<String>,
    #[serde(default)]
    pub extra: Option<String>,
    #[serde(default)]
    pub webhook: Option<WebhookTarget>,
}

/// Input of [`crate::CryptoGenerate`] (Koibanx).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualAssetQr {
    pub reference: String,
    /// `branch_code|branch_name|category|description`.
    pub glosa: String,
    /// Minimum Bs 50 when charging in BOB (sandbox).
    #[serde(
        deserialize_with = "amount::deserialize",
        serialize_with = "amount::serialize"
    )]
    pub amount: Decimal,
    #[serde(default)]
    pub currency: Currency,
    pub asset: VirtualAsset,
    /// `APP`, `WEB`, `WAP` or `OTHERS`.
    #[serde(default = "default_channel")]
    pub channel: String,
    /// Validity in seconds, 180..=600 (sandbox; the docs' 30..90 is wrong).
    pub expires_in: u64,
    #[serde(default)]
    pub extra: Option<String>,
    #[serde(default)]
    pub webhook: Option<WebhookTarget>,
}

/// Input of [`crate::BinanceGenerate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceQr {
    /// Your reference, digits only, at most 10 characters.
    pub reference: String,
    /// `branch_code|branch_name|category|description`.
    pub glosa: String,
    #[serde(
        deserialize_with = "amount::deserialize",
        serialize_with = "amount::serialize"
    )]
    pub amount: Decimal,
    #[serde(default)]
    pub currency: Currency,
    #[serde(default = "default_channel")]
    pub channel: String,
    /// Validity in seconds (`tiempoQr`), at most 300.
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub extra: Option<String>,
    #[serde(default)]
    pub webhook: Option<WebhookTarget>,
}

/// Input of the status (and PIX cancel) operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyFx {
    /// ATC's `numeroReferencia` returned by generation.
    pub reference: String,
}

/// A generated FX QR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FxQr {
    /// ATC's reference; use it for status queries.
    pub reference: String,
    /// `origenNumeroReferencia`: the reference you sent.
    pub merchant_reference: Option<String>,
    pub status: PaymentStatus,
    pub raw_status: String,
    pub detail: Option<String>,
    #[serde(serialize_with = "amount::serialize")]
    pub amount: Decimal,
    pub currency: String,
    /// What the payer pays, in `converted_currency` (BRL, USDT, USDC...).
    #[serde(serialize_with = "amount::serialize_opt")]
    pub converted_amount: Option<Decimal>,
    pub converted_currency: Option<String>,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub exchange_rate: Option<Decimal>,
    /// `qrExpiracion` as sent (format varies by product).
    pub expires_at: Option<String>,
    pub image_base64: String,
    /// `image/png` (PIX, Koibanx) or `image/jpeg` (Binance), sniffed from the data.
    pub image_mime: &'static str,
}

/// Result of a status query or a PIX cancellation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FxStatusInfo {
    pub reference: String,
    pub status: PaymentStatus,
    pub raw_status: String,
    pub detail: Option<String>,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub amount: Option<Decimal>,
    pub currency: Option<String>,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub converted_amount: Option<Decimal>,
    pub converted_currency: Option<String>,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub exchange_rate: Option<Decimal>,
    /// PIX: reversal/chargeback details when the payment was reversed.
    pub reversal: Option<serde_json::Value>,
    /// PIX cancel: when the request was processed (`fechaSolicitud`).
    pub requested_at: Option<String>,
}

/// Binance Pay notification, plus the body the merchant must answer with.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BinanceNotification {
    pub reference: String,
    pub status: PaymentStatus,
    pub raw_status: String,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub amount: Option<Decimal>,
    pub currency: Option<String>,
    pub transaction_at: Option<String>,
    pub payer_name: Option<String>,
    pub payer_document_id: Option<String>,
    /// Return this JSON with HTTP 200; ATC requires it (`codigoRespuesta: "00"`).
    pub ack: serde_json::Value,
}

/// PIX / Koibanx notification. Their payload is not documented, so this is a
/// best-effort reading; the full payload is kept in `payload`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FxNotification {
    pub reference: Option<String>,
    pub status: PaymentStatus,
    pub raw_status: Option<String>,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub amount: Option<Decimal>,
    pub currency: Option<String>,
    pub payload: serde_json::Value,
}
