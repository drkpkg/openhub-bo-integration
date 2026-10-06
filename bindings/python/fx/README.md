# openhub-bo-fx

Cobros QR con conversión de moneda vía Red Enlace (ATC) OpenHub, Bolivia:

- **PIX**: el pagador (Brasil) paga en BRL; tú recibes BOB/USD.
- **Activos virtuales (Koibanx)**: liquidación en USDT/USDC.
- **Binance Pay**.

```sh
pip install openhub-bo[fx]
```

> Requiere que ATC habilite cada producto para tu comercio. Si no lo está, OpenHub
> responde `ApiError` con código `GQ-00005` (PIX), `GQ-00006` (Koibanx) o `GQ-00000`
> (Binance). Las validaciones y los errores están verificados en el sandbox; el caso
> exitoso sigue la documentación de ATC.

## Uso

```python
from openhub_bo.core import Session, Webhook
from openhub_bo.fx import Glosa, PixClient, VirtualAssetsClient, BinanceClient, VirtualAsset

webhook = Webhook(url="https://mitienda.bo/webhooks/fx", value="<secreto-largo-aleatorio>")
glosa = Glosa("1", "Tienda Central", "7011", "Pedido 42")  # sucursal|nombre|rubro|descripción

with Session("CLIENT_ID", "CLIENT_SECRET") as session:
    pix = PixClient(session)
    qr = pix.generate_qr(
        amount="145.00",
        glosa=glosa,
        reference="1042",
        payer_cpf="12345678901",
        payer_phone="+5511999999999",  # + y 13 dígitos
        webhook=webhook,
        expires_in=600,
    )
    print(qr.converted_amount, qr.converted_currency)  # lo que paga el cliente en BRL
    html = f'<img src="{qr.data_uri}">'
    pix.get_status(qr.reference)
    pix.cancel(qr.reference)  # en sandbox solo aplica a QR pagados (devolución)

    crypto = VirtualAssetsClient(session).generate_qr(
        amount="50",
        glosa=glosa,
        reference="1043",
        asset=VirtualAsset.USDT,
        webhook=webhook,
        expires_in=180,  # mínimo Bs 50; vigencia 180..600 s
    )
    binance = BinanceClient(session).generate_qr(
        amount="10",
        glosa=glosa,
        reference="1044",
        webhook=webhook,
        expires_in=300,
    )
```

Async: `AsyncPixClient`, `AsyncVirtualAssetsClient`, `AsyncBinanceClient` con `AsyncSession`.

## Webhooks

```python
from openhub_bo.fx import parse_binance_webhook, parse_fx_webhook

n = parse_binance_webhook(headers, body, webhook=webhook)
return JSONResponse(n.ack)  # Binance exige esta respuesta con HTTP 200

n = parse_fx_webhook(headers, body, webhook=webhook)  # PIX / Koibanx (payload no documentado)
n.payload  # cuerpo completo; confirma siempre con get_status
```

## Tests sin red

`openhub_bo.fx.testing.MockFx` simula los tres productos, con las reglas y errores del
sandbox, `pay(reference)` y `disable("pix")` para simular un comercio no habilitado.
