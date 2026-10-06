# @openhub-bo/payouts

Pagos a terceros desde tu cuenta en Red Enlace (ATC) OpenHub: **pagar QR** interoperables
y **lotes de transferencias ACH**. ⚠️ Mueven dinero real en producción.

```ts
import { AmbiguousOutcomeError } from "@openhub-bo/core";
import { isOpenAmount, parseBatchWebhook, PayoutsClient } from "@openhub-bo/payouts";

const payouts = new PayoutsClient(session);
const scanned = await payouts.scanQr(qrText);        // texto del QR, no la imagen
try {
  await payouts.payQr(scanned, {
    sourceAccount: "7010123451", transactionId: "PAY-1042",
    ...(isOpenAmount(scanned) ? { amount: "100.50" } : {}),
    ...(scanned.description ? {} : { description: "Pago proveedor" }),
  });
} catch (e) {
  if (e instanceof AmbiguousOutcomeError) { /* 94/96 o respuesta perdida: consulta getPayout */ }
}

const auth = await payouts.authorizeBatch("455544", transfers, {
  webhookUrl: "https://mitienda.bo/webhooks/ach", webhookToken: "<token-largo-aleatorio>",
});
// En tu endpoint (ATC llama a webhookUrl?token=...):
const n = parseBatchWebhook(url.searchParams.get("token") ?? "", body, TOKEN);
return Response.json(n.ack());
```

Timeouts recomendados por ATC (40 s leer, 90 s pagar/consultar) se aplican solos.
