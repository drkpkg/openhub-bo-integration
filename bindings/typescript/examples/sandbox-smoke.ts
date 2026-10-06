/**
 * End-to-end check against the real OpenHub sandbox: generate a Bs 1.00 QR
 * (valid 2 minutes), query it, cancel it, and check that cancelling again is
 * rejected. Reads CLIENT_ID / CLIENT_SECRET from the environment:
 *
 *     node --env-file=../../.env examples/sandbox-smoke.ts   (from bindings/typescript)
 *
 * Never prints credentials or tokens.
 */
import { ApiError, Session, Webhook } from "@openhub-bo/core";
import { QrClient } from "@openhub-bo/qr";

const clientId = process.env["CLIENT_ID"];
const clientSecret = process.env["CLIENT_SECRET"];
if (!clientId || !clientSecret) throw new Error("set CLIENT_ID and CLIENT_SECRET");

const client = new QrClient(new Session({ clientId, clientSecret, environment: "sandbox" }));
const qr = await client.generateQr({
  amount: "1.00",
  description: "Prueba @openhub-bo/qr",
  reference: String(Math.floor(Date.now() / 1000)),
  establishmentId: 1,
  establishmentName: "Prueba",
  expiresIn: 120,
  webhook: new Webhook({ url: "https://example.com/openhub/webhook", value: "sandbox-smoke-test" }),
});
console.log(`generated  ref=${qr.reference} status=${qr.status} expiresAt=${qr.expiresAt}`);
console.log(`           png=${Buffer.from(qr.qrImageBase64, "base64").length} bytes, amount=${qr.amount} ${qr.currency}`);

const status = await client.getQrStatus(qr.reference);
console.log(`status     ${status.status} (${status.message})`);

const cancelled = await client.cancelQr(qr.reference);
console.log(`cancelled  ${cancelled.status} (${cancelled.message})`);

try {
  await client.cancelQr(qr.reference);
  console.log("re-cancel  unexpectedly succeeded");
  process.exitCode = 1;
} catch (error) {
  if (!(error instanceof ApiError)) throw error;
  console.log(`re-cancel  rejected as expected: HTTP ${error.status} ${error.code}`);
}
console.log(process.exitCode ? "FAILED" : "OK");
