//! FX notifications. Binance's payload and required answer are documented;
//! PIX and Koibanx payloads are not, so they get a best-effort reader.

use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::model::{BinanceNotification, FxNotification};
use crate::wire::{non_empty, opt_text};
use openhub_core::amount;
use openhub_core::status::{EnglishCodes, SpanishQr, StatusVocabulary};
use openhub_core::{Error, Handler, PaymentStatus, Result, WebhookCall};

/// `binance.webhook.parse`
pub struct ParseBinanceWebhook;

/// `fx.webhook.parse` (PIX, Koibanx; payload undocumented).
pub struct ParseFxWebhook;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceWire {
    #[serde(deserialize_with = "opt_text")]
    numero_referencia: Option<String>,
    estado: String,
    #[serde(default)]
    transacciones: Option<BinanceTransaction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceTransaction {
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    monto: Option<Decimal>,
    #[serde(default)]
    moneda: Option<String>,
    #[serde(default)]
    fecha_hora_transaccion: Option<String>,
    #[serde(default)]
    cliente: Option<BinanceClient>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceClient {
    #[serde(default)]
    nombre_cliente: Option<String>,
    #[serde(default)]
    ci_cliente: Option<String>,
}

impl Handler for ParseBinanceWebhook {
    const NAME: &'static str = "binance.webhook.parse";
    type Input = WebhookCall;
    type Output = BinanceNotification;

    fn handle(call: WebhookCall) -> Result<BinanceNotification> {
        call.authenticate()?;
        let wire: BinanceWire = serde_json::from_str(&call.body)
            .map_err(|e| Error::decode(format!("invalid Binance webhook payload: {e}")))?;
        let reference = non_empty(wire.numero_referencia)
            .ok_or_else(|| Error::decode("Binance webhook has no `numeroReferencia`"))?;
        let tx = wire.transacciones;
        let client = tx.as_ref().and_then(|t| t.cliente.as_ref());
        Ok(BinanceNotification {
            status: fx_status(&wire.estado),
            raw_status: wire.estado,
            amount: tx.as_ref().and_then(|t| t.monto),
            currency: tx.as_ref().and_then(|t| non_empty(t.moneda.clone())),
            transaction_at: tx
                .as_ref()
                .and_then(|t| non_empty(t.fecha_hora_transaccion.clone())),
            payer_name: client.and_then(|c| non_empty(c.nombre_cliente.clone())),
            payer_document_id: client.and_then(|c| non_empty(c.ci_cliente.clone())),
            ack: json!({
                "numeroReferencia": reference,
                "codigoRespuesta": "00",
                "detalleRespuesta": null,
            }),
            reference,
        })
    }
}

impl Handler for ParseFxWebhook {
    const NAME: &'static str = "fx.webhook.parse";
    type Input = WebhookCall;
    type Output = FxNotification;

    fn handle(call: WebhookCall) -> Result<FxNotification> {
        call.authenticate()?;
        let payload: Value = serde_json::from_str(&call.body)
            .map_err(|e| Error::decode(format!("invalid webhook payload: {e}")))?;
        let text = |keys: &[&str]| {
            keys.iter().find_map(|k| match payload.get(*k) {
                Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_owned()),
                Some(Value::Number(n)) => Some(n.to_string()),
                _ => None,
            })
        };
        let raw_status = text(&["codigoRespuesta", "estado"]);
        let amount = ["monto", "importe"]
            .iter()
            .find_map(|k| payload.get(*k).and_then(amount_of));
        Ok(FxNotification {
            reference: text(&["numeroReferencia"]),
            status: raw_status
                .as_deref()
                .map_or(PaymentStatus::Unknown, fx_status),
            raw_status,
            amount,
            currency: text(&["moneda"]),
            payload,
        })
    }
}

/// `00` means approved in Binance callbacks; otherwise English codes, then Spanish.
fn fx_status(raw: &str) -> PaymentStatus {
    if raw.trim() == "00" {
        return PaymentStatus::Paid;
    }
    match EnglishCodes::map(raw) {
        PaymentStatus::Unknown => SpanishQr::map(raw),
        status => status,
    }
}

fn amount_of(value: &Value) -> Option<Decimal> {
    let text = match value {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => return None,
    };
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::str::FromStr;

    use super::*;

    const BINANCE: &str = include_str!("../../../fixtures/openhub/fx/binance_webhook.json");

    fn call(body: &str, secret: &str) -> WebhookCall {
        WebhookCall {
            headers: BTreeMap::from([("x-api-key".to_owned(), secret.to_owned())]),
            body: body.to_owned(),
            key: "x-api-key".into(),
            value: "s3cret".into(),
        }
    }

    #[test]
    fn binance_notification_and_ack() {
        let n = ParseBinanceWebhook::handle(call(BINANCE, "s3cret")).unwrap();
        assert_eq!(n.reference, "4221");
        assert_eq!(n.status, PaymentStatus::Paid);
        assert_eq!(n.amount, Some(Decimal::from_str("1.00").unwrap()));
        assert_eq!(n.payer_name, None);
        assert_eq!(
            n.ack,
            json!({"numeroReferencia": "4221", "codigoRespuesta": "00", "detalleRespuesta": null})
        );
    }

    #[test]
    fn binance_rejects_wrong_secret() {
        assert!(matches!(
            ParseBinanceWebhook::handle(call(BINANCE, "nope")),
            Err(Error::WebhookAuth { .. })
        ));
    }

    #[test]
    fn generic_fx_reads_what_it_can() {
        let body = r#"{"numeroReferencia": 6780, "codigoRespuesta": "PAID", "monto": "145.00", "moneda": "BOB", "extra": 1}"#;
        let n = ParseFxWebhook::handle(call(body, "s3cret")).unwrap();
        assert_eq!(n.reference.as_deref(), Some("6780"));
        assert_eq!(n.status, PaymentStatus::Paid);
        assert_eq!(n.amount, Some(Decimal::from_str("145.00").unwrap()));
        assert_eq!(n.payload["extra"], 1);

        let unknown = ParseFxWebhook::handle(call(r#"{"foo": "bar"}"#, "s3cret")).unwrap();
        assert_eq!(unknown.status, PaymentStatus::Unknown);
        assert_eq!(unknown.reference, None);
    }
}
