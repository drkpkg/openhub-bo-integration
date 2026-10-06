use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use openhub_core::PaymentStatus;
use openhub_core::amount;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    /// `ACTIVA`
    Active,
    /// `BLOQUEADA`
    Blocked,
    /// `SUSPENDIDA`
    Suspended,
    /// `CERRADA`: final; requires zero balance.
    Closed,
    /// Not documented; see `raw_status`.
    Unknown,
}

impl AccountStatus {
    pub(crate) fn from_wire(raw: &str) -> Self {
        match raw.trim().to_ascii_uppercase().as_str() {
            "ACTIVA" | "ACTIVO" => AccountStatus::Active,
            "BLOQUEADA" => AccountStatus::Blocked,
            "SUSPENDIDA" => AccountStatus::Suspended,
            "CERRADA" => AccountStatus::Closed,
            _ => AccountStatus::Unknown,
        }
    }

    pub(crate) fn to_wire(self) -> Option<&'static str> {
        match self {
            AccountStatus::Active => Some("ACTIVA"),
            AccountStatus::Blocked => Some("BLOQUEADA"),
            AccountStatus::Suspended => Some("SUSPENDIDA"),
            AccountStatus::Closed => Some("CERRADA"),
            AccountStatus::Unknown => None,
        }
    }
}

// -- inputs ------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerchantRef {
    pub nit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRef {
    pub nit: String,
    pub account_number: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewAccount {
    /// What the account is used for, max 45 characters.
    pub alias: String,
    /// `rubro`, max 45 characters.
    pub category: String,
}

/// Input of [`crate::Create`]: up to 1000 accounts per call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateAccounts {
    pub nit: String,
    pub establishment_id: u64,
    pub accounts: Vec<NewAccount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusChange {
    pub account_number: String,
    pub status: AccountStatus,
    /// `descripcionMotivo`, max 50 characters.
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusChanges {
    pub nit: String,
    pub changes: Vec<StatusChange>,
}

/// Merchant-wide date range (`yyyy-mm-dd`, inclusive). Reconciliation allows
/// at most 7 days (sandbox; the docs say 31).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerchantRange {
    pub nit: String,
    pub date_from: String,
    pub date_to: String,
}

/// One account's date range; credits/debits allow at most 31 days.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRange {
    pub nit: String,
    pub account_number: String,
    pub date_from: String,
    pub date_to: String,
}

/// Up to 10 accounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalancesQuery {
    pub nit: String,
    pub account_numbers: Vec<String>,
}

// -- outputs ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Account {
    pub number: String,
    pub alias: Option<String>,
    pub status: AccountStatus,
    pub raw_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MerchantAccount {
    pub nit: String,
    pub merchant_name: Option<String>,
    pub establishment_id: Option<u64>,
    pub establishment_name: Option<String>,
    pub account: Account,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Establishment {
    pub id: u64,
    pub name: Option<String>,
    pub accounts: Vec<Account>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MerchantAccounts {
    pub nit: String,
    pub merchant_name: Option<String>,
    pub establishments: Vec<Establishment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreatedAccounts {
    pub nit: String,
    pub merchant_name: Option<String>,
    pub establishment_id: u64,
    pub establishment_name: Option<String>,
    /// New accounts start as active.
    pub accounts: Vec<Account>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusChanged {
    pub account_number: String,
    pub status: AccountStatus,
    pub raw_status: String,
}

/// One side of a movement.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Party {
    pub account: Option<String>,
    pub document_id: Option<String>,
    pub name: Option<String>,
    pub bank_code: Option<String>,
    pub bank_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Movement {
    /// Payouts only: the id you sent when authorising it.
    pub transaction_id: Option<String>,
    /// `PAYIN QR`, `PAYIN ACH`, `PAYOUT ACH`, ...
    pub operation_type: String,
    pub status: PaymentStatus,
    pub raw_status: String,
    pub message: Option<String>,
    /// ISO 8601 without offset, Bolivia time.
    pub transaction_at: Option<String>,
    #[serde(serialize_with = "amount::serialize")]
    pub amount: Decimal,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub fee: Option<Decimal>,
    #[serde(serialize_with = "amount::serialize_opt")]
    pub total: Option<Decimal>,
    pub currency: Option<String>,
    pub origin: Party,
    pub destination: Party,
    pub reference: Option<String>,
    pub ach_order_number: Option<String>,
    pub recipient_order_number: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Balance {
    pub account_number: String,
    pub status: AccountStatus,
    pub raw_status: Option<String>,
    pub currency: Option<String>,
    #[serde(serialize_with = "amount::serialize")]
    pub available: Decimal,
    /// `saldoContable`: total booked balance.
    #[serde(serialize_with = "amount::serialize")]
    pub booked: Decimal,
    /// `saldoRetenido`.
    #[serde(serialize_with = "amount::serialize")]
    pub held: Decimal,
    pub last_credit_at: Option<String>,
    pub last_debit_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reconciliation {
    pub movements: Vec<Movement>,
    pub balances: Vec<Balance>,
}
