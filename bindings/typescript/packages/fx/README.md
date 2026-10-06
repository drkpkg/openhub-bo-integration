# @openhub-bo/fx

Cobros QR con conversión de moneda vía Red Enlace (ATC) OpenHub: **PIX** (pagador en
Brasil), **activos virtuales** USDT/USDC (Koibanx) y **Binance Pay**. Cada producto debe
estar habilitado por ATC para tu comercio (si no, `ApiError` con código `GQ-…`).

```ts
import { Session, Webhook } from "@openhub-bo/core";
import { BinanceClient, glosa, parseBinanceWebhook, PixClient, VirtualAssetsClient } from "@openhub-bo/fx";

const g = glosa({ branchCode: "1", branchName: "Tienda Central", category: "7011", description: "Pedido 42" });
const pix = await new PixClient(session).generateQr({
  amount: "145.00", glosa: g, reference: "1042", payerCpf: "12345678901",
  payerPhone: "+5511999999999", webhook, expiresIn: 600,
});
pix.convertedAmount; pix.convertedCurrency;           // lo que paga el cliente (BRL)

await new VirtualAssetsClient(session).generateQr({ amount: "50", glosa: g, reference: "1043", asset: "usdt", webhook });
await new BinanceClient(session).generateQr({ amount: "10", glosa: g, reference: "1044", webhook, expiresIn: 300 });

const n = parseBinanceWebhook(headers, body, webhook);
return Response.json(n.ack);                          // Binance exige esta respuesta
```
