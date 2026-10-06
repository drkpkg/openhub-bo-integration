//! Paying third-party QRs from the merchant's ATC account
//! (`/payout/sync/v3/qr`). Flow: scan → pay → status.

use serde_json::{Value, json};

use crate::model::{PayQr, Payout, PayoutRef, ScanQr, ScannedQr};
use crate::validate;
use crate::wire;
use openhub_bo_core::amount;
use openhub_bo_core::envelope::{CodeData, Envelope};
use openhub_bo_core::validate::require_text;
use openhub_bo_core::{Ctx, Error, HttpRequest, HttpResponse, Method, Operation, Result};

const BASE: &str = "payout/sync/v3/qr";

/// Codes meaning "result not confirmed": the payout may or may not have
/// happened (94: balance system, 96: issuer/ATC).
const UNCONFIRMED: [&str; 2] = ["94", "96"];

/// `POST /payout/sync/v3/qr/scan`: decodes a QR's text (recipient, amount).
pub struct ScanOp;
/// `POST /payout/sync/v3/qr/confirm`: pays a scanned QR. Moves money.
pub struct PayOp;
/// `GET /payout/sync/v3/qr/status/{numeroReferencia}`.
pub struct PayoutStatusOp;

impl Operation for ScanOp {
    const NAME: &'static str = "payouts.scan";
    const IDEMPOTENT: bool = true;
    const TIMEOUT_SECS: Option<u32> = Some(40);
    type Input = ScanQr;
    type Output = ScannedQr;

    fn request(ctx: &Ctx<'_>, input: &ScanQr) -> Result<HttpRequest> {
        require_text("qr_text", &input.qr_text)?;
        post(ctx, "scan", json!({"imagen": input.qr_text.trim()}))
    }

    fn response(response: &HttpResponse, _: &ScanQr) -> Result<ScannedQr> {
        CodeData::open::<wire::Scan>(response).map(Into::into)
    }
}

impl Operation for PayOp {
    const NAME: &'static str = "payouts.pay";
    const IDEMPOTENT: bool = false;
    const TIMEOUT_SECS: Option<u32> = Some(90);
    type Input = PayQr;
    type Output = Payout;

    fn request(ctx: &Ctx<'_>, input: &PayQr) -> Result<HttpRequest> {
        validate::digits("scanned.reference", &input.scanned.reference, 1, 20)?;
        validate::digits("source_account", &input.source_account, 6, 20)?;
        validate::identifier("transaction_id", &input.transaction_id, 1, 32)?;

        // Docs: send the amount only for open-amount QRs; otherwise 0.00.
        let open_amount = input.scanned.amount.is_zero();
        let importe = match (open_amount, input.amount) {
            (true, Some(amount)) => amount::to_json_number(amount::validate("amount", amount)?)?,
            (true, None) => {
                return Err(Error::validation(
                    "amount",
                    "required: the QR has no amount",
                ));
            }
            (false, Some(amount)) if amount != input.scanned.amount => {
                return Err(Error::validation(
                    "amount",
                    "the QR has a fixed amount; omit it",
                ));
            }
            (false, _) => zero_amount(),
        };
        let mut body = json!({
            "numeroReferencia": input.scanned.reference,
            "cuentaOrigen": input.source_account,
            "transaccionId": input.transaction_id,
            "importe": importe,
        });
        // Docs: send `glosa` only when the QR has none.
        if input
            .scanned
            .description
            .as_deref()
            .is_none_or(|d| d.trim().is_empty())
        {
            let description = input.description.as_deref().ok_or_else(|| {
                Error::validation("description", "required: the QR has no description")
            })?;
            validate::text("description", description, 1, 255)?;
            body["glosa"] = description.trim().into();
        }
        post(ctx, "confirm", body)
    }

    fn response(response: &HttpResponse, _: &PayQr) -> Result<Payout> {
        match CodeData::open::<wire::WPayout>(response) {
            Err(Error::Api {
                message, errors, ..
            }) if errors
                .iter()
                .any(|e| e.code.as_deref().is_some_and(|c| UNCONFIRMED.contains(&c))) =>
            {
                Err(Error::Ambiguous { message, errors })
            }
            other => other.map(Into::into),
        }
    }
}

impl Operation for PayoutStatusOp {
    const NAME: &'static str = "payouts.status";
    const IDEMPOTENT: bool = true;
    const TIMEOUT_SECS: Option<u32> = Some(90);
    type Input = PayoutRef;
    type Output = Payout;

    fn request(ctx: &Ctx<'_>, input: &PayoutRef) -> Result<HttpRequest> {
        validate::digits("reference", &input.reference, 1, 20)?;
        Ok(HttpRequest {
            method: Method::Get,
            url: ctx
                .config
                .url(&format!("{BASE}/status/{}", input.reference)),
            headers: ctx.api_headers()?,
            body: None,
        })
    }

    fn response(response: &HttpResponse, _: &PayoutRef) -> Result<Payout> {
        CodeData::open::<wire::WPayout>(response).map(Into::into)
    }
}

/// `0.00` written exactly, as the docs require for fixed-amount QRs.
fn zero_amount() -> Value {
    Value::Number("0.00".parse().expect("valid JSON number"))
}

fn post(ctx: &Ctx<'_>, path: &str, body: Value) -> Result<HttpRequest> {
    Ok(HttpRequest {
        method: Method::Post,
        url: ctx.config.url(&format!("{BASE}/{path}")),
        headers: ctx.api_headers()?,
        body: Some(body.to_string()),
    })
}
