# @openhub-bo/accounts

Cuentas de comercio en Red Enlace (ATC) OpenHub: consulta y alta, cambio de estado,
**saldos** y **movimientos** para conciliar cobros y pagos.

```ts
import { AccountsClient } from "@openhub-bo/accounts";

const accounts = new AccountsClient(session);
const { establishments } = await accounts.listAccounts("1000000019");
const balances = await accounts.balances("1000000019", ["7014227171"]);            // ≤ 10 cuentas
const rec = await accounts.reconcile("1000000019", { dateFrom: "2026-09-29", dateTo: "2026-10-06" });  // ≤ 7 días
const payins = await accounts.credits("1000000019", "7014227171", { dateFrom: new Date(Date.now() - 30 * 864e5), dateTo: new Date() });
```

Límites verificados en el sandbox: conciliación ≤ 7 días, créditos/débitos ≤ 31 días,
saldos ≤ 10 cuentas. Cerrar una cuenta exige saldo 0 y es irreversible.
