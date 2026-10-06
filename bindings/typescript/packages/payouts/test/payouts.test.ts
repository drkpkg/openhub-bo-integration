import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

import { AmbiguousOutcomeError, ApiError, type HttpRequest, ValidationError, WebhookAuthError } from "@openhub-bo/core";
import { MockGateway } from "@openhub-bo/core/testing";
import { isOpenAmount, parseBatchWebhook, PayoutsClient } from "@openhub-bo/payouts";

const fixture = (name: string) => readFileSync(new URL(`../../../../../fixtures/openhub/payouts/${name}`, import.meta.url), "utf8");
const TOKEN = "tok_0123456789abcdef";
const transfer = {
  transactionId: "001002",
  amount: "50.00",
  sourceAccount: "484811311404044",
  destinationAccount: "1311404044",
  bankCode: "1018",
  branchCity: "lpz",
  description: "Pago proveedor",
  recipientDocumentId: "5452452",
  recipientName: "PROVEEDOR SRL",
  date: "2026-10-06",
};

describe("PayoutsClient — QR", () => {
  it("scans, pays a fixed-amount QR with 0.00 and applies ATC's timeouts", async () => {
    const gw = new MockGateway()
      .respond("POST", /\/qr\/scan$/, 200, fixture("sandbox_scan_response.json"))
      .respond("POST", /\/qr\/confirm$/, 200, fixture("pay_response.json"));
    const client = new PayoutsClient(gw.session());
    const scanned = await client.scanQr("Wg74o8sx");
    assert.equal(scanned.recipient.bankName, "ADMINISTRADORA DE TARJETAS - ATC S.A.");
    assert.equal(isOpenAmount(scanned), false);
    assert.equal((gw.requests.at(-1) as HttpRequest).timeout, 40);

    const payout = await client.payQr(scanned, { sourceAccount: "7010123451", transactionId: "PAY-1" });
    assert.equal(payout.status, "paid");
    const sent = gw.requests.at(-1) as HttpRequest;
    assert.equal(sent.timeout, 90);
    assert.match(sent.body ?? "", /"importe":0\.00/);
    assert.equal(JSON.parse(sent.body ?? "").glosa, undefined);
  });

  it("requires amount for open-amount QRs and treats 96 as ambiguous", async () => {
    const gw = new MockGateway()
      .respond("POST", /\/qr\/scan$/, 200, fixture("scan_open_amount_response.json"))
      .respond("POST", /\/qr\/confirm$/, 200, fixture("pay_unconfirmed_response.json"));
    const client = new PayoutsClient(gw.session());
    const scanned = await client.scanQr("x");
    assert.equal(isOpenAmount(scanned), true);
    await assert.rejects(client.payQr(scanned, { sourceAccount: "7010123451", transactionId: "PAY-2" }), (e: unknown) => e instanceof ValidationError && e.field === "amount");
    await assert.rejects(
      client.payQr(scanned, { sourceAccount: "7010123451", transactionId: "PAY-2", amount: "20" }),
      (e: unknown) => e instanceof AmbiguousOutcomeError && e.operation === "payouts.pay" && e.code === "96",
    );
  });

  it("maps sandbox errors", async () => {
    const gw = new MockGateway()
      .respond("POST", /\/qr\/scan$/, 200, fixture("sandbox_scan_invalid.json"))
      .respond("GET", /\/qr\/status\/\d+$/, 200, fixture("sandbox_payment_not_found.json"));
    const client = new PayoutsClient(gw.session());
    await assert.rejects(client.scanQr("x"), (e: unknown) => e instanceof ApiError && e.code === "09");
    await assert.rejects(client.getPayout("1"), (e: unknown) => e instanceof ApiError && e.code === "04");
  });
});

describe("PayoutsClient — ACH batches", () => {
  it("authorizes with branchCode, a UUID and the token in the webhook URL", async () => {
    const gw = new MockGateway()
      .respond("POST", /\/lote\/autorizar$/, 200, fixture("batch_authorize_response.json"))
      .respond("POST", /\/bancos$/, 200, fixture("sandbox_banks_response.json"))
      .respond("GET", /\/lote\/estado\/[0-9a-f-]+$/, 200, fixture("batch_status_response.json"));
    const client = new PayoutsClient(gw.session());
    const auth = await client.authorizeBatch("455544", [transfer], { webhookUrl: "https://shop.example/ach", webhookToken: TOKEN });
    const sent = gw.requests.at(-1) as HttpRequest;
    const body = JSON.parse(sent.body ?? "");
    assert.equal(sent.headers["branchCode"], "455544");
    assert.match(body.processId, /^[0-9a-f-]{36}$/);
    assert.equal(body.webhookUrl, `https://shop.example/ach?token=${TOKEN}`);
    assert.equal(body.transacciones[0].codeSucursal, "LPZ");
    assert.deepEqual(auth.transfers.map((t) => t.status), ["pending", "error"]);

    assert.equal((await client.listBanks("455544"))[0]?.code, "1005");
    const status = await client.getBatchStatus("455544", body.processId, { batchNumber: "2601191045" });
    assert.equal(status.transfers[0]?.status, "paid");
    assert.match((gw.requests.at(-1) as HttpRequest).url, /\?nroLote=2601191045$/);
  });

  it("parses batch webhooks and builds both acks", () => {
    const n = parseBatchWebhook(TOKEN, fixture("batch_webhook.json"), TOKEN);
    assert.equal(n.transfer.status, "paid");
    assert.equal(n.ack().codigoRespuesta, "EXITOSO");
    assert.deepEqual(n.ack({ processed: false, detail: "dup" }), {
      nroLote: "2601191045",
      numeroReferencia: "51021455454645646",
      codigoRespuesta: "FALLIDO",
      detalleRespuesta: "dup",
    });
    assert.throws(() => parseBatchWebhook("wrong", fixture("batch_webhook.json"), TOKEN), WebhookAuthError);
  });
});
