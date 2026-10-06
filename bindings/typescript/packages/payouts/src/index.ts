/**
 * Payouts from your Red Enlace (ATC) OpenHub account: pay third-party
 * interoperable QRs and send ACH transfer batches. **Moves real money in
 * production.**
 *
 * QR flow: `scanQr` (show the recipient) → `payQr` → `getPayout`. `payQr`
 * throws `AmbiguousOutcomeError` when the result is unconfirmed (codes 94/96)
 * or the response is lost: query `getPayout` before retrying.
 */
import { randomUUID } from "node:crypto";

import { type Amount, Handler, Op, type PaymentStatus, type Session, toAmount, toIsoDate } from "@openhub-bo/core";

import { NATIVE } from "./native.js";

export interface Recipient {
  account: string | null;
  documentId: string | null;
  holder: string | null;
  bankCode: string | null;
  bankName: string | null;
}

export interface ScannedQr {
  reference: string;
  /** `"0"` for open-amount QRs: the payer chooses the amount. */
  amount: string;
  currency: string | null;
  description: string | null;
  recipient: Recipient;
  expiresOn: string | null;
}

export function isOpenAmount(qr: ScannedQr): boolean {
  return Number(qr.amount) === 0;
}

export interface Payout {
  reference: string;
  transactionId: string | null;
  transactionAt: string | null;
  status: PaymentStatus;
  rawStatus: string;
  message: string | null;
  amount: string | null;
  currency: string | null;
  description: string | null;
  sourceAccount: string | null;
  sourceHolder: string | null;
  recipient: Recipient;
  achOrderNumber: string | null;
}

export interface PayQrOptions {
  /** Your ATC account to debit. */
  sourceAccount: string;
  /** Your unique id (≤ 32 characters). */
  transactionId: string;
  /** Only for open-amount QRs. */
  amount?: Amount;
  /** Only when the QR has no description. */
  description?: string;
}

export interface BatchTransfer {
  /** 3–14 characters, unique within the batch. */
  transactionId: string;
  amount: Amount;
  sourceAccount: string;
  destinationAccount: string;
  /** See `listBanks`. */
  bankCode: string;
  /** CBB, COB, LPZ, ORU, POT, SCZ, SUC, TJA, TRI. */
  branchCity: string;
  description: string;
  recipientDocumentId: string;
  recipientName: string;
  currency?: "BOB" | "USD";
  /** Today or later; defaults to today (UTC). */
  date?: Date | string;
}

export interface BatchTransferResult {
  transactionId: string;
  status: PaymentStatus;
  rawStatus: string;
  reference: string | null;
  message: string | null;
}

export interface BatchAuthorization {
  batchNumber: string;
  processId: string;
  transfers: BatchTransferResult[];
}

export interface BatchTransferStatus {
  transactionId: string | null;
  reference: string | null;
  status: PaymentStatus;
  rawStatus: string;
  message: string | null;
  sourceAccount: string | null;
  destinationAccount: string | null;
  achNumber: string | null;
  recipientNumber: string | null;
  recipientDocumentId: string | null;
  recipientName: string | null;
  transactionAt: string | null;
  bankCode: string | null;
  bankName: string | null;
  amount: string | null;
  currency: string | null;
}

export interface BatchStatus {
  batchNumber: string | null;
  transfers: BatchTransferStatus[];
}

export interface Bank {
  code: string;
  name: string | null;
}

export interface BatchNotification {
  batchNumber: string | null;
  transfer: BatchTransferStatus;
  /** JSON body to answer the webhook with (HTTP 200). */
  ack(options?: { processed?: boolean; detail?: string | null }): Record<string, unknown>;
}

const SCAN = new Op<ScannedQr>(NATIVE, "payouts.scan");
const PAY = new Op<Payout>(NATIVE, "payouts.pay");
const STATUS = new Op<Payout>(NATIVE, "payouts.status");
const AUTHORIZE = new Op<BatchAuthorization>(NATIVE, "batch.authorize");
const BATCH_STATUS = new Op<BatchStatus>(NATIVE, "batch.status");
const BANKS = new Op<Bank[]>(NATIVE, "batch.banks");
const PARSE_WEBHOOK = new Handler<{
  batchNumber: string | null;
  transfer: BatchTransferStatus;
  ack: Record<string, unknown>;
}>(NATIVE, "batch.webhook.parse");

export class PayoutsClient {
  constructor(readonly session: Session) {}

  /** Decodes a QR from its text content (what a QR reader returns). */
  async scanQr(qrText: string): Promise<ScannedQr> {
    return this.session.execute(SCAN, { qrText });
  }

  /** Pays a scanned QR from your ATC account. Moves money. */
  async payQr(scanned: ScannedQr, options: PayQrOptions): Promise<Payout> {
    return this.session.execute(PAY, {
      scanned,
      sourceAccount: options.sourceAccount,
      transactionId: options.transactionId,
      amount: options.amount === undefined ? undefined : toAmount(options.amount),
      description: options.description,
    });
  }

  async getPayout(reference: string): Promise<Payout> {
    return this.session.execute(STATUS, { reference: String(reference) });
  }

  /**
   * Submits an ACH batch. ATC POSTs one notification per transfer to
   * `webhookUrl?token=<webhookToken>` (see `parseBatchWebhook`).
   * `processId` defaults to a new UUID; keep it to query the batch.
   */
  async authorizeBatch(
    branchCode: string,
    transfers: BatchTransfer[],
    options: { webhookUrl: string; webhookToken: string; processId?: string },
  ): Promise<BatchAuthorization> {
    return this.session.execute(AUTHORIZE, {
      branchCode,
      processId: options.processId ?? randomUUID(),
      webhookUrl: options.webhookUrl,
      webhookToken: options.webhookToken,
      transfers: transfers.map((t) => ({
        ...t,
        amount: toAmount(t.amount),
        currency: t.currency ?? "BOB",
        date: toIsoDate(t.date ?? new Date()),
      })),
    });
  }

  /** Give exactly one of `batchNumber` or `transactionId`. */
  async getBatchStatus(
    branchCode: string,
    processId: string,
    query: { batchNumber?: string; transactionId?: string },
  ): Promise<BatchStatus> {
    return this.session.execute(BATCH_STATUS, { branchCode, processId, ...query });
  }

  async listBanks(branchCode: string): Promise<Bank[]> {
    return this.session.execute(BANKS, { branchCode });
  }
}

/**
 * Authenticates and parses one batch-transfer notification. `token` is the
 * `token` query parameter of the incoming request.
 */
export function parseBatchWebhook(token: string, body: string, expectedToken: string): BatchNotification {
  const parsed = PARSE_WEBHOOK.call({ token, body, expectedToken });
  return {
    batchNumber: parsed.batchNumber,
    transfer: parsed.transfer,
    ack: ({ processed = true, detail = null } = {}) => ({
      ...parsed.ack,
      codigoRespuesta: processed ? "EXITOSO" : "FALLIDO",
      detalleRespuesta: detail,
    }),
  };
}

export const version: string = NATIVE.version;
