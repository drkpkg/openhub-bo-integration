/**
 * OpenHub QR demo service — TypeScript (@openhub-bo/qr).
 *
 * Serves its demo app (demo/apps/caja) and the demo API shared by the three language
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
const APP = "caja";
/** Per app: browsers share cookies across localhost ports. */
const COOKIE = `demo_${APP}`;
const MAX_AMOUNT = 10;
const INDEX = readFileSync(new URL(`../apps/${APP}/index.html`, import.meta.url));
const LOGIN = readFileSync(new URL("../web/login.html", import.meta.url));
const sessions = new Set<string>();
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
/** "kind:reference" -> record shown by the app. */
const payments = new Map<string, Record<string, unknown>>();

/** Keeps the app's payment list in sync with what OpenHub reports. */
function track(kind: string, reference: string, fields: Record<string, unknown>) {
  const record = payments.get(`${kind}:${reference}`);
  if (record) Object.assign(record, fields, { updatedAt: new Date().toISOString() });
}

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

/** A paid notification shaped like ATC's (fixtures/openhub/webhook_payment.json).
 * Fictitious payer; only used by the demo's "Simular pago" button. */
function simulatedWebhook(record: Record<string, unknown>): string {
  const bolivia = new Date(Date.now() - 4 * 3600_000).toISOString().slice(0, 19);
  return JSON.stringify({
    detalleRespuesta: "Transacción procesada correctamente",
    codigoRespuesta: "SUCCESS",
    numeroReferencia: record["reference"],
    monto: Number(record["amount"]),
    fechaHoraTransaccion: bolivia,
    moneda: record["currency"],
    clienteOrigen: { ciCliente: "0000000", nombreCliente: "Cliente de prueba", numeroCuenta: "0000000000" },
    bancoOrigen: { codigoBanco: "000", nombreBanco: "Banco simulado", numeroOrdenAch: `SIM${record["reference"]}` },
  });
}

/** Status as the app sees it: ATC still reports a simulated payment as pending. */
async function statusAndTrack(kind: QrKind, reference: string) {
  const record = payments.get(`${kind}:${reference}`);
  if (record?.["simulated"])
    return {
      kind,
      reference,
      status: record["status"],
      rawStatus: "PAGADO (simulado)",
      message: null,
      amount: record["amount"],
      payer: record["payer"] ?? null,
      payerBank: record["payerBank"] ?? null,
      simulated: true,
    };
  const result = statusJson(await qrClient.getQrStatus(reference, { kind }));
  track(kind, reference, { status: result.status, payer: result.payer, payerBank: result.payerBank });
  return result;
}

/** Sends ATC's paid notification to this service's public webhook URL, through the
 * tunnel, with the QR's secret header: same path as a real payment. */
async function simulate(kind: QrKind, reference: string) {
  const record = payments.get(`${kind}:${reference}`);
  if (!record) throw new Error("unknown QR in this session");
  if (record["status"] !== "pending") throw new Error(`only pending QRs can be paid (status: ${record["status"]})`);
  const webhookUrl = String(record["webhookUrl"]);
  let response: Response;
  try {
    response = await fetch(webhookUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json", "x-api-key": WEBHOOK_SECRET, "X-Demo-Simulated": "1" },
      body: simulatedWebhook(record),
      signal: AbortSignal.timeout(20_000),
    });
  } catch (err) {
    throw new Error(`webhook no entregado: ${err instanceof Error ? err.message : String(err)}`);
  }
  return { webhookUrl, webhookStatus: response.status, ...(await statusAndTrack(kind, reference)) };
}

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

const safeEqual = (a: string, b: string) => {
  const x = Buffer.from(a);
  const y = Buffer.from(b);
  return x.length === y.length && timingSafeEqual(x, y);
};

/** Session cookie (browser) or HTTP Basic (scripts). Never triggers the
 * browser's Basic-auth dialog: pages redirect to /login, APIs get 401. */
function authorized(req: IncomingMessage, res: ServerResponse, path: string): boolean {
  if (safeEqual(req.headers.authorization ?? "", `Basic ${Buffer.from(`demo:${PASSWORD}`).toString("base64")}`))
    return true;
  const token = new RegExp(`(?:^|;\\s*)${COOKIE}=([^;]+)`).exec(req.headers.cookie ?? "")?.[1];
  if (token && sessions.has(token)) return true;
  if (path === "/") res.writeHead(303, { Location: "/login" }).end();
  else json(res, 401, { error: "unauthorized" });
  return false;
}

function login(req: IncomingMessage, res: ServerResponse, body: string) {
  const password = new URLSearchParams(body).get("password") ?? "";
  if (!safeEqual(password, PASSWORD)) return res.writeHead(303, { Location: "/login?error=1" }).end();
  const token = randomBytes(24).toString("base64url");
  sessions.add(token);
  const secure = req.headers["x-forwarded-proto"] === "https" ? "; Secure" : "";
  res
    .writeHead(303, { Location: "/", "Set-Cookie": `${COOKIE}=${token}; Path=/; HttpOnly; SameSite=Lax${secure}` })
    .end();
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
  const result = qrJson(qr, webhookUrl);
  const now = new Date().toISOString();
  payments.set(`${qr.kind}:${qr.reference}`, {
    kind: qr.kind,
    reference: qr.reference,
    merchantReference: qr.merchantReference,
    label: String(form["label"] ?? form["description"] ?? ""),
    amount: qr.amount,
    currency: qr.currency,
    status: qr.status,
    expiresAt: qr.expiresAt,
    webhookUrl,
    createdAt: now,
    updatedAt: now,
  });
  return result;
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
    const payer = { payer: n.payer?.name ?? null, payerBank: n.payerBank?.bankName ?? null };
    if (req.headers["x-demo-simulated"] === "1") {
      // ATC knows nothing about a simulated payment: don't ask it to confirm.
      Object.assign(event, { simulated: true, confirmedStatus: n.status });
      track(kind, n.reference, { status: n.status, viaWebhook: true, simulated: true, ...payer });
    } else {
      event["confirmedStatus"] = (await qrClient.getQrStatus(n.reference, { kind })).status;
      track(kind, n.reference, { status: event["confirmedStatus"], viaWebhook: true, ...payer });
    }
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
      if (path === "/login") return login(req, res, body);
      if (!authorized(req, res, path)) return;
      if (path === "/api/qr") return await call(res, () => generate(req, JSON.parse(body || "{}")));
      const cancel = /^\/api\/qr\/simple\/(\d+)\/cancel$/.exec(path);
      if (cancel)
        return await call(res, async () => {
          const result = statusJson(await qrClient.cancelQr(cancel[1]!));
          track("simple", cancel[1]!, { status: result.status });
          return result;
        });
      const sim = /^\/api\/qr\/(simple|mld)\/(\d+)\/simulate$/.exec(path);
      if (sim) return await call(res, () => simulate(sim[1] as QrKind, sim[2]!));
      return json(res, 404, { error: "not found" });
    }
    if (path === "/login") return send(res, 200, LOGIN, "text/html; charset=utf-8");
    if (!authorized(req, res, path)) return;
    if (path === "/") return send(res, 200, INDEX, "text/html; charset=utf-8");
    if (path === "/api/info")
      return json(res, 200, {
        language: LANGUAGE,
        app: APP,
        library: "@openhub-bo/qr",
        version: VERSION,
        environment: "sandbox",
        links: links(),
      });
    if (path === "/api/events") return json(res, 200, [...events].reverse());
    if (path === "/api/payments")
      return json(
        res,
        200,
        [...payments.values()].sort((a, b) => String(b["createdAt"]).localeCompare(String(a["createdAt"]))),
      );
    const status = /^\/api\/qr\/(simple|mld)\/(\d+)$/.exec(path);
    if (status)
      return await call(res, () => statusAndTrack(status[1] as QrKind, status[2]!));
    json(res, 404, { error: "not found" });
  } catch (err) {
    console.error(err);
    if (!res.headersSent) json(res, 500, { error: "internal error" });
  }
});

const port = Number(process.env["PORT"] ?? 8102);
server.listen(port, () => console.log(`${LANGUAGE} demo on :${port}`));
