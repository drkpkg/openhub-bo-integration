use std::str::FromStr;

use pretty_assertions::assert_eq;
use rust_decimal::Decimal;
use serde_json::{Value, json};

use openhub_bo_core::ffi::{JsonValue, call};
use openhub_bo_core::{
    AccessToken, Chain, Config, CoreOps, Ctx, Error, Handler, HttpRequest, HttpResponse, Method,
    Operation, PaymentStatus,
};
use openhub_bo_payouts::webhook::BatchWebhookCall;
use openhub_bo_payouts::*;

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("../../../fixtures/openhub/payouts/", $name))
    };
}

fn build<O: Operation>(input: &O::Input) -> openhub_bo_core::Result<HttpRequest> {
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

fn raw_body(request: &HttpRequest) -> &str {
    request.body.as_deref().unwrap()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn ok(body: &str) -> HttpResponse {
    HttpResponse::new(200, body)
}

fn scanned(amount: &str, description: Option<&str>) -> ScannedQr {
    ScannedQr {
        reference: "547260814000002110".into(),
        amount: dec(amount),
        currency: Some("BOB".into()),
        description: description.map(Into::into),
        recipient: Recipient::default(),
        expires_on: None,
    }
}

fn pay(scanned: ScannedQr, amount: Option<&str>, description: Option<&str>) -> PayQr {
    PayQr {
        scanned,
        source_account: "7010123451".into(),
        transaction_id: "REQ-TEST07".into(),
        amount: amount.map(dec),
        description: description.map(Into::into),
    }
}

fn field(result: openhub_bo_core::Result<HttpRequest>) -> String {
    match result {
        Err(Error::Validation { field, .. }) => field,
        other => panic!("expected validation error, got {other:?}"),
    }
}

fn transfer() -> BatchTransfer {
    BatchTransfer {
        transaction_id: "001002".into(),
        amount: dec("50.00"),
        date: "2026-10-06".into(),
        source_account: "484811311404044".into(),
        destination_account: "1311404044".into(),
        bank_code: "1018".into(),
        branch_city: "lpz".into(),
        description: "DETALLE".into(),
        recipient_document_id: "5452452".into(),
        recipient_name: "CLIENTE DESTINO".into(),
        currency: "BOB".into(),
    }
}

fn batch() -> AuthorizeBatch {
    AuthorizeBatch {
        branch_code: "455544".into(),
        process_id: "66ec5b3d-61ea-4254-b366-7104545aa3c6".into(),
        webhook_url: "https://shop.example/api/confirmed".into(),
        webhook_token: "tok_0123456789abcdef".into(),
        transfers: vec![transfer()],
    }
}

// -- QR payouts ---------------------------------------------------------------------

#[test]
fn scan_request_and_sandbox_response() {
    let request = build::<ScanOp>(&ScanQr {
        qr_text: " Wg74o8sx ".into(),
    })
    .unwrap();
    assert!(request.url.ends_with("/payout/sync/v3/qr/scan"));
    assert_eq!(body(&request), json!({"imagen": "Wg74o8sx"}));

    let qr = ScanOp::response(
        &ok(fixture!("sandbox_scan_response.json")),
        &ScanQr {
            qr_text: "x".into(),
        },
    )
    .unwrap();
    assert_eq!(qr.reference, "547261006000003099");
    assert_eq!(
        (qr.amount, qr.expires_on.as_deref()),
        (dec("1"), Some("2026-10-06"))
    );
    assert_eq!(
        qr.recipient.bank_name.as_deref(),
        Some("ADMINISTRADORA DE TARJETAS - ATC S.A.")
    );
}

#[test]
fn fixed_amount_qr_sends_zero_and_no_glosa() {
    let request = build::<PayOp>(&pay(scanned("100.50", Some("Factura 1")), None, None)).unwrap();
    assert!(request.url.ends_with("/payout/sync/v3/qr/confirm"));
    assert!(
        raw_body(&request).contains(r#""importe":0.00"#),
        "{}",
        raw_body(&request)
    );
    let b = body(&request);
    assert_eq!(b["numeroReferencia"], "547260814000002110");
    assert_eq!(b.get("glosa"), None);
}

#[test]
fn open_amount_qr_requires_amount_and_description_when_missing() {
    let request = build::<PayOp>(&pay(
        scanned("0", None),
        Some("100.50"),
        Some("Pago proveedor"),
    ))
    .unwrap();
    let b = body(&request);
    assert_eq!(
        (b["importe"].clone(), b["glosa"].clone()),
        (json!(100.5), json!("Pago proveedor"))
    );

    assert_eq!(
        field(build::<PayOp>(&pay(scanned("0", Some("x")), None, None))),
        "amount"
    );
    assert_eq!(
        field(build::<PayOp>(&pay(scanned("0", None), Some("1"), None))),
        "description"
    );
    assert_eq!(
        field(build::<PayOp>(&pay(
            scanned("10", Some("x")),
            Some("11"),
            None
        ))),
        "amount"
    );
    assert_eq!(
        field(build::<PayOp>(&PayQr {
            transaction_id: "has space".into(),
            ..pay(scanned("1", Some("x")), None, None)
        })),
        "transaction_id"
    );
}

#[test]
fn unconfirmed_payments_are_ambiguous() {
    let result = PayOp::response(
        &ok(fixture!("pay_unconfirmed_response.json")),
        &pay(scanned("1", Some("x")), None, None),
    );
    assert!(
        matches!(result, Err(Error::Ambiguous { ref errors, .. }) if errors[0].code.as_deref() == Some("96"))
    );
}

#[test]
fn parses_documented_payment_and_status() {
    let paid = PayOp::response(
        &ok(fixture!("pay_response.json")),
        &pay(scanned("1", Some("x")), None, None),
    )
    .unwrap();
    assert_eq!(
        (paid.status, paid.amount),
        (PaymentStatus::Paid, Some(dec("100.50")))
    );
    assert_eq!(
        paid.ach_order_number.as_deref(),
        Some("14262608140249823031")
    );

    let request = build::<PayoutStatusOp>(&PayoutRef {
        reference: "547250827000000004".into(),
    })
    .unwrap();
    assert_eq!(request.method, Method::Get); // POST gets 404 in the sandbox
    assert!(
        request
            .url
            .ends_with("/payout/sync/v3/qr/status/547250827000000004")
    );
    let status = PayoutStatusOp::response(
        &ok(fixture!("payment_status_response.json")),
        &PayoutRef {
            reference: "1".into(),
        },
    )
    .unwrap();
    assert_eq!(
        status.message.as_deref(),
        Some("La transacción fue aprobada.")
    );
}

#[test]
fn maps_sandbox_payout_errors() {
    let code = |r: openhub_bo_core::Result<Value>| match r {
        Err(Error::Api { status, errors, .. }) => (status, errors.len(), errors[0].code.clone()),
        other => panic!("expected Api error, got {other:?}"),
    };
    let v = |r: openhub_bo_core::Result<ScannedQr>| r.map(|x| serde_json::to_value(x).unwrap());
    assert_eq!(
        code(v(ScanOp::response(
            &ok(fixture!("sandbox_scan_invalid.json")),
            &ScanQr {
                qr_text: "x".into()
            }
        ))),
        (200, 1, Some("09".into()))
    );
    let pv = |r: openhub_bo_core::Result<Payout>| r.map(|x| serde_json::to_value(x).unwrap());
    assert_eq!(
        code(pv(PayOp::response(
            &HttpResponse::new(400, fixture!("sandbox_pay_validation.json")),
            &pay(scanned("1", Some("x")), None, None)
        ))),
        (400, 4, Some("02".into()))
    );
    assert_eq!(
        code(pv(PayoutStatusOp::response(
            &ok(fixture!("sandbox_payment_not_found.json")),
            &PayoutRef {
                reference: "1".into()
            }
        ))),
        (200, 1, Some("04".into()))
    );
}

// -- ACH batches ----------------------------------------------------------------------

#[test]
fn authorize_batch_request() {
    let request = build::<AuthorizeBatchOp>(&batch()).unwrap();
    assert!(request.url.ends_with("/payout/async/v3/lote/autorizar"));
    assert_eq!(request.headers["branchCode"], "455544");
    assert_eq!(
        body(&request),
        json!({
            "processId": "66ec5b3d-61ea-4254-b366-7104545aa3c6",
            "webhookUrl": "https://shop.example/api/confirmed?token=tok_0123456789abcdef",
            "transacciones": [{
                "transaccionId": "001002", "importe": 50, "fechaTransaccion": "2026-10-06",
                "cuentaOrigen": "484811311404044", "cuentaDestino": "1311404044", "codeBanco": "1018",
                "codeSucursal": "LPZ", "glosa": "DETALLE", "ciNitDestino": "5452452",
                "titularDestino": "CLIENTE DESTINO", "tipoMoneda": "BOB",
            }],
        })
    );
}

#[test]
fn batch_validation() {
    let with = |f: fn(&mut AuthorizeBatch)| {
        let mut b = batch();
        f(&mut b);
        field(build::<AuthorizeBatchOp>(&b))
    };
    assert_eq!(
        with(|b| b.process_id = "4554645646446".into()),
        "process_id"
    );
    assert_eq!(with(|b| b.webhook_token = "short".into()), "webhook_token");
    assert_eq!(with(|b| b.transfers.clear()), "transfers");
    assert_eq!(
        with(|b| b.transfers.push(transfer())),
        "transfers.transaction_id"
    );
    assert_eq!(
        with(|b| b.transfers[0].branch_city = "XYZ".into()),
        "transfers.branch_city"
    );
    assert_eq!(
        with(|b| b.transfers[0].date = "2026-02-30".into()),
        "transfers.date"
    );
    assert_eq!(
        with(|b| b.transfers[0].currency = "EUR".into()),
        "transfers.currency"
    );
}

#[test]
fn batch_status_and_banks_requests() {
    let query = |batch: Option<&str>, tx: Option<&str>| BatchQuery {
        branch_code: "455544".into(),
        process_id: "66ec5b3d-61ea-4254-b366-7104545aa3c6".into(),
        batch_number: batch.map(Into::into),
        transaction_id: tx.map(Into::into),
    };
    let request = build::<BatchStatusOp>(&query(Some("2601191045"), None)).unwrap();
    assert_eq!(request.method, Method::Get); // POST gets 404 in the sandbox
    assert!(
        request
            .url
            .ends_with("/lote/estado/66ec5b3d-61ea-4254-b366-7104545aa3c6?nroLote=2601191045")
    );
    assert!(
        build::<BatchStatusOp>(&query(None, Some("545455")))
            .unwrap()
            .url
            .ends_with("?transaccionId=545455")
    );
    assert_eq!(
        field(build::<BatchStatusOp>(&query(Some("1"), Some("2")))),
        "batch_number"
    );
    assert_eq!(
        field(build::<BatchStatusOp>(&query(None, None))),
        "batch_number"
    );

    let banks = build::<BanksOp>(&BanksQuery {
        branch_code: "455544".into(),
    })
    .unwrap();
    assert_eq!(
        (banks.method, banks.headers["branchCode"].as_str()),
        (Method::Post, "455544")
    );
}

#[test]
fn parses_batch_responses() {
    let auth = AuthorizeBatchOp::response(&ok(fixture!("batch_authorize_response.json")), &batch())
        .unwrap();
    assert_eq!(auth.batch_number, "2601191040");
    assert_eq!(auth.transfers[0].status, PaymentStatus::Pending);
    assert_eq!(
        (
            auth.transfers[1].status,
            auth.transfers[1].reference.as_deref()
        ),
        (PaymentStatus::Error, None)
    );

    let q = BatchQuery {
        branch_code: "1".into(),
        process_id: "x".into(),
        batch_number: None,
        transaction_id: None,
    };
    let status = BatchStatusOp::response(&ok(fixture!("batch_status_response.json")), &q).unwrap();
    assert_eq!(status.batch_number.as_deref(), Some("L20260518000123")); // flat response
    assert_eq!(
        (status.transfers[0].status, status.transfers[0].amount),
        (PaymentStatus::Paid, Some(dec("85.00")))
    );

    let banks = BanksOp::response(
        &ok(fixture!("sandbox_banks_response.json")),
        &BanksQuery {
            branch_code: "1".into(),
        },
    )
    .unwrap();
    assert_eq!(
        (banks[0].code.as_str(), banks[0].name.as_deref()),
        ("1005", None)
    );

    let err = |r: openhub_bo_core::Result<Value>| match r {
        Err(Error::Api { status, errors, .. }) => (status, errors.len()),
        other => panic!("{other:?}"),
    };
    let av = |r: openhub_bo_core::Result<BatchAuthorization>| {
        r.map(|x| serde_json::to_value(x).unwrap())
    };
    assert_eq!(
        err(av(AuthorizeBatchOp::response(
            &HttpResponse::new(400, fixture!("sandbox_batch_validation.json")),
            &batch()
        ))),
        (400, 2)
    );
    let sv = |r: openhub_bo_core::Result<BatchStatus>| r.map(|x| serde_json::to_value(x).unwrap());
    assert_eq!(
        err(sv(BatchStatusOp::response(
            &ok(fixture!("sandbox_batch_not_found.json")),
            &q
        ))),
        (200, 1)
    );
}

#[test]
fn batch_webhook_with_token_and_ack() {
    let call = |token: &str| BatchWebhookCall {
        token: token.into(),
        body: fixture!("batch_webhook.json").into(),
        expected_token: "tok_0123456789abcdef".into(),
    };
    let n = ParseBatchWebhook::handle(call("tok_0123456789abcdef")).unwrap();
    assert_eq!(n.batch_number.as_deref(), Some("2601191045"));
    assert_eq!(n.transfer.status, PaymentStatus::Paid);
    assert_eq!(
        n.ack,
        json!({"nroLote": "2601191045", "numeroReferencia": "51021455454645646", "codigoRespuesta": "EXITOSO", "detalleRespuesta": null})
    );
    assert!(matches!(
        ParseBatchWebhook::handle(call("wrong")),
        Err(Error::WebhookAuth { .. })
    ));
}

#[test]
fn registry_describes_payouts_with_timeouts() {
    let ops = Chain(CoreOps, PayoutsOps);
    let described: JsonValue = serde_json::from_str(&call(&ops, "describe", "{}")).unwrap();
    let find = |name: &str| {
        described["value"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == name)
            .unwrap()
            .clone()
    };
    assert_eq!(find("payouts.scan")["timeout_secs"], 40);
    assert_eq!(
        (
            find("payouts.pay")["timeout_secs"].clone(),
            find("payouts.pay")["idempotent"].clone()
        ),
        (json!(90), json!(false))
    );
    assert_eq!(find("batch.banks").get("timeout_secs"), None);
    assert_eq!(find("batch.webhook.parse")["kind"], "handler");
}
