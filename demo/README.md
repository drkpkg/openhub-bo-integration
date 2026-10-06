# Demo: QR Simple y MLD-BCB en los tres lenguajes

Una misma página web (`web/index.html`) servida por **tres servicios**, cada uno con su
propia librería sobre el mismo núcleo Rust:

| Servicio | Librería | Puerto |
|---|---|---|
| `python/server.py` | `openhub-bo-qr` (PyPI) | 8101 |
| `typescript/server.ts` | `@openhub-bo/qr` (npm) | 8102 |
| `ruby/server.rb` | gema `openhub-bo-qr` | 8103 |

La página permite generar un QR Simple o MLD-BCB, ver la imagen, consultar el estado,
cancelar (solo Simple) y ver los webhooks que recibe ese servicio. Cada servicio publica
su propia URL de webhook y la autentica con un secreto aleatorio.

## Ejecutar

Requisitos: `.env` en la raíz con `CLIENT_ID` y `CLIENT_SECRET`; `cloudflared`; los
bindings compilados (`bindings/python`: `uv sync --all-packages`; `bindings/typescript`:
`pnpm install && pnpm run build:wasm && pnpm -r build`; `bindings/ruby`: `rake`);
`demo/typescript`: `pnpm install`; gemas `webrick` y `erb` (`gem install --user-install
webrick erb`).

```sh
demo/run.sh
```

Levanta tres **túneles rápidos de Cloudflare** (no modifican tu cuenta ni tu DNS; las URLs
`*.trycloudflare.com` viven mientras corre el script) y los tres servicios, e imprime:

```
OpenHub QR demo (usuario: demo, clave en demo/.run/password)
  Python     https://….trycloudflare.com
  TypeScript https://….trycloudflare.com
  Ruby       https://….trycloudflare.com
```

## Seguridad

- La página y la API piden autenticación básica (`demo` + clave aleatoria en
  `demo/.run/password`, o `DEMO_PASSWORD`).
- Los webhooks (`/webhooks/qr/{simple|mld}`) no usan esa clave: se autentican con el
  header secreto configurado al generar cada QR.
- Monto máximo Bs 10. Solo sandbox.
- Errores de OpenHub se devuelven con HTTP 422: Cloudflare reemplaza las respuestas 502 del
  origen por su propia página de error.

## Lo que la demo dejó al descubierto (sandbox, 2026-10-06)

- `nombreEstablecimiento` solo admite letras (con tildes/ñ), números y espacios; un
  guion, punto, coma, `_`, `&`, `'` o `/` dan `400 INVALID_FORMAT`. La glosa acepta todo.
- La referencia del comercio debe ser ≤ 2.147.483.647 (entero de 32 bits).
