# openhub-bo

Librerías para cobrar y pagar en Bolivia con las APIs de **Red Enlace (ATC) OpenHub**: QR Simple,
QR MLD-BCB, PIX, activos virtuales, Binance Pay, cuentas de comercio y dispersión de fondos.
Disponibles para **Rust, Python, TypeScript y Ruby**, con un mismo núcleo en Rust.

> Integración **no oficial**: este proyecto no está afiliado ni respaldado por Red Enlace (ATC).
> Necesitas tus propias credenciales de OpenHub.

## Instalación

```sh
cargo add openhub-bo-qr          # Rust ≥ 1.85
pip install "openhub-bo[qr]"     # Python ≥ 3.10
gem install openhub-bo-qr        # Ruby ≥ 3.2
```

Paquetes en [crates.io](https://crates.io/crates/openhub-bo-qr),
[PyPI](https://pypi.org/project/openhub-bo/) y [RubyGems](https://rubygems.org/gems/openhub-bo-qr).
Python y Ruby traen binarios precompilados para Linux, macOS y Windows: no hace falta Rust.
El paquete de TypeScript (`@openhub-bo/*`, Node ≥ 18) todavía no está publicado en npm.

## Ejemplo: cobrar con QR

Python:

```python
from decimal import Decimal
from openhub_bo.core import Session, Webhook
from openhub_bo.qr import QrClient

with Session(CLIENT_ID, CLIENT_SECRET, environment="sandbox") as session:
    client = QrClient(session)
    qr = client.generate_qr(
        amount=Decimal("25.50"),
        description="Pedido 1042",
        reference="1042",
        establishment_id=1,
        establishment_name="Mi Tienda",
        expires_in=600,
        webhook=Webhook(url="https://mi-tienda.bo/openhub/webhook", value=WEBHOOK_SECRET),
    )
    # qr.qr_png: imagen PNG para mostrar al cliente; el pago llega al webhook.
    status = client.get_qr_status(qr.reference)
```

Ruby:

```ruby
require "bigdecimal"
require "openhub_bo/qr"

session = OpenhubBo::Core::Session.new(CLIENT_ID, CLIENT_SECRET)
client = OpenhubBo::Qr::QrClient.new(session)
qr = client.generate_qr(
  amount: BigDecimal("25.50"), description: "Pedido 1042", reference: "1042",
  establishment_id: 1, establishment_name: "Mi Tienda", expires_in: 600,
  webhook: OpenhubBo::Core::Webhook.new(url: "https://mi-tienda.bo/openhub/webhook", value: WEBHOOK_SECRET)
)
```

TypeScript:

```ts
import { Session, Webhook } from "@openhub-bo/core";
import { QrClient } from "@openhub-bo/qr";

const client = new QrClient(new Session({ clientId, clientSecret, environment: "sandbox" }));
const qr = await client.generateQr({
  amount: "25.50", description: "Pedido 1042", reference: "1042",
  establishmentId: 1, establishmentName: "Mi Tienda", expiresIn: 600,
  webhook: new Webhook({ url: "https://mi-tienda.bo/openhub/webhook", value: webhookSecret }),
});
```

En Rust, los crates son *sans-IO*: arman la petición HTTP y leen la respuesta, y tú eliges el
cliente HTTP (reqwest, ureq, hyper…). Más ejemplos, incluido un receptor de webhooks, en
`bindings/*/examples/`.

## Productos

| Paquete | Producto de OpenHub | Estado |
|---|---|---|
| `openhub-bo-qr` | QR Simple y QR MLD-BCB: generar, consultar, cancelar, webhook | Verificado en sandbox |
| `openhub-bo-fx` | QR PIX, activos virtuales (USDT/USDC vía Koibanx), Binance Pay | Contratos y errores verificados en sandbox |
| `openhub-bo-accounts` | Cuenta digital: cuentas de comercio, saldos y movimientos | Contratos y errores verificados en sandbox |
| `openhub-bo-payouts` | Dispersión de fondos: pagar QR de terceros, lotes ACH y su webhook | Lectura de QR y bancos verificada en sandbox |
| `openhub-bo-core` | Sesión OAuth, errores, estados de pago y mocks; lo usan todos los anteriores | — |

En Python, el metapaquete `openhub-bo` instala los productos por extras: `[qr]`, `[fx]`,
`[accounts]`, `[payouts]` o `[all]`. En Ruby, la gema `openhub-bo` instala todos.
Detalle de cada producto y su API en [docs/openhub-productos.md](docs/openhub-productos.md).

## Características

- **Un núcleo, cuatro lenguajes.** Validación, formato de los mensajes de OpenHub, estados de pago
  y errores viven en Rust; Python (PyO3), Ruby (magnus) y TypeScript (WebAssembly) solo hacen el HTTP.
  Todos se prueban contra los mismos fixtures.
- **Montos exactos.** `Decimal` en Python, `BigDecimal` en Ruby, string decimal en TypeScript;
  nunca coma flotante.
- **Errores útiles.** `ApiError` con el código de OpenHub, el estado HTTP y `retryable` para
  decidir si reintentar.
- **Webhooks.** Parseo y verificación del secreto de las notificaciones de pago.
- **Mocks** por producto para probar tu integración sin red ni credenciales.
- **Solo servidor.** El `clientSecret` nunca debe llegar a un navegador o app móvil.

## Cómo está hecho

```
crates/openhub-bo-*/     núcleo sans-IO y un crate por producto
bindings/python/         paquetes de PyPI (namespace openhub_bo)
bindings/typescript/     paquetes @openhub-bo/* (WASM)
bindings/ruby/           gemas nativas
fixtures/openhub/        payloads de la doc y del sandbox, compartidos por todos los tests
```

El diseño está en [docs/arquitectura.md](docs/arquitectura.md); cómo compilar y probar cada
lenguaje, en [CONTRIBUTING.md](CONTRIBUTING.md); los cambios por versión, en
[CHANGELOG.md](CHANGELOG.md).

## Licencia

[Apache-2.0](LICENSE). "Red Enlace", "ATC" y "OpenHub" son marcas de sus respectivos titulares
(ver [NOTICE](NOTICE)). Para reportar una vulnerabilidad, ver [SECURITY.md](SECURITY.md).
