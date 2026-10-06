"""In-memory fake of the OpenHub merchant-accounts API.

Seed it with merchants, accounts and movements, then point an
:class:`AccountsClient` at it::

    mock = MockAccounts()
    mock.add_account("1000000019", 111369, "7011234561", available="150.00")
    mock.add_movement("1000000019", "7011234561", "PAYIN QR", "100.00")
    client = mock.client()

Error codes mirror the sandbox: ``17`` unknown merchant, ``15`` unknown account,
``99`` merchant without enabled accounts, ``02`` validation (with every message
listed in ``data``).
"""

from __future__ import annotations

import itertools
import json
import re
from dataclasses import dataclass, field
from datetime import datetime
from decimal import Decimal
from typing import Any

import httpx

from openhub_bo.core import BOLIVIA_TZ
from openhub_bo.core.testing import MockGateway

from .client import AccountsClient, AsyncAccountsClient

_BASE = "/cuentas-comercios/v1/cuentas"


@dataclass
class _Account:
    number: str
    alias: str
    status: str = "ACTIVA"
    available: Decimal = Decimal("0")
    held: Decimal = Decimal("0")
    last_credit_at: str | None = None
    last_debit_at: str | None = None


@dataclass
class _Establishment:
    id: int
    name: str
    accounts: dict[str, _Account] = field(default_factory=dict)


@dataclass
class _Merchant:
    nit: str
    name: str
    establishments: dict[int, _Establishment] = field(default_factory=dict)
    movements: list[dict[str, Any]] = field(default_factory=list)

    def account(self, number: str) -> _Account | None:
        return next(
            (e.accounts[number] for e in self.establishments.values() if number in e.accounts),
            None,
        )


class MockAccounts(MockGateway):
    def __init__(self, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        self.merchants: dict[str, _Merchant] = {}
        self._numbers = itertools.count(7011113695)
        self.route("GET", rf"{_BASE}/(\d+)/(\d+)$", self._get)
        self.route("GET", rf"{_BASE}/(\d+)$", self._list)
        self.route("POST", rf"{_BASE}$", self._create)
        self.route("PATCH", rf"{_BASE}/estados$", self._set_status)
        self.route("POST", rf"{_BASE}/transacciones$", self._reconcile)
        self.route("POST", rf"{_BASE}/(creditos|debitos)$", self._movements)
        self.route("POST", rf"{_BASE}/saldos$", self._balances)

    def client(self, **kw: Any) -> AccountsClient:
        return AccountsClient(self.session(**kw))

    def async_client(self, **kw: Any) -> AsyncAccountsClient:
        return AsyncAccountsClient(self.async_session(**kw))

    # -- seeding -------------------------------------------------------------------

    def add_merchant(self, nit: str, name: str = "COMERCIO DEMO") -> None:
        self.merchants.setdefault(nit, _Merchant(nit, name))

    def add_account(
        self,
        nit: str,
        establishment_id: int,
        number: str,
        *,
        alias: str = "CAJA 1 - COMERCIALES",
        status: str = "ACTIVA",
        available: Decimal | str = "0",
        establishment_name: str = "SUCURSAL CENTRO",
    ) -> None:
        self.add_merchant(nit)
        merchant = self.merchants[nit]
        est = merchant.establishments.setdefault(
            establishment_id, _Establishment(establishment_id, establishment_name)
        )
        est.accounts[number] = _Account(number, alias, status, Decimal(available))

    def add_movement(
        self,
        nit: str,
        account_number: str,
        operation_type: str,
        amount: Decimal | str,
        *,
        at: str | None = None,
        status: str = "COMPLETADO",
    ) -> None:
        """Records a movement: ``PAYIN ...`` credits the account, ``PAYOUT ...`` debits it."""
        account = self.merchants[nit].account(account_number)
        assert account is not None, f"unknown account {account_number}"
        amount = Decimal(amount)
        at = at or datetime.now(BOLIVIA_TZ).replace(tzinfo=None, microsecond=0).isoformat()
        credit = operation_type.upper().startswith("PAYIN")
        if credit:
            account.available += amount
            account.last_credit_at = at
        else:
            account.available -= amount
            account.last_debit_at = at
        atc = {
            "cuentaOrigen": "ATC-RED ENLACE",
            "codigoBancoOrigen": "140",
            "nombreBancoOrigen": "ATC-RED ENLACE",
        }
        own = {
            "cuentaDestino": account_number,
            "codigoBancoDestino": "140",
            "nombreBancoDestino": "ATC-RED ENLACE",
        }
        self.merchants[nit].movements.append(
            {
                "account": account_number,
                "kind": "C" if credit else "D",
                "row": {
                    "transactionId": "" if credit else str(len(self.merchants[nit].movements) + 1),
                    "tipoOperacion": operation_type,
                    "estado": status,
                    "mensaje": operation_type,
                    "fechaHoraTransaccion": at,
                    "importe": float(amount),
                    "importeComision": 0,
                    "importeTotal": float(amount),
                    "moneda": "BOB",
                    **(
                        atc | own
                        if credit
                        else {"cuentaOrigen": account_number, "cuentaDestino": "98765432109"}
                    ),
                    "numeroReferencia": "N/A" if credit else "20260505105158",
                    "numOrdenAch": "20260505105049",
                    "numOrdenDestinatario": None,
                },
            }
        )

    # -- routes ----------------------------------------------------------------------

    def _merchant(self, nit: str) -> _Merchant | httpx.Response:
        merchant = self.merchants.get(nit)
        return merchant if merchant else _error("17", "Comercio no encontrado.")

    def _get(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        merchant = self._merchant(match.group(1))
        if isinstance(merchant, httpx.Response):
            return merchant
        for est in merchant.establishments.values():
            if (account := est.accounts.get(match.group(2))) is not None:
                return _ok(
                    {
                        "nit": merchant.nit,
                        "nombreComercio": merchant.name,
                        "idEstablecimiento": est.id,
                        "nombreEstablecimiento": est.name,
                        "cuenta": _account_wire(account),
                    }
                )
        return _error("15", "Número de cuenta no encontrada.")

    def _list(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        merchant = self._merchant(match.group(1))
        if isinstance(merchant, httpx.Response):
            return merchant
        return _ok(
            {
                "nit": merchant.nit,
                "nombreComercio": merchant.name,
                "establecimientos": [
                    {
                        "idEstablecimiento": e.id,
                        "nombreEstablecimiento": e.name,
                        "cuentas": [_account_wire(a) for a in e.accounts.values()],
                    }
                    for e in merchant.establishments.values()
                ],
            }
        )

    def _create(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        body = json.loads(request.content)
        merchant = self._merchant(body.get("nit", ""))
        if isinstance(merchant, httpx.Response):
            return merchant
        est_body = body["establecimiento"]
        est = merchant.establishments.setdefault(
            est_body["idEstablecimiento"], _Establishment(est_body["idEstablecimiento"], "SUCURSAL")
        )
        created = []
        for spec in est_body["cuenta"]:
            number = str(next(self._numbers))
            alias = f"{spec['alias']} - {spec['rubro']}"
            est.accounts[number] = _Account(number, alias)
            created.append({"numeroCuenta": number, "alias": alias})
        return _ok(
            {
                "nit": merchant.nit,
                "nombreComercio": merchant.name,
                "idEstablecimiento": est.id,
                "nombreEstablecimiento": est.name,
                "cuentas": created,
            }
        )

    def _set_status(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        body = json.loads(request.content)
        merchant = self._merchant(body.get("nit", ""))
        if isinstance(merchant, httpx.Response):
            return merchant
        result = []
        for change in body["cuentas"]:
            account = merchant.account(change["numeroCuenta"])
            if account is None:
                return _error("15", "Número de cuenta no encontrada.")
            if account.status == "CERRADA":
                return _error("05", "No se puede cambiar el estado de una cuenta cerrada.")
            if change["estado"] == "CERRADA" and account.available != 0:
                return _error("05", "La cuenta debe tener saldo 0 para cerrarse.")
            account.status = change["estado"]
            result.append({"numeroCuenta": account.number, "estado": account.status})
        return _ok(result)

    def _reconcile(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        body = json.loads(request.content)
        merchant = self._merchant(body.get("nit", ""))
        if isinstance(merchant, httpx.Response):
            return merchant
        rows = [m["row"] for m in merchant.movements if _in_range(m["row"], body)]
        accounts = [a for e in merchant.establishments.values() for a in e.accounts.values()]
        return _ok({"movimientos": rows, "saldos": [_balance_wire(a, "moneda") for a in accounts]})

    def _movements(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        body = json.loads(request.content)
        merchant = self.merchants.get(body.get("nit", ""))
        number = body["numeroCuenta"][0]
        if merchant is None or merchant.account(number) is None:
            return _error("99", "El comercio no tiene cuentas habilitadas.")
        kind = "C" if match.group(1) == "creditos" else "D"
        return _ok(
            [
                m["row"]
                for m in merchant.movements
                if m["account"] == number and m["kind"] == kind and _in_range(m["row"], body)
            ]
        )

    def _balances(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        body = json.loads(request.content)
        merchant = self._merchant(body.get("nit", ""))
        if isinstance(merchant, httpx.Response):
            return merchant
        rows = []
        for number in body["numeroCuentas"]:
            account = merchant.account(number)
            if account is None:
                return _error("15", "Número de cuenta no encontrada.")
            rows.append(_balance_wire(account, "tipoMoneda"))
        return _ok(rows)


def _account_wire(account: _Account) -> dict[str, str]:
    return {"numeroCuenta": account.number, "alias": account.alias, "estado": account.status}


def _balance_wire(account: _Account, currency_key: str) -> dict[str, Any]:
    return {
        "numeroCuenta": account.number,
        "estado": account.status,
        currency_key: "BOB",
        "saldoDisponible": float(account.available),
        "saldoContable": float(account.available + account.held),
        "saldoRetenido": float(account.held),
        "fechaUltimoCredito": account.last_credit_at,
        "fechaUltimoDebito": account.last_debit_at,
    }


def _in_range(row: dict[str, Any], body: dict[str, Any]) -> bool:
    day = row["fechaHoraTransaccion"][:10]
    return bool(body["fechaInicio"] <= day <= body["fechaFin"])


def _ok(data: Any) -> httpx.Response:
    return httpx.Response(
        200, json={"data": data, "code": "00", "errorCode": None, "errorMessage": ""}
    )


def _error(code: str, message: str, details: list[str] | None = None) -> httpx.Response:
    return httpx.Response(200, json={"data": details, "code": code, "errorMessage": message})
