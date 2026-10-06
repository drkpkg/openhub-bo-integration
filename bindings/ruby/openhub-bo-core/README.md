# openhub-bo-core

Base compartida de las gemas `openhub-bo` para **Red Enlace (ATC) OpenHub**, Bolivia:
sesión OAuth2 con caché de token (thread-safe), transporte HTTP (`Net::HTTP`, sin
dependencias), errores tipados y tipos comunes. Normalmente llega como dependencia
de cada producto.

```ruby
require "openhub_bo/core"

session = OpenhubBo::Core::Session.new(ENV["CLIENT_ID"], ENV["CLIENT_SECRET"], environment: :sandbox)
# pásala a OpenhubBo::Qr::QrClient.new(session), etc. Todos comparten el token.
```

- Errores: `Core::Error` → `ValidationError#field`, `AuthenticationError#status`,
  `ApiError#status/#code/#errors/#retryable?`, `TransportError#maybe_sent`,
  `AmbiguousOutcomeError#operation/#code` (una operación no idempotente pudo
  aplicarse), `DecodeError`, `WebhookAuthError`.
- Montos: entradas `BigDecimal`, `Integer` o `String` (`Float` → `TypeError`); salidas `BigDecimal`.
- Transporte propio: cualquier objeto con `perform(request) -> Core::Response`.
- Tests: `require "openhub_bo/core/testing"` → `Core::Testing::MockGateway`.
