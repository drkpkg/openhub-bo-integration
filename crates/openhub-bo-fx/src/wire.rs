//! OpenHub's JSON for the FX family, and its mapping to [`crate::model`].

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};

use crate::model::{FxQr, FxStatusInfo};
use openhub_bo_core::amount;
use openhub_bo_core::status::{EnglishCodes, StatusVocabulary};
use openhub_bo_core::{Error, Result};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Generated {
    codigo_respuesta: String,
    #[serde(default)]
    detalle_respuesta: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    numero_referencia: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    origen_numero_referencia: Option<String>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    monto: Option<Decimal>,
    #[serde(default)]
    moneda: Option<String>,
    #[serde(default)]
    imagen: Option<String>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    monto_conversion: Option<Decimal>,
    #[serde(default)]
    moneda_conversion: Option<String>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    tipo_cambio: Option<Decimal>,
    #[serde(default)]
    qr_expiracion: Option<String>,
}

impl Generated {
    pub(crate) fn into_qr(self) -> Result<FxQr> {
        let reference = non_empty(self.numero_referencia)
            .ok_or_else(|| Error::decode("generation response has no `numeroReferencia`"))?;
        let image = non_empty(self.imagen)
            .ok_or_else(|| Error::decode("generation response has no `imagen`"))?;
        Ok(FxQr {
            reference,
            merchant_reference: non_empty(self.origen_numero_referencia),
            status: EnglishCodes::map(&self.codigo_respuesta),
            raw_status: self.codigo_respuesta,
            detail: non_empty(self.detalle_respuesta),
            amount: self
                .monto
                .ok_or_else(|| Error::decode("generation response has no `monto`"))?,
            currency: non_empty(self.moneda).unwrap_or_default(),
            converted_amount: self.monto_conversion,
            converted_currency: non_empty(self.moneda_conversion).map(|c| c.to_ascii_uppercase()),
            exchange_rate: self.tipo_cambio,
            expires_at: non_empty(self.qr_expiracion),
            image_mime: sniff_mime(&image),
            image_base64: image,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    codigo_respuesta: String,
    #[serde(default)]
    detalle_respuesta: Option<String>,
    #[serde(default)]
    data: Option<StatusData>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct StatusData {
    #[serde(default, deserialize_with = "opt_text")]
    numero_referencia: Option<String>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    monto: Option<Decimal>,
    #[serde(default)]
    moneda: Option<String>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    monto_conversion: Option<Decimal>,
    #[serde(default)]
    moneda_conversion: Option<String>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    tipo_cambio: Option<Decimal>,
    #[serde(default)]
    reversa: Option<serde_json::Value>,
    #[serde(default)]
    fecha_solicitud: Option<String>,
}

impl Status {
    /// `reference` is the one queried: some responses omit it (or `data`).
    pub(crate) fn into_info(self, reference: &str) -> FxStatusInfo {
        let data = self.data.unwrap_or_default();
        FxStatusInfo {
            reference: non_empty(data.numero_referencia).unwrap_or_else(|| reference.to_owned()),
            status: EnglishCodes::map(&self.codigo_respuesta),
            raw_status: self.codigo_respuesta,
            detail: non_empty(self.detalle_respuesta),
            amount: data.monto,
            currency: non_empty(data.moneda),
            converted_amount: data.monto_conversion,
            converted_currency: non_empty(data.moneda_conversion).map(|c| c.to_ascii_uppercase()),
            exchange_rate: data.tipo_cambio,
            reversal: data.reversa.filter(|r| !r.is_null()),
            requested_at: non_empty(data.fecha_solicitud),
        }
    }
}

/// References come as strings or numbers depending on the product.
pub(crate) fn opt_text<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    Ok(match Option::<serde_json::Value>::deserialize(d)? {
        Some(serde_json::Value::String(s)) => Some(s),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        _ => None,
    })
}

pub(crate) fn non_empty(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

fn sniff_mime(base64: &str) -> &'static str {
    if base64.starts_with("iVBOR") {
        "image/png"
    } else if base64.starts_with("/9j/") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    }
}
