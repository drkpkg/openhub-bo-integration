# openhub-bo-fx

Cobros QR con conversión de moneda vía Red Enlace (ATC) OpenHub, Bolivia: **PIX** (el
pagador paga en BRL), **activos virtuales** vía Koibanx (USDT/USDC) y **Binance Pay**.
Requiere que ATC habilite cada producto para tu comercio (si no: `ApiError` `GQ-...`).

```ruby
require "openhub_bo/fx"
Fx = OpenhubBo::Fx

glosa = Fx::Glosa.new(branch_code: "1", branch_name: "Tienda", category: "7011", description: "Pedido 42")
qr = Fx::PixClient.new(session).generate_qr(amount: "145.00", glosa: glosa, reference: "1042",
                                            payer_cpf: "12345678901", payer_phone: "+5511999999999",
                                            webhook: webhook, expires_in: 600)
qr.converted_amount  # lo que paga el cliente en BRL

Fx::VirtualAssetsClient.new(session).generate_qr(amount: "50", glosa: glosa, reference: "1043",
                                                 asset: :usdt, webhook: webhook)   # mín. Bs 50; 180..600 s
Fx::BinanceClient.new(session).generate_qr(amount: "10", glosa: glosa, reference: "1044", webhook: webhook)

n = Fx.parse_binance_webhook(headers, body, webhook: webhook)   # responde con n.ack (HTTP 200)
```
