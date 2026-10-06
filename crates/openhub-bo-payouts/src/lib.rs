//! Payouts: paying third-party interoperable QRs from the merchant's ATC
//! account (`ATC.API.PAYOUT.SYNC 3`) and ACH transfer batches
//! (`ATC.API.PAYOUT.ASYNC`). These move real money in production.
//!
//! Verified against the sandbox (2026-10-06): QR scan (success, with a QR the
//! integrator generated), bank list, validation and not-found errors, HTTP
//! methods. Payment confirmation and batch authorisation were not executed;
//! they follow the docs.

pub mod batch;
pub mod model;
pub mod qr;
pub mod validate;
pub mod webhook;
mod wire;

pub use batch::{AuthorizeBatchOp, BanksOp, BatchStatusOp};
pub use model::{
    AuthorizeBatch, Bank, BanksQuery, BatchAuthorization, BatchNotification, BatchQuery,
    BatchStatus, BatchTransfer, BatchTransferResult, BatchTransferStatus, PayQr, Payout, PayoutRef,
    Recipient, ScanQr, ScannedQr,
};
pub use qr::{PayOp, PayoutStatusOp, ScanOp};
pub use webhook::ParseBatchWebhook;

openhub_bo_core::registry!(pub PayoutsOps {
    operations: [ScanOp, PayOp, PayoutStatusOp, AuthorizeBatchOp, BatchStatusOp, BanksOp],
    handlers: [ParseBatchWebhook],
});
