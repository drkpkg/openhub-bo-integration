/**
 * OpenHub QR demo service — TypeScript (@openhub-bo/qr).
 *
 * Serves demo/web/index.html and the demo API shared by the three language
 * services. Environment: CLIENT_ID, CLIENT_SECRET, DEMO_PASSWORD, PORT (8102),
 * PUBLIC_URL (optional; else derived from the Host header), DEMO_LINKS
 * (optional "Python=https://...,TypeScript=...,Ruby=...").
 */
import { randomBytes, timingSafeEqual } from "node:crypto";
import { readFileSync } from "node:fs";
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { ApiError, OpenHubError, Session, ValidationError, Webhook, WebhookAuthError } from "@openhub-bo/core";
import { QrClient, parseWebhook, type GeneratedQr, type QrKind, type QrStatusInfo } from "@openhub-bo/qr";

const LANGUAGE = "TypeScript";
const MAX_AMOUNT = 10;
const INDEX = readFileSync(new URL("../web/index.html", import.meta.url));
const VERSION = JSON.parse(
  readFileSync(new URL("../../bindings/typescript/packages/qr/package.json", import.meta.url), "utf8"),
).version as string;
const env = (name: string): string => {
  const value = process.env[name];
  if (!value) throw new Error(`missing ${name}`);
  return value;
};
const PASSWORD = env("DEMO_PASSWORD");
const WEBHOOK_SECRET = randomBytes(32).toString("base64url");
const qrClient = new QrClient(
  new Session({ clientId: env("CLIENT_ID"), clientSecret: env("CLIENT_SECRET"), environment: "sandbox" }),
);
const events: Record<string, unknown>[] = [];

const links = () =>
  (process.env["DEMO_LINKS"] ?? "")
    .split(",")
    .filter((part) => part.includes("="))
    .map((part) => {
      const [language, url] = part.split(/=(.*)/s);
      return { language, url };
    });

// OpenHub stores the merchant reference as a 32-bit integer.
const newReference = () => String(Date.now() % 2_147_483_647);

const qrJson = (qr: GeneratedQr, webhookUrl: string) => ({
  kind: qr.kind,
  reference: qr.reference,
  merchantReference: qr.merchantReference,
  status: qr.status,
  rawStatus: qr.rawStatus,
  amount: qr.amount,
  currency: qr.currency,
  expiresAt: qr.expiresAt,
  qrDataUri: `data:image/png;base64,${qr.qrImageBase64}`,
  webhookUrl,
});

const statusJson = (st: QrStatusInfo) => ({
  kind: st.kind,
  reference: st.reference,
  status: st.status,
  rawStatus: st.rawStatus,
  message: st.message,
  amount: st.amount,
  payer: st.payer?.name ?? null,
  payerBank: st.payerBank?.bankName ?? null,
});

function send(res: ServerResponse, status: number, body: string | Buffer, type = "application/json") {
  res.writeHead(status, { "Content-Type": type, "Cache-Control": "no-store" });
  res.end(body);
}
const json = (res: ServerResponse, status: number, payload: unknown) => send(res, status, JSON.stringify(payload));

function authorized(req: IncomingMessage, res: ServerResponse): boolean {
  const expected = Buffer.from(`Basic ${Buffer.from(`demo:${PASSWORD}`).toString("base64")}`);
  const got = Buffer.from(req.headers.authorization ?? "");
  if (got.length === expected.length && timingSafeEqual(got, expected)) return true;
  res.writeHead(401, { "WWW-Authenticate": 'Basic realm="openhub-demo"' }).end();
  return false;
}

async function readBody(req: IncomingMessage): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(chunk as Buffer);
  return Buffer.concat(chunks).toString("utf8");
}

async function call(res: ServerResponse, fn: () => Promise<unknown>) {
  try {
    json(res, 200, await fn());
  } catch (err) {
    if (err instanceof ValidationError) json(res, 400, { error: err.message, field: err.field });
    else if (err instanceof ApiError) json(res, 422, { error: err.message, code: err.code, retryable: err.retryable });
    else json(res, 422, { error: err instanceof Error ? `${err.name}: ${err.message}` : String(err) });
  }
}

async function generate(req: IncomingMessage, form: Record<string, string>) {
  const amount = String(form["amount"] ?? "").trim();
  if (!/^\d+(\.\d{1,2})?$/.test(amount)) throw new ValidationError("must be a decimal number", "amount");
  if (Number(amount) > MAX_AMOUNT) throw new ValidationError(`the demo allows at most Bs ${MAX_AMOUNT}`, "amount");
  const kind = (form["kind"] === "mld" ? "mld" : "simple") as QrKind;
  const publicUrl = process.env["PUBLIC_URL"] ?? `https://${req.headers.host ?? "localhost"}`;
  const webhookUrl = `${publicUrl}/webhooks/qr/${kind}`;
  const qr = await qrClient.generateQr({
    kind,
    amount,
    description: String(form["description"] ?? "Prueba demo"),
    reference: newReference(),
    establishmentId: 1,
    establishmentName: "Demo openhub bo",
    expiresIn: Number(form["expiresIn"] ?? 600),
    webhook: new Webhook({ url: webhookUrl, value: WEBHOOK_SECRET }),
  });
  return qrJson(qr, webhookUrl);
}

async function webhook(req: IncomingMessage, res: ServerResponse, kind: QrKind, body: string) {
  const event: Record<string, unknown> = { receivedAt: new Date().toISOString(), kind };
  let status = 200;
  try {
    const headers = Object.fromEntries(
      Object.entries(req.headers).map(([k, v]) => [k, Array.isArray(v) ? v.join(", ") : (v ?? "")]),
    );
    const n = parseWebhook(headers, body, new Webhook({ url: "", value: WEBHOOK_SECRET }));
    Object.assign(event, { reference: n.reference, status: n.status, amount: n.amount });
    event["confirmedStatus"] = (await qrClient.getQrStatus(n.reference, { kind })).status;
  } catch (err) {
    if (err instanceof WebhookAuthError) {
      status = 401;
      event["error"] = `rechazado: ${err.message}`;
    } else if (err instanceof OpenHubError) {
      event["error"] = `${err.name}: ${err.message} · body=${body.slice(0, 300)}`;
    } else throw err;
  }
  events.push(event);
  if (events.length > 50) events.shift();
  console.log(`webhook ${kind} -> ${status}`, event);
  json(res, status, { ok: status === 200 });
}

const server = createServer(async (req, res) => {
  const path = (req.url ?? "/").split("?")[0]!;
  try {
    if (req.method === "GET" && path === "/health") return json(res, 200, { ok: true, language: LANGUAGE });
    if (req.method === "POST") {
      const body = await readBody(req);
      const hook = /^\/webhooks\/qr\/(simple|mld)$/.exec(path);
      if (hook) return await webhook(req, res, hook[1] as QrKind, body);
      if (!authorized(req, res)) return;
      if (path === "/api/qr") return await call(res, () => generate(req, JSON.parse(body || "{}")));
      const cancel = /^\/api\/qr\/simple\/(\d+)\/cancel$/.exec(path);
      if (cancel) return await call(res, async () => statusJson(await qrClient.cancelQr(cancel[1]!)));
      return json(res, 404, { error: "not found" });
    }
    if (!authorized(req, res)) return;
    if (path === "/") return send(res, 200, INDEX, "text/html; charset=utf-8");
    if (path === "/api/info")
      return json(res, 200, {
        language: LANGUAGE,
        library: "@openhub-bo/qr",
        version: VERSION,
        environment: "sandbox",
        links: links(),
      });
    if (path === "/api/events") return json(res, 200, [...events].reverse());
    const status = /^\/api\/qr\/(simple|mld)\/(\d+)$/.exec(path);
    if (status)
      return await call(res, async () =>
        statusJson(await qrClient.getQrStatus(status[2]!, { kind: status[1] as QrKind })),
      );
    json(res, 404, { error: "not found" });
  } catch (err) {
    console.error(err);
    if (!res.headersSent) json(res, 500, { error: "internal error" });
  }
});

const port = Number(process.env["PORT"] ?? 8102);
server.listen(port, () => console.log(`${LANGUAGE} demo on :${port}`));
