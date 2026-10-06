//! Merchant accounts at ATC (`ATC.API.CUENTA.DIGITAL`, `/cuentas-comercios/v1`):
//! account lookup and creation, status changes, balances, and the movements
//! needed to reconcile QR collections and payouts.
//!
//! Verified against the sandbox (2026-10-06): validation rules and error codes
//! (with a made-up NIT). Success payloads follow the docs only.

pub mod model;
pub mod ops;
pub mod validate;
mod wire;

pub use model::{
    Account, AccountRange, AccountRef, AccountStatus, Balance, BalancesQuery, CreateAccounts,
    CreatedAccounts, Establishment, MerchantAccount, MerchantAccounts, MerchantRange, MerchantRef,
    Movement, NewAccount, Party, Reconciliation, StatusChange, StatusChanged, StatusChanges,
};
pub use ops::{
    Balances, ChangeStatus, Create, Credits, Debits, GetAccount, ListAccounts, Reconcile,
};

openhub_core::registry!(pub AccountsOps {
    operations: [
        GetAccount,
        ListAccounts,
        Create,
        ChangeStatus,
        Reconcile,
        Credits,
        Debits,
        Balances,
    ],
    handlers: [],
});
