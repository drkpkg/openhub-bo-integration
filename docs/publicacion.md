# Publicación en crates.io, PyPI, RubyGems y npm

Todos los paquetes se publican juntos, con la misma versión, desde
`.github/workflows/release.yml` al subir un tag `vX.Y.Z`. Ningún registro necesita
tokens guardados en el repo: se usa *trusted publishing* (OIDC). GitHub demuestra al
registro que la publicación viene de este workflow.

| Registro | Paquetes | Artefactos |
|---|---|---|
| crates.io | `openhub-bo-core`, `-qr`, `-fx`, `-accounts`, `-payouts` | crates sans-IO en Rust (sin dependencias de red ni runtime) |
| PyPI | `openhub-bo-core`, `-qr`, `-fx`, `-accounts`, `-payouts`, `openhub-bo` | wheels abi3 (Python ≥ 3.10) para Linux glibc/musl x86_64/aarch64, macOS x86_64/arm64, Windows x64, más sdist |
| npm | `@openhub-bo/core`, `qr`, `fx`, `accounts`, `payouts` | un paquete por producto con su `.wasm` (Node ≥ 18, cualquier plataforma) |
| RubyGems | `openhub-bo-core`, `-qr`, `-fx`, `-accounts`, `-payouts`, `openhub-bo` | gemas precompiladas (Ruby 3.2, 3.3, 3.4 y 4.0) para Linux glibc/musl x86_64/aarch64, macOS x86_64/arm64, Windows x64, más la gema fuente (compila con Rust) |

## Configuración inicial (una sola vez)

Valores comunes para los cuatro registros:

- Owner / repositorio: `drkpkg` / `openhub-bo-integration`
- Workflow: `release.yml`
- Environment: el indicado para cada registro (`crates-io`, `pypi`, `testpypi`, `npm`, `rubygems`)

En GitHub (*Settings → Environments*) conviene crear esos environments con
**Required reviewers**: así cada publicación espera tu aprobación.

### crates.io

1. Inicia sesión en <https://crates.io> con tu cuenta de GitHub y verifica tu correo en
   *Account Settings* (crates.io no deja publicar sin correo verificado).
2. **Primera publicación:** crates.io solo permite configurar *trusted publishing* sobre un crate
   que ya existe. Para la 0.1.0 crea un token en <https://crates.io/settings/tokens> con los
   scopes `publish-new` y `publish-update`, restringido a `openhub-bo-*` y con expiración corta, y
   guárdalo como secret `CARGO_REGISTRY_TOKEN` del environment `crates-io`.
3. Después de la primera publicación, en cada crate ve a *Settings → Trusted Publishing* y
   registra los valores de arriba con el environment `crates-io`. Luego borra el secret
   `CARGO_REGISTRY_TOKEN` y revoca el token: el workflow usa OIDC cuando el secret no existe.

### PyPI (y TestPyPI)

1. Activa 2FA en tu cuenta.
2. En <https://pypi.org/manage/account/publishing/>, en *Add a new pending publisher*, registra
   **cada uno de los 6 paquetes** con los valores de arriba y el environment `pypi`.
   Con un *pending publisher* el nombre queda reservado hasta la primera publicación.
3. Opcional, para ensayar: crea una cuenta en <https://test.pypi.org> y repite el paso 2 con el
   environment `testpypi`. Luego, en *Actions → Release → Run workflow*, elige `testpypi`.

### npm

npm está **desactivado** en el workflow hasta que la organización exista: el tag no publica en
npm salvo que la variable de repositorio `PUBLISH_NPM` (*Settings → Secrets and variables →
Actions → Variables*) valga `true`.

1. Crea la cuenta en <https://www.npmjs.com/signup> y activa 2FA.
2. Crea la organización **`openhub-bo`** (*Add Organization*, plan gratuito para paquetes
   públicos). Reserva el scope `@openhub-bo/*`.
3. **Primera publicación:** npm solo permite configurar *trusted publishing* sobre un paquete
   que ya existe. Para la 0.1.0 crea un *granular access token* (permiso *Read and write* sobre
   la organización `openhub-bo`, expiración corta) y guárdalo como secret `NPM_TOKEN` del
   environment `npm`.
4. Después de la primera publicación, en cada paquete ve a *Settings → Trusted Publisher →
   GitHub Actions* y registra los valores de arriba con el environment `npm`. Luego borra el
   secret `NPM_TOKEN` y revoca el token.

### RubyGems

1. Activa MFA en tu cuenta (las gemas exigen `rubygems_mfa_required`).
2. En <https://rubygems.org/profile/oidc/pending_trusted_publishers>, en *Create*, registra
   **cada una de las 6 gemas** con los valores de arriba y el environment `rubygems`.

## Publicar una versión

```sh
python3 scripts/release.py set-version 0.2.0   # actualiza todos los manifiestos
cargo update -w && (cd bindings/typescript && cargo update -w)
# edita CHANGELOG.md: renombra la sección a "## [0.2.0] - AAAA-MM-DD"
git commit -am "Release 0.2.0" && git push   # vía PR, como cualquier cambio
git tag v0.2.0 && git push origin v0.2.0      # dispara la publicación
```

El workflow verifica que el tag coincida con las versiones (`scripts/release.py check --tag`).
Luego construye todo, empaqueta y compila los crates (`cargo publish --dry-run`), prueba la instalación de los artefactos sin Rust (wheels en Linux, macOS
y Windows, npm en Node 18, gemas en Ruby 3.2 y 4.0) y publica. Al final crea el release de
GitHub con las notas del CHANGELOG y todos los artefactos.

Si una publicación falla a mitad de camino, se puede volver a ejecutar el workflow: los
paquetes ya publicados se saltan.

## Ensayo sin publicar

- **En cada PR** que toque archivos de empaquetado, el workflow construye y prueba todo, pero
  no publica.
- **A mano:** *Actions → Release → Run workflow → dry-run*.
- **Localmente:**

  ```sh
  python3 scripts/release.py check
  (cd bindings/python/qr && uvx maturin build --release && uvx maturin sdist)
  (cd bindings/typescript && pnpm run build:wasm && pnpm run build && cd packages/qr && pnpm pack)
  cd bindings/ruby && rake vendor && rb-sys-dock -p x86_64-linux -r 3.4 -d "$PWD/openhub-bo-qr" --build
  ```

## Versionado

- Todos los paquetes comparten versión (*lockstep*), aunque un producto no cambie.
- Las dependencias internas aceptan cualquier versión de la misma serie menor
  (`>=0.2.0,<0.3.0` en Python, `~> 0.2.0` en Ruby, `^0.2.0` en npm).
- Mientras la versión sea 0.x, un cambio de versión menor puede romper la API. Anótalo en el
  CHANGELOG.
