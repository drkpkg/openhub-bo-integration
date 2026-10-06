# openhub-bo-core

Base compartida de los clientes `openhub-bo` para las APIs de **Red Enlace (ATC) OpenHub**
(Bolivia): sesión OAuth2 con caché de token, transporte HTTP intercambiable, errores
tipados y tipos comunes. Normalmente no se instala sola: viene con cada producto.

```sh
pip install openhub-bo[qr]     # core + QR Simple / MLD-BCB
```

```python
from openhub_bo.core import Session, Environment

with Session("CLIENT_ID", "CLIENT_SECRET", environment=Environment.SANDBOX) as session:
    ...  # pasa la sesión a QrClient(session), etc. Todos comparten el token.
```

- `Session` / `AsyncSession`: credenciales, token (se renueva solo; reintenta una vez si
  OpenHub lo rechaza) y ejecución de operaciones. Thread-safe.
- `Transport` / `AsyncTransport`: inyecta el tuyo para proxies, logging o fakes
  (por defecto httpx).
- Errores: `OpenHubError` → `ValidationError`, `AuthenticationError`, `ApiError`
  (`.status`, `.code`, `.retryable`), `TransportError` (`.maybe_sent`),
  `AmbiguousOutcomeError` (una operación no idempotente pudo haberse aplicado),
  `DecodeError`, `WebhookAuthError`.
- `openhub_bo.core.testing.MockGateway`: gateway falso (token + headers) para tests.
