/**
 * The Rust core speaks snake_case JSON; TypeScript users write camelCase.
 * Conversion is generic and deep, except for fields that carry third-party
 * keys verbatim (HTTP headers in, raw ATC payloads out).
 */

/** Values under these (converted) keys are copied as-is. */
const VERBATIM_IN = new Set(["headers"]);
const VERBATIM_OUT = new Set(["payload", "ack", "reversal"]);

export function camelToSnake(key: string): string {
  return key.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);
}

export function snakeToCamel(key: string): string {
  return key.replace(/_([a-z0-9])/g, (_, c: string) => c.toUpperCase());
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function convert(value: unknown, rename: (k: string) => string, verbatim: Set<string>): unknown {
  if (Array.isArray(value)) return value.map((v) => convert(v, rename, verbatim));
  if (!isPlainObject(value)) return value;
  const out: Record<string, unknown> = {};
  for (const [key, inner] of Object.entries(value)) {
    if (inner === undefined) continue;
    const renamed = rename(key);
    out[renamed] = verbatim.has(renamed) ? inner : convert(inner, rename, verbatim);
  }
  return out;
}

/** camelCase input → snake_case for the core. Drops `undefined` fields. */
export function toSnake(value: unknown): unknown {
  return convert(value, camelToSnake, VERBATIM_IN);
}

/** snake_case core output → camelCase for TypeScript. */
export function toCamel<T>(value: unknown): T {
  return convert(value, snakeToCamel, VERBATIM_OUT) as T;
}
