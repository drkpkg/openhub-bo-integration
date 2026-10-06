# Cómo contribuir

¡Gracias por tu interés! Issues y pull requests son bienvenidos.

## Antes de abrir un PR

- Un cambio por PR, enlazado a su issue.
- Corre las pruebas del lenguaje que tocaste (ver [Desarrollo](#desarrollo)); el CI corre todas.
- Si cambias el núcleo Rust, los tres bindings se ven afectados: agrega casos a `fixtures/` cuando
  el cambio dependa de un payload de OpenHub.
- Nunca subas credenciales, tokens ni datos reales de clientes. Anonimiza los payloads del sandbox.

## Desarrollo

Requisitos: Rust estable y, según el lenguaje, [uv](https://docs.astral.sh/uv/), Node ≥ 18 con
pnpm, o Ruby ≥ 3.2. Las pruebas contra el sandbox leen `CLIENT_ID` / `CLIENT_SECRET` de `.env`
(nunca lo subas).

### Rust

```sh
cargo test --workspace
```

### Python

```sh
cd bindings/python
uv sync --all-packages            # compila las extensiones nativas
uv run --no-sync pytest           # usa MockOpenHub, sin red
```

Tras cambiar código Rust: `uv sync --all-packages --reinstall-package openhub-bo-core --reinstall-package openhub-bo-qr`
(y el resto de paquetes que hayas tocado).

Contra el sandbox:

```sh
set -a && . ../../.env && set +a
uv run --no-sync python examples/sandbox_smoke.py            # QR
uv run --no-sync python examples/fx_sandbox_smoke.py         # PIX / Koibanx / Binance
uv run --no-sync python examples/accounts_sandbox_smoke.py   # cuentas (solo lectura; OPENHUB_NIT opcional)
uv run --no-sync --with zxing-cpp --with pillow python examples/payouts_sandbox_smoke.py   # pagos (solo lectura)
```

### TypeScript

Requiere el target `wasm32-unknown-unknown` y `wasm-bindgen-cli` 0.2.129.

```sh
cd bindings/typescript
pnpm install && pnpm run build:wasm && pnpm run typecheck && pnpm test
pnpm run smoke        # QR contra el sandbox (lee ../../.env)
```

### Ruby

```sh
gem install --user-install rb_sys rake-compiler rake minitest bigdecimal   # una vez
cd bindings/ruby && rake compile test        # vendoriza los crates, compila y prueba
ruby -Iopenhub-bo-core/lib -Iopenhub-bo-qr/lib examples/sandbox_smoke.rb   # sandbox
```

## Licencia y DCO

El proyecto se distribuye bajo [Apache-2.0](LICENSE). Al contribuir, aceptas que tu aporte se publique
bajo esa licencia.

Cada commit debe llevar la línea `Signed-off-by` del
[Developer Certificate of Origin](https://developercertificate.org/), que certifica que tienes derecho
a enviar ese código:

```sh
git commit -s -m "Describe el cambio"
```

## Versiones

Todos los paquetes comparten versión. No edites versiones a mano: usa
`python3 scripts/release.py set-version X.Y.Z` (ver [docs/publicacion.md](docs/publicacion.md)).
