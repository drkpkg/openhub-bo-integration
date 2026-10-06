/**
 * Merchant accounts at Red Enlace (ATC) OpenHub: lookup, creation, status,
 * balances and movements for reconciliation. Dates accept `Date` (UTC day) or
 * `"yyyy-mm-dd"`. Sandbox limits: reconcile ≤ 7 days, credits/debits ≤ 31 days,
 * balances ≤ 10 accounts.
 */
import { Op, type PaymentStatus, type Session, toIsoDate } from "@openhub-bo/core";

import { NATIVE } from "./native.js";

export type AccountStatus = "active" | "blocked" | "suspended" | "closed" | "unknown";
type DateLike = Date | string;

export interface Account {
  number: string;
  alias: string | null;
  status: AccountStatus;
  rawStatus: string | null;
}

export interface MerchantAccount {
  nit: string;
  merchantName: string | null;
  establishmentId: number | null;
  establishmentName: string | null;
  account: Account;
}

export interface Establishment {
  id: number;
  name: string | null;
  accounts: Account[];
}

export interface MerchantAccounts {
  nit: string;
  merchantName: string | null;
  establishments: Establishment[];
}

export interface CreatedAccounts {
  nit: string;
  merchantName: string | null;
  establishmentId: number;
  establishmentName: string | null;
  accounts: Account[];
}

export interface StatusChanged {
  accountNumber: string;
  status: AccountStatus;
  rawStatus: string;
}

export interface Party {
  account: string | null;
  documentId: string | null;
  name: string | null;
  bankCode: string | null;
  bankName: string | null;
}

export interface Movement {
  /** Payouts only: the id you sent when authorising it. */
  transactionId: string | null;
  /** `PAYIN QR`, `PAYIN ACH`, `PAYOUT ACH`... */
  operationType: string;
  status: PaymentStatus;
  rawStatus: string;
  message: string | null;
  transactionAt: string | null;
  amount: string;
  fee: string | null;
  total: string | null;
  currency: string | null;
  origin: Party;
  destination: Party;
  reference: string | null;
  achOrderNumber: string | null;
  recipientOrderNumber: string | null;
}

export interface Balance {
  accountNumber: string;
  status: AccountStatus;
  rawStatus: string | null;
  currency: string | null;
  available: string;
  /** `saldoContable`. */
  booked: string;
  /** `saldoRetenido`. */
  held: string;
  lastCreditAt: string | null;
  lastDebitAt: string | null;
}

export interface Reconciliation {
  movements: Movement[];
  balances: Balance[];
}

export interface NewAccount {
  /** Max 45 characters. */
  alias: string;
  /** `rubro`, max 45 characters. */
  category: string;
}

export interface StatusChange {
  accountNumber: string;
  status: Exclude<AccountStatus, "unknown">;
  /** Max 50 characters. */
  reason: string;
}

export interface DateRange {
  dateFrom: DateLike;
  dateTo: DateLike;
}

const GET = new Op<MerchantAccount>(NATIVE, "accounts.get");
const LIST = new Op<MerchantAccounts>(NATIVE, "accounts.list");
const CREATE = new Op<CreatedAccounts>(NATIVE, "accounts.create");
const SET_STATUS = new Op<StatusChanged[]>(NATIVE, "accounts.set_status");
const RECONCILE = new Op<Reconciliation>(NATIVE, "accounts.reconcile");
const CREDITS = new Op<Movement[]>(NATIVE, "accounts.credits");
const DEBITS = new Op<Movement[]>(NATIVE, "accounts.debits");
const BALANCES = new Op<Balance[]>(NATIVE, "accounts.balances");

function range(range: DateRange) {
  return { dateFrom: toIsoDate(range.dateFrom), dateTo: toIsoDate(range.dateTo) };
}

export class AccountsClient {
  constructor(readonly session: Session) {}

  async getAccount(nit: string, accountNumber: string): Promise<MerchantAccount> {
    return this.session.execute(GET, { nit, accountNumber });
  }

  async listAccounts(nit: string): Promise<MerchantAccounts> {
    return this.session.execute(LIST, { nit });
  }

  /**
   * Creates 1..1000 accounts (BOB). Not idempotent: a lost response raises
   * `AmbiguousOutcomeError`; check `listAccounts` before retrying.
   */
  async createAccounts(nit: string, establishmentId: number, accounts: NewAccount[]): Promise<CreatedAccounts> {
    return this.session.execute(CREATE, { nit, establishmentId, accounts });
  }

  /** Changing to `closed` requires zero balance and cannot be undone. */
  async setAccountStatus(nit: string, changes: StatusChange[]): Promise<StatusChanged[]> {
    return this.session.execute(SET_STATUS, { nit, changes });
  }

  /** Movements and balances of every account of the merchant (≤ 7 days). */
  async reconcile(nit: string, dates: DateRange): Promise<Reconciliation> {
    return this.session.execute(RECONCILE, { nit, ...range(dates) });
  }

  /** Incoming movements of one account (≤ 31 days). */
  async credits(nit: string, accountNumber: string, dates: DateRange): Promise<Movement[]> {
    return this.session.execute(CREDITS, { nit, accountNumber, ...range(dates) });
  }

  /** Outgoing movements of one account (≤ 31 days). */
  async debits(nit: string, accountNumber: string, dates: DateRange): Promise<Movement[]> {
    return this.session.execute(DEBITS, { nit, accountNumber, ...range(dates) });
  }

  /** Current balances of up to 10 accounts. */
  async balances(nit: string, accountNumbers: string[]): Promise<Balance[]> {
    return this.session.execute(BALANCES, { nit, accountNumbers });
  }
}

export const version: string = NATIVE.version;
