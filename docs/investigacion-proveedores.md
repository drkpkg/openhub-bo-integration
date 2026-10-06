# APIs de cobro en Bolivia: investigación (2026-10-06)

Leyenda: **[OF]** fuente oficial · **[3ro]** blog/GitHub/prensa · **[INF]** inferido.

## Proveedores con API, por accesibilidad

| Prioridad | Proveedor | Docs | Sandbox | Auth | Notas |
|---|---|---|---|---|---|
| 1 | **OpenHub (Red Enlace/ATC)** | Públicas | Sí | OAuth2 client credentials | QR Simple, QR MLD-BCB, PIX, cripto, Binance, payouts. Cobra desde cualquier banco. Lanzado 2026-10-05 [OF] |
| 1 | **BNB Open Banking 2.0** | Públicas | Sí | `accountId`/`authorizationId` → token | QR, QR terceros, débito automático. Requiere cuenta BNB y contrato [OF] |
| 2 | **Libélula (Todotix)** | PDF v2.7 (2020) | Sí | `appkey` en body | Link de pago, QR, tarjeta, Tigo Money, factura. Callback GET sin firma [OF] |
| 2 | **CUCU** | Públicas | — | `X-Api-Key` | Webhook firmado, idempotencia. Bs 3.200 + plan + 1,3–2,5% [OF] |
| 2 | **Xmart Cloud** | Públicas | — | — | Sobre BNB/BISA/Económico. USD 49,99 + 24,99/mes [OF] |
| 3 | **BCP** | Por formulario | ? | user/pass + certificado [3ro] | [bcp.com.bo/Desarrollo](https://www.bcp.com.bo/Desarrollo) |
| 3 | **Banco Económico** | Privadas | Host desa [3ro] | user/pass, token 30 min [3ro] | "QR Integrado", ~30 días de implementación [OF] |
| 3 | **OpenBCB (BCB)** | No publicadas | ? | ? | Gratuito, QR BCB, notificaciones. CP-44/2025 [OF] |
| 4 | BISA, Ganadero | Con contrato | — | API Key / web service | Contactar banco [OF] |
| ✗ | BMSC, Unión*, FIE, BancoSol, Prodem, Fortaleza, Ecofuturo, BNA, Yape, Tigo Money, MultiPago, Veripagos | Sin API pública | | | *Unión solo vía Pasarela de Pagos del Estado (entidades públicas) |

Tarjetas: CyberSource vía Red Enlace, o Libélula.

## OpenHub: detalle

- Portal: <https://openhub.redenlace.com.bo/es>, docs en `/es/products`, registro en `/es/auth/register`.
- Soporte: WhatsApp +591 72108958. Precios no publicados.
- Ambientes: desarrollo `https://atcgwapitest.redenlace.com.bo/desarrollo/`,
  certificación `https://atcgwapitest.redenlace.com.bo/sandbox/`,
  producción `https://api.redenlace.com.bo/`.
- Endpoints: `POST /oauth-client-credentials/access-token?grant_type=client_credentials`,
  `POST /qr/{simple|mld}/v2/generate`, `GET /qr/{simple|mld}/v2/verify/{numeroReferencia}`.
- Estados: `PENDIENTE`, `PAGADO`, `CANCELADO`, `EXPIRADO`, `ERROR`.
- Webhook: POST con header estático `key: value` definido al generar el QR; responder 200.
- Productos adicionales: QR PIX, QR activos virtuales (USDT/USDC), QR Binance,
  dispersión de fondos síncrona/asíncrona (OETF), cuentas y saldos, cajas TCP/IP, SDK NFC Android.
- Comercial: ATC provisiona credenciales por integrador, así que cada usuario
  de la librería necesita su propio alta en OpenHub.

## Proyectos existentes (GitHub / registries)

- No hay SDK open-source multi-banco ni multi-lenguaje para Bolivia. Nada supera 5★.
- [sincpro-payments-sdk](https://pypi.org/project/sincpro-payments-sdk/): Python, BNB/Económico/Linkser/Cybersource; repo privado, licencia propietaria.
- [QR-del-Banco-Nacional-de-Bolivia-BNB-](https://github.com/4n7h0ny07/QR-del-Banco-Nacional-de-Bolivia-BNB-): Laravel, MIT.
- [poc_api_market](https://github.com/GatoCoder282/poc_api_market): FastAPI con mock del sandbox BNB.
- [BCP-ADB-BOLIVIA](https://github.com/99Arrzel/BCP-ADB-BOLIVIA): genera QR BCP automatizando la app por ADB.
- Plugins abandonados: PagosNet / Tigo Money (WooCommerce, Spree), [solunes/payments](https://github.com/solunes/payments) (multi-pasarela Laravel).
- **Ningún cliente para OpenHub ni OpenBCB.**
- Odoo: módulos comerciales de CUCU e Iterasoft (BNB); nada open source.

### Referencias de diseño

- SDKs oficiales en Python/Ruby/Node: Transbank (Chile), Mercado Pago.
- Multi-banco: [pyCobranca](https://github.com/Maxwbh/pyCobranca) (Brasil, 19 bancos).
- Adaptadores multi-gateway: activemerchant (Ruby), Omnipay (PHP).
- Pix/EMV: [pix-utils](https://github.com/thalesog/pix-utils), [pix-qrcode-utils](https://github.com/NascentSecureTech/pix-qrcode-utils).

## Contexto regulatorio

- QR "Simple" interoperable: ASOBAN, mayo 2019; base normativa BCB RD 137/2019 [OF ASFI].
- QR en 2024: Bs 156.926 millones; 27% del valor de transferencias electrónicas a ene-2025 [OF ASFI].
- Estándar "QR BCB Bolivia" sobre el MLD [OF]; especificación del payload no publicada,
  así que no se puede generar QR sin pasar por un banco o adquirente.
- OpenBCB (8-oct-2025): API gratuita del BCB para QR, estado y notificaciones [OF].
