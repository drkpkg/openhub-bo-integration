//! OpenHub's JSON for merchant accounts, and its mapping to [`crate::model`].

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};

use crate::model::{
    Account, AccountStatus, Balance, CreatedAccounts, Establishment, MerchantAccount,
    MerchantAccounts, Movement, Party, Reconciliation, StatusChanged,
};
use openhub_bo_core::amount;
use openhub_bo_core::status::{Payout, StatusVocabulary};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WAccount {
    #[serde(deserialize_with = "text")]
    numero_cuenta: String,
    #[serde(default)]
    alias: Option<String>,
    #[serde(default)]
    estado: Option<String>,
}

impl WAccount {
    /// Accounts without `estado` (just created) are active.
    fn into_account(self) -> Account {
        let raw_status = non_empty(self.estado);
        Account {
            number: self.numero_cuenta,
            alias: non_empty(self.alias),
            status: raw_status
                .as_deref()
                .map_or(AccountStatus::Active, AccountStatus::from_wire),
            raw_status,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Detail {
    #[serde(deserialize_with = "text")]
    nit: String,
    #[serde(default)]
    nombre_comercio: Option<String>,
    #[serde(default, deserialize_with = "opt_u64")]
    id_establecimiento: Option<u64>,
    #[serde(default)]
    nombre_establecimiento: Option<String>,
    cuenta: WAccount,
}

impl From<Detail> for MerchantAccount {
    fn from(w: Detail) -> Self {
        MerchantAccount {
            nit: w.nit,
            merchant_name: non_empty(w.nombre_comercio),
            establishment_id: w.id_establecimiento,
            establishment_name: non_empty(w.nombre_establecimiento),
            account: w.cuenta.into_account(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WEstablishment {
    #[serde(deserialize_with = "u64_any")]
    id_establecimiento: u64,
    #[serde(default)]
    nombre_establecimiento: Option<String>,
    #[serde(default)]
    cuentas: Vec<WAccount>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct List {
    #[serde(deserialize_with = "text")]
    nit: String,
    #[serde(default)]
    nombre_comercio: Option<String>,
    #[serde(default)]
    establecimientos: Vec<WEstablishment>,
}

impl From<List> for MerchantAccounts {
    fn from(w: List) -> Self {
        MerchantAccounts {
            nit: w.nit,
            merchant_name: non_empty(w.nombre_comercio),
            establishments: w
                .establecimientos
                .into_iter()
                .map(|e| Establishment {
                    id: e.id_establecimiento,
                    name: non_empty(e.nombre_establecimiento),
                    accounts: e.cuentas.into_iter().map(WAccount::into_account).collect(),
                })
                .collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Created {
    #[serde(deserialize_with = "text")]
    nit: String,
    #[serde(default)]
    nombre_comercio: Option<String>,
    #[serde(deserialize_with = "u64_any")]
    id_establecimiento: u64,
    #[serde(default)]
    nombre_establecimiento: Option<String>,
    #[serde(default)]
    cuentas: Vec<WAccount>,
}

impl From<Created> for CreatedAccounts {
    fn from(w: Created) -> Self {
        CreatedAccounts {
            nit: w.nit,
            merchant_name: non_empty(w.nombre_comercio),
            establishment_id: w.id_establecimiento,
            establishment_name: non_empty(w.nombre_establecimiento),
            accounts: w.cuentas.into_iter().map(WAccount::into_account).collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Changed {
    #[serde(deserialize_with = "text")]
    numero_cuenta: String,
    estado: String,
}

impl From<Changed> for StatusChanged {
    fn from(w: Changed) -> Self {
        StatusChanged {
            account_number: w.numero_cuenta,
            status: AccountStatus::from_wire(&w.estado),
            raw_status: w.estado,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WMovement {
    #[serde(default, deserialize_with = "opt_text")]
    transaction_id: Option<String>,
    tipo_operacion: String,
    estado: String,
    #[serde(default)]
    mensaje: Option<String>,
    #[serde(default)]
    fecha_hora_transaccion: Option<String>,
    #[serde(deserialize_with = "amount::deserialize")]
    importe: Decimal,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    importe_comision: Option<Decimal>,
    #[serde(default, deserialize_with = "amount::deserialize_opt")]
    importe_total: Option<Decimal>,
    #[serde(default)]
    moneda: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    cuenta_origen: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    ci_cliente_origen: Option<String>,
    #[serde(default)]
    nombre_cliente_origen: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    codigo_banco_origen: Option<String>,
    #[serde(default)]
    nombre_banco_origen: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    cuenta_destino: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    ci_cliente_destino: Option<String>,
    #[serde(default)]
    nombre_cliente_destino: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    codigo_banco_destino: Option<String>,
    #[serde(default)]
    nombre_banco_destino: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    numero_referencia: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    num_orden_ach: Option<String>,
    #[serde(default, deserialize_with = "opt_text")]
    num_orden_destinatario: Option<String>,
}

impl From<WMovement> for Movement {
    fn from(w: WMovement) -> Self {
        Movement {
            transaction_id: non_empty(w.transaction_id),
            operation_type: w.tipo_operacion,
            status: Payout::map(&w.estado),
            raw_status: w.estado,
            message: non_empty(w.mensaje),
            transaction_at: non_empty(w.fecha_hora_transaccion),
            amount: w.importe,
            fee: w.importe_comision,
            total: w.importe_total,
            currency: non_empty(w.moneda),
            origin: Party {
                account: non_empty(w.cuenta_origen),
                document_id: non_empty(w.ci_cliente_origen),
                name: non_empty(w.nombre_cliente_origen),
                bank_code: non_empty(w.codigo_banco_origen),
                bank_name: non_empty(w.nombre_banco_origen),
            },
            destination: Party {
                account: non_empty(w.cuenta_destino),
                document_id: non_empty(w.ci_cliente_destino),
                name: non_empty(w.nombre_cliente_destino),
                bank_code: non_empty(w.codigo_banco_destino),
                bank_name: non_empty(w.nombre_banco_destino),
            },
            // The docs show "N/A" for payins without a reference.
            reference: non_empty(w.numero_referencia).filter(|r| r != "N/A"),
            ach_order_number: non_empty(w.num_orden_ach),
            recipient_order_number: non_empty(w.num_orden_destinatario),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WBalance {
    #[serde(deserialize_with = "text")]
    numero_cuenta: String,
    #[serde(default)]
    estado: Option<String>,
    /// `moneda` in reconciliation, `tipoMoneda` in the balances endpoint.
    #[serde(default, alias = "tipoMoneda")]
    moneda: Option<String>,
    #[serde(deserialize_with = "amount::deserialize")]
    saldo_disponible: Decimal,
    #[serde(deserialize_with = "amount::deserialize")]
    saldo_contable: Decimal,
    #[serde(deserialize_with = "amount::deserialize")]
    saldo_retenido: Decimal,
    #[serde(default)]
    fecha_ultimo_credito: Option<String>,
    #[serde(default)]
    fecha_ultimo_debito: Option<String>,
}

impl From<WBalance> for Balance {
    fn from(w: WBalance) -> Self {
        let raw_status = non_empty(w.estado);
        Balance {
            account_number: w.numero_cuenta,
            status: raw_status
                .as_deref()
                .map_or(AccountStatus::Unknown, AccountStatus::from_wire),
            raw_status,
            currency: non_empty(w.moneda),
            available: w.saldo_disponible,
            booked: w.saldo_contable,
            held: w.saldo_retenido,
            last_credit_at: non_empty(w.fecha_ultimo_credito),
            last_debit_at: non_empty(w.fecha_ultimo_debito),
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct WReconciliation {
    #[serde(default)]
    movimientos: Vec<WMovement>,
    #[serde(default)]
    saldos: Vec<WBalance>,
}

impl From<WReconciliation> for Reconciliation {
    fn from(w: WReconciliation) -> Self {
        Reconciliation {
            movements: w.movimientos.into_iter().map(Into::into).collect(),
            balances: w.saldos.into_iter().map(Into::into).collect(),
        }
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
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

fn opt_u64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    opt_text(d)?
        .map(|s| s.parse().map_err(serde::de::Error::custom))
        .transpose()
}

fn u64_any<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    opt_u64(d)?.ok_or_else(|| serde::de::Error::custom("expected an id"))
}
