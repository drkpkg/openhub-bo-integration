import { CORE_NATIVE } from "./core-native.js";
import { AmbiguousOutcomeError, AuthenticationError, TransportError } from "./errors.js";
import type { Environment } from "./models.js";
import type { AccessToken, Config, Op } from "./ops.js";
import { FetchTransport, type HttpRequest, type HttpResponse, type Transport } from "./transport.js";

export interface SessionOptions {
  clientId: string;
  clientSecret?: string;
  /** Ready-made "Token Basic" provided by ATC, instead of `clientSecret`. */
  basicToken?: string;
  /** Defaults to `sandbox`. */
  environment?: Environment;
  /** Overrides the environment URL (mock servers, proxies). */
  baseUrl?: string;
  /** Default timeout in seconds for the built-in transport. */
  timeout?: number;
  transport?: Transport;
}

/**
 * Credentials, token cache and execution of operations. Share one session
 * between product clients (`new QrClient(session)`, `new PixClient(session)`)
 * so they reuse one OAuth token. Concurrent calls fetch the token only once.
 */
export class Session {
  readonly environment: Environment;
  private readonly config: Config;
  private readonly transport: Transport;
  private token: AccessToken | null = null;
  private pendingToken: Promise<AccessToken> | null = null;

  constructor(options: SessionOptions) {
    this.environment = options.environment ?? "sandbox";
    this.config = {
      client_id: options.clientId,
      client_secret: options.clientSecret ?? null,
      basic_token: options.basicToken ?? null,
      environment: this.environment,
      base_url: options.baseUrl ?? null,
    };
    this.transport =
      options.transport ?? new FetchTransport(options.timeout === undefined ? {} : { timeout: options.timeout });
    // Fail fast on missing credentials instead of at the first request.
    this.tokenRequest();
  }

  /** Runs `op`: token, build, send, parse; refreshes a rejected token once. */
  async execute<T>(op: Op<T>, input: unknown): Promise<T> {
    for (let attempt = 0; ; attempt++) {
      const token = await this.ensureToken();
      const response = await this.send(op.build(this.config, token, input), op.name, op.idempotent);
      try {
        return op.parse(input, response);
      } catch (error) {
        if (attempt === 0 && error instanceof AuthenticationError) {
          this.invalidate(token);
          continue;
        }
        throw error;
      }
    }
  }

  private async ensureToken(): Promise<AccessToken> {
    if (this.token && Date.now() / 1000 < this.token.expires_at) return this.token;
    this.pendingToken ??= this.fetchToken().finally(() => {
      this.pendingToken = null;
    });
    return this.pendingToken;
  }

  private async fetchToken(): Promise<AccessToken> {
    const response = await this.send(this.tokenRequest(), "token", true);
    this.token = CORE_NATIVE.invoke("token.parse", { input: { now: nowSecs() }, response }) as AccessToken;
    return this.token;
  }

  private tokenRequest(): HttpRequest {
    return CORE_NATIVE.invoke("token.build", {
      config: this.config,
      input: { now: nowSecs() },
    }) as HttpRequest;
  }

  /** Drops `token` unless another caller already refreshed it. */
  private invalidate(token: AccessToken): void {
    if (this.token?.access_token === token.access_token) this.token = null;
  }

  private async send(request: HttpRequest, operation: string, idempotent: boolean): Promise<HttpResponse> {
    try {
      return await this.transport.send(request);
    } catch (error) {
      if (error instanceof TransportError && error.maybeSent && !idempotent) {
        throw new AmbiguousOutcomeError(operation, error);
      }
      throw error;
    }
  }
}

function nowSecs(): number {
  return Math.floor(Date.now() / 1000);
}
