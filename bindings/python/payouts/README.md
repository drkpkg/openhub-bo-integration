# openhub-bo-payouts

Pagos a terceros desde tu cuenta en Red Enlace (ATC) OpenHub, Bolivia:

- **Pagar QR** interoperables (`ATC.API.PAYOUT.SYNC 3`): leer → pagar → consultar.
- **Lotes de transferencias ACH** (`ATC.API.PAYOUT.ASYNC`) con notificación por webhook.

> ⚠️ Mueven dinero real en producción. Pruébalo primero con `MockPayouts` y el sandbox.

```sh
pip install openhub-bo[payouts]
```

## Pagar un QR

```python
from openhub_bo.core import Session, AmbiguousOutcomeError
from openhub_bo.payouts import PayoutsClient

with Session("CLIENT_ID", "CLIENT_SECRET") as session:
    payouts = PayoutsClient(session)
    scanned = payouts.scan_qr(qr_text)  # texto del QR (lo que devuelve un lector), no la imagen
    print(scanned.recipient.holder, scanned.amount)  # muéstralo antes de pagar
    try:
        payout = payouts.pay_qr(
            scanned,
            source_account="7010123451",
            transaction_id="PAY-1042",
            amount=None if not scanned.open_amount else "100.50",  # solo para QR sin monto
            description=None if scanned.description else "Pago proveedor",
        )
    except AmbiguousOutcomeError:
        ...  # códigos 94/96 o respuesta perdida: consulta get_payout antes de reintentar
```

Reglas de la documentación aplicadas por la librería: con un QR de monto fijo se envía
`0.00`; la glosa solo va si el QR no la trae. Timeouts recomendados por ATC: 40 s para
leer y 90 s para pagar y consultar (se aplican solos).

## Lotes ACH

```python
from openhub_bo.payouts import BatchTransfer, parse_batch_webhook

auth = payouts.authorize_batch(
    "455544",  # branchCode (código de comercio)
    [
        BatchTransfer(
            "001002",
            "50.00",
            "484811311404044",
            "1311404044",
            "1018",
            "LPZ",
            "Pago proveedor",
            "5452452",
            "PROVEEDOR SRL",
        )
    ],
    webhook_url="https://mitienda.bo/webhooks/ach",
    webhook_token="<token-largo-aleatorio>",
)
auth.rejected  # transferencias rechazadas al autorizar
payouts.get_batch_status("455544", auth.process_id, batch_number=auth.batch_number)
payouts.list_banks("455544")

# En tu endpoint (ATC llama a webhook_url?token=...):
n = parse_batch_webhook(request.query_params["token"], await request.body(), expected_token=TOKEN)
return JSONResponse(n.ack())  # o n.ack(processed=False, detail="...")
```

Según ATC, el webhook de lotes requiere coordinación previa con su área de
Infraestructura y Redes.

Verificado en el sandbox: lectura de QR (con un QR propio), lista de bancos, errores y
métodos HTTP. Confirmar pagos y autorizar lotes siguen la documentación (no se
ejecutaron contra el sandbox).
