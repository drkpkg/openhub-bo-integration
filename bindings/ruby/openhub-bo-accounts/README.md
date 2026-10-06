# openhub-bo-accounts

Cuentas de comercio en Red Enlace (ATC) OpenHub, Bolivia: consulta, alta, estado,
**saldos** y **movimientos** para conciliar cobros y pagos.

```ruby
require "openhub_bo/accounts"
accounts = OpenhubBo::Accounts::AccountsClient.new(session)

accounts.list_accounts(nit).accounts.each { |a| puts a.number, a.status }
accounts.balances(nit, ["7014227171"])                         # hasta 10 cuentas
accounts.reconcile(nit, date_from: Date.today - 7, date_to: Date.today)   # máx. 7 días
accounts.credits(nit, "7014227171", date_from: Date.today - 30, date_to: Date.today)
```

Límites del sandbox: conciliación 7 días, créditos/débitos 31, saldos 10 cuentas.
Cerrar una cuenta (`:closed`) exige saldo 0 y es irreversible.
