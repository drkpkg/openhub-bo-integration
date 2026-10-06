import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

import { ApiError, type HttpRequest, ValidationError } from "@openhub-bo/core";
import { MockGateway } from "@openhub-bo/core/testing";
import { AccountsClient } from "@openhub-bo/accounts";

const fixture = (name: string) => readFileSync(new URL(`../../../../../fixtures/openhub/accounts/${name}`, import.meta.url), "utf8");
const NIT = "1000000019";

describe("AccountsClient", () => {
  it("lists accounts, balances and movements", async () => {
    const gw = new MockGateway()
      .respond("GET", /\/cuentas\/\d+$/, 200, fixture("accounts_list_response.json"))
      .respond("POST", /\/cuentas\/saldos$/, 200, fixture("balances_response.json"))
      .respond("POST", /\/cuentas\/creditos$/, 200, fixture("credits_response.json"))
      .respond("POST", /\/cuentas\/transacciones$/, 200, fixture("reconciliation_response.json"));
    const client = new AccountsClient(gw.session());
    const listing = await client.listAccounts(NIT);
    assert.equal(listing.establishments[0]?.accounts[1]?.status, "blocked");
    const [balance] = await client.balances(NIT, ["7014227171"]);
    assert.equal(balance?.available, "59.5");
    const credits = await client.credits(NIT, "7011234561", { dateFrom: "2026-09-10", dateTo: new Date("2026-10-06T00:00:00Z") });
    assert.equal(credits[0]?.operationType, "PAYIN QR");
    assert.equal(credits[0]?.reference, null);
    const rec = await client.reconcile(NIT, { dateFrom: "2026-09-29", dateTo: "2026-10-06" });
    assert.equal(rec.movements.length, 2);
    assert.deepEqual(JSON.parse((gw.requests.at(-1) as HttpRequest).body ?? ""), { nit: NIT, fechaInicio: "2026-09-29", fechaFin: "2026-10-06" });
  });

  it("changes status with PATCH", async () => {
    const gw = new MockGateway().respond("PATCH", /\/cuentas\/estados$/, 200, fixture("accounts_status_response.json"));
    const [changed] = await new AccountsClient(gw.session()).setAccountStatus(NIT, [
      { accountNumber: "7011113693", status: "blocked", reason: "Sospecha de fraude" },
    ]);
    assert.equal(changed?.status, "blocked");
    assert.equal(JSON.parse((gw.requests.at(-1) as HttpRequest).body ?? "").cuentas[0].estado, "BLOQUEADA");
  });

  it("validates sandbox limits and maps error codes", async () => {
    const gw = new MockGateway().respond("GET", /\/cuentas\/\d+$/, 200, fixture("sandbox_merchant_not_found.json"));
    const client = new AccountsClient(gw.session());
    await assert.rejects(client.reconcile(NIT, { dateFrom: "2026-09-28", dateTo: "2026-10-06" }), (e: unknown) => e instanceof ValidationError && e.field === "date_to");
    await assert.rejects(client.balances(NIT, Array.from({ length: 11 }, (_, i) => String(7014227100 + i))), (e: unknown) => e instanceof ValidationError && e.field === "account_numbers");
    await assert.rejects(client.listAccounts("1234567"), (e: unknown) => e instanceof ApiError && e.code === "17");
  });
});
