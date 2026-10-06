import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { inspect } from "node:util";

import {
  AmbiguousOutcomeError,
  ApiError,
  CORE_NATIVE,
  errorFromCore,
  FetchTransport,
  Native,
  type NativeModule,
  Op,
  Session,
  toAmount,
  toCamel,
  toSnake,
  TransportError,
  ValidationError,
  Webhook,
} from "@openhub-bo/core";
import { MockGateway } from "@openhub-bo/core/testing";

describe("casing", () => {
  it("converts deeply and keeps verbatim fields", () => {
    assert.deepEqual(toSnake({ establishmentId: 1, webhook: { url: "u" }, headers: { "X-Api-Key": "v" }, skip: undefined }), {
      establishment_id: 1,
      webhook: { url: "u" },
      headers: { "X-Api-Key": "v" },
    });
    assert.deepEqual(toCamel({ payer_bank: { bank_name: "B" }, payload: { numero_referencia: 1 }, items: [{ a_b: 1 }] }), {
      payerBank: { bankName: "B" },
      payload: { numero_referencia: 1 },
      items: [{ aB: 1 }],
    });
  });
});

describe("native bridge", () => {
  it("rejects a native module with another protocol", () => {
    const fake: NativeModule = { call: () => "{}", protocolVersion: () => 1, version: () => "0" };
    assert.throws(() => new Native(fake, "fake"), /protocol 1/);
  });

  it("validates declared operations at import time", () => {
    assert.throws(() => new Op(CORE_NATIVE, "qr.nope"), /qr\.nope/);
    assert.equal(new Op(CORE_NATIVE, "token").idempotent, true);
  });

  it("maps core errors, including ambiguous ones with their operation", () => {
    const api = errorFromCore({ kind: "api", status: 502, message: "down", errors: [], retryable: true });
    assert.ok(api instanceof ApiError && api.retryable);
    const ambiguous = errorFromCore(
      { kind: "ambiguous", message: "no confirmado", errors: [{ message: "m", code: "96" }] },
      "payouts.pay.parse",
    );
    assert.ok(ambiguous instanceof AmbiguousOutcomeError);
    assert.equal(ambiguous.operation, "payouts.pay");
    assert.equal(ambiguous.code, "96");
    assert.equal(ambiguous.retryable, false);
  });
});

describe("session", () => {
  it("fails fast without credentials", () => {
    assert.throws(
      () => new Session({ clientId: "id" }),
      (e: unknown) => e instanceof ValidationError && e.field === "client_secret",
    );
  });

  it("defaults to the sandbox", () => {
    assert.equal(new MockGateway().session().environment, "sandbox");
  });
});

describe("amounts and webhooks", () => {
  it("accepts exact amounts and rejects floats", () => {
    assert.equal(toAmount("10.50"), "10.50");
    assert.equal(toAmount(10), "10");
    assert.equal(toAmount(10n), "10");
    assert.throws(() => toAmount(10.5), /floats/);
  });

  it("never prints the webhook secret", () => {
    const webhook = new Webhook({ url: "https://a.bo/h", value: "s3cret" });
    assert.ok(!inspect(webhook).includes("s3cret"));
    assert.ok(!JSON.stringify(webhook).includes("s3cret"));
    assert.equal(webhook.toWire().value, "s3cret");
  });
});

describe("fetch transport", () => {
  const request = { method: "GET" as const, url: "https://x.bo", headers: {}, body: null };

  it("knows when a request never left", async () => {
    const refused = Object.assign(new TypeError("fetch failed"), { cause: { code: "ECONNREFUSED" } });
    const transport = new FetchTransport({ fetch: async () => Promise.reject(refused) });
    await assert.rejects(transport.send(request), (e: unknown) => e instanceof TransportError && !e.maybeSent && e.retryable);
  });

  it("applies the per-request timeout and marks the outcome as unknown", async () => {
    const hang: typeof fetch = (_url, init) =>
      new Promise((_, reject) => init?.signal?.addEventListener("abort", () => reject(init.signal?.reason)));
    const transport = new FetchTransport({ fetch: hang, timeout: 30 });
    const started = Date.now();
    await assert.rejects(transport.send({ ...request, timeout: 0.05 }), (e: unknown) => e instanceof TransportError && e.maybeSent);
    assert.ok(Date.now() - started < 2000);
  });
});
