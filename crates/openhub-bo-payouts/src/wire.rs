//! OpenHub's JSON for payouts, and its mapping to [`crate::model`].

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};

use crate::model::{
    Bank, BatchAuthorization, BatchStatus, BatchTransferResult, BatchTransferStatus, Payout,
    Recipient, ScannedQr,
};
use openhub_bo_core::amount;
use openhub_bo_core::status::{Payout as PayoutVocabulary, StatusVocabulary};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Scan {
    #[serde(deserialize_with = "text")]
    numero_referencia: String,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    importe: Option<Decimal>,
    #[serde(default)]
    moneda: Option<String>,
    #[serde(default)]
    glosa: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    cuenta_destino: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    ci_nit_destino: Option<String>,
    #[serde(default)]
    titular_destino: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    codigo_banco_destino: Option<String>,
    #[serde(default)]
    nombre_banco_destino: Option<String>,
    #[serde(default)]
    fecha_vencimiento: Option<String>,
}

impl From<Scan> for ScannedQr {
    fn from(w: Scan) -> Self {
        ScannedQr {
            reference: w.numero_referencia,
            amount: w.importe.unwrap_or_default(),
            currency: non_empty(w.moneda),
            description: non_empty(w.glosa),
            recipient: Recipient {
                account: non_empty(w.cuenta_destino),
                document_id: non_empty(w.ci_nit_destino),
                holder: non_empty(w.titular_destino),
                bank_code: non_empty(w.codigo_banco_destino),
                bank_name: non_empty(w.nombre_banco_destino),
            },
            expires_on: non_empty(w.fecha_vencimiento),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WPayout {
    #[serde(deserialize_with = "text")]
    numero_referencia: String,
    #[serde(default, deserialize_with = "opt_text")]
    transaccion_id: Option<String>,
    #[serde(default)]
    fecha_hora_transaccion: Option<String>,
    estado: String,
    #[serde(default)]
    mensaje: Option<String>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    importe: Option<Decimal>,
    #[serde(default)]
    moneda: Option<String>,
    #[serde(default)]
    glosa: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    cuenta_origen: Option<String>,
    #[serde(default)]
    titular_origen: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    cuenta_destino: Option<String>,
    #[serde(default)]
    titular_destino: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    ci_nit_destino: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    codigo_banco_destino: Option<String>,
    #[serde(default)]
    nombre_banco_destino: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    num_orden_ach: Option<String>,
}

impl From<WPayout> for Payout {
    fn from(w: WPayout) -> Self {
        Payout {
            reference: w.numero_referencia,
            transaction_id: non_empty(w.transaccion_id),
            transaction_at: non_empty(w.fecha_hora_transaccion),
            status: PayoutVocabulary::map(&w.estado),
            raw_status: w.estado,
            message: non_empty(w.mensaje),
            amount: w.importe,
            currency: non_empty(w.moneda),
            description: non_empty(w.glosa),
            source_account: non_empty(w.cuenta_origen),
            source_holder: non_empty(w.titular_origen),
            recipient: Recipient {
                account: non_empty(w.cuenta_destino),
                document_id: non_empty(w.ci_nit_destino),
                holder: non_empty(w.titular_destino),
                bank_code: non_empty(w.codigo_banco_destino),
                bank_name: non_empty(w.nombre_banco_destino),
            },
            ach_order_number: non_empty(w.num_orden_ach),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WAuthorized {
    #[serde(deserialize_with = "text")]
    transaccion_id: String,
    estado: String,
    #[serde(default, deserialize_with = "opt_text")]
    numero_referencia: Option<String>,
    #[serde(default)]
    mensaje: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Authorized {
    #[serde(deserialize_with = "text")]
    nro_lote: String,
    #[serde(deserialize_with = "text")]
    process_id: String,
    #[serde(default)]
    transacciones: Vec<WAuthorized>,
}

impl From<Authorized> for BatchAuthorization {
    fn from(w: Authorized) -> Self {
        BatchAuthorization {
            batch_number: w.nro_lote,
            process_id: w.process_id,
            transfers: w
                .transacciones
                .into_iter()
                .map(|t| BatchTransferResult {
                    transaction_id: t.transaccion_id,
                    status: PayoutVocabulary::map(&t.estado),
                    raw_status: t.estado,
                    reference: non_empty(t.numero_referencia),
                    message: non_empty(t.mensaje),
                })
                .collect(),
        }
    }
}

/// A transfer in batch status responses and webhook payloads.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WTransfer {
    #[serde(default, deserialize_with = "opt_text")]
    pub(crate) nro_lote: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    transaccion_id: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    numero_referencia: Option<String>,
    estado: String,
    #[serde(default)]
    mensaje: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    cuenta_origen: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    cuenta_destino: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    numero_ach: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    numero_destinatario: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    ci_cliente: Option<String>,
    #[serde(default)]
    nombre_cliente: Option<String>,
    #[serde(default)]
    fecha_hora_transaccion: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    codigo_banco: Option<String>,
    #[serde(default)]
    nombre_banco: Option<String>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    importe: Option<Decimal>,
    #[serde(default)]
    moneda: Option<String>,
}

impl From<WTransfer> for BatchTransferStatus {
    fn from(w: WTransfer) -> Self {
        BatchTransferStatus {
            transaction_id: non_empty(w.transaccion_id),
            reference: non_empty(w.numero_referencia),
            status: PayoutVocabulary::map(&w.estado),
            raw_status: w.estado,
            message: non_empty(w.mensaje),
            source_account: non_empty(w.cuenta_origen),
            destination_account: non_empty(w.cuenta_destino),
            ach_number: non_empty(w.numero_ach),
            recipient_number: non_empty(w.numero_destinatario),
            recipient_document_id: non_empty(w.ci_cliente),
            recipient_name: non_empty(w.nombre_cliente),
            transaction_at: non_empty(w.fecha_hora_transaccion),
            bank_code: non_empty(w.codigo_banco),
            bank_name: non_empty(w.nombre_banco),
            amount: w.importe,
            currency: non_empty(w.moneda),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WBatchStatus {
    #[serde(default, deserialize_with = "opt_text")]
    nro_lote: Option<String>,
    #[serde(default)]
    transacciones: Vec<WTransfer>,
}

impl From<WBatchStatus> for BatchStatus {
    fn from(w: WBatchStatus) -> Self {
        BatchStatus {
            batch_number: non_empty(w.nro_lote),
            transfers: w.transacciones.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WBank {
    #[serde(deserialize_with = "text")]
    codigo_banco: String,
    #[serde(default)]
    descripcion: Option<String>,
}

impl From<WBank> for Bank {
    fn from(w: WBank) -> Self {
        Bank {
            code: w.codigo_banco,
            name: non_empty(w.descripcion),
        }
    }
}

pub(crate) fn non_empty(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

fn opt_text<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(match Option::<serde_json::Value>::deserialize(d)? {
        Some(serde_json::Value::String(s)) => Some(s),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        _ => None,
    })
}

fn text<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    opt_text(d)?.ok_or_else(|| serde::de::Error::custom("expected a string or number"))
}
