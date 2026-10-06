use std::str::FromStr;

use pretty_assertions::assert_eq;
use rust_decimal::Decimal;
use serde_json::{Value, json};

use openhub_accounts::*;
use openhub_core::ffi::{JsonValue, call};
use openhub_core::{
    AccessToken, Chain, Config, CoreOps, Ctx, Error, HttpRequest, HttpResponse, Method, Operation,
    PaymentStatus,
};

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("../../../fixtures/openhub/accounts/", $name))
    };
}

fn build<O: Operation>(input: &O::Input) -> openhub_core::Result<HttpRequest> {
    let config = Config::new("cid", "s");
    let token = AccessToken {
        access_token: "tok".into(),
        token_type: "access_token".into(),
        expires_at: i64::MAX,
        scope: None,
    };
    O::request(&Ctx::new(&config, Some(&token)), input)
}

fn body(request: &HttpRequest) -> Value {
    serde_json::from_str(request.body.as_deref().unwrap()).unwrap()
}

fn ok(body: &str) -> HttpResponse {
    HttpResponse::new(200, body)
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn range(from: &str, to: &str) -> MerchantRange {
    MerchantRange {
        nit: "1000000019".into(),
        date_from: from.into(),
        date_to: to.into(),
    }
}

fn account_range(from: &str, to: &str) -> AccountRange {
    AccountRange {
        nit: "1000000019".into(),
        account_number: "7011234561".into(),
        date_from: from.into(),
        date_to: to.into(),
    }
}

fn validation_field(result: openhub_core::Result<HttpRequest>) -> String {
    match result {
        Err(Error::Validation { field, .. }) => field,
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn lookup_requests() {
    let get = build::<GetAccount>(&AccountRef {
        nit: "1000000019".into(),
        account_number: "7011113693".into(),
    })
    .unwrap();
    assert_eq!(get.method, Method::Get);
    assert!(
        get.url
            .ends_with("/cuentas-comercios/v1/cuentas/1000000019/7011113693")
    );
    assert_eq!(get.headers["access_token"], "tok");

    let list = build::<ListAccounts>(&MerchantRef {
        nit: "1000000019".into(),
    })
    .unwrap();
    assert!(
        list.url
            .ends_with("/cuentas-comercios/v1/cuentas/1000000019")
    );
}

#[test]
fn create_and_status_change_bodies() {
    let create = build::<Create>(&CreateAccounts {
        nit: "1000000019".into(),
        establishment_id: 111369,
        accounts: vec![NewAccount {
            alias: "CAJA 5".into(),
            category: "COMERCIALES".into(),
        }],
    })
    .unwrap();
    assert_eq!(create.method, Method::Post);
    assert_eq!(
        body(&create),
        json!({"nit": "1000000019", "establecimiento": {"idEstablecimiento": 111369,
            "cuenta": [{"alias": "CAJA 5", "rubro": "COMERCIALES", "moneda": "068"}]}})
    );

    let change = build::<ChangeStatus>(&StatusChanges {
        nit: "1000000019".into(),
        changes: vec![StatusChange {
            account_number: "7011113693".into(),
            status: AccountStatus::Blocked,
            reason: "Sospecha de fraude".into(),
        }],
    })
    .unwrap();
    assert_eq!(change.method, Method::Patch);
    assert!(change.url.ends_with("/cuentas/estados"));
    assert_eq!(
        body(&change),
        json!({"nit": "1000000019", "cuentas": [{"numeroCuenta": "7011113693",
            "descripcionMotivo": "Sospecha de fraude", "estado": "BLOQUEADA"}]})
    );
}

#[test]
fn movement_and_balance_bodies() {
    let reconcile = build::<Reconcile>(&range("2026-09-29", "2026-10-06")).unwrap();
    assert!(reconcile.url.ends_with("/cuentas/transacciones"));
    assert_eq!(
        body(&reconcile),
        json!({"nit": "1000000019", "fechaInicio": "2026-09-29", "fechaFin": "2026-10-06"})
    );

    let credits = build::<Credits>(&account_range("2026-09-05", "2026-10-06")).unwrap();
    assert_eq!(
        (
            body(&credits)["tipo"].clone(),
            body(&credits)["numeroCuenta"].clone()
        ),
        (json!("C"), json!(["7011234561"]))
    );
    let debits = build::<Debits>(&account_range("2026-09-05", "2026-10-06")).unwrap();
    assert!(debits.url.ends_with("/cuentas/debitos"));
    assert_eq!(body(&debits)["tipo"], "D");

    let balances = build::<Balances>(&BalancesQuery {
        nit: "1000000019".into(),
        account_numbers: vec!["7014227171".into()],
    })
    .unwrap();
    assert_eq!(
        body(&balances),
        json!({"nit": "1000000019", "numeroCuentas": ["7014227171"]})
    );
}

#[test]
fn validation_mirrors_sandbox_limits() {
    assert_eq!(
        validation_field(build::<Reconcile>(&range("2026-09-28", "2026-10-06"))),
        "date_to"
    );
    assert_eq!(
        validation_field(build::<Credits>(&account_range("2026-08-01", "2026-10-06"))),
        "date_to"
    );
    assert_eq!(
        validation_field(build::<Debits>(&account_range("2026-10-06", "2026-10-01"))),
        "date_to"
    );
    assert_eq!(
        validation_field(build::<Reconcile>(&range("06/10/2026", "2026-10-06"))),
        "date_from"
    );
    let eleven = (0..11).map(|i| format!("70142271{i:02}")).collect();
    assert_eq!(
        validation_field(build::<Balances>(&BalancesQuery {
            nit: "1000000019".into(),
            account_numbers: eleven
        })),
        "account_numbers"
    );
    assert_eq!(
        validation_field(build::<ListAccounts>(&MerchantRef {
            nit: "12-34".into()
        })),
        "nit"
    );
    assert_eq!(
        validation_field(build::<ChangeStatus>(&StatusChanges {
            nit: "1000000019".into(),
            changes: vec![StatusChange {
                account_number: "7011113693".into(),
                status: AccountStatus::Closed,
                reason: "x".repeat(51),
            }],
        })),
        "changes.reason"
    );
    assert_eq!(
        validation_field(build::<Create>(&CreateAccounts {
            nit: "1000000019".into(),
            establishment_id: 1,
            accounts: vec![]
        })),
        "accounts"
    );
}

#[test]
fn parses_documented_accounts() {
    let input = AccountRef {
        nit: "1000000019".into(),
        account_number: "7011113693".into(),
    };
    let detail =
        GetAccount::response(&ok(fixture!("account_detail_response.json")), &input).unwrap();
    assert_eq!(detail.establishment_id, Some(111369));
    assert_eq!(detail.account.status, AccountStatus::Blocked);

    let list = ListAccounts::response(
        &ok(fixture!("accounts_list_response.json")),
        &MerchantRef { nit: "1".into() },
    )
    .unwrap();
    assert_eq!(list.establishments[0].accounts.len(), 2);
    assert_eq!(
        list.establishments[0].accounts[0].status,
        AccountStatus::Active
    );

    let created = Create::response(
        &ok(fixture!("accounts_create_response.json")),
        &CreateAccounts {
            nit: "1".into(),
            establishment_id: 1,
            accounts: vec![],
        },
    )
    .unwrap();
    assert_eq!(created.accounts[0].number, "7011113695");
    assert_eq!(created.accounts[0].status, AccountStatus::Active); // no `estado` → active

    let changed = ChangeStatus::response(
        &ok(fixture!("accounts_status_response.json")),
        &StatusChanges {
            nit: "1".into(),
            changes: vec![],
        },
    )
    .unwrap();
    assert_eq!(changed[0].status, AccountStatus::Blocked);
}

#[test]
fn parses_documented_movements_and_balances() {
    let rec = Reconcile::response(
        &ok(fixture!("reconciliation_response.json")),
        &range("2026-05-01", "2026-05-07"),
    )
    .unwrap();
    assert_eq!(rec.movements.len(), 2);
    let payout = &rec.movements[0];
    assert_eq!(
        (payout.operation_type.as_str(), payout.status),
        ("PAYOUT ACH", PaymentStatus::Paid)
    );
    assert_eq!(
        payout.destination.bank_name.as_deref(),
        Some("BANCO GANADERO")
    );
    assert_eq!(rec.movements[1].transaction_id, None); // "" for payins
    assert_eq!(rec.balances[0].available, dec("56.86"));

    let credits = Credits::response(
        &ok(fixture!("credits_response.json")),
        &account_range("2026-05-01", "2026-05-31"),
    )
    .unwrap();
    assert_eq!(credits[0].operation_type, "PAYIN QR");
    assert_eq!(credits[0].reference, None); // "N/A"
    assert_eq!(credits[0].amount, dec("868.09"));

    let debits = Debits::response(
        &ok(fixture!("debits_response.json")),
        &account_range("2026-05-01", "2026-05-31"),
    )
    .unwrap();
    assert_eq!(debits[0].total, Some(dec("299.70")));

    let balances = Balances::response(
        &ok(fixture!("balances_response.json")),
        &BalancesQuery {
            nit: "1".into(),
            account_numbers: vec![],
        },
    )
    .unwrap();
    assert_eq!(
        (
            balances[1].currency.as_deref(),
            balances[1].last_debit_at.as_deref()
        ),
        (Some("BOB"), None)
    );
    assert_eq!(balances[0].status, AccountStatus::Active);
}

#[test]
fn maps_sandbox_errors() {
    let code = |result: openhub_core::Result<Value>| match result {
        Err(Error::Api {
            status,
            errors,
            retryable,
            ..
        }) => (
            status,
            errors
                .iter()
                .map(|e| e.code.clone().unwrap())
                .collect::<Vec<_>>(),
            retryable,
        ),
        other => panic!("expected Api error, got {other:?}"),
    };
    let input = AccountRef {
        nit: "1234567".into(),
        account_number: "7011234561".into(),
    };
    let as_value =
        |r: openhub_core::Result<MerchantAccount>| r.map(|v| serde_json::to_value(v).unwrap());
    assert_eq!(
        code(as_value(GetAccount::response(
            &ok(fixture!("sandbox_account_not_found.json")),
            &input
        ))),
        (200, vec!["15".into()], false)
    );

    let list = ListAccounts::response(
        &ok(fixture!("sandbox_merchant_not_found.json")),
        &MerchantRef { nit: "1".into() },
    );
    assert!(
        matches!(list, Err(Error::Api { ref errors, .. }) if errors[0].code.as_deref() == Some("17"))
    );

    let rec = Reconcile::response(
        &ok(fixture!("sandbox_reconciliation_range.json")),
        &range("2026-08-01", "2026-10-06"),
    );
    assert!(
        matches!(rec, Err(Error::Api { ref errors, .. }) if errors[0].message == "El rango no puede ser mayor a 7 días.")
    );

    let credits = Credits::response(
        &ok(fixture!("sandbox_no_enabled_accounts.json")),
        &account_range("2026-10-01", "2026-10-06"),
    );
    assert!(
        matches!(credits, Err(Error::Api { retryable: false, ref errors, .. }) if errors[0].code.as_deref() == Some("99"))
    );

    let balances = Balances::response(
        &ok(fixture!("sandbox_balances_validation.json")),
        &BalancesQuery {
            nit: "1".into(),
            account_numbers: vec![],
        },
    );
    assert!(matches!(balances, Err(Error::Api { ref errors, .. }) if errors.len() == 2));
}

#[test]
fn registry_describes_account_operations() {
    let ops = Chain(CoreOps, AccountsOps);
    let described: JsonValue = serde_json::from_str(&call(&ops, "describe", "{}")).unwrap();
    let entries = described["value"].as_array().unwrap();
    let names: Vec<_> = entries
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "token",
            "accounts.get",
            "accounts.list",
            "accounts.create",
            "accounts.set_status",
            "accounts.reconcile",
            "accounts.credits",
            "accounts.debits",
            "accounts.balances",
        ]
    );
    assert_eq!(entries[3]["idempotent"], false);
}
