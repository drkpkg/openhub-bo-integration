import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

import { ApiError, type HttpRequest, ValidationError, Webhook } from "@openhub-bo/core";
import { MockGateway } from "@openhub-bo/core/testing";
import { BinanceClient, glosa, parseBinanceWebhook, parseFxWebhook, PixClient, VirtualAssetsClient } from "@openhub-bo/fx";

const fixture = (name: string) => readFileSync(new URL(`../../../../../fixtures/openhub/fx/${name}`, import.meta.url), "utf8");
const webhook = new Webhook({ url: "https://shop.example/fx", value: "s3cret" });
const g = glosa({ branchCode: "1", branchName: "Tienda", category: "7011", description: "Pedido" });
const pix = { amount: "145.00", glosa: g, reference: "311113", payerCpf: "12345678901", payerPhone: "+5511999999999", webhook };

describe("fx", () => {
  it("generates PIX with conversion fields", async () => {
    const gw = new MockGateway().respond("POST", /\/qr\/pix\/v2\/generar$/, 200, fixture("pix_generate_response.json"));
    const qr = await new PixClient(gw.session()).generateQr({ ...pix, expiresIn: 120 });
    assert.equal(qr.convertedCurrency, "BRL");
    assert.equal(qr.convertedAmount, "592.52");
    assert.equal(qr.imageMime, "image/png");
    const body = JSON.parse((gw.requests.at(-1) as HttpRequest).body ?? "");
    assert.equal(body.glosa, "1|Tienda|7011|Pedido");
    assert.equal(body.tiempoQr, "00:02:00");
  });

  it("surfaces the sandbox's merchant-not-enabled code", async () => {
    const gw = new MockGateway().respond("POST", /\/qr\/pix\/v2\/generar$/, 200, fixture("sandbox_pix_merchant_disabled.json"));
    await assert.rejects(new PixClient(gw.session()).generateQr(pix), (e: unknown) => e instanceof ApiError && e.code === "GQ-00005" && !e.retryable);
  });

  it("validates sandbox rules locally", async () => {
    const gw = new MockGateway();
    const crypto = new VirtualAssetsClient(gw.session());
    await assert.rejects(
      crypto.generateQr({ amount: "49.99", glosa: g, reference: "1", asset: "usdt", webhook }),
      (e: unknown) => e instanceof ValidationError && e.field === "amount",
    );
    await assert.rejects(
      new BinanceClient(gw.session()).generateQr({ amount: "1", glosa: g, reference: "1", webhook, expiresIn: 301 }),
      (e: unknown) => e instanceof ValidationError && e.field === "expires_in",
    );
    await assert.rejects(new PixClient(gw.session()).generateQr({ ...pix, payerPhone: "+59171234567" }), (e: unknown) => e instanceof ValidationError && e.field === "payer_phone");
  });

  it("parses statuses for each product", async () => {
    const gw = new MockGateway()
      .respond("GET", /\/koibanx\/v2\/estado\/\d+$/, 200, fixture("crypto_status_response.json"))
      .respond("GET", /\/binance\/v2\/verificar\/\d+$/, 200, fixture("binance_status_response.json"));
    assert.equal((await new VirtualAssetsClient(gw.session()).getStatus("200699")).status, "expired");
    assert.equal((await new BinanceClient(gw.session()).getStatus("11193577")).convertedCurrency, "USDT");
  });

  it("keeps raw webhook payloads and Binance's ack verbatim", () => {
    const n = parseBinanceWebhook({ "x-api-key": "s3cret" }, fixture("binance_webhook.json"), webhook);
    assert.deepEqual(n.ack, { numeroReferencia: "4221", codigoRespuesta: "00", detalleRespuesta: null });
    const fx = parseFxWebhook({ "x-api-key": "s3cret" }, JSON.stringify({ numeroReferencia: 6780, codigoRespuesta: "PAID", monto_extra: 1 }), webhook);
    assert.equal(fx.reference, "6780");
    assert.equal(fx.status, "paid");
    // Raw payload keys are not camelCased.
    assert.deepEqual(Object.keys(fx.payload).sort(), ["codigoRespuesta", "monto_extra", "numeroReferencia"]);
  });
});
