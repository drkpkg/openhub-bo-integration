# @openhub-bo/qr

Cobros con **QR Simple** y **QR MLD-BCB** vía Red Enlace (ATC) OpenHub, Bolivia.

```sh
npm install @openhub-bo/core @openhub-bo/qr
```

```ts
import { Session, Webhook } from "@openhub-bo/core";
import { parseWebhook, QrClient } from "@openhub-bo/qr";

const webhook = new Webhook({ url: "https://mitienda.bo/webhooks/openhub", value: "<secreto-largo-aleatorio>" });
const qrClient = new QrClient(new Session({ clientId, clientSecret }));

const qr = await qrClient.generateQr({
  amount: "150.00", description: "Pedido #1042", reference: "1042",   // referencia: solo dígitos
  establishmentId: 422717, establishmentName: "Mi Tienda", expiresIn: 900, webhook,
});
const html = `<img src="data:image/png;base64,${qr.qrImageBase64}">`;
await qrClient.getQrStatus(qr.reference);
await qrClient.cancelQr(qr.reference);      // solo QR Simple pendientes

// Webhook (ATC no firma el payload: confirma con getQrStatus antes de entregar)
const n = parseWebhook(request.headers, await request.text(), webhook);
```
