# openhub-bo-payouts

Pagos a terceros desde tu cuenta en Red Enlace (ATC) OpenHub, Bolivia: **pagar QR**
interoperables y **lotes de transferencias ACH**. ⚠️ Mueve dinero real en producción.

```ruby
require "openhub_bo/payouts"
payouts = OpenhubBo::Payouts::PayoutsClient.new(session)

scanned = payouts.scan_qr(qr_text)              # texto del QR, no la imagen
payout = payouts.pay_qr(scanned, source_account: "7010123451", transaction_id: "PAY-1042")
# AmbiguousOutcomeError (códigos 94/96 o respuesta perdida): consulta get_payout antes de reintentar

auth = payouts.authorize_batch("455544", [OpenhubBo::Payouts::BatchTransfer.new(...)],
                               webhook_url: "https://mitienda.bo/ach", webhook_token: ENV["ACH_TOKEN"])
n = OpenhubBo::Payouts.parse_batch_webhook(params["token"], request.body.read, expected_token: ENV["ACH_TOKEN"])
# responde con n.ack (HTTP 200)
```

Timeouts de ATC aplicados por operación: 40 s leer QR, 90 s pagar y consultar.
