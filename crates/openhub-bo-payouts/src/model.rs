use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use openhub_bo_core::PaymentStatus;
use openhub_bo_core::amount;

// -- QR payouts (sync) -------------------------------------------------------------

/// Input of [`crate::ScanOp`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanQr {
    /// Text content of the scanned QR (not the image).
    pub qr_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Recipient {
    pub account: Option<String>,
    pub document_id: Option<String>,
    pub holder: Option<String>,
    pub bank_code: Option<String>,
    pub bank_name: Option<String>,
}

/// A decoded QR, ready to be paid with [`crate::PayOp`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedQr {
    /// ATC's reference for this payment (18–20 digits).
    pub reference: String,
    /// Zero for open-amount QRs: the payer chooses the amount.
    #[serde(
        deserialize_with = "amount::deserialize",
        serialize_with = "amount::serialize"
    )]
    pub amount: Decimal,
    pub currency: Option<String>,
    pub description: Option<String>,
    pub recipient: Recipient,
    /// `fechaVencimiento` as sent (`yyyy-mm-dd` in the sandbox).
    pub expires_on: Option<String>,
}

/// Input of [`crate::PayOp`]. The amount/description rules depend on the
/// scanned QR, so it travels with the request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayQr {
    pub scanned: ScannedQr,
    /// The merchant's ATC account to debit (`cuentaOrigen`).
    pub source_account: String,
    /// Your unique id for this payout (≤ 32 characters).
    pub transaction_id: String,
    /// Required (and > 0) only for open-amount QRs.
    #[serde(
        default,
        deserialize_with = "amount::deserialize_opt",
        serialize_with = "amount::serialize_opt"
    )]
    pub amount: Option<Decimal>,
    /// Required only when the QR has no description of its own.
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayoutRef {
    pub reference: String,
}

/// A QR payout, as returned by payment and status queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Payout {
    pub reference: String,
    pub transaction_id: Option<String>,
    pub transaction_at: Option<String>,
    pub status: PaymentStatus,
    pub raw_status: String,
    pub message: Option<String>,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub amount: Option<Decimal>,
    pub currency: Option<String>,
    pub description: Option<String>,
    pub source_account: Option<String>,
    pub source_holder: Option<String>,
    pub recipient: Recipient,
    pub ach_order_number: Option<String>,
}

// -- ACH batches (async) -------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchTransfer {
    /// Your id for this transfer (3–14 characters), unique within the batch.
    pub transaction_id: String,
    #[serde(
        deserialize_with = "amount::deserialize",
        serialize_with = "amount::serialize"
    )]
    pub amount: Decimal,
    /// `yyyy-mm-dd`, today or later.
    pub date: String,
    /// The merchant's ATC virtual account.
    pub source_account: String,
    pub destination_account: String,
    /// Destination bank code (see [`crate::BanksOp`]).
    pub bank_code: String,
    /// City of the source account: CBB, COB, LPZ, ORU, POT, SCZ, SUC, TJA, TRI.
    pub branch_city: String,
    /// 3–80 characters.
    pub description: String,
    pub recipient_document_id: String,
    pub recipient_name: String,
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "BOB".to_owned()
}

/// Input of [`crate::AuthorizeBatchOp`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizeBatch {
    /// Merchant code sent as the `branchCode` header.
    pub branch_code: String,
    /// UUID identifying this batch.
    pub process_id: String,
    /// Where ATC POSTs one notification per transfer.
    pub webhook_url: String,
    /// Secret appended as `?token=`; ATC echoes it on every notification.
    pub webhook_token: String,
    pub transfers: Vec<BatchTransfer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchTransferResult {
    pub transaction_id: String,
    pub status: PaymentStatus,
    pub raw_status: String,
    pub reference: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchAuthorization {
    pub batch_number: String,
    pub process_id: String,
    /// Transfers rejected up front come back with status `error`.
    pub transfers: Vec<BatchTransferResult>,
}

/// Input of [`crate::BatchStatusOp`]: exactly one of `batch_number` / `transaction_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchQuery {
    pub branch_code: String,
    pub process_id: String,
    #[serde(default)]
    pub batch_number: Option<String>,
    #[serde(default)]
    pub transaction_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchTransferStatus {
    pub transaction_id: Option<String>,
    pub reference: Option<String>,
    pub status: PaymentStatus,
    pub raw_status: String,
    pub message: Option<String>,
    pub source_account: Option<String>,
    pub destination_account: Option<String>,
    pub ach_number: Option<String>,
    pub recipient_number: Option<String>,
    pub recipient_document_id: Option<String>,
    pub recipient_name: Option<String>,
    pub transaction_at: Option<String>,
    pub bank_code: Option<String>,
    pub bank_name: Option<String>,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub amount: Option<Decimal>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchStatus {
    pub batch_number: Option<String>,
    pub transfers: Vec<BatchTransferStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BanksQuery {
    pub branch_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bank {
    pub code: String,
    pub name: Option<String>,
}

/// One transfer's result, POSTed by ATC to the batch webhook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchNotification {
    pub batch_number: Option<String>,
    pub transfer: BatchTransferStatus,
    /// Answer with this JSON (HTTP 200) once processed; use `codigoRespuesta:
    /// "FALLIDO"` instead if you could not process it.
    pub ack: serde_json::Value,
}
