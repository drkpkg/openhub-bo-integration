# openhub-bo-qr

Cobros con **QR Simple** y **QR MLD-BCB** vía Red Enlace (ATC) OpenHub, Bolivia.

```ruby
require "openhub_bo/qr"

session = OpenhubBo::Core::Session.new(ENV["CLIENT_ID"], ENV["CLIENT_SECRET"])
qr_client = OpenhubBo::Qr::QrClient.new(session)
webhook = OpenhubBo::Core::Webhook.new(url: "https://mitienda.bo/webhooks/openhub", value: ENV["WEBHOOK_SECRET"])

qr = qr_client.generate_qr(amount: "150.00", description: "Pedido #1042", reference: "1042",
                           establishment_id: 422_717, establishment_name: "Mi Tienda",
                           expires_in: 900, webhook: webhook)        # kind: :mld para MLD-BCB
# guarda qr.reference y qr.kind: el webhook solo trae la referencia de ATC
qr_client.get_qr_status(qr.reference, kind: qr.kind).status      # => :pending / :paid / ...
qr_client.cancel_qr(qr.reference)                                # solo QR Simple

n = OpenhubBo::Qr.parse_webhook(request.headers, request.body.read, webhook: webhook)
# ATC no firma el payload: confirma con get_qr_status antes de entregar; responde HTTP 200
```
