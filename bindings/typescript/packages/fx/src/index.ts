/**
 * Cross-currency QR collections for the Red Enlace (ATC) OpenHub APIs:
 * PIX (payers in Brazil), virtual assets via Koibanx (USDT/USDC), Binance Pay.
 * Each product must be enabled for your merchant by ATC (`GQ-…` errors otherwise).
 */
import {
  type Amount,
  Handler,
  Op,
  type PaymentStatus,
  type Session,
  toAmount,
  toSeconds,
  type Webhook,
} from "@openhub-bo/core";

import { NATIVE } from "./native.js";

export type Currency = "BOB" | "USD";
export type VirtualAsset = "usdt" | "usdc" | "bk";

/** `glosa` parts; the FX APIs require `branch|name|category|description`. */
export interface GlosaParts {
  branchCode: string;
  branchName: string;
  /** e.g. an MCC like `7011` or `MISCELANEAS`. */
  category: string;
  description: string;
}

export function glosa(parts: GlosaParts): string {
  return [parts.branchCode, parts.branchName, parts.category, parts.description].join("|");
}

function glosaWire(value: GlosaParts | string): string {
  return typeof value === "string" ? value : glosa(value);
}

export interface FxQr {
  /** ATC's reference; use it for status queries. */
  reference: string;
  merchantReference: string | null;
  status: PaymentStatus;
  rawStatus: string;
  detail: string | null;
  amount: string;
  currency: string;
  /** What the payer pays, in `convertedCurrency` (BRL, USDT, USDC...). */
  convertedAmount: string | null;
  convertedCurrency: string | null;
  exchangeRate: string | null;
  /** As sent by ATC; the format differs per product. */
  expiresAt: string | null;
  imageBase64: string;
  /** `image/png` (PIX, Koibanx) or `image/jpeg` (Binance). */
  imageMime: string;
}

export interface FxStatusInfo {
  reference: string;
  status: PaymentStatus;
  rawStatus: string;
  detail: string | null;
  amount: string | null;
  currency: string | null;
  convertedAmount: string | null;
  convertedCurrency: string | null;
  exchangeRate: string | null;
  /** PIX: reversal details, if any (raw ATC JSON). */
  reversal: unknown;
  /** PIX cancel: when the request was processed. */
  requestedAt: string | null;
}

export interface BinanceNotification {
  reference: string;
  status: PaymentStatus;
  rawStatus: string;
  amount: string | null;
  currency: string | null;
  transactionAt: string | null;
  payerName: string | null;
  payerDocumentId: string | null;
  /** Answer the webhook with this JSON and HTTP 200; ATC requires it. */
  ack: Record<string, unknown>;
}

/** PIX / Koibanx notification; payload undocumented, read best-effort. */
export interface FxNotification {
  reference: string | null;
  status: PaymentStatus;
  rawStatus: string | null;
  amount: string | null;
  currency: string | null;
  /** Full body as sent by ATC. */
  payload: Record<string, unknown>;
}

interface CommonQrInput {
  amount: Amount;
  glosa: GlosaParts | string;
  /** Your reference, digits only. */
  reference: string;
  webhook: Webhook;
  currency?: Currency;
  channel?: string;
}

export interface PixQrInput extends CommonQrInput {
  /** Brazilian CPF, 11 digits. */
  payerCpf: string;
  /** `+` and 13 digits, e.g. `+5511999999999`. */
  payerPhone: string;
  payerEmail?: string;
  /** Seconds; OpenHub defaults to 100. */
  expiresIn?: number;
  extra?: string;
}

export interface VirtualAssetQrInput extends CommonQrInput {
  asset: VirtualAsset;
  /** Seconds, 180..600. Defaults to 180. */
  expiresIn?: number;
  extra?: string;
}

export interface BinanceQrInput extends CommonQrInput {
  /** Seconds, at most 300. */
  expiresIn?: number;
  extra?: string;
}

const PIX_GENERATE = new Op<FxQr>(NATIVE, "pix.generate");
const PIX_VERIFY = new Op<FxStatusInfo>(NATIVE, "pix.verify");
const PIX_CANCEL = new Op<FxStatusInfo>(NATIVE, "pix.cancel");
const CRYPTO_GENERATE = new Op<FxQr>(NATIVE, "crypto.generate");
const CRYPTO_VERIFY = new Op<FxStatusInfo>(NATIVE, "crypto.verify");
const BINANCE_GENERATE = new Op<FxQr>(NATIVE, "binance.generate");
const BINANCE_VERIFY = new Op<FxStatusInfo>(NATIVE, "binance.verify");
const PARSE_BINANCE = new Handler<BinanceNotification>(NATIVE, "binance.webhook.parse");
const PARSE_FX = new Handler<FxNotification>(NATIVE, "fx.webhook.parse");

function common(input: CommonQrInput) {
  return {
    reference: String(input.reference),
    glosa: glosaWire(input.glosa),
    amount: toAmount(input.amount),
    currency: input.currency ?? "BOB",
    channel: input.channel ?? "WEB",
    webhook: input.webhook.toWire(),
  };
}

function seconds(value: number | undefined): number | undefined {
  return value === undefined ? undefined : toSeconds(value);
}

/** PIX: the payer pays BRL, you receive BOB/USD. */
export class PixClient {
  constructor(readonly session: Session) {}

  async generateQr(input: PixQrInput): Promise<FxQr> {
    return this.session.execute(PIX_GENERATE, {
      ...common(input),
      payerCpf: input.payerCpf,
      payerPhone: input.payerPhone,
      payerEmail: input.payerEmail,
      expiresIn: seconds(input.expiresIn),
      extra: input.extra,
    });
  }

  async getStatus(reference: string): Promise<FxStatusInfo> {
    return this.session.execute(PIX_VERIFY, { reference: String(reference) });
  }

  /**
   * OpenHub's "cancela". The sandbox only allows it on paid ("aprobado") QRs,
   * i.e. it acts as a refund request.
   */
  async cancel(reference: string): Promise<FxStatusInfo> {
    return this.session.execute(PIX_CANCEL, { reference: String(reference) });
  }
}

/** Charge in BOB/USD, settle in USDT/USDC through Koibanx. Minimum Bs 50. */
export class VirtualAssetsClient {
  constructor(readonly session: Session) {}

  async generateQr(input: VirtualAssetQrInput): Promise<FxQr> {
    return this.session.execute(CRYPTO_GENERATE, {
      ...common(input),
      asset: input.asset,
      expiresIn: toSeconds(input.expiresIn ?? 180),
      extra: input.extra,
    });
  }

  async getStatus(reference: string): Promise<FxStatusInfo> {
    return this.session.execute(CRYPTO_VERIFY, { reference: String(reference) });
  }
}

/** Binance Pay QR. */
export class BinanceClient {
  constructor(readonly session: Session) {}

  async generateQr(input: BinanceQrInput): Promise<FxQr> {
    return this.session.execute(BINANCE_GENERATE, {
      ...common(input),
      expiresIn: seconds(input.expiresIn),
      extra: input.extra,
    });
  }

  async getStatus(reference: string): Promise<FxStatusInfo> {
    return this.session.execute(BINANCE_VERIFY, { reference: String(reference) });
  }
}

function plainHeaders(headers: Record<string, string> | Headers): Record<string, string> {
  return headers instanceof Headers ? Object.fromEntries(headers.entries()) : headers;
}

/** Binance Pay notification. Reply with `notification.ack` and HTTP 200. */
export function parseBinanceWebhook(
  headers: Record<string, string> | Headers,
  body: string,
  webhook: Webhook,
): BinanceNotification {
  return PARSE_BINANCE.call({ headers: plainHeaders(headers), body, key: webhook.key, value: webhook.value });
}

/** PIX / Koibanx notification (undocumented payload). Confirm with `getStatus`. */
export function parseFxWebhook(
  headers: Record<string, string> | Headers,
  body: string,
  webhook: Webhook,
): FxNotification {
  return PARSE_FX.call({ headers: plainHeaders(headers), body, key: webhook.key, value: webhook.value });
}

export const version: string = NATIVE.version;
