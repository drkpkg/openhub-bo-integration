# bo_payment_providers

Librerías para cobrar por QR en Bolivia a través de **Red Enlace (ATC) OpenHub**,
con un único núcleo en Rust y bindings por lenguaje.

| Paquete | Estado |
|---|---|
| `openhub-core` + `openhub-qr` + `openhub-fx` + `openhub-accounts` + `openhub-payouts` (Rust) | 🟡 v0.1 |
| `openhub-bo` (Python, PyPI) | 🟡 v0.1, metapaquete con extras: `pip install openhub-bo[qr]`, `[fx]`, `[accounts]`, `[payouts]`, `[all]` |
| ↳ `openhub-bo-qr` | QR Simple + MLD-BCB. Generar / consultar / cancelar **verificados en sandbox** |
| ↳ `openhub-bo-fx` | PIX, USDT/USDC (Koibanx), Binance Pay. Validaciones y errores verificados en sandbox; **caso exitoso pendiente** (el comercio no tiene habilitados estos productos) |
| ↳ `openhub-bo-accounts` | Cuentas, saldos y movimientos (conciliación). Validaciones y errores verificados en sandbox; **caso exitoso pendiente** (falta un NIT con cuentas) |
| ↳ `openhub-bo-payouts` | Pagar QR de terceros y lotes ACH. Lectura de QR y bancos **verificados en sandbox**; pagar y autorizar lotes siguen la doc (no ejecutados) |
| TypeScript (npm) | ⚪ planificado (WASM) |
| Ruby (RubyGems) | 🟡 v0.1, gemas `openhub-bo-{core,qr,fx,accounts,payouts}` (magnus); QR verificado en sandbox |

Productos de OpenHub cubiertos: **8 de 8**: QR Simple, QR MLD-BCB, QR PIX, QR activos
virtuales, QR Binance, cuenta digital, dispersión de fondos síncrona y asíncrona (ver
[docs/openhub-productos.md](docs/openhub-productos.md)).

## Estructura

```
crates/openhub-core/      núcleo sans-IO: Operation/Registry, sobres, estados, token, errores
crates/openhub-qr/        familia QR Simple + MLD-BCB (operaciones, modelos, webhook)
crates/openhub-fx/        familia PIX + Koibanx + Binance (conversión de moneda)
crates/openhub-accounts/  cuentas de comercio, saldos y movimientos
crates/openhub-payouts/   pagar QR de terceros y lotes de transferencias ACH
bindings/python/          workspace uv con tres distribuciones (namespace `openhub_bo`)
  core/  -> openhub-bo-core   Session, Transport, errores, MockGateway   (_native = CoreOps)
  qr/    -> openhub-bo-qr     QrClient, AsyncQrClient, MockOpenHub       (_native = QrOps)
  fx/    -> openhub-bo-fx     PixClient, VirtualAssetsClient, BinanceClient, MockFx (_native = FxOps)
  accounts/ -> openhub-bo-accounts  AccountsClient, MockAccounts              (_native = AccountsOps)
  payouts/  -> openhub-bo-payouts   PayoutsClient, MockPayouts                (_native = PayoutsOps)
  meta/  -> openhub-bo        metapaquete con extras [qr], [all]
fixtures/openhub/         payloads de la doc y del sandbox, compartidos por todos los tests
docs/                     arquitectura e investigación de proveedores
```

Ver [docs/arquitectura.md](docs/arquitectura.md) para el diseño y cómo agregar
un lenguaje u operación, y [docs/investigacion-proveedores.md](docs/investigacion-proveedores.md)
para el panorama de APIs de cobro en Bolivia.

## Desarrollo

Requisitos: Rust estable, [uv](https://docs.astral.sh/uv/).

```sh
cargo test --workspace                     # núcleo
cd bindings/python && uv sync --all-packages   # compila las extensiones nativas
uv run --no-sync pytest                        # tests Python (usan MockOpenHub)
```

Tras cambiar código Rust: `uv sync --all-packages --reinstall-package openhub-bo-core --reinstall-package openhub-bo-qr`.

Prueba contra el sandbox (requiere `CLIENT_ID`/`CLIENT_SECRET` en `.env`):

```sh
cd bindings/python && set -a && . ../../.env && set +a
uv run --no-sync python examples/sandbox_smoke.py      # QR
uv run --no-sync python examples/fx_sandbox_smoke.py   # PIX / Koibanx / Binance
uv run --no-sync python examples/accounts_sandbox_smoke.py   # cuentas (solo lectura; OPENHUB_NIT opcional)
uv run --no-sync --with zxing-cpp --with pillow python examples/payouts_sandbox_smoke.py   # pagos (solo lectura)
```

## TypeScript (npm)

`bindings/typescript`: paquetes `@openhub-bo/{core,qr,fx,accounts,payouts}`, el mismo
núcleo Rust compilado a WebAssembly (un `.wasm` por paquete). Solo servidor (Node ≥ 18):
el `clientSecret` nunca debe llegar a un navegador. Montos como string decimal.

```sh
cd bindings/typescript
pnpm install && pnpm run build:wasm && pnpm run typecheck && pnpm test
pnpm run smoke        # QR contra el sandbox real (lee ../../.env)
```

Requiere el target `wasm32-unknown-unknown` y `wasm-bindgen-cli` 0.2.129.

## Ruby (RubyGems)

Gemas nativas (magnus + rb_sys), una por producto, con el mismo núcleo Rust:
`openhub-bo-core`, `-qr`, `-fx`, `-accounts`, `-payouts` y la metagema `openhub-bo`.
Requiere Ruby ≥ 3.2 y Rust para compilar al instalar.

```sh
gem install --user-install rb_sys rake minitest bigdecimal   # una vez
cd bindings/ruby && rake compile test                         # vendoriza crates, compila y prueba
ruby -Iopenhub-bo-core/lib -Iopenhub-bo-qr/lib examples/sandbox_smoke.rb   # sandbox real
```

## Pendiente antes de publicar

- [x] Registrarse en OpenHub y contrastar contratos contra la doc autenticada y el sandbox.
- [ ] Capturar un webhook real (pagar un QR de sandbox con un endpoint público).
- [ ] Pedir a ATC habilitar PIX, Koibanx y Binance para el comercio (`GQ-00005/6/0`) y verificar el caso exitoso.
- [ ] Verificar cuentas con el NIT real del comercio (`OPENHUB_NIT`).
- [ ] Probar un pago QR y un lote ACH reales en sandbox (con cuenta de origen habilitada) y coordinar el webhook de lotes con ATC.
- [ ] Certificación con ATC para credenciales de producción.
- [ ] Elegir licencia.
- [ ] Workflow de release de wheels (maturin-action) y publicación en PyPI.
