import { toCamel, toSnake } from "./casing.js";
import type { Native } from "./native.js";
import type { HttpRequest, HttpResponse } from "./transport.js";

export type Config = Record<string, unknown>;
export type AccessToken = Record<string, unknown> & { access_token: string; expires_at: number };

/**
 * An endpoint implemented by `native`. Declaring an Op throws at import time if
 * the native module does not know `name`, so packages cannot drift from it.
 */
export class Op<T> {
  constructor(
    readonly native: Native,
    readonly name: string,
  ) {
    const info = native.operations.get(name);
    if (info?.kind !== "operation") {
      throw new Error(`${native.packageName}: native module has no operation \`${name}\``);
    }
  }

  get idempotent(): boolean {
    return Boolean(this.native.operations.get(this.name)?.idempotent);
  }

  /** Timeout in seconds recommended by ATC for this endpoint, if any. */
  get timeoutSecs(): number | undefined {
    return this.native.operations.get(this.name)?.timeout_secs;
  }

  build(config: Config, token: AccessToken | null, input: unknown): HttpRequest {
    const request = this.native.invoke(`${this.name}.build`, {
      config,
      token,
      input: toSnake(input),
    }) as HttpRequest;
    if (this.timeoutSecs !== undefined) request.timeout = this.timeoutSecs;
    return request;
  }

  parse(input: unknown, response: HttpResponse): T {
    return toCamel<T>(
      this.native.invoke(`${this.name}.parse`, { input: toSnake(input), response }),
    );
  }
}

/** A pure native function (no HTTP), e.g. webhook parsing. */
export class Handler<T> {
  constructor(
    readonly native: Native,
    readonly name: string,
  ) {
    const info = native.operations.get(name);
    if (info?.kind !== "handler") {
      throw new Error(`${native.packageName}: native module has no handler \`${name}\``);
    }
  }

  call(input: unknown): T {
    return toCamel<T>(this.native.invoke(this.name, toSnake(input)));
  }
}
