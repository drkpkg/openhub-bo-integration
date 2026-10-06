//! ACH transfer batches (`/payout/async/v3`). Every call carries the merchant
//! code in the `branchCode` header (the sandbox answers 400 without it).

use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::model::{AuthorizeBatch, Bank, BanksQuery, BatchAuthorization, BatchQuery, BatchStatus};
use crate::validate;
use crate::wire;
use openhub_bo_core::amount;
use openhub_bo_core::envelope::{CodeData, Envelope};
use openhub_bo_core::{Ctx, Error, HttpRequest, HttpResponse, Method, Operation, Result};

const BASE: &str = "payout/async/v3";

/// `POST /payout/async/v3/lote/autorizar`. Moves money once processed.
pub struct AuthorizeBatchOp;
/// `GET /payout/async/v3/lote/estado/{processId}?nroLote=|transaccionId=`.
pub struct BatchStatusOp;
/// `POST /payout/async/v3/bancos`: destination banks.
pub struct BanksOp;

impl Operation for AuthorizeBatchOp {
    const NAME: &'static str = "batch.authorize";
    const IDEMPOTENT: bool = false;
    type Input = AuthorizeBatch;
    type Output = BatchAuthorization;

    fn request(ctx: &Ctx<'_>, input: &AuthorizeBatch) -> Result<HttpRequest> {
        validate::identifier("branch_code", &input.branch_code, 1, 20)?;
        validate::uuid("process_id", &input.process_id)?;
        let webhook_url = validate::webhook(&input.webhook_url, &input.webhook_token)?;
        if input.transfers.is_empty() {
            return Err(Error::validation("transfers", "at least one transfer"));
        }
        let mut seen = BTreeSet::new();
        let transfers = input
            .transfers
            .iter()
            .map(|t| {
                validate::identifier("transfers.transaction_id", &t.transaction_id, 3, 14)?;
                if !seen.insert(t.transaction_id.as_str()) {
                    return Err(Error::validation("transfers.transaction_id", "must be unique in the batch"));
                }
                validate::date("transfers.date", &t.date)?;
                validate::digits("transfers.source_account", &t.source_account, 6, 30)?;
                validate::digits("transfers.destination_account", &t.destination_account, 6, 30)?;
                validate::digits("transfers.bank_code", &t.bank_code, 3, 8)?;
                let city = t.branch_city.trim().to_ascii_uppercase();
                if !validate::CITIES.contains(&city.as_str()) {
                    return Err(Error::validation(
                        "transfers.branch_city",
                        format!("one of {}", validate::CITIES.join(", ")),
                    ));
                }
                validate::text("transfers.description", &t.description, 3, 80)?;
                validate::identifier("transfers.recipient_document_id", &t.recipient_document_id, 5, 20)?;
                validate::text("transfers.recipient_name", &t.recipient_name, 3, 80)?;
                if !matches!(t.currency.as_str(), "BOB" | "USD") {
                    return Err(Error::validation("transfers.currency", "BOB or USD"));
                }
                Ok(json!({
                    "transaccionId": t.transaction_id,
                    "importe": amount::to_json_number(amount::validate("transfers.amount", t.amount)?)?,
                    "fechaTransaccion": t.date,
                    "cuentaOrigen": t.source_account,
                    "cuentaDestino": t.destination_account,
                    "codeBanco": t.bank_code,
                    "codeSucursal": city,
                    "glosa": t.description.trim(),
                    "ciNitDestino": t.recipient_document_id,
                    "titularDestino": t.recipient_name.trim(),
                    "tipoMoneda": t.currency,
                }))
            })
            .collect::<Result<Vec<_>>>()?;
        request(
            ctx,
            Method::Post,
            "lote/autorizar",
            &input.branch_code,
            Some(json!({
                "processId": input.process_id,
                "webhookUrl": webhook_url,
                "transacciones": transfers,
            })),
        )
    }

    fn response(response: &HttpResponse, _: &AuthorizeBatch) -> Result<BatchAuthorization> {
        CodeData::open::<wire::Authorized>(response).map(Into::into)
    }
}

impl Operation for BatchStatusOp {
    const NAME: &'static str = "batch.status";
    const IDEMPOTENT: bool = true;
    type Input = BatchQuery;
    type Output = BatchStatus;

    fn request(ctx: &Ctx<'_>, input: &BatchQuery) -> Result<HttpRequest> {
        validate::identifier("branch_code", &input.branch_code, 1, 20)?;
        validate::uuid("process_id", &input.process_id)?;
        let query = match (&input.batch_number, &input.transaction_id) {
            (Some(batch), None) => {
                validate::identifier("batch_number", batch, 1, 20)?;
                format!("nroLote={batch}")
            }
            (None, Some(tx)) => {
                validate::identifier("transaction_id", tx, 3, 36)?;
                format!("transaccionId={tx}")
            }
            _ => {
                return Err(Error::validation(
                    "batch_number",
                    "give exactly one of batch_number or transaction_id",
                ));
            }
        };
        request(
            ctx,
            Method::Get,
            &format!("lote/estado/{}?{query}", input.process_id),
            &input.branch_code,
            None,
        )
    }

    fn response(response: &HttpResponse, _: &BatchQuery) -> Result<BatchStatus> {
        CodeData::open::<wire::WBatchStatus>(response).map(Into::into)
    }
}

impl Operation for BanksOp {
    const NAME: &'static str = "batch.banks";
    const IDEMPOTENT: bool = true;
    type Input = BanksQuery;
    type Output = Vec<Bank>;

    fn request(ctx: &Ctx<'_>, input: &BanksQuery) -> Result<HttpRequest> {
        validate::identifier("branch_code", &input.branch_code, 1, 20)?;
        request(ctx, Method::Post, "bancos", &input.branch_code, None)
    }

    fn response(response: &HttpResponse, _: &BanksQuery) -> Result<Vec<Bank>> {
        let rows: Vec<wire::WBank> = CodeData::open(response)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

fn request(
    ctx: &Ctx<'_>,
    method: Method,
    path: &str,
    branch_code: &str,
    body: Option<Value>,
) -> Result<HttpRequest> {
    Ok(HttpRequest {
        method,
        url: ctx.config.url(&format!("{BASE}/{path}")),
        headers: ctx.api_headers_with(&[("branchCode", branch_code)])?,
        body: body.map(|b| b.to_string()),
    })
}
