# Changelog

Todos los paquetes (`openhub-bo*` en crates.io, PyPI y RubyGems, `@openhub-bo/*` en npm) se publican
juntos y con la misma versión. Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/);
versiones según [SemVer](https://semver.org/lang/es/) (en 0.x, un cambio de minor puede romper la API).

## [0.1.0] - sin publicar

Primera versión. Integración no oficial con las APIs de Red Enlace (ATC) OpenHub, Bolivia.

### Agregado

- Núcleo en Rust sans-IO compartido por los tres lenguajes: sesión OAuth, sobres de respuesta,
  estados de pago comunes, errores con `retryable`, montos decimales exactos.
- `qr`: QR Simple y QR MLD-BCB (generar, consultar, cancelar, webhook). Verificado en sandbox.
- `fx`: QR PIX, activos virtuales (USDT/USDC vía Koibanx) y Binance Pay. Caso exitoso pendiente
  (el comercio de pruebas no tiene estos productos habilitados).
- `accounts`: cuentas de comercio, saldos y movimientos.
- `payouts`: pagar QR de terceros, lotes ACH y su webhook.
- Rust (`openhub-bo-*` en crates.io: núcleo y un crate por producto, sans-IO).
- Python (`openhub-bo-*`, wheels abi3 para Python ≥ 3.10), TypeScript (`@openhub-bo/*`, WASM,
  Node ≥ 18) y Ruby (`openhub-bo-*`, gemas precompiladas para Ruby ≥ 3.2).
- Licencia Apache-2.0.
