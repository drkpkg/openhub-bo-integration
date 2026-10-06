# Cómo contribuir

¡Gracias por tu interés! Issues y pull requests son bienvenidos.

## Antes de abrir un PR

- Un cambio por PR, enlazado a su issue.
- Corre las pruebas del lenguaje que tocaste (ver [README](README.md#desarrollo)); el CI corre todas.
- Si cambias el núcleo Rust, los tres bindings se ven afectados: agrega casos a `fixtures/` cuando
  el cambio dependa de un payload de OpenHub.
- Nunca subas credenciales, tokens ni datos reales de clientes. Anonimiza los payloads del sandbox.

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
