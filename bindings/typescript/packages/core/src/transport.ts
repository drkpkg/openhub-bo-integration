import { TransportError } from "./errors.js";

/** A request built by the core, plus an optional per-endpoint timeout. */
export interface HttpRequest {
  method: "GET" | "POST" | "PATCH";
  url: string;
  headers: Record<string, string>;
  body: string | null;
  /** Seconds, when ATC recommends a timeout for this endpoint. */
  timeout?: number;
}

export interface HttpResponse {
  status: number;
  body: string;
}

/** How requests reach OpenHub. Swap it for proxies, logging or fakes. */
export interface Transport {
  send(request: HttpRequest): Promise<HttpResponse>;
}

export interface FetchTransportOptions {
  /** Default timeout in seconds when the endpoint has none. */
  timeout?: number;
  fetch?: typeof globalThis.fetch;
}

/** Errors where the request provably never reached the server. */
const NOT_SENT = new Set([
  "ECONNREFUSED",
  "ENOTFOUND",
  "EAI_AGAIN",
  "ENETUNREACH",
  "EHOSTUNREACH",
  "UND_ERR_CONNECT_TIMEOUT",
]);

function causeCode(error: unknown): string | undefined {
  let current: unknown = error;
  for (let depth = 0; current && depth < 5; depth++) {
    const code = (current as { code?: unknown }).code;
    if (typeof code === "string") return code;
    current = (current as { cause?: unknown }).cause;
  }
  return undefined;
}

/** Transport over the global `fetch` (Node >= 18). */
export class FetchTransport implements Transport {
  private readonly timeout: number;
  private readonly fetchImpl: typeof globalThis.fetch;

  constructor(options: FetchTransportOptions = {}) {
    this.timeout = options.timeout ?? 30;
    this.fetchImpl = options.fetch ?? globalThis.fetch;
  }

  async send(request: HttpRequest): Promise<HttpResponse> {
    const seconds = request.timeout ?? this.timeout;
    try {
      const response = await this.fetchImpl(request.url, {
        method: request.method,
        headers: request.headers,
        body: request.body ?? undefined,
        signal: AbortSignal.timeout(seconds * 1000),
      });
      return { status: response.status, body: await response.text() };
    } catch (error) {
      const code = causeCode(error);
      const maybeSent = !(code && NOT_SENT.has(code));
      const detail = error instanceof Error ? `${error.name}: ${error.message}` : String(error);
      throw new TransportError(code ? `${detail} (${code})` : detail, maybeSent, { cause: error });
    }
  }
}
