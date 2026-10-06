//! Cross-currency QR collections: PIX (payer pays BRL), virtual assets via
//! Koibanx (USDT/USDC) and Binance Pay. The three share one contract: a flat
//! body with `codigoRespuesta`, currency conversion fields and a base64 image.
//!
//! Verified against the sandbox (2026-10-06): request validation and error
//! shapes. Success paths follow the docs only; the integrator's merchant was
//! not enabled for these products in the sandbox.

pub mod model;
pub mod ops;
pub mod validate;
pub mod webhook;
mod wire;

pub use model::{
    BinanceNotification, BinanceQr, Currency, FxNotification, FxQr, FxStatusInfo, PixQr, VerifyFx,
    VirtualAsset, VirtualAssetQr,
};
pub use ops::{
    BinanceGenerate, BinanceVerify, CryptoGenerate, CryptoVerify, PixCancel, PixGenerate, PixVerify,
};
pub use webhook::{ParseBinanceWebhook, ParseFxWebhook};

openhub_bo_core::registry!(pub FxOps {
    operations: [
        PixGenerate,
        PixVerify,
        PixCancel,
        CryptoGenerate,
        CryptoVerify,
        BinanceGenerate,
        BinanceVerify,
    ],
    handlers: [ParseBinanceWebhook, ParseFxWebhook],
});
