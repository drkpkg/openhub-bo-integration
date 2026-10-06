//! OpenHub's JSON for the QR family, and its mapping to [`crate::model`].

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::model::{GeneratedQr, Payer, PayerBank, QrKind, QrStatusInfo};
use openhub_bo_core::amount;
use openhub_bo_core::status::{SpanishQr, StatusVocabulary};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Generated {
    numero_referencia: String,
    estado: String,
    #[serde(default)]
    fecha_expiracion: Option<String>,
    #[serde(default)]
    moneda: Option<String>,
    #[serde(deserialize_with = "amount::deserialize")]
    monto: Decimal,
    #[serde(default)]
    numero_referencia_originante: Option<String>,
    qr: String,
}

impl Generated {
    pub(crate) fn into_qr(self, kind: QrKind) -> GeneratedQr {
        let w = self;
        GeneratedQr {
            kind,
            reference: w.numero_referencia,
            merchant_reference: non_empty(w.numero_referencia_originante),
            status: SpanishQr::map(&w.estado),
            raw_status: w.estado,
            expires_at: non_empty(w.fecha_expiracion),
            currency: w.moneda.unwrap_or_else(|| crate::CURRENCY.to_owned()),
            amount: w.monto,
            qr_image_base64: w.qr,
        }
    }
}

/// `data` of status and cancel responses.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Status {
    numero_referencia: String,
    estado: String,
    #[serde(default)]
    mensaje: Option<String>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    importe: Option<Decimal>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    monto: Option<Decimal>,
    #[serde(default)]
    moneda: Option<String>,
    #[serde(default)]
    numero_referencia_originante: Option<String>,
    #[serde(default)]
    cliente_origen: Option<WirePayer>,
    #[serde(default)]
    banco_origen: Option<WireBank>,
}

impl Status {
    pub(crate) fn into_info(self, kind: QrKind) -> QrStatusInfo {
        let w = self;
        QrStatusInfo {
            kind,
            reference: w.numero_referencia,
            merchant_reference: non_empty(w.numero_referencia_originante),
            status: SpanishQr::map(&w.estado),
            raw_status: w.estado,
            message: non_empty(w.mensaje),
            amount: w.importe.or(w.monto),
            currency: non_empty(w.moneda),
            payer: w.cliente_origen.and_then(WirePayer::into_payer),
            payer_bank: w.banco_origen.and_then(WireBank::into_bank),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WirePayer {
    #[serde(default)]
    nombre_cliente: Option<String>,
    #[serde(default)]
    numero_cuenta: Option<String>,
    /// `ciNitCliente` in status responses, `ciCliente` in webhooks.
    #[serde(default, alias = "ciCliente")]
    ci_nit_cliente: Option<String>,
}

impl WirePayer {
    pub(super) fn into_payer(self) -> Option<Payer> {
        let payer = Payer {
            name: non_empty(self.nombre_cliente),
            account_number: non_empty(self.numero_cuenta),
            document_id: non_empty(self.ci_nit_cliente),
        };
        (payer != Payer::default()).then_some(payer)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WireBank {
    #[serde(default)]
    codigo_banco: Option<String>,
    #[serde(default)]
    nombre_banco: Option<String>,
    #[serde(default)]
    numero_orden_ach: Option<String>,
    #[serde(default)]
    fecha_transaccion: Option<String>,
}

impl WireBank {
    pub(super) fn into_bank(self) -> Option<PayerBank> {
        let bank = PayerBank {
            bank_code: non_empty(self.codigo_banco),
            bank_name: non_empty(self.nombre_banco),
            ach_order_number: non_empty(self.numero_orden_ach),
            transaction_date: non_empty(self.fecha_transaccion),
        };
        (bank != PayerBank::default()).then_some(bank)
    }
}

pub(super) fn non_empty(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}
