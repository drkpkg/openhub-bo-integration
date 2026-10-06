use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::model::{
    AccountRange, AccountRef, Balance, BalancesQuery, CreateAccounts, CreatedAccounts,
    MerchantAccount, MerchantAccounts, MerchantRange, MerchantRef, Movement, Reconciliation,
    StatusChanged, StatusChanges,
};
use crate::validate;
use crate::wire;
use openhub_bo_core::envelope::{CodeData, Envelope};
use openhub_bo_core::{Ctx, Error, HttpRequest, HttpResponse, Method, Operation, Result};

const BASE: &str = "cuentas-comercios/v1/cuentas";
/// ISO 4217 numeric code for BOB, the only currency accepted for new accounts.
const BOB_NUMERIC: &str = "068";

/// `GET /cuentas-comercios/v1/cuentas/{nit}/{numeroCuenta}`
pub struct GetAccount;
/// `GET /cuentas-comercios/v1/cuentas/{nit}`
pub struct ListAccounts;
/// `POST /cuentas-comercios/v1/cuentas` (up to 1000 accounts).
pub struct Create;
/// `PATCH /cuentas-comercios/v1/cuentas/estados`. Closing requires zero
/// balance and is final.
pub struct ChangeStatus;
/// `POST /cuentas-comercios/v1/cuentas/transacciones`: movements and balances
/// of every account of the merchant, at most 7 days.
pub struct Reconcile;
/// `POST /cuentas-comercios/v1/cuentas/creditos`, at most 31 days.
pub struct Credits;
/// `POST /cuentas-comercios/v1/cuentas/debitos`, at most 31 days.
pub struct Debits;
/// `POST /cuentas-comercios/v1/cuentas/saldos`, up to 10 accounts.
pub struct Balances;

impl Operation for GetAccount {
    const NAME: &'static str = "accounts.get";
    const IDEMPOTENT: bool = true;
    type Input = AccountRef;
    type Output = MerchantAccount;

    fn request(ctx: &Ctx<'_>, input: &AccountRef) -> Result<HttpRequest> {
        validate::nit(&input.nit)?;
        validate::account_number("account_number", &input.account_number)?;
        get(
            ctx,
            &format!("{BASE}/{}/{}", input.nit, input.account_number),
        )
    }

    fn response(response: &HttpResponse, _: &AccountRef) -> Result<MerchantAccount> {
        open::<wire::Detail, _>(response)
    }
}

impl Operation for ListAccounts {
    const NAME: &'static str = "accounts.list";
    const IDEMPOTENT: bool = true;
    type Input = MerchantRef;
    type Output = MerchantAccounts;

    fn request(ctx: &Ctx<'_>, input: &MerchantRef) -> Result<HttpRequest> {
        validate::nit(&input.nit)?;
        get(ctx, &format!("{BASE}/{}", input.nit))
    }

    fn response(response: &HttpResponse, _: &MerchantRef) -> Result<MerchantAccounts> {
        open::<wire::List, _>(response)
    }
}

impl Operation for Create {
    const NAME: &'static str = "accounts.create";
    const IDEMPOTENT: bool = false;
    type Input = CreateAccounts;
    type Output = CreatedAccounts;

    fn request(ctx: &Ctx<'_>, input: &CreateAccounts) -> Result<HttpRequest> {
        validate::nit(&input.nit)?;
        if input.establishment_id == 0 {
            return Err(Error::validation(
                "establishment_id",
                "must be greater than zero",
            ));
        }
        if !(1..=1000).contains(&input.accounts.len()) {
            return Err(Error::validation(
                "accounts",
                "between 1 and 1000 accounts per call",
            ));
        }
        let accounts = input
            .accounts
            .iter()
            .map(|a| {
                validate::text_max("accounts.alias", &a.alias, 45)?;
                validate::text_max("accounts.category", &a.category, 45)?;
                Ok(json!({"alias": a.alias.trim(), "rubro": a.category.trim(), "moneda": BOB_NUMERIC}))
            })
            .collect::<Result<Vec<_>>>()?;
        send(
            ctx,
            Method::Post,
            BASE,
            json!({
                "nit": input.nit,
                "establecimiento": {"idEstablecimiento": input.establishment_id, "cuenta": accounts},
            }),
        )
    }

    fn response(response: &HttpResponse, _: &CreateAccounts) -> Result<CreatedAccounts> {
        open::<wire::Created, _>(response)
    }
}

impl Operation for ChangeStatus {
    const NAME: &'static str = "accounts.set_status";
    /// Setting the same target status twice has the same effect.
    const IDEMPOTENT: bool = true;
    type Input = StatusChanges;
    type Output = Vec<StatusChanged>;

    fn request(ctx: &Ctx<'_>, input: &StatusChanges) -> Result<HttpRequest> {
        validate::nit(&input.nit)?;
        if input.changes.is_empty() {
            return Err(Error::validation("changes", "at least one change"));
        }
        let changes = input
            .changes
            .iter()
            .map(|c| {
                validate::account_number("changes.account_number", &c.account_number)?;
                validate::text_max("changes.reason", &c.reason, 50)?;
                let status = c
                    .status
                    .to_wire()
                    .ok_or_else(|| Error::validation("changes.status", "cannot set `unknown`"))?;
                Ok(json!({
                    "numeroCuenta": c.account_number,
                    "descripcionMotivo": c.reason.trim(),
                    "estado": status,
                }))
            })
            .collect::<Result<Vec<_>>>()?;
        send(
            ctx,
            Method::Patch,
            &format!("{BASE}/estados"),
            json!({"nit": input.nit, "cuentas": changes}),
        )
    }

    fn response(response: &HttpResponse, _: &StatusChanges) -> Result<Vec<StatusChanged>> {
        let rows: Vec<wire::Changed> = CodeData::open(response)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

impl Operation for Reconcile {
    const NAME: &'static str = "accounts.reconcile";
    const IDEMPOTENT: bool = true;
    type Input = MerchantRange;
    type Output = Reconciliation;

    fn request(ctx: &Ctx<'_>, input: &MerchantRange) -> Result<HttpRequest> {
        validate::nit(&input.nit)?;
        validate::date_range(&input.date_from, &input.date_to, 7)?;
        send(
            ctx,
            Method::Post,
            &format!("{BASE}/transacciones"),
            json!({"nit": input.nit, "fechaInicio": input.date_from, "fechaFin": input.date_to}),
        )
    }

    fn response(response: &HttpResponse, _: &MerchantRange) -> Result<Reconciliation> {
        open::<wire::WReconciliation, _>(response)
    }
}

fn movements_request(
    ctx: &Ctx<'_>,
    input: &AccountRange,
    path: &str,
    kind: &str,
) -> Result<HttpRequest> {
    validate::nit(&input.nit)?;
    validate::account_number("account_number", &input.account_number)?;
    validate::date_range(&input.date_from, &input.date_to, 31)?;
    send(
        ctx,
        Method::Post,
        &format!("{BASE}/{path}"),
        json!({
            "nit": input.nit,
            "numeroCuenta": [input.account_number],
            "fechaInicio": input.date_from,
            "fechaFin": input.date_to,
            "tipo": kind,
        }),
    )
}

fn movements_response(response: &HttpResponse) -> Result<Vec<Movement>> {
    let rows: Vec<wire::WMovement> = CodeData::open(response)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

impl Operation for Credits {
    const NAME: &'static str = "accounts.credits";
    const IDEMPOTENT: bool = true;
    type Input = AccountRange;
    type Output = Vec<Movement>;

    fn request(ctx: &Ctx<'_>, input: &AccountRange) -> Result<HttpRequest> {
        movements_request(ctx, input, "creditos", "C")
    }

    fn response(response: &HttpResponse, _: &AccountRange) -> Result<Vec<Movement>> {
        movements_response(response)
    }
}

impl Operation for Debits {
    const NAME: &'static str = "accounts.debits";
    const IDEMPOTENT: bool = true;
    type Input = AccountRange;
    type Output = Vec<Movement>;

    fn request(ctx: &Ctx<'_>, input: &AccountRange) -> Result<HttpRequest> {
        movements_request(ctx, input, "debitos", "D")
    }

    fn response(response: &HttpResponse, _: &AccountRange) -> Result<Vec<Movement>> {
        movements_response(response)
    }
}

impl Operation for Balances {
    const NAME: &'static str = "accounts.balances";
    const IDEMPOTENT: bool = true;
    type Input = BalancesQuery;
    type Output = Vec<Balance>;

    fn request(ctx: &Ctx<'_>, input: &BalancesQuery) -> Result<HttpRequest> {
        validate::nit(&input.nit)?;
        if !(1..=10).contains(&input.account_numbers.len()) {
            return Err(Error::validation(
                "account_numbers",
                "between 1 and 10 accounts",
            ));
        }
        for number in &input.account_numbers {
            validate::account_number("account_numbers", number)?;
        }
        send(
            ctx,
            Method::Post,
            &format!("{BASE}/saldos"),
            json!({"nit": input.nit, "numeroCuentas": input.account_numbers}),
        )
    }

    fn response(response: &HttpResponse, _: &BalancesQuery) -> Result<Vec<Balance>> {
        let rows: Vec<wire::WBalance> = CodeData::open(response)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

fn open<W: DeserializeOwned, T: From<W>>(response: &HttpResponse) -> Result<T> {
    CodeData::open::<W>(response).map(Into::into)
}

fn get(ctx: &Ctx<'_>, path: &str) -> Result<HttpRequest> {
    Ok(HttpRequest {
        method: Method::Get,
        url: ctx.config.url(path),
        headers: ctx.api_headers()?,
        body: None,
    })
}

fn send(ctx: &Ctx<'_>, method: Method, path: &str, body: Value) -> Result<HttpRequest> {
    Ok(HttpRequest {
        method,
        url: ctx.config.url(path),
        headers: ctx.api_headers()?,
        body: Some(body.to_string()),
    })
}
