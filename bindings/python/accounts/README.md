# openhub-bo-accounts

Cuentas de comercio en Red Enlace (ATC) OpenHub, Bolivia (`ATC.API.CUENTA.DIGITAL`):
consulta y alta de cuentas, cambio de estado, **saldos** y **movimientos** para conciliar
los cobros QR y los pagos.

```sh
pip install openhub-bo[accounts]
```

```python
from datetime import date, timedelta
from openhub_bo.core import Session
from openhub_bo.accounts import AccountsClient, NewAccount, StatusChange, AccountStatus

with Session("CLIENT_ID", "CLIENT_SECRET") as session:
    accounts = AccountsClient(session)
    nit = "1000000019"

    for acc in accounts.list_accounts(nit).accounts:
        print(acc.number, acc.alias, acc.status)

    for b in accounts.balances(nit, ["7014227171"]):  # hasta 10 cuentas
        print(b.account_number, b.available, b.held)

    today = date.today()
    rec = accounts.reconcile(nit, date_from=today - timedelta(days=7), date_to=today)  # máx. 7 días
    payins = accounts.credits(
        nit, "7014227171", date_from=today - timedelta(days=30), date_to=today
    )
    qr_total = sum(m.amount for m in payins if m.operation_type == "PAYIN QR")

    accounts.create_accounts(
        nit, establishment_id=111369, accounts=[NewAccount("CAJA 5", "COMERCIALES")]
    )
    accounts.set_account_status(
        nit, [StatusChange("7011113693", AccountStatus.BLOCKED, "Sospecha de fraude")]
    )
```

Límites verificados en el sandbox: conciliación hasta **7 días** (la documentación dice 31),
créditos/débitos hasta 31 días, saldos hasta 10 cuentas. Cerrar una cuenta (`CLOSED`)
exige saldo 0 y es irreversible. Errores: `ApiError.code` `17` (comercio no encontrado),
`15` (cuenta no encontrada), `99` (sin cuentas habilitadas), `02` (validación; un
`ApiFieldError` por mensaje).

Async: `AsyncAccountsClient`. Tests sin red: `openhub_bo.accounts.testing.MockAccounts`.
