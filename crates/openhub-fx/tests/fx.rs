use std::str::FromStr;

use pretty_assertions::assert_eq;
use rust_decimal::Decimal;
use serde_json::{Value, json};

use openhub_core::ffi::{JsonValue, call};
use openhub_core::{
    AccessToken, Chain, Config, CoreOps, Ctx, Error, HttpRequest, HttpResponse, Method, Operation,
    PaymentStatus, WebhookTarget,
};
use openhub_fx::*;

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("../../../fixtures/openhub/fx/", $name))
    };
}

fn token() -> AccessToken {
    AccessToken {
        access_token: "tok".into(),
        token_type: "access_token".into(),
        expires_at: i64::MAX,
        scope: None,
    }
}

fn webhook() -> Option<WebhookTarget> {
    Some(WebhookTarget {
        url: "https://shop.example/hook".into(),
        key: "x-api-key".into(),
        value: "s3cret".into(),
    })
}

fn build<O: Operation>(input: &O::Input) -> openhub_core::Result<HttpRequest> {
    let (config, token) = (Config::new("cid", "s"), token());
    O::request(&Ctx::new(&config, Some(&token)), input)
}

fn body(request: &HttpRequest) -> Value {
    serde_json::from_str(request.body.as_deref().unwrap()).unwrap()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn pix() -> PixQr {
    PixQr {
        reference: "311113".into(),
        glosa: "311113|Compras QR Calacoto La paz|7011|Compra por Web QR".into(),
        amount: dec("145.00"),
        currency: Currency::Bob,
        channel: "web".into(),
        expires_in: Some(120),
        payer_cpf: "12345678901".into(),
        payer_phone: "+5511999999999".into(),
        payer_email: Some("cliente@email.com".into()),
        extra: None,
        webhook: webhook(),
    }
}

fn crypto() -> VirtualAssetQr {
    VirtualAssetQr {
        reference: "321".into(),
        glosa: "422717|Comercio test|MISCELANEAS|glosa test".into(),
        amount: dec("50"),
        currency: Currency::Bob,
        asset: VirtualAsset::Usdc,
        channel: "WEB".into(),
        expires_in: 180,
        extra: None,
        webhook: webhook(),
    }
}

fn binance() -> BinanceQr {
    BinanceQr {
        reference: "200397".into(),
        glosa: "401306|COMERCIO ATC|MISCELANEAS|TRANSACCION QR BINANCE".into(),
        amount: dec("0.01"),
        currency: Currency::Bob,
        channel: "WEB".into(),
        expires_in: Some(300),
        extra: None,
        webhook: webhook(),
    }
}

fn verify(reference: &str) -> VerifyFx {
    VerifyFx {
        reference: reference.into(),
    }
}

fn api_code(result: openhub_core::Result<impl std::fmt::Debug>) -> (u16, Option<String>, bool) {
    match result {
        Err(Error::Api {
            status,
            errors,
            retryable,
            ..
        }) => (status, errors[0].code.clone(), retryable),
        other => panic!("expected Api error, got {other:?}"),
    }
}

// -- requests -----------------------------------------------------------------

#[test]
fn pix_request_body() {
    let request = build::<PixGenerate>(&pix()).unwrap();
    assert_eq!(request.method, Method::Post);
    assert!(request.url.ends_with("/sandbox/qr/pix/v2/generar"));
    assert_eq!(request.headers["access_token"], "tok");
    assert_eq!(
        body(&request),
        json!({
            "numeroReferencia": "311113",
            "glosa": "311113|Compras QR Calacoto La paz|7011|Compra por Web QR",
            "monto": 145,
            "moneda": "BOB",
            "canal": "WEB",
            "tiempoQr": "00:02:00",
            "cpf": "12345678901",
            "telefono": "+5511999999999",
            "correoElectronico": "cliente@email.com",
            "webhook": {"url": "https://shop.example/hook", "key": "x-api-key", "value": "s3cret"},
        })
    );
}

#[test]
fn crypto_and_binance_request_bodies() {
    let request = build::<CryptoGenerate>(&crypto()).unwrap();
    assert!(request.url.ends_with("/qr/koibanx/v2/generar"));
    let b = body(&request);
    assert_eq!(
        (b["activoVirtual"].clone(), b["tiempoVencimientoQR"].clone()),
        (json!("UP"), json!(180))
    );
    assert_eq!(b["campoExtra"], "");

    let request = build::<BinanceGenerate>(&binance()).unwrap();
    assert!(request.url.ends_with("/qr/binance/v2/generar"));
    let b = body(&request);
    assert_eq!(
        (b["tiempoQr"].clone(), b["monto"].clone()),
        (json!("00:05:00"), json!(0.01))
    );
}

#[test]
fn status_requests_use_each_products_path() {
    let cases = [
        (
            build::<PixVerify>(&verify("6780")).unwrap(),
            "/qr/pix/v2/verifica/6780",
        ),
        (
            build::<PixCancel>(&verify("6780")).unwrap(),
            "/qr/pix/v2/cancela/6780",
        ),
        (
            build::<CryptoVerify>(&verify("6780")).unwrap(),
            "/qr/koibanx/v2/estado/6780",
        ),
        (
            build::<BinanceVerify>(&verify("6780")).unwrap(),
            "/qr/binance/v2/verificar/6780",
        ),
    ];
    for (request, path) in cases {
        assert_eq!(request.method, Method::Get, "{path}");
        assert!(request.url.ends_with(path), "{}", request.url);
    }
}

#[test]
fn validation_mirrors_sandbox_rules() {
    let field = |result: openhub_core::Result<HttpRequest>| match result {
        Err(Error::Validation { field, .. }) => field,
        other => panic!("expected validation error, got {other:?}"),
    };
    assert_eq!(
        field(build::<PixGenerate>(&PixQr {
            glosa: "Pago".into(),
            ..pix()
        })),
        "glosa"
    );
    assert_eq!(
        field(build::<PixGenerate>(&PixQr {
            payer_phone: "+59171234567".into(),
            ..pix()
        })),
        "payer_phone"
    );
    assert_eq!(
        field(build::<PixGenerate>(&PixQr {
            payer_cpf: "123".into(),
            ..pix()
        })),
        "payer_cpf"
    );
    assert_eq!(
        field(build::<PixGenerate>(&PixQr {
            webhook: None,
            ..pix()
        })),
        "webhook"
    );
    assert_eq!(
        field(build::<CryptoGenerate>(&VirtualAssetQr {
            amount: dec("49.99"),
            ..crypto()
        })),
        "amount"
    );
    assert_eq!(
        field(build::<CryptoGenerate>(&VirtualAssetQr {
            expires_in: 60,
            ..crypto()
        })),
        "expires_in"
    );
    assert_eq!(
        field(build::<BinanceGenerate>(&BinanceQr {
            expires_in: Some(301),
            ..binance()
        })),
        "expires_in"
    );
    assert_eq!(
        field(build::<BinanceGenerate>(&BinanceQr {
            reference: "12345678901".into(),
            ..binance()
        })),
        "reference"
    );
    // USD has no documented minimum.
    assert!(
        build::<CryptoGenerate>(&VirtualAssetQr {
            amount: dec("1"),
            currency: Currency::Usd,
            ..crypto()
        })
        .is_ok()
    );
}

// -- documented success responses ------------------------------------------------

#[test]
fn parses_documented_generations() {
    let qr = PixGenerate::response(
        &HttpResponse::new(200, fixture!("pix_generate_response.json")),
        &pix(),
    )
    .unwrap();
    assert_eq!(
        (qr.reference.as_str(), qr.merchant_reference.as_deref()),
        ("6780", Some("311113"))
    );
    assert_eq!(qr.status, PaymentStatus::Pending);
    assert_eq!(
        (qr.converted_amount, qr.converted_currency.as_deref()),
        (Some(dec("592.52")), Some("BRL"))
    );
    assert_eq!(qr.image_mime, "image/png");

    let qr = CryptoGenerate::response(
        &HttpResponse::new(200, fixture!("crypto_generate_response.json")),
        &crypto(),
    )
    .unwrap();
    assert_eq!(qr.converted_currency.as_deref(), Some("USDC"));
    assert_eq!(qr.exchange_rate, Some(dec("12.72")));

    let qr = BinanceGenerate::response(
        &HttpResponse::new(200, fixture!("binance_generate_response.json")),
        &binance(),
    )
    .unwrap();
    assert_eq!(qr.image_mime, "image/jpeg");
    assert_eq!(qr.converted_amount, Some(dec("0.00083046")));
    assert_eq!(qr.expires_at.as_deref(), Some("2026-09-17 14:43:51"));
}

#[test]
fn parses_documented_statuses() {
    let info = PixVerify::response(
        &HttpResponse::new(200, fixture!("pix_verify_response.json")),
        &verify("6780"),
    )
    .unwrap();
    assert_eq!(info.status, PaymentStatus::Cancelled);
    assert_eq!(info.reversal, None);

    let info = PixCancel::response(
        &HttpResponse::new(200, fixture!("pix_cancel_response.json")),
        &verify("6780"),
    )
    .unwrap();
    assert_eq!(info.reference, "6780");
    assert_eq!(info.converted_amount, Some(dec("592.52"))); // string in the payload
    assert_eq!(
        info.requested_at.as_deref(),
        Some("2026-04-20T15:30:00.000+00:00")
    );

    let info = CryptoVerify::response(
        &HttpResponse::new(200, fixture!("crypto_status_response.json")),
        &verify("200699"),
    )
    .unwrap();
    assert_eq!(
        (info.status, info.reference.as_str()),
        (PaymentStatus::Expired, "200699")
    );

    let info = BinanceVerify::response(
        &HttpResponse::new(200, fixture!("binance_status_response.json")),
        &verify("11193577"),
    )
    .unwrap();
    assert_eq!(info.status, PaymentStatus::Pending);
}

// -- real sandbox errors ---------------------------------------------------------

#[test]
fn maps_sandbox_errors() {
    let r = |status, body| HttpResponse::new(status, body);
    assert_eq!(
        api_code(PixGenerate::response(
            &r(200, fixture!("sandbox_pix_merchant_disabled.json")),
            &pix()
        )),
        (200, Some("GQ-00005".into()), false)
    );
    assert_eq!(
        api_code(PixGenerate::response(
            &r(400, fixture!("sandbox_pix_validation_error.json")),
            &pix()
        )),
        (400, Some("EG-00002".into()), false)
    );
    assert_eq!(
        api_code(PixVerify::response(
            &r(500, fixture!("sandbox_pix_not_found.json")),
            &verify("1")
        )),
        (500, Some("EG-00001".into()), false)
    );
    assert_eq!(
        api_code(PixCancel::response(
            &r(500, fixture!("sandbox_pix_cancel_not_allowed.json")),
            &verify("1")
        )),
        (500, Some("EG-00001".into()), false)
    );
    assert_eq!(
        api_code(CryptoGenerate::response(
            &r(400, fixture!("sandbox_crypto_min_amount.json")),
            &crypto()
        )),
        (400, Some("INVALID_VALUE".into()), false)
    );
    assert_eq!(
        api_code(CryptoGenerate::response(
            &r(200, fixture!("sandbox_crypto_generate_error.json")),
            &crypto()
        )),
        (200, Some("GQ-00006".into()), false)
    );
    assert_eq!(
        api_code(CryptoVerify::response(
            &r(200, fixture!("sandbox_crypto_status_not_found.json")),
            &verify("1")
        )),
        (200, Some("ERROR".into()), false)
    );
    assert_eq!(
        api_code(BinanceGenerate::response(
            &r(200, fixture!("sandbox_binance_merchant_error.json")),
            &binance()
        )),
        (200, Some("GQ-00000".into()), false)
    );
}

#[test]
fn sandbox_cancelled_status_without_data() {
    let info = PixVerify::response(
        &HttpResponse::new(200, fixture!("sandbox_pix_verify_cancelled_no_data.json")),
        &verify("11195888"),
    )
    .unwrap();
    assert_eq!(
        (info.status, info.reference.as_str(), info.amount),
        (PaymentStatus::Cancelled, "11195888", None)
    );
}

// -- FFI ---------------------------------------------------------------------------

#[test]
fn registry_describes_fx_operations() {
    let ops = Chain(CoreOps, FxOps);
    let described: JsonValue = serde_json::from_str(&call(&ops, "describe", "{}")).unwrap();
    let names: Vec<_> = described["value"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "token",
            "pix.generate",
            "pix.verify",
            "pix.cancel",
            "crypto.generate",
            "crypto.verify",
            "binance.generate",
            "binance.verify",
            "binance.webhook.parse",
            "fx.webhook.parse",
        ]
    );
    let cancel = &described["value"][3];
    assert_eq!(cancel["idempotent"], false);
}
