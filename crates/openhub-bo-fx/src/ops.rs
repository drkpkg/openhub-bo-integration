use serde_json::{Map, Value, json};

use crate::model::{BinanceQr, Currency, FxQr, FxStatusInfo, PixQr, VerifyFx, VirtualAssetQr};
use crate::validate;
use crate::wire;
use openhub_bo_core::amount;
use openhub_bo_core::envelope::{CodigoRespuesta, Envelope};
use openhub_bo_core::validate::numeric_reference;
use openhub_bo_core::{Ctx, Error, HttpRequest, HttpResponse, Method, Operation, Result};

const PIX: &str = "qr/pix/v2";
const KOIBANX: &str = "qr/koibanx/v2";
const BINANCE: &str = "qr/binance/v2";

/// `POST /qr/pix/v2/generar`
pub struct PixGenerate;
/// `GET /qr/pix/v2/verifica/{numeroReferencia}`
pub struct PixVerify;
/// `GET /qr/pix/v2/cancela/{numeroReferencia}`. The docs say it voids an
/// unpaid QR; the sandbox only allows it once the QR is "aprobado" (paid),
/// i.e. it behaves as a refund request.
pub struct PixCancel;
/// `POST /qr/koibanx/v2/generar`
pub struct CryptoGenerate;
/// `GET /qr/koibanx/v2/estado/{numeroReferencia}`
pub struct CryptoVerify;
/// `POST /qr/binance/v2/generar`
pub struct BinanceGenerate;
/// `GET /qr/binance/v2/verificar/{numeroReferencia}`
pub struct BinanceVerify;

impl Operation for PixGenerate {
    const NAME: &'static str = "pix.generate";
    const IDEMPOTENT: bool = false;
    type Input = PixQr;
    type Output = FxQr;

    fn request(ctx: &Ctx<'_>, input: &PixQr) -> Result<HttpRequest> {
        validate::reference(&input.reference, None)?;
        validate::cpf(&input.payer_cpf)?;
        validate::phone(&input.payer_phone)?;
        let mut body = common_body(
            &input.reference,
            &input.glosa,
            input.amount,
            input.currency,
            &input.channel,
            input.webhook.as_ref(),
        )?;
        if let Some(seconds) = input.expires_in {
            body.insert(
                "tiempoQr".into(),
                validate::hhmmss("expires_in", seconds, 86_399)?.into(),
            );
        }
        body.insert("cpf".into(), input.payer_cpf.clone().into());
        body.insert("telefono".into(), input.payer_phone.clone().into());
        if let Some(email) = &input.payer_email {
            validate::email(email)?;
            body.insert("correoElectronico".into(), email.clone().into());
        }
        if let Some(extra) = &input.extra {
            body.insert("campoExtra".into(), extra.clone().into());
        }
        post(ctx, &format!("{PIX}/generar"), body)
    }

    fn response(response: &HttpResponse, _input: &PixQr) -> Result<FxQr> {
        CodigoRespuesta::open::<wire::Generated>(response)?.into_qr()
    }
}

impl Operation for CryptoGenerate {
    const NAME: &'static str = "crypto.generate";
    const IDEMPOTENT: bool = false;
    type Input = VirtualAssetQr;
    type Output = FxQr;

    fn request(ctx: &Ctx<'_>, input: &VirtualAssetQr) -> Result<HttpRequest> {
        validate::reference(&input.reference, None)?;
        if input.currency == Currency::Bob && input.amount < 50.into() {
            return Err(Error::validation("amount", "minimum is Bs 50"));
        }
        if !(180..=600).contains(&input.expires_in) {
            return Err(Error::validation(
                "expires_in",
                "must be between 180 and 600 seconds",
            ));
        }
        let mut body = common_body(
            &input.reference,
            &input.glosa,
            input.amount,
            input.currency,
            &input.channel,
            input.webhook.as_ref(),
        )?;
        body.insert("activoVirtual".into(), input.asset.code().into());
        body.insert("tiempoVencimientoQR".into(), input.expires_in.into());
        body.insert(
            "campoExtra".into(),
            input.extra.clone().unwrap_or_default().into(),
        );
        post(ctx, &format!("{KOIBANX}/generar"), body)
    }

    fn response(response: &HttpResponse, _input: &VirtualAssetQr) -> Result<FxQr> {
        CodigoRespuesta::open::<wire::Generated>(response)?.into_qr()
    }
}

impl Operation for BinanceGenerate {
    const NAME: &'static str = "binance.generate";
    const IDEMPOTENT: bool = false;
    type Input = BinanceQr;
    type Output = FxQr;

    fn request(ctx: &Ctx<'_>, input: &BinanceQr) -> Result<HttpRequest> {
        validate::reference(&input.reference, Some(10))?;
        let mut body = common_body(
            &input.reference,
            &input.glosa,
            input.amount,
            input.currency,
            &input.channel,
            input.webhook.as_ref(),
        )?;
        if let Some(seconds) = input.expires_in {
            body.insert(
                "tiempoQr".into(),
                validate::hhmmss("expires_in", seconds, 300)?.into(),
            );
        }
        body.insert(
            "campoExtra".into(),
            input.extra.clone().unwrap_or_default().into(),
        );
        post(ctx, &format!("{BINANCE}/generar"), body)
    }

    fn response(response: &HttpResponse, _input: &BinanceQr) -> Result<FxQr> {
        CodigoRespuesta::open::<wire::Generated>(response)?.into_qr()
    }
}

/// Status-shaped operations: one GET with ATC's reference.
macro_rules! status_op {
    ($ty:ident, $name:literal, $idempotent:literal, $path:expr) => {
        impl Operation for $ty {
            const NAME: &'static str = $name;
            const IDEMPOTENT: bool = $idempotent;
            type Input = VerifyFx;
            type Output = FxStatusInfo;

            fn request(ctx: &Ctx<'_>, input: &VerifyFx) -> Result<HttpRequest> {
                numeric_reference("reference", &input.reference)?;
                Ok(HttpRequest {
                    method: Method::Get,
                    url: ctx.config.url(&format!("{}/{}", $path, input.reference)),
                    headers: ctx.api_headers()?,
                    body: None,
                })
            }

            fn response(response: &HttpResponse, input: &VerifyFx) -> Result<FxStatusInfo> {
                Ok(CodigoRespuesta::open::<wire::Status>(response)?.into_info(&input.reference))
            }
        }
    };
}

status_op!(PixVerify, "pix.verify", true, format!("{PIX}/verifica"));
status_op!(PixCancel, "pix.cancel", false, format!("{PIX}/cancela"));
status_op!(
    CryptoVerify,
    "crypto.verify",
    true,
    format!("{KOIBANX}/estado")
);
status_op!(
    BinanceVerify,
    "binance.verify",
    true,
    format!("{BINANCE}/verificar")
);

fn common_body(
    reference: &str,
    glosa: &str,
    amount: rust_decimal::Decimal,
    currency: Currency,
    channel: &str,
    webhook: Option<&openhub_bo_core::WebhookTarget>,
) -> Result<Map<String, Value>> {
    let amount = amount::validate("amount", amount)?;
    let webhook = validate::webhook(webhook)?;
    let body = json!({
        "numeroReferencia": reference,
        "glosa": validate::glosa(glosa)?,
        "monto": amount::to_json_number(amount)?,
        "moneda": currency.code(),
        "canal": validate::channel(channel)?,
        "webhook": webhook,
    });
    match body {
        Value::Object(map) => Ok(map),
        _ => unreachable!("json! object literal"),
    }
}

fn post(ctx: &Ctx<'_>, path: &str, body: Map<String, Value>) -> Result<HttpRequest> {
    Ok(HttpRequest {
        method: Method::Post,
        url: ctx.config.url(path),
        headers: ctx.api_headers()?,
        body: Some(Value::Object(body).to_string()),
    })
}
