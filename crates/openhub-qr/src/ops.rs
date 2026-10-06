use crate::model::{CancelQr, GenerateQr, GeneratedQr, QrKind, QrStatusInfo, VerifyQr};
use crate::wire;
use openhub_core::amount;
use openhub_core::envelope::{Envelope, SuccessData};
use openhub_core::validate::{numeric_reference, positive, require_text};
use openhub_core::{Ctx, Operation};
use openhub_core::{Error, Result};
use openhub_core::{HttpRequest, HttpResponse, Method};

/// Largest merchant reference OpenHub accepts (`i32::MAX`).
pub const MAX_REFERENCE: u64 = 2_147_483_647;

/// `POST /qr/{simple|mld}/v2/generate`
pub struct Generate;

/// `GET /qr/{simple|mld}/v2/verify/{numeroReferencia}`
pub struct Verify;

/// `POST /qr/simple/v2/cancel/{numeroReferencia}`. Only `PENDIENTE` QRs can be
/// cancelled; otherwise OpenHub answers 409 `ESTADO_INVALIDO`. MLD-BCB has no
/// cancel endpoint.
pub struct Cancel;

impl Operation for Generate {
    const NAME: &'static str = "qr.generate";
    const IDEMPOTENT: bool = false;

    type Input = GenerateQr;
    type Output = GeneratedQr;

    fn request(ctx: &Ctx<'_>, input: &GenerateQr) -> Result<HttpRequest> {
        let body = generate_body(input)?;
        Ok(HttpRequest {
            method: Method::Post,
            url: ctx.config.url(&format!("{}/generate", input.kind.path())),
            headers: ctx.api_headers()?,
            body: Some(body.to_string()),
        })
    }

    fn response(response: &HttpResponse, input: &GenerateQr) -> Result<GeneratedQr> {
        Ok(SuccessData::open::<wire::Generated>(response)?.into_qr(input.kind))
    }
}

impl Operation for Verify {
    const NAME: &'static str = "qr.verify";
    const IDEMPOTENT: bool = true;

    type Input = VerifyQr;
    type Output = QrStatusInfo;

    fn request(ctx: &Ctx<'_>, input: &VerifyQr) -> Result<HttpRequest> {
        numeric_reference("reference", &input.reference)?;
        Ok(HttpRequest {
            method: Method::Get,
            url: ctx
                .config
                .url(&format!("{}/verify/{}", input.kind.path(), input.reference)),
            headers: ctx.api_headers()?,
            body: None,
        })
    }

    fn response(response: &HttpResponse, input: &VerifyQr) -> Result<QrStatusInfo> {
        Ok(SuccessData::open::<wire::Status>(response)?.into_info(input.kind))
    }
}

impl Operation for Cancel {
    const NAME: &'static str = "qr.cancel";
    const IDEMPOTENT: bool = false;

    type Input = CancelQr;
    /// Same `data` as a status query, without payer details (verified in sandbox).
    type Output = QrStatusInfo;

    fn request(ctx: &Ctx<'_>, input: &CancelQr) -> Result<HttpRequest> {
        numeric_reference("reference", &input.reference)?;
        Ok(HttpRequest {
            method: Method::Post,
            url: ctx.config.url(&format!(
                "{}/cancel/{}",
                QrKind::Simple.path(),
                input.reference
            )),
            headers: ctx.api_headers()?,
            body: None,
        })
    }

    fn response(response: &HttpResponse, _input: &CancelQr) -> Result<QrStatusInfo> {
        Ok(SuccessData::open::<wire::Status>(response)?.into_info(QrKind::Simple))
    }
}

fn generate_body(input: &GenerateQr) -> Result<serde_json::Value> {
    require_text("description", &input.description)?;
    require_text("establishment_name", &input.establishment_name)?;
    numeric_reference("reference", &input.reference)?;
    // Verified in sandbox: references above i32::MAX fail with 500
    // QR_GENERATION_ERROR (Simple and MLD); leading zeros are kept.
    if input
        .reference
        .parse::<u64>()
        .map_or(true, |n| n > MAX_REFERENCE)
    {
        return Err(Error::validation(
            "reference",
            format!("must be at most {MAX_REFERENCE} (OpenHub stores it as a 32-bit integer)"),
        ));
    }
    let amount = amount::validate("amount", input.amount)?;
    positive("expires_in", input.expires_in)?;
    positive("establishment_id", input.establishment_id)?;
    let webhook = input
        .webhook
        .as_ref()
        .ok_or_else(|| Error::validation("webhook", "is required by OpenHub"))?;
    webhook.validate()?;

    Ok(serde_json::json!({
        "glosa": input.description.trim(),
        "moneda": crate::CURRENCY,
        "monto": amount::to_json_number(amount)?,
        "numeroReferencia": input.reference,
        "vigencia": input.expires_in,
        "idEstablecimiento": input.establishment_id,
        "nombreEstablecimiento": input.establishment_name.trim(),
        "webhook": webhook,
    }))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;
    use rust_decimal::Decimal;

    use super::*;
    use openhub_core::AccessToken;
    use openhub_core::ApiFieldError;
    use openhub_core::Config;
    use openhub_core::status::PaymentStatus;
    use openhub_core::webhook::WebhookTarget;

    const GENERATE_REQUEST: &str = include_str!("../../../fixtures/openhub/generate_request.json");
    const GENERATE_RESPONSE: &str =
        include_str!("../../../fixtures/openhub/generate_response.json");
    const VERIFY_PENDING: &str =
        include_str!("../../../fixtures/openhub/verify_pending_response.json");
    const NOT_FOUND: &str = include_str!("../../../fixtures/openhub/error_not_found_response.json");
    const CANCEL_OK: &str = include_str!("../../../fixtures/openhub/cancel_response.json");
    const CANCEL_INVALID: &str =
        include_str!("../../../fixtures/openhub/cancel_invalid_state_response.json");
    const GENERATE_INVALID: &str =
        include_str!("../../../fixtures/openhub/generate_validation_error_response.json");

    fn token() -> AccessToken {
        AccessToken {
            access_token: "tok".into(),
            token_type: "Bearer".into(),
            expires_at: i64::MAX,
            scope: None,
        }
    }

    fn config() -> Config {
        Config::new("cid", "s")
    }

    fn sample_input() -> GenerateQr {
        GenerateQr {
            kind: QrKind::Simple,
            description: "Pago de servicio".into(),
            amount: Decimal::from_str("10.50").unwrap(),
            reference: "4024".into(),
            expires_in: 45,
            establishment_id: 422717,
            establishment_name: "Tienda Central".into(),
            webhook: Some(WebhookTarget {
                url: "https://dominio.com/qr/confirmed".into(),
                key: "x-api-key".into(),
                value: "46bc-b2a2-ea12258c99ab".into(),
            }),
        }
    }

    fn build(input: &GenerateQr) -> Result<HttpRequest> {
        let (config, token) = (config(), token());
        Generate::request(&Ctx::new(&config, Some(&token)), input)
    }

    #[test]
    fn generate_body_matches_documented_example() {
        let request = build(&sample_input()).unwrap();
        let body: serde_json::Value =
            serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        let expected: serde_json::Value = serde_json::from_str(GENERATE_REQUEST).unwrap();
        assert_eq!(body, expected);
        assert!(request.url.ends_with("/sandbox/qr/simple/v2/generate"));
        assert_eq!(request.headers["access_token"], "tok");
        assert_eq!(request.headers["Authorization"], "Bearer tok");
        assert_eq!(request.headers["client_id"], "cid");
    }

    #[test]
    fn reference_limit_is_inclusive_and_keeps_leading_zeros() {
        for reference in ["2147483647", "0000000123"] {
            let request = build(&GenerateQr {
                reference: reference.into(),
                ..sample_input()
            })
            .unwrap();
            let body: serde_json::Value =
                serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
            assert_eq!(body["numeroReferencia"], reference);
        }
    }

    #[test]
    fn mld_uses_its_own_path() {
        let request = build(&GenerateQr {
            kind: QrKind::Mld,
            ..sample_input()
        })
        .unwrap();
        assert!(request.url.ends_with("/qr/mld/v2/generate"));
    }

    #[test]
    fn requires_a_token() {
        let config = config();
        assert!(matches!(
            Generate::request(&Ctx::new(&config, None), &sample_input()),
            Err(Error::Validation { field, .. }) if field == "token"
        ));
    }

    #[test]
    fn rejects_invalid_input_locally() {
        let cases = [
            (
                GenerateQr {
                    amount: Decimal::from_str("1.001").unwrap(),
                    ..sample_input()
                },
                "amount",
            ),
            (
                GenerateQr {
                    reference: "../x".into(),
                    ..sample_input()
                },
                "reference",
            ),
            (
                GenerateQr {
                    reference: "ord-42".into(),
                    ..sample_input()
                },
                "reference",
            ),
            (
                GenerateQr {
                    description: " ".into(),
                    ..sample_input()
                },
                "description",
            ),
            (
                GenerateQr {
                    expires_in: 0,
                    ..sample_input()
                },
                "expires_in",
            ),
            (
                GenerateQr {
                    webhook: None,
                    ..sample_input()
                },
                "webhook",
            ),
        ];
        for (input, field) in cases {
            match build(&input) {
                Err(Error::Validation { field: f, .. }) => assert_eq!(f, field),
                other => panic!("expected validation error on {field}, got {other:?}"),
            }
        }
    }

    #[test]
    fn parses_generate_response() {
        let qr = Generate::response(&HttpResponse::new(200, GENERATE_RESPONSE), &sample_input())
            .unwrap();
        assert_eq!(qr.reference, "153980");
        assert_eq!(qr.merchant_reference.as_deref(), Some("2320"));
        assert_eq!(qr.status, PaymentStatus::Pending);
        assert_eq!(qr.amount, Decimal::from_str("10.5").unwrap());
        assert_eq!(qr.expires_at.as_deref(), Some("2026-03-12T18:01:52.304302"));
    }

    #[test]
    fn maps_sandbox_validation_errors() {
        let Err(Error::Api { status, errors, .. }) =
            Generate::response(&HttpResponse::new(400, GENERATE_INVALID), &sample_input())
        else {
            panic!("expected Api error");
        };
        assert_eq!(status, 400);
        let codes: Vec<_> = errors.iter().filter_map(|e| e.code.as_deref()).collect();
        assert_eq!(codes, ["REQUIRED_FIELD", "INVALID_FORMAT"]);
    }

    fn verify_input() -> VerifyQr {
        VerifyQr {
            kind: QrKind::Simple,
            reference: "200393".into(),
        }
    }

    #[test]
    fn parses_pending_status_with_empty_payer_as_none() {
        let info =
            Verify::response(&HttpResponse::new(200, VERIFY_PENDING), &verify_input()).unwrap();
        assert_eq!(info.status, PaymentStatus::Pending);
        assert_eq!(info.reference, "200393");
        assert_eq!(info.amount, Some(Decimal::from_str("10.5").unwrap()));
        assert_eq!(info.payer, None);
        assert_eq!(info.payer_bank, None);
    }

    #[test]
    fn maps_api_errors() {
        let Err(Error::Api { status, errors, .. }) =
            Verify::response(&HttpResponse::new(404, NOT_FOUND), &verify_input())
        else {
            panic!("expected Api error");
        };
        assert_eq!(status, 404);
        assert_eq!(
            errors,
            vec![ApiFieldError {
                field: Some("numeroReferencia".into()),
                message: "Transacción no encontrada con número de referencia: 260825000010075"
                    .into(),
                code: Some("TRANSACCION_NO_ENCONTRADA".into()),
            }]
        );
    }

    #[test]
    fn success_false_with_http_200_is_an_error() {
        assert!(matches!(
            Verify::response(&HttpResponse::new(200, NOT_FOUND), &verify_input()),
            Err(Error::Api { status: 200, .. })
        ));
    }

    #[test]
    fn expired_token_is_authentication_error() {
        assert!(matches!(
            Generate::response(&HttpResponse::new(401, ""), &sample_input()),
            Err(Error::Authentication { status: 401, .. })
        ));
    }

    #[test]
    fn cancel_request_and_responses() {
        let (config, token) = (config(), token());
        let input = CancelQr {
            reference: "11195853".into(),
        };
        let request = Cancel::request(&Ctx::new(&config, Some(&token)), &input).unwrap();
        assert_eq!(request.method, Method::Post);
        assert!(
            request
                .url
                .ends_with("/sandbox/qr/simple/v2/cancel/11195853")
        );
        assert_eq!(request.body, None);

        let cancelled = Cancel::response(&HttpResponse::new(200, CANCEL_OK), &input).unwrap();
        assert_eq!(cancelled.status, PaymentStatus::Cancelled);
        assert_eq!(cancelled.reference, "11195853");

        let Err(Error::Api { status, errors, .. }) =
            Cancel::response(&HttpResponse::new(409, CANCEL_INVALID), &input)
        else {
            panic!("expected Api error");
        };
        assert_eq!(status, 409);
        assert_eq!(errors[0].code.as_deref(), Some("ESTADO_INVALIDO"));
    }
}
