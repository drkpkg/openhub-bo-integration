/**
 * In-memory fake of the OpenHub gateway (Sensedia) for tests: OAuth token and
 * the `access_token`/`client_id` check like the real gateway, plus a route
 * table for product endpoints. Use it as the session's transport.
 */
import { Session, type SessionOptions } from "./session.js";
import type { HttpRequest, HttpResponse, Transport } from "./transport.js";

export type RouteHandler = (request: HttpRequest, match: RegExpMatchArray) => HttpResponse;

export interface MockGatewayOptions {
  clientId?: string;
  clientSecret?: string;
  tokenTtl?: number;
}

const TOKEN_PATH = "/oauth-client-credentials/access-token";

export class MockGateway implements Transport {
  static readonly BASE_URL = "https://openhub.mock";

  readonly clientId: string;
  readonly clientSecret: string;
  tokenTtl: number;
  tokenRequests = 0;
  readonly requests: HttpRequest[] = [];
  private readonly tokens = new Set<string>();
  private readonly routes: { method: string; pattern: RegExp; handler: RouteHandler }[] = [];
  private nextToken = 1;

  constructor(options: MockGatewayOptions = {}) {
    this.clientId = options.clientId ?? "test-client";
    this.clientSecret = options.clientSecret ?? "test-secret";
    this.tokenTtl = options.tokenTtl ?? 3600;
  }

  /** Registers `handler` for authenticated calls whose path matches `pattern`. */
  route(method: string, pattern: RegExp, handler: RouteHandler): this {
    this.routes.push({ method: method.toUpperCase(), pattern, handler });
    return this;
  }

  /** Answers every call matching `pattern` with a fixed status and body. */
  respond(method: string, pattern: RegExp, status: number, body: string): this {
    return this.route(method, pattern, () => ({ status, body }));
  }

  session(options: Partial<SessionOptions> = {}): Session {
    return new Session({
      clientId: this.clientId,
      clientSecret: this.clientSecret,
      baseUrl: MockGateway.BASE_URL,
      transport: this,
      ...options,
    });
  }

  /** Invalidates every issued token, as if they had expired server-side. */
  revokeTokens(): void {
    this.tokens.clear();
  }

  async send(request: HttpRequest): Promise<HttpResponse> {
    this.requests.push(request);
    const url = new URL(request.url);
    if (url.pathname.endsWith(TOKEN_PATH)) return this.token(request);
    if (!this.tokens.has(request.headers["access_token"] ?? "") || request.headers["client_id"] !== this.clientId) {
      return {
        status: 401,
        body: "Access Token in the request, identified by HEADER access_token, is invalid.",
      };
    }
    for (const route of this.routes) {
      const match = url.pathname.match(route.pattern);
      if (route.method === request.method && match) return route.handler(request, match);
    }
    return { status: 404, body: JSON.stringify({ success: false, message: "Not found", errors: [] }) };
  }

  private token(request: HttpRequest): HttpResponse {
    this.tokenRequests++;
    const expected = Buffer.from(`${this.clientId}:${this.clientSecret}`).toString("base64");
    if (request.headers["Authorization"] !== `Basic ${expected}`) {
      return {
        status: 401,
        body: JSON.stringify({ error: "invalid_client", error_description: "Bad credentials" }),
      };
    }
    const token = `token-${this.nextToken++}`;
    this.tokens.add(token);
    // Sandbox answers 201 with token_type "access_token" and no scope.
    return {
      status: 201,
      body: JSON.stringify({ access_token: token, token_type: "access_token", expires_in: this.tokenTtl }),
    };
  }
}
