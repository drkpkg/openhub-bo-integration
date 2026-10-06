import { inspect } from "node:util";

export type Environment = "development" | "sandbox" | "production";

/** Status shared by every product family; `rawStatus` keeps ATC's value. */
export type PaymentStatus =
  | "pending"
  | "processing"
  | "paid"
  | "cancelled"
  | "expired"
  | "rejected"
  | "reversed"
  | "error"
  | "unknown";

const NOT_FINAL: ReadonlySet<PaymentStatus> = new Set(["pending", "processing", "unknown"]);

export function isFinal(status: PaymentStatus): boolean {
  return !NOT_FINAL.has(status);
}

/**
 * Where ATC must POST notifications. ATC sends `key: value` as a header; it is
 * the only way to authenticate the call, so use a long random `value`.
 * The value is hidden from `console.log` and `JSON.stringify`.
 */
export class Webhook {
  readonly url: string;
  readonly key: string;
  readonly #value: string;

  constructor(init: { url: string; value: string; key?: string }) {
    this.url = init.url;
    this.key = init.key ?? "x-api-key";
    this.#value = init.value;
  }

  get value(): string {
    return this.#value;
  }

  toWire(): { url: string; key: string; value: string } {
    return { url: this.url, key: this.key, value: this.#value };
  }

  toJSON(): { url: string; key: string } {
    return { url: this.url, key: this.key };
  }

  [inspect.custom](): string {
    return `Webhook { url: ${JSON.stringify(this.url)}, key: ${JSON.stringify(this.key)}, value: [redacted] }`;
  }
}

/** A decimal amount: a string like `"10.50"` or an integer. */
export type Amount = string | bigint | number;

/**
 * Amounts cross the native boundary as exact decimal strings. Non-integer
 * numbers are rejected: they cannot represent money exactly.
 */
export function toAmount(amount: Amount): string {
  if (typeof amount === "string") return amount;
  if (typeof amount === "bigint") return amount.toString();
  if (typeof amount === "number" && Number.isSafeInteger(amount)) return String(amount);
  throw new TypeError(
    `amount must be a decimal string (e.g. "10.50"), a bigint or a safe integer, not ${amount} ` +
      "(floats are rejected to avoid rounding errors)",
  );
}

/** Seconds from a number of seconds. */
export function toSeconds(value: number): number {
  if (!Number.isInteger(value)) throw new TypeError("seconds must be an integer");
  return value;
}

/** `yyyy-mm-dd` from a Date (UTC parts) or a string. */
export function toIsoDate(value: Date | string): string {
  return typeof value === "string" ? value : value.toISOString().slice(0, 10);
}
