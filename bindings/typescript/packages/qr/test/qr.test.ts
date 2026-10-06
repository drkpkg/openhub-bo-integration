import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

import {
  AmbiguousOutcomeError,
  ApiError,
  type HttpRequest,
  TransportError,
  ValidationError,
  Webhook,
  WebhookAuthError,
} from "@openhub-bo/core";
import { MockGateway } from "@openhub-bo/core/testing";
import { parseWebhook, QrClient } from "@openhub-bo/qr";

const fixture = (name: string) => readFileSync(new URL(`../../../../../fixtures/openhub/${name}`, import.meta.url), "utf8");
// Same webhook as the documented request in fixtures/openhub/generate_request.json.
const webhook = new Webhook({ url: "https://dominio.com/qr/confirmed", value: "46bc-b2a2-ea12258c99ab" });
const args = {
  amount: "10.50",
  description: "Pago de servicio",
  reference: "4024",
  establishmentId: 422717,
  establishmentName: "Tienda Central",
  expiresIn: 45,
  webhook,
};

// Specific routes first: the first match wins.
function gateway() {
  return new MockGateway()
    .respond("GET", /\/qr\/simple\/v2\/verify\/999$/, 404, fixture("error_not_found_response.json"))
    .respond("GET", /\/qr\/simple\/v2\/verify\/502$/, 502, "Error forwarding call")
    .respond("POST", /\/qr\/(simple|mld)\/v2\/generate$/, 200, fixture("generate_response.json"))
    .respond("GET", /\/qr\/simple\/v2\/verify\/\d+$/, 200, fixture("verify_pending_response.json"))
    .respond("POST", /\/qr\/simple\/v2\/cancel\/11195853$/, 200, fixture("cancel_response.json"))
    .respond("POST", /\/qr\/simple\/v2\/cancel\/1$/, 409, fixture("cancel_invalid_state_response.json"));
}

describe("QrClient", () => {
  it("generates with the documented wire body and camelCase output", async () => {
    const gw = gateway();
    const qr = await new QrClient(gw.session()).generateQr(args);
    assert.equal(qr.reference, "153980");
    assert.equal(qr.merchantReference, "2320");
    assert.equal(qr.status, "pending");
    assert.equal(qr.amount, "10.5");
    assert.equal(qr.kind, "simple");
    const sent = gw.requests.at(-1) as HttpRequest;
    assert.deepEqual(JSON.parse(sent.body ?? ""), JSON.parse(fixture("generate_request.json")));
    assert.equal(sent.headers["Authorization"], `Bearer ${sent.headers["access_token"]}`);
    assert.equal(sent.timeout, undefined);
  });

  it("keeps the MLD kind on generated QRs", async () => {
    const gw = gateway();
    const qr = await new QrClient(gw.session()).generateQr({ ...args, kind: "mld" });
    assert.equal(qr.kind, "mld");
    assert.match((gw.requests.at(-1) as HttpRequest).url, /\/qr\/mld\/v2\/generate$/);
  });

  it("queries, cancels and maps API errors", async () => {
    const client = new QrClient(gateway().session());
    const status = await client.getQrStatus("200393");
    assert.equal(status.payer, null);
    assert.equal(status.kind, "simple");
    assert.equal((await client.cancelQr("11195853")).status, "cancelled");
    await assert.rejects(client.cancelQr("1"), (e: unknown) => e instanceof ApiError && e.status === 409 && e.code === "ESTADO_INVALIDO");
    await assert.rejects(client.getQrStatus("999"), (e: unknown) => e instanceof ApiError && e.code === "TRANSACCION_NO_ENCONTRADA" && !e.retryable);
  });

  it("fetches the token once for concurrent calls and refreshes it when rejected", async () => {
    const gw = gateway();
    const client = new QrClient(gw.session());
    await Promise.all([client.getQrStatus("1"), client.getQrStatus("2"), client.generateQr(args)]);
    assert.equal(gw.tokenRequests, 1);
    gw.revokeTokens();
    await client.getQrStatus("3");
    assert.equal(gw.tokenRequests, 2);
  });

  it("validates before the network", async () => {
    const gw = gateway();
    const client = new QrClient(gw.session());
    for (const [override, field] of [
      [{ amount: "10.505" }, "amount"],
      [{ reference: "ord-42" }, "reference"],
      [{ description: " " }, "description"],
      [{ expiresIn: 0 }, "expires_in"],
      [{ establishmentName: "Demo openhub-bo" }, "establishment_name"],
      [{ reference: "2147483648" }, "reference"],
    ] as const) {
      await assert.rejects(client.generateQr({ ...args, ...override }), (e: unknown) => e instanceof ValidationError && e.field === field);
    }
    assert.ok(gw.requests.every((r) => !r.url.endsWith("/generate")));
    await assert.rejects(client.generateQr({ ...args, amount: 10.5 }), /floats/);
  });

  it("classifies gateway failures and lost responses", async () => {
    const gw = gateway();
    const lost = (path: RegExp) => ({
      send: (request: HttpRequest) =>
        path.test(request.url) ? Promise.reject(new TransportError("socket hang up", true)) : gw.send(request),
    });
    const client = new QrClient(gw.session());
    await assert.rejects(client.getQrStatus("502"), (e: unknown) => e instanceof ApiError && e.retryable);

    const generateLost = new QrClient(gw.session({ transport: lost(/\/generate$/) }));
    await assert.rejects(generateLost.generateQr(args), (e: unknown) => e instanceof AmbiguousOutcomeError && e.operation === "qr.generate");

    const verifyLost = new QrClient(gw.session({ transport: lost(/\/verify\//) }));
    await assert.rejects(verifyLost.getQrStatus("1"), (e: unknown) => e instanceof TransportError && e.retryable);
  });
});

describe("parseWebhook", () => {
  it("authenticates and parses the documented payload", () => {
    const n = parseWebhook(new Headers({ "X-API-KEY": webhook.value }), fixture("webhook_payment.json"), webhook);
    assert.equal(n.success, true);
    assert.equal(n.status, "paid");
    assert.equal(n.payer?.documentId, "12345678");
    assert.throws(() => parseWebhook({ "x-api-key": "nope" }, fixture("webhook_payment.json"), webhook), WebhookAuthError);
  });
});
