# openhub-bo-qr

Cobros con **QR Simple** y **QR MLD-BCB** vía Red Enlace (ATC) OpenHub, Bolivia.
Sync y async, montos exactos con `Decimal`, cancelación, validación de webhooks y un
mock en memoria para tus tests. Núcleo en Rust.

```sh
pip install openhub-bo[qr]
```

## Uso

```python
from decimal import Decimal
from openhub_bo.core import Session, Environment, Webhook, PaymentStatus
from openhub_bo.qr import QrClient

webhook = Webhook(url="https://mitienda.bo/webhooks/openhub", value="<secreto-largo-aleatorio>")

with Session("CLIENT_ID", "CLIENT_SECRET", environment=Environment.SANDBOX) as session:
    qr_client = QrClient(session)
    qr = qr_client.generate_qr(
        amount=Decimal("150.00"),
        description="Pedido #1042",
        reference="1042",  # tu referencia (solo dígitos)
        establishment_id=422717,  # asignado por ATC
        establishment_name="Mi Tienda",
        expires_in=900,  # segundos (o timedelta)
        webhook=webhook,  # obligatorio en OpenHub
    )
    save(order_id=1042, atc_reference=qr.reference)  # el webhook solo trae esta
    html = f'<img src="{qr.qr_data_uri}">'

    status = qr_client.get_qr_status(qr.reference)
    if status.status is PaymentStatus.PAID:
        ...
    elif status.status is PaymentStatus.PENDING:
        qr_client.cancel_qr(qr.reference)  # solo QR Simple
```

Async: `AsyncSession` + `AsyncQrClient`, misma API con `await`.

Si `generate_qr` pierde la respuesta (timeout tras enviar), lanza
`AmbiguousOutcomeError`: el QR pudo haberse creado; no reintentes a ciegas.

### Webhook

```python
from openhub_bo.core import WebhookAuthError
from openhub_bo.qr import parse_webhook


@app.post("/webhooks/openhub")  # FastAPI / Starlette
async def openhub_webhook(request: Request):
    try:
        n = parse_webhook(request.headers, await request.body(), webhook=webhook)
    except WebhookAuthError:
        return Response(status_code=401)
    if n.success:
        # ATC no firma el payload: confirma estado e importe antes de entregar.
        confirmed = await qr_client.get_qr_status(n.reference)
        ...
    return Response(status_code=200)  # ATC espera HTTP 200
```

### Tests sin red

```python
from openhub_bo.qr.testing import MockOpenHub


def test_checkout():
    mock = MockOpenHub()
    qr_client = mock.client()
    qr = qr_client.generate_qr(...)
    call = mock.pay(qr.reference)  # simula el pago y devuelve el webhook
    n = parse_webhook(call.headers, call.body, webhook=webhook)
```
