/**
 * QR Simple / QR MLD-BCB collections for the Red Enlace (ATC) OpenHub APIs.
 *
 * ```ts
 * const session = new Session({ clientId, clientSecret });
 * const qr = await new QrClient(session).generateQr({ amount: "10.50", ... });
 * ```
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

export type QrKind = "simple" | "mld";

export interface Payer {
  name?: string | null;
  accountNumber?: string | null;
  /** CI or NIT. */
  documentId?: string | null;
}

export interface PayerBank {
  bankCode?: string | null;
  bankName?: string | null;
  achOrderNumber?: string | null;
  transactionDate?: string | null;
}

export interface GeneratedQr {
  /** ATC's reference. Store it: status queries and webhooks use it. */
  reference: string;
  /**
   * Simple and MLD references live in separate spaces (an MLD reference is not
   * found through the Simple endpoints): store the kind with the reference.
   */
  kind: QrKind;
  /** The reference you sent. */
  merchantReference: string | null;
  status: PaymentStatus;
  rawStatus: string;
  /** ISO 8601 without offset, Bolivia time (UTC-4). */
  expiresAt: string | null;
  currency: string;
  /** Decimal string. */
  amount: string;
  /** PNG image, base64. */
  qrImageBase64: string;
}

export interface QrStatusInfo {
  reference: string;
  kind: QrKind;
  merchantReference: string | null;
  status: PaymentStatus;
  rawStatus: string;
  message: string | null;
  amount: string | null;
  currency: string | null;
  payer: Payer | null;
  payerBank: PayerBank | null;
}

export interface PaymentNotification {
  /** ATC's reference, as returned by `generateQr`. */
  reference: string;
  amount: string;
  currency: string | null;
  status: PaymentStatus;
  success: boolean;
  responseCode: string;
  responseDetail: string | null;
  transactionAt: string | null;
  payer: Payer | null;
  payerBank: PayerBank | null;
}

export interface GenerateQrInput {
  /** Decimal string (e.g. `"10.50"`) or integer; at most 2 decimals. */
  amount: Amount;
  description: string;
  /** Your reference, digits only. */
  reference: string;
  establishmentId: number;
  establishmentName: string;
  /** Validity in seconds. */
  expiresIn: number;
  /** Required by OpenHub for every QR. */
  webhook: Webhook;
  kind?: QrKind;
}

const GENERATE = new Op<GeneratedQr>(NATIVE, "qr.generate");
const VERIFY = new Op<QrStatusInfo>(NATIVE, "qr.verify");
const CANCEL = new Op<QrStatusInfo>(NATIVE, "qr.cancel");
const PARSE_WEBHOOK = new Handler<PaymentNotification>(NATIVE, "qr.webhook.parse");

export class QrClient {
  constructor(readonly session: Session) {}

  /**
   * Creates a collection QR. A lost response raises `AmbiguousOutcomeError`:
   * the QR may exist.
   */
  async generateQr(input: GenerateQrInput): Promise<GeneratedQr> {
    return this.session.execute(GENERATE, {
      kind: input.kind ?? "simple",
      description: input.description,
      amount: toAmount(input.amount),
      reference: String(input.reference),
      expiresIn: toSeconds(input.expiresIn),
      establishmentId: input.establishmentId,
      establishmentName: input.establishmentName,
      webhook: input.webhook.toWire(),
    });
  }

  /**
   * `reference` is ATC's reference (`GeneratedQr.reference`); pass the same
   * `kind` the QR was generated with (`GeneratedQr.kind`).
   */
  async getQrStatus(reference: string, options: { kind?: QrKind } = {}): Promise<QrStatusInfo> {
    return this.session.execute(VERIFY, { kind: options.kind ?? "simple", reference: String(reference) });
  }

  /**
   * Cancels a pending QR Simple. Only `pending` QRs can be cancelled; otherwise
   * `ApiError` with `code === "ESTADO_INVALIDO"` (HTTP 409). MLD-BCB has no
   * cancel endpoint (the gateway answers 404 "without destination").
   */
  async cancelQr(reference: string): Promise<QrStatusInfo> {
    return this.session.execute(CANCEL, { reference: String(reference) });
  }
}

/**
 * Authenticates and parses a payment notification sent by ATC. ATC does not
 * sign payloads: confirm with `getQrStatus` before releasing goods.
 */
export function parseWebhook(
  headers: Record<string, string> | Headers,
  body: string,
  webhook: Webhook,
): PaymentNotification {
  const plain = headers instanceof Headers ? Object.fromEntries(headers.entries()) : headers;
  return PARSE_WEBHOOK.call({ headers: plain, body, key: webhook.key, value: webhook.value });
}

export const version: string = NATIVE.version;
