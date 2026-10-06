/** Errors raised by openhub-bo packages. All extend {@link OpenHubError}. */

export interface ApiFieldError {
  message: string;
  field?: string | null;
  code?: string | null;
}

export class OpenHubError extends Error {
  /** Whether repeating the same call later may succeed. */
  retryable = false;

  constructor(message: string, options?: { cause?: unknown }) {
    super(message, options);
    this.name = new.target.name;
  }
}

/** Input rejected locally, before calling OpenHub. */
export class ValidationError extends OpenHubError {
  readonly field: string;

  constructor(message: string, field: string) {
    super(`invalid \`${field}\`: ${message}`);
    this.field = field;
  }
}

/** Credentials or access token rejected by OpenHub. */
export class AuthenticationError extends OpenHubError {
  readonly status: number;

  constructor(message: string, status: number) {
    super(`authentication failed (HTTP ${status}): ${message}`);
    this.status = status;
  }
}

/** OpenHub answered with an error. */
export class ApiError extends OpenHubError {
  readonly status: number;
  readonly errors: ApiFieldError[];

  constructor(message: string, status: number, errors: ApiFieldError[], retryable: boolean) {
    super(`OpenHub error (HTTP ${status}): ${message}`);
    this.status = status;
    this.errors = errors;
    this.retryable = retryable;
  }

  /** Code of the first field error, e.g. `TRANSACCION_NO_ENCONTRADA`. */
  get code(): string | undefined {
    return this.errors.find((e) => e.code)?.code ?? undefined;
  }
}

/** The response could not be understood. */
export class DecodeError extends OpenHubError {}

/** Incoming webhook did not carry the expected secret. */
export class WebhookAuthError extends OpenHubError {}

/**
 * Network failure. `maybeSent` is false when the request provably never
 * left (connection refused, DNS); retrying is then always safe.
 */
export class TransportError extends OpenHubError {
  override retryable = true;
  readonly maybeSent: boolean;

  constructor(message: string, maybeSent: boolean, options?: { cause?: unknown }) {
    super(message, options);
    this.maybeSent = maybeSent;
  }
}

/**
 * A non-idempotent call (generating a QR, paying a payout...) may or may not
 * have been applied: the response was lost, or OpenHub reported the result as
 * unconfirmed (payout codes 94/96). Not a {@link TransportError} on purpose,
 * so generic retry loops do not repeat it blindly. Query the status first.
 */
export class AmbiguousOutcomeError extends OpenHubError {
  readonly operation: string;
  readonly errors: ApiFieldError[];

  constructor(operation: string, cause: unknown, errors: ApiFieldError[] = []) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    super(`outcome of \`${operation}\` is unknown: ${detail}`, { cause });
    this.operation = operation;
    this.errors = errors;
  }

  get code(): string | undefined {
    return this.errors.find((e) => e.code)?.code ?? undefined;
  }
}

/** Contract violation between a package and its native module (a bug). */
export class CoreError extends OpenHubError {}

interface CoreErrorData {
  kind?: string;
  message?: string;
  field?: string;
  status?: number;
  errors?: ApiFieldError[];
  retryable?: boolean;
}

/** Maps a core error; `op` is the native op name, e.g. `payouts.pay.parse`. */
export function errorFromCore(data: CoreErrorData, op = ""): OpenHubError {
  const message = data.message ?? "unknown error";
  const errors = data.errors ?? [];
  switch (data.kind) {
    case "validation":
      return new ValidationError(message, data.field ?? "");
    case "authentication":
      return new AuthenticationError(message, data.status ?? 0);
    case "api":
      return new ApiError(message, data.status ?? 0, errors, Boolean(data.retryable));
    case "ambiguous":
      return new AmbiguousOutcomeError(op.replace(/\.(build|parse)$/, ""), message, errors);
    case "decode":
      return new DecodeError(message);
    case "webhook_auth":
      return new WebhookAuthError(message);
    default:
      return new CoreError(message);
  }
}
