//! Response envelopes. OpenHub wraps payloads differently per product family;
//! each [`Envelope`] strategy turns an HTTP response into the payload or a
//! typed [`Error`], so operations only deal with their own data.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{ApiFieldError, Error, Result};
use crate::http::HttpResponse;

pub trait Envelope {
    fn open<T: DeserializeOwned>(response: &HttpResponse) -> Result<T>;
}

/// QR Simple / MLD-BCB: `{success, message, data | errors[]}`.
pub struct SuccessData;

/// PIX / virtual assets / Binance: flat body with a top-level
/// `codigoRespuesta`; the whole body is the payload. Errors seen in the
/// sandbox come in three shapes, all handled here:
/// - `{codigoRespuesta: "ERROR", detalleRespuesta: "... - GQ-00005"}` (HTTP 200)
/// - `{error: true, code: "EG-00002", message, data: [..] | null}` (HTTP 400/500)
/// - `{success: false, message, errors: [..]}` (HTTP 400, virtual assets)
pub struct CodigoRespuesta;

/// Payouts / merchant accounts: `{code: "00", data, message | errorMessage, errorCode}`.
pub struct CodeData;

const BODY_PREVIEW: usize = 200;

#[derive(Deserialize)]
struct SuccessWire {
    #[serde(default)]
    success: Option<bool>,
    #[serde(default)]
    message: Option<String>,
    data: Option<Value>,
    #[serde(default)]
    errors: Vec<ApiFieldError>,
}

impl SuccessWire {
    fn into_error(self, response: &HttpResponse) -> Error {
        Error::business(
            response.status,
            self.message.unwrap_or_else(|| failed(response)),
            self.errors,
        )
    }
}

impl Envelope for SuccessData {
    fn open<T: DeserializeOwned>(response: &HttpResponse) -> Result<T> {
        let wire: SuccessWire = json_body(response)?;
        if wire.success == Some(false) {
            return Err(wire.into_error(response));
        }
        if !response.is_success() {
            return Err(Error::api(
                response.status,
                wire.message.unwrap_or_else(|| failed(response)),
                wire.errors,
            ));
        }
        let data = wire
            .data
            .ok_or_else(|| Error::decode("response has no `data` field"))?;
        decode_payload(data)
    }
}

impl Envelope for CodigoRespuesta {
    fn open<T: DeserializeOwned>(response: &HttpResponse) -> Result<T> {
        let body: Value = json_body(response)?;

        if body.get("error").and_then(Value::as_bool) == Some(true) {
            return Err(gateway_error(response, &body));
        }
        if body.get("success").and_then(Value::as_bool) == Some(false) {
            let wire: SuccessWire = decode_payload(body)?;
            return Err(wire.into_error(response));
        }
        let code = body.get("codigoRespuesta").and_then(Value::as_str);
        if code.is_some_and(|c| c.eq_ignore_ascii_case("ERROR")) {
            let detail = text_field(&body, &["detalleRespuesta", "message"])
                .unwrap_or_else(|| failed(response));
            // Backend codes travel at the end of the detail: "... - GQ-00005".
            let backend_code = detail
                .rsplit_once(" - ")
                .map(|(_, c)| c.trim())
                .filter(|c| c.starts_with("GQ-"))
                .map(str::to_owned);
            return Err(Error::business(
                response.status,
                detail.clone(),
                vec![ApiFieldError {
                    field: None,
                    message: detail,
                    code: backend_code.or_else(|| Some("ERROR".to_owned())),
                }],
            ));
        }
        if !response.is_success() {
            let message = text_field(&body, &["detalleRespuesta", "message"])
                .unwrap_or_else(|| failed(response));
            return Err(Error::api(response.status, message, Vec::new()));
        }
        decode_payload(body)
    }
}

impl Envelope for CodeData {
    fn open<T: DeserializeOwned>(response: &HttpResponse) -> Result<T> {
        let mut body: Value = json_body(response)?;
        let code = body.get("code").and_then(Value::as_str).map(str::to_owned);
        let rejected = code.as_deref().is_some_and(|c| c != "00");
        if rejected || !response.is_success() {
            let message =
                text_field(&body, &["errorMessage", "message"]).unwrap_or_else(|| failed(response));
            let error_code = text_field(&body, &["errorCode"]).or(code);
            // Validation errors list every message in `data`, or join them
            // with "; " in the message (both seen in the sandbox).
            let mut details = string_list(body.get("data"));
            if details.is_empty() && message.contains("; ") {
                details = message.split("; ").map(str::to_owned).collect();
            }
            let errors = if details.is_empty() {
                vec![ApiFieldError {
                    field: None,
                    message: message.clone(),
                    code: error_code,
                }]
            } else {
                details
                    .into_iter()
                    .map(|detail| ApiFieldError {
                        field: None,
                        message: detail,
                        code: error_code.clone(),
                    })
                    .collect()
            };
            return Err(if rejected {
                Error::business(response.status, message, errors)
            } else {
                Error::api(response.status, message, errors)
            });
        }
        // Most responses nest the payload in `data`; some (batch status) are flat.
        let payload = match body.get_mut("data").map(Value::take) {
            Some(data) if !data.is_null() => data,
            _ => body,
        };
        decode_payload(payload)
    }
}

/// `{error: true, code, message, data: [messages] | null}` from the gateway layer.
fn gateway_error(response: &HttpResponse, body: &Value) -> Error {
    let code = text_field(body, &["code"]);
    let message = text_field(body, &["message"]).unwrap_or_else(|| failed(response));
    let details = string_list(body.get("data"));
    let errors = if details.is_empty() {
        vec![ApiFieldError {
            field: None,
            message: message.clone(),
            code,
        }]
    } else {
        details
            .into_iter()
            .map(|detail| ApiFieldError {
                field: None,
                message: detail,
                code: code.clone(),
            })
            .collect()
    };
    Error::business(response.status, message, errors)
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Auth failures and non-JSON bodies are handled the same for every family.
fn json_body<T: DeserializeOwned>(response: &HttpResponse) -> Result<T> {
    if matches!(response.status, 401 | 403) {
        return Err(Error::Authentication {
            status: response.status,
            message: message_from_body(&response.body),
        });
    }
    serde_json::from_str(&response.body).map_err(|e| {
        if response.is_success() {
            Error::decode(format!("invalid JSON body: {e}"))
        } else {
            Error::api(response.status, preview(&response.body), Vec::new())
        }
    })
}

fn decode_payload<T: DeserializeOwned>(value: Value) -> Result<T> {
    serde_json::from_value(value)
        .map_err(|e| Error::decode(format!("unexpected payload shape: {e}")))
}

fn text_field(body: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|k| body.get(*k).and_then(Value::as_str))
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map(str::to_owned)
}

fn failed(response: &HttpResponse) -> String {
    format!("request failed with HTTP {}", response.status)
}

/// Best-effort human message from an arbitrary error body.
pub fn message_from_body(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if let Some(text) = text_field(&value, &["error_description", "message", "error"]) {
            return text;
        }
    }
    preview(body)
}

fn preview(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "empty response body".to_owned();
    }
    trimmed.chars().take(BODY_PREVIEW).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api_err(result: Result<Value>) -> (u16, String, Vec<ApiFieldError>, bool) {
        match result {
            Err(Error::Api {
                status,
                message,
                errors,
                retryable,
            }) => (status, message, errors, retryable),
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[test]
    fn plain_text_gateway_errors_are_retryable() {
        let (status, _, _, retryable) = api_err(SuccessData::open(&HttpResponse::new(
            502,
            "Error forwarding call",
        )));
        assert_eq!(status, 502);
        assert!(retryable);
    }

    #[test]
    fn auth_failures_win_over_body_parsing() {
        let err = CodeData::open::<Value>(&HttpResponse::new(401, "Access Token ... is invalid"))
            .unwrap_err();
        assert!(matches!(err, Error::Authentication { status: 401, .. }));
    }

    #[test]
    fn codigo_respuesta_returns_whole_body() {
        // Documented PIX status response.
        let body = r#"{"codigoRespuesta":"CANCELLED","detalleRespuesta":"Transaccion cancelada",
            "data":{"numeroReferencia":"6780","monto":696.0}}"#;
        let value: Value = CodigoRespuesta::open(&HttpResponse::new(200, body)).unwrap();
        assert_eq!(value["codigoRespuesta"], "CANCELLED");
        assert_eq!(value["data"]["numeroReferencia"], "6780");
    }

    #[test]
    fn codigo_respuesta_error_extracts_backend_code() {
        // Sandbox: merchant not enabled for PIX.
        let body = r#"{"codigoRespuesta":"ERROR","detalleRespuesta":"El ID de comercio no está habilitado para utilizar el servicio PIX - GQ-00005"}"#;
        let (status, message, errors, retryable) =
            api_err(CodigoRespuesta::open(&HttpResponse::new(200, body)));
        assert_eq!(status, 200);
        assert!(message.contains("no está habilitado"));
        assert_eq!(errors[0].code.as_deref(), Some("GQ-00005"));
        assert!(!retryable);

        let plain = r#"{"codigoRespuesta":"ERROR","detalleRespuesta":"Transacción no encontrada","data":null}"#;
        let (_, _, errors, _) = api_err(CodigoRespuesta::open(&HttpResponse::new(200, plain)));
        assert_eq!(errors[0].code.as_deref(), Some("ERROR"));
    }

    #[test]
    fn gateway_business_errors_with_500_are_not_retryable() {
        // Sandbox: PIX answers "not found" with HTTP 500.
        let body =
            r#"{"data":null,"error":true,"code":"EG-00001","message":"Transacción no encontrada"}"#;
        let (status, message, errors, retryable) =
            api_err(CodigoRespuesta::open(&HttpResponse::new(500, body)));
        assert_eq!(
            (status, message.as_str()),
            (500, "Transacción no encontrada")
        );
        assert_eq!(errors[0].code.as_deref(), Some("EG-00001"));
        assert!(!retryable);
    }

    #[test]
    fn gateway_validation_lists_every_message() {
        let body = r#"{"data":["glosa no puede estar vacía","cpf no puede ser nulo"],"error":true,"code":"EG-00002","message":"Errores de validación"}"#;
        let (status, _, errors, _) = api_err(CodigoRespuesta::open(&HttpResponse::new(400, body)));
        assert_eq!(status, 400);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[1].message, "cpf no puede ser nulo");
        assert_eq!(errors[1].code.as_deref(), Some("EG-00002"));
    }

    #[test]
    fn codigo_respuesta_accepts_success_false_errors() {
        // Sandbox: virtual assets validation uses the QR-family shape.
        let body = r#"{"success":false,"message":"Error de validación","errors":[{"field":"monto","message":"El monto mínimo permitido es 50","code":"INVALID_VALUE"}]}"#;
        let (status, _, errors, _) = api_err(CodigoRespuesta::open(&HttpResponse::new(400, body)));
        assert_eq!(status, 400);
        assert_eq!(errors[0].field.as_deref(), Some("monto"));
    }

    #[test]
    fn code_data_success_and_failure() {
        let ok = r#"{"data":{"nit":"1023149021"},"code":"00","errorCode":null,"errorMessage":""}"#;
        let data: Value = CodeData::open(&HttpResponse::new(200, ok)).unwrap();
        assert_eq!(data["nit"], "1023149021");

        // Batch status responses are flat.
        let flat = r#"{"code":"00","message":"ok","nroLote":"L1","transacciones":[]}"#;
        let data: Value = CodeData::open(&HttpResponse::new(200, flat)).unwrap();
        assert_eq!(data["nroLote"], "L1");

        // Sandbox: validation lists every message.
        let invalid = r#"{"data":["Debe enviar el nit.","Debe enviar al menos una cuenta."],"code":"02","errorMessage":"Error de validación de solicitud"}"#;
        let (_, _, errors, _) = api_err(CodeData::open(&HttpResponse::new(200, invalid)));
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[1].message, "Debe enviar al menos una cuenta.");
        assert_eq!(errors[1].code.as_deref(), Some("02"));

        let joined = r#"{"code":"02","errorMessage":"El campo 'transaccionId' es obligatorio; El campo 'importe' es obligatorio"}"#;
        let (status, _, errors, _) = api_err(CodeData::open(&HttpResponse::new(400, joined)));
        assert_eq!(status, 400);
        assert_eq!(errors[1].message, "El campo 'importe' es obligatorio");

        let missing = r#"{"code":"04","errorMessage":"Transacción no encontrada."}"#;
        let (status, message, errors, retryable) =
            api_err(CodeData::open(&HttpResponse::new(200, missing)));
        assert_eq!(
            (status, message.as_str()),
            (200, "Transacción no encontrada.")
        );
        assert_eq!(errors[0].code.as_deref(), Some("04"));
        assert!(!retryable);
    }
}
