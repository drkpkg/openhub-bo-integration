# OpenHub (ATC / Red Enlace): revisión de productos

Fuente: portal autenticado <https://atc.sensedia-devportal.com/es/products>, revisado
el 2026-10-06. El gateway es **Sensedia API Manager**. Los Swagger publicados (OAS 2.0)
solo listan rutas: no tienen esquemas ni ejemplos, así que la fuente de verdad son las
páginas "Documentación".

## Común a todos los productos

- **Token:** `POST {base}/oauth-client-credentials/access-token?grant_type=client_credentials`
  con `Authorization: Basic base64(client_id:client_secret)` y
  `Content-Type: application/x-www-form-urlencoded`. Responde `{access_token, token_type, expires_in: 3600, scope?}`.
  - `token_type` varía entre `"Bearer"` y `"access_token"`, y `scope` puede faltar.
- **Headers de negocio:** la tabla teórica dice `Authorization: Bearer <token>`, pero **todos
  los ejemplos usan `access_token: <token>`**, y la guía PIX lo recomienda explícitamente
  "para evitar errores 401". Es el estilo de Sensedia, así que se envían ambos.
  Siempre va también `client_id`.
- **El `client_id` se obtiene creando una "App" en el devportal.**
- **Ambientes:**
  - desarrollo: `https://atcgwapitest.redenlace.com.bo/desarrollo`
  - certificación/sandbox: `https://atcgwapitest.redenlace.com.bo/sandbox`
  - QA: `https://atcgwapitest.redenlace.com.bo/qa` (solo lo menciona PIX)
  - producción: `https://api.redenlace.com.bo`
- **Webhooks:** autenticados con un header estático `key: value` (sin firma). La excepción
  es el payout asíncrono, que usa `?token=` en la URL.
- **Seguridad:** la página de payout asíncrono publica en claro un client ID y un client
  secret de desarrollo. No se usan ni se guardan aquí.

## 1. Cobro por QR SIMPLE (`/qr/simple/v2`), implementado

| Op | Método | Ruta |
|---|---|---|
| Generar | POST | `/generate` |
| Estado | GET | `/verify/{numeroReferencia ATC}` |
| Cancelar | POST | `/cancel/{numeroReferencia ATC}`, solo en el Swagger. Verificado: `data` igual a la consulta de estado; 409 `ESTADO_INVALIDO` si no está PENDIENTE |

- **Request:** `glosa, moneda(BOB), monto(≤2 dec), numeroReferencia, vigencia(seg), idEstablecimiento, nombreEstablecimiento, webhook{url,key,value}`.
- **Respuestas:** sobre `{success, message, data | errors[]}`.
- **Estados:** `PENDIENTE, PAGADO, CANCELADO, EXPIRADO, ERROR`.
- **Webhook:** la doc dice que tiene "la misma estructura que la consulta de estado",
  pero el ejemplo es otro (`codigoRespuesta: SUCCESS, monto, fechaHoraTransaccion, clienteOrigen.ciCliente, …`).
  El parser acepta ambos formatos. ATC recomienda validar que `numeroReferencia` e importe coincidan.
- `fechaExpiracion` viene sin zona horaria.
- **Verificado en sandbox:** `numeroReferencia` (del comercio) debe ser ≤ 2.147.483.647
  (entero de 32 bits; mayor → `500 QR_GENERATION_ERROR`) y `nombreEstablecimiento` solo
  admite letras (con tildes/ñ), números y espacios (cualquier signo → `400 INVALID_FORMAT`).
  La glosa acepta cualquier carácter. Aplica también a MLD-BCB.

## 2. Cobro por QR MLD-BCB (`/qr/mld/v2`), implementado

Mismo contrato que QR Simple, con `/generate` y `/verify/{ref}`. **No tiene cancelación.**

## 3. Cobro por QR PIX (`/qr/pix/v2`), implementado en `openhub-bo-fx`

| Op | Método | Ruta |
|---|---|---|
| Generar | POST | `/generar` |
| Estado | GET | `/verifica/{ref}` |
| Cancelar | GET | `/cancela/{ref}` |

- **Request:**
  - `numeroReferencia, glosa, monto, moneda (BOB|USD), canal (WEB|MOVIL|…), webhook`
  - `cpf` (11 dígitos, obligatorio), `telefono` (obligatorio)
  - opcionales: `tiempoQr "HH:MM:SS"` (default 100 s, máx. 23:59:59), `correoElectronico`, `campoExtra`
- **Respuesta (sin sobre `success/data`):** `numeroReferencia, origenNumeroReferencia, codigoRespuesta (PENDING), detalleRespuesta, imagen (b64), monto, moneda, montoConversion, monedaConversion (BRL), tipoCambio, qrExpiracion (ISO con offset)`.
- **Estado:** `{codigoRespuesta: PENDING|PAID|CANCELLED|EXPIRED, detalleRespuesta, data{numeroReferencia, monto, moneda, montoConversion, monedaConversion, tipoCambio, reversa}}`.
- **Cancelación:** `{codigoRespuesta: CANCELLED, data{monto, moneda, montoConversion (string), monedaConversion, fechaSolicitud}}`.
- Payload del webhook: no documentado.

## 4. Cobro por QR Activos Virtuales / Koibanx (`/qr/koibanx/v2`), implementado en `openhub-bo-fx`

| Op | Método | Ruta |
|---|---|---|
| Generar | POST | `/generar` |
| Estado | GET | `/estado/{ref}` |

- **Request:** `numeroReferencia, glosa ("sucursal|nombre|rubro|desc"), monto, moneda (BOB|USD), activoVirtual (UT=USDT, UP=USDC, BK), canal (APP|WEB|WAP|OTHERS), tiempoVencimientoQR (seg), campoExtra, webhook`.
  - La tabla de `tiempoVencimientoQR` dice mín. 30 y máx. 90, pero el ejemplo usa 180. Contradicción.
- **Respuesta:** igual que PIX: `codigoRespuesta PENDING`, `imagen`, conversión, `qrExpiracion` sin zona horaria.
- **Estados:** `PENDING, SUCCESS, CANCELLED, EXPIRED, ERROR`.
- **Webhook:** se exige en el request, pero su payload no está documentado.

## 5. Cobro por QR Activos Virtuales Binance (`/qr/binance/v2`), implementado en `openhub-bo-fx`

| Op | Método | Ruta |
|---|---|---|
| Generar | POST | `/generar` |
| Estado | GET | `/verificar/{ref}` |

- **Request:** `numeroReferencia (1–10 caracteres), glosa (con pipes), monto, moneda (BOB|USD), tiempoQr "HH:MM:SS" (máx. 00:05:00), canal, campoExtra, webhook`.
  - El ejemplo tiene `key` y `value` invertidos.
- **Respuesta:** como Koibanx. `imagen` es un JPEG (`/9j/…`) y `qrExpiracion` viene como `"yyyy-MM-dd HH:mm:ss"`.
- **Estados:** `PENDING, SUCCESS, CANCELLED, EXPIRED, ERROR`.
- **Webhook (documentado):** `{numeroReferencia, estado: "00", transacciones{monto, moneda, fechaHoraTransaccion, cliente{nombreCliente, ciCliente}}}`.
  El comercio **debe responder** HTTP 200 con `{numeroReferencia, codigoRespuesta: "00", detalleRespuesta: null}`.

## 6. Dispersión de fondos síncrona: pagar un QR de terceros (`/payout/sync/v3/qr`), implementado en `openhub-bo-payouts`

| Op | Método | Ruta | Timeout |
|---|---|---|---|
| Leer QR | POST | `/scan` `{imagen}` | 40 s |
| Pagar QR | POST | `/confirm` `{numeroReferencia, cuentaOrigen, transaccionId(≤32), importe, glosa?}` | 90 s |
| Estado | GET (catálogo) / POST (sección 5) | `/status/{numeroReferencia}` | 90 s |

- **Sobre:** `{data, code: "00", errorMessage?}`.
- **Códigos:** 00 OK, 02 datos inválidos, 04 no encontrada, 08 QR expirado, 09 QR no válido,
  10 moneda, 11 monto obligatorio, 12 glosa obligatoria, 13 saldo insuficiente, 14 estado inválido,
  15 cuenta no encontrada, 94/96 resultado no confirmado, 95/99 error interno.
- **Estados:** `APROBADA, RECHAZADA, PENDIENTE_PAGO, EN_PROCESO, PENDIENTE_CONFIRMACION`.
- **Reglas de `importe` y `glosa`:** dependen de lo que devolvió `/scan`. Si el QR trae importe 0,
  hay que enviar un importe mayor a cero; si no, `0.00`. La glosa se envía solo si el QR no la trae.

## 7. Dispersión de fondos asíncrona: lotes ACH (`/payout/async/v3`), implementado en `openhub-bo-payouts`

- Header adicional obligatorio: `branchCode` (código de comercio).

| Op | Método | Ruta |
|---|---|---|
| Autorizar lote | POST | `/lote/autorizar` |
| Estado | GET/POST (la doc es inconsistente) | `/lote/estado/{processId}?nroLote=…` o `?transaccionId=…` |
| Bancos | POST | `/bancos` |

- **Lote:**
  - Cabecera: `{processId (UUID), webhookUrl, transacciones[…]}`.
  - Cada transacción: `transaccionId(3–14), importe, fechaTransaccion (≥ hoy), cuentaOrigen, cuentaDestino, codeBanco, codeSucursal (LPZ, SCZ, CBB…), glosa, ciNitDestino, titularDestino, tipoMoneda`.
- **Respuestas:** `{code, message, data}`. Estados por transacción: `PENDIENTE/ERROR`, y luego `PAGADO, TRANSITO, CANCELADO`.
- **Webhook:** se autentica con `?token=` en la URL y llega uno por transacción. Hay que responder
  `{nroLote, numeroReferencia, codigoRespuesta: EXITOSO|FALLIDO, detalleRespuesta}`.
  La doc avisa: "Solo se permite conexión tras coordinación con Infraestructura y Redes".

## 8. Cuentas de establecimientos y saldos (`/cuentas-comercios/v1/cuentas`), implementado en `openhub-bo-accounts`

| Op | Método | Ruta |
|---|---|---|
| Detalle de cuenta | GET | `/{nit}/{numeroCuenta}` |
| Cuentas del NIT | GET | `/{nit}` |
| Alta de cuentas | POST | `/` `{nit, establecimiento{idEstablecimiento, cuenta[{alias, rubro, moneda: "068"}]}}` |
| Cambiar estado | PATCH | `/estados`. Estados: `ACTIVA, BLOQUEADA, SUSPENDIDA, CERRADA` (de CERRADA no se sale) |
| Conciliación | POST | `/transacciones` `{nit, fechaInicio, fechaFin}` (rango ≤ 31 días) → movimientos y saldos |
| Créditos | POST | `/creditos` `{nit, numeroCuenta[1], fechaInicio, fechaFin, tipo: "C"}` |
| Débitos | POST | `/debitos` (igual con `tipo: "D"`) |
| Saldos | POST | `/saldos` `{nit, numeroCuentas[≤10]}` |

- **Sobre:** `{data, code: "00"|"05", errorCode, errorMessage}`.
- **Útil para conciliar los cobros QR:** los movimientos `PAYIN QR` aparecen en créditos.
- **Verificado en el sandbox (2026-10-06, NIT inventado, solo lectura):** todo responde
  HTTP 200 con `code`: `15` cuenta no encontrada, `17` comercio no encontrado, `99` tanto
  "El comercio no tiene cuentas habilitadas" como "Error interno del servidor", `02`
  validación con cada mensaje en `data`. **La conciliación (`transacciones`) admite como
  máximo 7 días** (la doc dice 31); créditos/débitos 31 días; saldos hasta 10 cuentas.
  Caso exitoso pendiente: hace falta el NIT real del comercio.

## 9. Integración a cajas por TCP/IP (fuera de alcance)

Es un servicio Node.js (≥ 16) que se instala on-premise como puente entre la caja y los
terminales POS, configurado con `devices.json` y PM2.
- API local: `POST /pago/` `{device, typePay: chip|ctl|qr|qrpix|crypto, importe, crypto?, pix?}`.
- También tiene anulación, cierre de lote y `GET /dispositivos`.
- Soporta DCC y propinas.

## 10. SDK contactless para Android (fuera de alcance)

Son AAR para Android 8 o superior con lector NFC (SDK FTT + Visa Sensory Branding).
Métodos: `initService`, `doTransaction`, `doTaxTransaction`, `doManualTransaction`,
check-in/out, `doRefundTransaction`, `sendTransactionAnnulment`, `sendSettlement`, `getHistoryTransactions`, etc.

## Verificado en el sandbox para PIX / Koibanx / Binance (2026-10-06)

- **Comercio no habilitado** para los tres: PIX `GQ-00005`, Koibanx `GQ-00006`, Binance
  `GQ-00000` (vienen al final de `detalleRespuesta`, con HTTP 200 y `codigoRespuesta: ERROR`).
  Hay que pedir a ATC la habilitación; el caso exitoso aún no está verificado.
- **Cuatro formatos de error** en la familia: `{error: true, code: "EG-00002", message,
  data: [mensajes]}` (validación, HTTP 400), `{error: true, code: "EG-00001", data: null}`
  (negocio con **HTTP 500**, p. ej. "Transacción no encontrada"), `{success: false, errors}`
  (validación de Koibanx) y `codigoRespuesta: ERROR` con HTTP 200.
- **PIX:** `telefono` = `+` y 13 dígitos (14 caracteres); `glosa` con formato
  `sucursal|nombre|rubro|descripción`. Un error de negocio igual crea una referencia ATC,
  que queda `CANCELLED` (con `data: null` en la consulta). `cancela` solo se permite si el
  QR está "aprobado" (pagado): funciona como devolución, no como anulación.
- **Koibanx:** monto mínimo Bs 50; `tiempoVencimientoQR` entre **180 y 600** s (la tabla de
  la doc dice 30–90). Referencia inexistente → HTTP 200 `codigoRespuesta: ERROR`.
- **Binance:** referencia inexistente → HTTP 500 `EG-00001`.

## Verificado en el sandbox para pagos (2026-10-06, sin ejecutar pagos ni lotes)

- **`scan` funciona** con un QR Simple generado por el propio integrador: devuelve
  destinatario ATC, monto y glosa. `fechaVencimiento` llega como `yyyy-mm-dd` (la doc dice
  `AAAAMMDD`) y la referencia tiene 18 dígitos (la doc dice 20).
- **Métodos:** el estado de pagos y de lotes es `GET` (`POST` da 404 "without destination");
  `bancos` es `POST` (121 bancos en sandbox, `descripcion` vacía).
- **Errores** con `code`: `02` validación (varios mensajes unidos por `; `), `04` no
  encontrada, `09` QR no válido. Sin `branchCode`, los lotes responden 400 `99`.
- Pendiente: confirmar un pago y autorizar un lote reales (requieren cuenta de origen
  habilitada); el webhook de lotes requiere coordinación con ATC.

## Implicaciones para la librería

1. **Corregido:** enviar `access_token` (además de `Authorization: Bearer`) en las llamadas de negocio.
2. **Corregido:** agregar `cancel_qr` para QR Simple. Para MLD no existe.
3. **Corregido:** el parser de webhook acepta los dos formatos de QR Simple/MLD.
4. **Hecho:** PIX, Koibanx y Binance en `openhub-fx` / `openhub-bo-fx`.
5. **Hecho:** `cuentas-comercios` en `openhub-accounts` / `openhub-bo-accounts`.
6. **Hecho:** pagos (síncronos y por lotes) en `openhub-payouts` / `openhub-bo-payouts`.
