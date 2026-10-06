//! Batch notifications: ATC POSTs one per transfer to the `webhookUrl` given
//! at authorisation, authenticated by the `token` query parameter.

use serde::Deserialize;
use serde_json::json;

use crate::model::{BatchNotification, BatchTransferStatus};
use crate::wire::{WTransfer, non_empty};
use openhub_core::webhook::verify_secret;
use openhub_core::{Error, Handler, Result};

/// `batch.webhook.parse`
pub struct ParseBatchWebhook;

#[derive(Debug, Clone, Deserialize)]
pub struct BatchWebhookCall {
    /// The `token` query parameter of the incoming request.
    pub token: String,
    pub body: String,
    /// The token you set when authorising the batch.
    pub expected_token: String,
}

impl Handler for ParseBatchWebhook {
    const NAME: &'static str = "batch.webhook.parse";
    type Input = BatchWebhookCall;
    type Output = BatchNotification;

    fn handle(call: BatchWebhookCall) -> Result<BatchNotification> {
        verify_secret("`token` query parameter", &call.token, &call.expected_token)?;
        let wire: WTransfer = serde_json::from_str(&call.body)
            .map_err(|e| Error::decode(format!("invalid batch webhook payload: {e}")))?;
        let batch_number = non_empty(wire.nro_lote.clone());
        let transfer: BatchTransferStatus = wire.into();
        Ok(BatchNotification {
            ack: json!({
                "nroLote": batch_number,
                "numeroReferencia": transfer.reference,
                "codigoRespuesta": "EXITOSO",
                "detalleRespuesta": null,
            }),
            batch_number,
            transfer,
        })
    }
}
