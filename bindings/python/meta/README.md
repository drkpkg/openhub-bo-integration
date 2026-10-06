# openhub-bo

Clientes para las APIs de cobro de **Red Enlace (ATC) OpenHub**, Bolivia.
Instala solo los productos que necesitas:

```sh
pip install openhub-bo[qr]     # QR Simple + QR MLD-BCB
pip install openhub-bo[fx]     # PIX, USDT/USDC (Koibanx), Binance Pay
pip install openhub-bo[accounts]  # cuentas de comercio, saldos, conciliación
pip install openhub-bo[payouts]   # pagar QR de terceros, lotes ACH
pip install openhub-bo[all]    # todo
```

| Extra | Paquete | Import |
|---|---|---|
| (siempre) | `openhub-bo-core` | `openhub_bo.core` |
| `qr` | `openhub-bo-qr` | `openhub_bo.qr` |
| `fx` | `openhub-bo-fx` | `openhub_bo.fx` |
| `accounts` | `openhub-bo-accounts` | `openhub_bo.accounts` |
| `payouts` | `openhub-bo-payouts` | `openhub_bo.payouts` |
