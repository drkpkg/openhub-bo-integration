import { errorFromCore } from "./errors.js";

/** Core FFI protocol this package speaks (see `openhub_core::ffi`). */
export const SUPPORTED_PROTOCOL = 2;

/** What every package's WASM module exports. */
export interface NativeModule {
  call(op: string, payload: string): string;
  protocolVersion(): number;
  version(): string;
}

export interface OperationInfo {
  kind: "operation" | "handler";
  name: string;
  idempotent?: boolean;
  timeout_secs?: number;
}

type Envelope = { ok: true; value: unknown } | { ok: false; error: Record<string, unknown> };

/**
 * A package's native module: checks the protocol on load and caches the
 * operation catalog (`describe`) so declared operations are validated at import.
 */
export class Native {
  readonly version: string;
  readonly operations: ReadonlyMap<string, OperationInfo>;

  constructor(
    private readonly module: NativeModule,
    readonly packageName: string,
  ) {
    const protocol = module.protocolVersion();
    if (protocol !== SUPPORTED_PROTOCOL) {
      throw new Error(
        `${packageName} native module speaks core protocol ${protocol}, ` +
          `expected ${SUPPORTED_PROTOCOL}; install matching versions`,
      );
    }
    this.version = module.version();
    const catalog = this.invoke("describe", {}) as OperationInfo[];
    this.operations = new Map(catalog.map((info) => [info.name, info]));
  }

  /** Calls the core with a snake_case payload; returns the snake_case value. */
  invoke(op: string, payload: unknown): unknown {
    const result = JSON.parse(this.module.call(op, JSON.stringify(payload))) as Envelope;
    if (result.ok) return result.value;
    throw errorFromCore(result.error, op);
  }
}
