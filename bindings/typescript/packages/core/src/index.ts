/**
 * Shared foundation of the `@openhub-bo/*` packages for the Red Enlace (ATC)
 * OpenHub APIs: session, transport, errors and common types.
 *
 * Server-side only: the client secret must never reach a browser.
 */
import { CORE_NATIVE } from "./core-native.js";

export { camelToSnake, snakeToCamel, toCamel, toSnake } from "./casing.js";
export { CORE_NATIVE } from "./core-native.js";
export {
  AmbiguousOutcomeError,
  ApiError,
  type ApiFieldError,
  AuthenticationError,
  CoreError,
  DecodeError,
  errorFromCore,
  OpenHubError,
  TransportError,
  ValidationError,
  WebhookAuthError,
} from "./errors.js";
export {
  type Amount,
  type Environment,
  isFinal,
  type PaymentStatus,
  toAmount,
  toIsoDate,
  toSeconds,
  Webhook,
} from "./models.js";
export { Native, type NativeModule, type OperationInfo, SUPPORTED_PROTOCOL } from "./native.js";
export { type AccessToken, type Config, Handler, Op } from "./ops.js";
export { Session, type SessionOptions } from "./session.js";
export {
  FetchTransport,
  type FetchTransportOptions,
  type HttpRequest,
  type HttpResponse,
  type Transport,
} from "./transport.js";

export const version: string = CORE_NATIVE.version;
