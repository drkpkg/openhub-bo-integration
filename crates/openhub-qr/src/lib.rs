//! QR Simple and QR MLD-BCB collections. Both share one contract and differ
//! only in the path prefix; only QR Simple can cancel.
//!
//! Layout: [`model`] is the public API (English names), `wire` mirrors
//! OpenHub's Spanish JSON, [`ops`] binds them to endpoints. [`QrOps`] exposes
//! them over the FFI; combine it with `openhub_core::CoreOps` via `Chain`.

pub mod model;
pub mod ops;
pub mod webhook;
mod wire;

pub use model::{
    CancelQr, GenerateQr, GeneratedQr, Payer, PayerBank, PaymentNotification, QrKind, QrStatusInfo,
    VerifyQr,
};
pub use ops::{Cancel, Generate, Verify};
pub use webhook::ParseWebhook;

/// Only currency accepted by the QR Simple / MLD-BCB APIs.
pub const CURRENCY: &str = "BOB";

openhub_core::registry!(pub QrOps {
    operations: [Generate, Verify, Cancel],
    handlers: [ParseWebhook],
});
