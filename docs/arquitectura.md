# Arquitectura

## Visión general

```
 Rust (sin I/O)                                    Python (namespace openhub_bo)
 ┌──────────────────────────────┐                 ┌────────────────────────────────────┐
 │ openhub-core                 │  CoreOps ──────▶│ openhub-bo-core   openhub_bo.core   │
 │  Operation / Handler         │                 │  Session · AsyncSession · Transport │
 │  registry! · Chain           │                 │  Op · errores · MockGateway         │
 │  sobres · estados · token    │                 └───────────────▲────────────────────┘
 └──────────────▲───────────────┘                                 │ Session compartida
                │                                 ┌───────────────┴────────────────────┐
 ┌──────────────┴───────────────┐  QrOps ────────▶│ openhub-bo-qr     openhub_bo.qr     │
 │ openhub-qr                   │                 │  QrClient · AsyncQrClient           │
 │  Generate · Verify · Cancel  │                 │  parse_webhook · MockOpenHub        │
 │  ParseWebhook                │                 └────────────────────────────────────┘
 └──────────────────────────────┘                 openhub-bo = metapaquete [qr] [all]
```

## Núcleo Rust

- **Sin I/O.** Cada endpoint es un `Operation` con `request(ctx, input) -> HttpRequest`
  y `response(resp, input) -> Output`. El host ejecuta el HTTP.
- **`registry!`** genera un `Registry` (despacho FFI + `describe`). `Chain(a, b)` combina
  registros; cada extensión nativa expone solo el suyo.
- **Sobres por familia** (`envelope::Envelope`): `SuccessData` (QR), `CodigoRespuesta`
  (PIX/cripto/Binance; reconoce además `{error: true, code: EG-…}`, `{success: false}` y
  `codigoRespuesta: ERROR` con el código `GQ-…` extraído del detalle), `CodeData`
  (pagos/cuentas). Un 401/403 siempre es `Authentication`.
- **Estados comunes** (`PaymentStatus`) con un vocabulario por familia (`SpanishQr`,
  `EnglishCodes`, `Payout`); `raw_status` conserva el original.
- **Errores** con `kind` y `retryable`. `Error::api` lo deduce del HTTP (5xx, 408, 429);
  `Error::business` lo fija en `false` para rechazos estructurados del backend, que en
  FX llegan incluso con HTTP 500 (`EG-00001` "Transacción no encontrada").
- **Timeouts por operación** (`Operation::TIMEOUT_SECS`, expuesto en `describe`): los que
  recomienda ATC (p. ej. 40 s leer QR, 90 s pagar). Los bindings los aplican por llamada.
- **Resultado no confirmado** (`Error::Ambiguous`, kind `ambiguous`): pagos con código
  94/96. En Python es `AmbiguousOutcomeError`, igual que una respuesta perdida.
- **Montos** exactos: `Decimal`, cruzan la frontera como string y se escriben al JSON
  sin pasar por float (`serde_json/arbitrary_precision`).
- Cada familia separa `model` (API pública en inglés) de `wire` (JSON en español de ATC).

### Protocolo FFI (`PROTOCOL_VERSION = 2`)

| op | payload | resultado |
|---|---|---|
| `<op>.build` | `{config, token?, input}` | HttpRequest |
| `<op>.parse` | `{input, response}` | salida de la operación |
| `<handler>` | input del handler | salida |
| `describe` | `{}` | `[{kind, name, idempotent?, timeout_secs?}]` |
| `version` | `{}` | `{core, protocol}` |

Respuesta: `{"ok": true, "value"}` o `{"ok": false, "error": {kind, ..., retryable}}`.

Operaciones actuales:
- core: `token`
- qr: `qr.generate`, `qr.verify`, `qr.cancel`, `qr.webhook.parse`
- fx: `pix.generate`, `pix.verify`, `pix.cancel`, `crypto.generate`, `crypto.verify`,
  `binance.generate`, `binance.verify`, `binance.webhook.parse`, `fx.webhook.parse`
- accounts: `accounts.get`, `accounts.list`, `accounts.create`, `accounts.set_status`
  (PATCH), `accounts.reconcile`, `accounts.credits`, `accounts.debits`, `accounts.balances`
- payouts: `payouts.scan`, `payouts.pay`, `payouts.status`, `batch.authorize`,
  `batch.status`, `batch.banks`, `batch.webhook.parse`

## Python

- **`Session` / `AsyncSession`** (core): credenciales, caché del token (lock por sesión,
  creado de forma perezosa en async), `execute(op, input)` = token → build → send →
  parse, con un reintento si OpenHub rechaza el token.
- **`Op`** declara un endpoint contra el módulo nativo; falla al importar si el nativo no lo
  conoce. La idempotencia sale de `describe`.
- **`Transport`**: inyectable. `HttpxTransport` marca `maybe_sent`; si una operación no
  idempotente pierde la respuesta, la sesión lanza `AmbiguousOutcomeError` (no es un
  `TransportError`, para que los reintentos genéricos no la repitan).
- **Clientes de producto** (`QrClient(session)`): solo arman el input y llaman a
  `session.execute`. Varios clientes comparten la sesión y el token.

## Paquetes y versiones

- Namespace `openhub_bo` (sin `__init__.py`); cada distribución aporta un subpaquete con su
  propia extensión `abi3` (una wheel por plataforma).
- **Versiones sincronizadas**: todas las distribuciones salen con el mismo número;
  los productos dependen de `openhub-bo-core>=X.Y,<X.(Y+1)` y cada módulo nativo verifica
  `PROTOCOL_VERSION` al importar.
- `openhub-bo` es un metapaquete: `pip install openhub-bo[qr]`.

## Agregar un producto (ej. pagos)

`openhub-fx` / `openhub-bo-fx` es el ejemplo completo a seguir.

1. `crates/openhub-<x>`: `model`, `wire`, `validate`, operaciones con su sobre y
   vocabulario de estados; `registry!(pub XOps {...})`; tests con fixtures (doc + sandbox).
2. `bindings/python/<x>`: `Cargo.toml` (cdylib que expone `XOps`, nombre de lib único),
   `pyproject.toml` (`module-name = "openhub_bo.<x>._native"`), `_ops.py` con los `Op`,
   clientes `XClient(session)`, mock sobre `MockGateway`.
3. Miembro en ambos workspaces (Cargo y uv), extra en `meta/pyproject.toml`, rutas en CI.

## TypeScript (WebAssembly)

- `bindings/typescript` es un workspace de Cargo propio (excluido del raíz) con un crate
  `cdylib` por paquete en `native/<pkg>` que expone `call`, `protocolVersion` y `version`
  con `wasm-bindgen` (target `nodejs`, CommonJS dentro de `wasm/`). Cada paquete npm carga
  solo su registro (`CoreOps`, `QrOps`, ...), igual que las extensiones de Python.
- `@openhub-bo/core` replica el núcleo de Python: `Native` (verifica el protocolo y lee
  `describe`), `Op`/`Handler` declarados y validados al importar, `Session` asíncrona
  (token compartido por llamadas concurrentes, reintento único ante 401,
  `AmbiguousOutcomeError` si una operación no idempotente pierde la respuesta),
  `FetchTransport` con el timeout de cada endpoint, errores tipados y `MockGateway`.
- La conversión camelCase ↔ snake_case es genérica en el puente; se respetan tal cual los
  `headers` de entrada y los JSON crudos de ATC (`payload`, `ack`, `reversal`).
- Montos: string decimal (JS no tiene decimal exacto); `number` solo si es entero seguro.
- Solo servidor: el `clientSecret` no debe llegar al navegador, por eso el target es Node.

## Ruby (gemas nativas)

- Una gema por producto (`openhub-bo-{core,qr,fx,accounts,payouts}` + metagema `openhub-bo`),
  cada una con su extensión magnus/rb_sys (`OpenhubBo::<Pkg>::Native.call(op, json)`)
  registrando solo sus operaciones; namespaces `OpenhubBo::Core`, `OpenhubBo::Qr`, ...
- **Crates vendorizados:** `rake vendor` copia los crates de `crates/` a
  `ext/<gema>/vendor/` (resolviendo `*.workspace = true`); la extensión depende de esa
  copia tanto en desarrollo como dentro de la `.gem`, que se compila al instalar sin
  acceso al repo. Cada extensión es su propio workspace Cargo.
- `Core::Session` (Mutex), `Core::Op` validado contra `describe` al hacer `require`,
  `NetHttpTransport` (stdlib, sin dependencias; timeout por operación; `maybe_sent`),
  modelos inmutables `Data.define` construidos desde el JSON del núcleo, montos `BigDecimal`
  (`Float` → `TypeError`), estados como símbolos.
- Tests con minitest + `Core::Testing::MockGateway` y los fixtures compartidos.

## Contrato de OpenHub: confirmado y pendiente

Documentación revisada el 2026-10-06 ([openhub-productos.md](openhub-productos.md)) y
**verificada contra el sandbox real** ese mismo día (`bindings/python/examples/sandbox_smoke.py`).

Verificado en el sandbox:
- **Token:** `Authorization: Basic base64(client_id:client_secret)`. Responde **HTTP 201** con
  `{access_token, token_type: "access_token", expires_in: 3600}`, sin `scope`.
  Funciona igual en desarrollo y en sandbox.
- **El token va en el header `access_token`.** Si solo se manda `Authorization: Bearer`, el
  gateway responde 401: "Access Token in the request, identified by HEADER access_token,
  is invalid". La librería envía ambos headers.
- **`numeroReferencia` solo admite dígitos** (`INVALID_FORMAT`) y **el webhook es obligatorio**
  (`REQUIRED_FIELD`). Las dos reglas se validan localmente.
- **`vigencia` va en segundos y `fechaExpiracion` está en hora de Bolivia (UTC-4).**
- **Cancelar** (`POST /qr/simple/v2/cancel/{ref}`) responde con el sobre y un `data` igual al de
  la consulta de estado (`estado: CANCELADO`). Un QR que no está `PENDIENTE` da 409 `ESTADO_INVALIDO`.
- **Errores:** 404 `TRANSACCION_NO_ENCONTRADA` y 400 de validación con `errors[]`. Si el backend
  está caído, el gateway devuelve 502 en texto plano.
- **El sandbox acepta cualquier `idEstablecimiento`.**
- **QR MLD-BCB:** funciona en sandbox. En desarrollo daba 502 (backend caído) el 2026-10-06.

Pendiente:
- La forma real del webhook. Para verla hay que pagar un QR de sandbox con un webhook público;
  el parser acepta los dos formatos documentados.
- Producción figura como `http://` en la página de QR Simple; se usa HTTPS.
