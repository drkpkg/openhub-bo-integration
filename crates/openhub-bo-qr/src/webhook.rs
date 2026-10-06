//! QR Simple / MLD-BCB payment notifications.

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::model::PaymentNotification;
use crate::wire::{WireBank, WirePayer, non_empty};
use openhub_bo_core::Handler;
use openhub_bo_core::WebhookCall;
use openhub_bo_core::amount;
use openhub_bo_core::status::{PaymentStatus, SpanishQr, StatusVocabulary};
use openhub_bo_core::{Error, Result};

/// Authenticates and parses a payment notification (`qr.webhook.parse`).
pub struct ParseWebhook;

impl Handler for ParseWebhook {
    const NAME: &'static str = "qr.webhook.parse";

    type Input = WebhookCall;
    type Output = PaymentNotification;

    fn handle(call: WebhookCall) -> Result<PaymentNotification> {
        call.authenticate()?;
        parse_payload(&call.body)
    }
}

/// The docs show two shapes: the example (`codigoRespuesta`, `monto`, ...)
/// and the prose ("same structure as the status query": `estado`,
/// `importe`, ...). Both are accepted.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire {
    numero_referencia: String,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    monto: Option<Decimal>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    importe: Option<Decimal>,
    #[serde(default)]
    moneda: Option<String>,
    #[serde(default)]
    codigo_respuesta: Option<String>,
    #[serde(default)]
    estado: Option<String>,
    #[serde(default, alias = "mensaje")]
    detalle_respuesta: Option<String>,
    #[serde(default)]
    fecha_hora_transaccion: Option<String>,
    #[serde(default)]
    cliente_origen: Option<WirePayer>,
    #[serde(default)]
    banco_origen: Option<WireBank>,
}

fn parse_payload(body: &str) -> Result<PaymentNotification> {
    let wire: Wire = serde_json::from_str(body)
        .map_err(|e| Error::decode(format!("invalid webhook payload: {e}")))?;
    let amount = wire
        .monto
        .or(wire.importe)
        .ok_or_else(|| Error::decode("webhook payload has no `monto`/`importe`"))?;
    let response_code = non_empty(wire.codigo_respuesta)
        .or_else(|| non_empty(wire.estado))
        .ok_or_else(|| Error::decode("webhook payload has no `codigoRespuesta`/`estado`"))?;
    // `SUCCESS` in the documented example, `PAGADO` in the status-shaped variant.
    let status = if response_code.eq_ignore_ascii_case("SUCCESS") {
        PaymentStatus::Paid
    } else {
        SpanishQr::map(&response_code)
    };
    let payer_bank = wire.banco_origen.and_then(WireBank::into_bank);
    let transaction_at = non_empty(wire.fecha_hora_transaccion)
        .or_else(|| payer_bank.as_ref().and_then(|b| b.transaction_date.clone()));
    Ok(PaymentNotification {
        reference: wire.numero_referencia,
        amount,
        currency: non_empty(wire.moneda),
        status,
        response_code,
        response_detail: non_empty(wire.detalle_respuesta),
        transaction_at,
        payer: wire.cliente_origen.and_then(WirePayer::into_payer),
        payer_bank,
        success: status == PaymentStatus::Paid,
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use std::collections::BTreeMap;

    use super::*;

    const PAYLOAD: &str = include_str!("../../../fixtures/openhub/webhook_payment.json");
    const SECRET: &str = "46bc-b2a2-ea12258c99ab";

    fn call(header: (&str, &str), body: &str) -> WebhookCall {
        WebhookCall {
            headers: BTreeMap::from([(header.0.to_owned(), header.1.to_owned())]),
            body: body.to_owned(),
            key: "x-api-key".into(),
            value: SECRET.into(),
        }
    }

    #[test]
    fn parses_documented_payload() {
        let n = ParseWebhook::handle(call(("X-API-KEY", SECRET), PAYLOAD)).unwrap();
        assert!(n.success);
        assert_eq!(n.status, PaymentStatus::Paid);
        assert_eq!(n.reference, "233324");
        assert_eq!(n.amount, Decimal::from_str("10.5").unwrap());
        assert_eq!(n.payer.unwrap().document_id.as_deref(), Some("12345678"));
        assert_eq!(
            n.payer_bank.unwrap().bank_name.as_deref(),
            Some("Banco Unión")
        );
    }

    #[test]
    fn accepts_status_shaped_payload() {
        let body = r#"{"estado":"PAGADO","mensaje":"QR pagado","importe":10.5,"moneda":"BOB",
            "numeroReferenciaOriginante":"4023","numeroReferencia":"200393",
            "clienteOrigen":{"nombreCliente":"Ana","numeroCuenta":"1","ciNitCliente":"9"},
            "bancoOrigen":{"numeroOrdenAch":"7","codigoBanco":"101","nombreBanco":"BNB",
            "fechaTransaccion":"2026-05-26T14:35:20"}}"#;
        let n = ParseWebhook::handle(call(("x-api-key", SECRET), body)).unwrap();
        assert!(n.success);
        assert_eq!(n.response_code, "PAGADO");
        assert_eq!(n.transaction_at.as_deref(), Some("2026-05-26T14:35:20"));
        assert_eq!(n.payer.unwrap().document_id.as_deref(), Some("9"));
    }

    #[test]
    fn non_payment_codes_are_not_success() {
        let body = r#"{"numeroReferencia":"1","monto":1,"codigoRespuesta":"ERROR"}"#;
        let n = ParseWebhook::handle(call(("x-api-key", SECRET), body)).unwrap();
        assert!(!n.success);
        assert_eq!(n.status, PaymentStatus::Error);
    }

    #[test]
    fn rejects_unauthenticated_calls() {
        assert!(matches!(
            ParseWebhook::handle(call(("x-api-key", "nope"), PAYLOAD)),
            Err(Error::WebhookAuth { .. })
        ));
    }
}
