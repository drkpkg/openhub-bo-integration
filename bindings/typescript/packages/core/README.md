# @openhub-bo/core

Base compartida de los clientes `@openhub-bo/*` para las APIs de **Red Enlace (ATC)
OpenHub**, Bolivia: sesión OAuth2 con caché de token, transporte HTTP intercambiable,
errores tipados y tipos comunes. Núcleo en Rust compilado a WebAssembly.

```sh
npm install @openhub-bo/core @openhub-bo/qr
```

```ts
import { Session } from "@openhub-bo/core";

const session = new Session({ clientId: "CLIENT_ID", clientSecret: "CLIENT_SECRET", environment: "sandbox" });
// Pásala a QrClient, PixClient, AccountsClient, PayoutsClient...: comparten el token.
```

- **Solo servidor** (Node ≥ 18, Bun, Deno con compatibilidad Node): el `clientSecret`
  nunca debe llegar a un navegador.
- **Montos** como string decimal (`"10.50"`) o entero; los `number` con decimales se
  rechazan para no perder precisión.
- **Errores:** `OpenHubError` → `ValidationError` (`.field`), `AuthenticationError`,
  `ApiError` (`.status`, `.code`, `.retryable`), `TransportError` (`.maybeSent`),
  `AmbiguousOutcomeError` (una operación no idempotente pudo aplicarse), `DecodeError`,
  `WebhookAuthError`.
- **Transporte:** `FetchTransport` por defecto (aplica los timeouts que recomienda ATC por
  endpoint); inyecta el tuyo con `transport`.
- **Tests:** `import { MockGateway } from "@openhub-bo/core/testing"` (token + rutas).
