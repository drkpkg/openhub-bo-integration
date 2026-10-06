"""In-memory fake of the OpenHub payout APIs (QR payouts and ACH batches).

    mock = MockPayouts()
    mock.add_source_account("7010123451", balance="500")
    text = mock.add_qr(amount="100.50", holder="PROVEEDOR SRL")
    client = mock.client()
    scanned = client.scan_qr(text)
    payout = client.pay_qr(scanned, source_account="7010123451", transaction_id="PAY-1")

Error codes follow the sandbox and the docs: ``02`` validation, ``04`` not found,
``09`` invalid QR, ``13`` insufficient balance, ``15`` unknown account, ``96``
unconfirmed result (see :meth:`MockPayouts.next_payment_unconfirmed`).
"""

from __future__ import annotations

import itertools
import json
import re
from dataclasses import dataclass
from datetime import datetime
from decimal import Decimal
from typing import Any

import httpx

from openhub_bo.core import BOLIVIA_TZ
from openhub_bo.core.testing import MockGateway

from .client import AsyncPayoutsClient, PayoutsClient

_SYNC = "/payout/sync/v3/qr"
_ASYNC = "/payout/async/v3"


@dataclass(frozen=True, slots=True)
class BatchWebhookCall:
    """One notification ATC would POST for a settled batch transfer."""

    url: str
    token: str
    body: str


class MockPayouts(MockGateway):
    def __init__(self, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        self.qrs: dict[str, dict[str, Any]] = {}
        self.accounts: dict[str, Decimal] = {}
        self.payouts: dict[str, dict[str, Any]] = {}
        self.batches: dict[str, dict[str, Any]] = {}
        self.banks: dict[str, str] = {
            "1005": "",
            "1014": "Banco Nacional de Bolivia S.A.",
            "1018": "",
        }
        self._refs = itertools.count(547261006000003100)
        self._lots = itertools.count(2610060001)
        self._unconfirmed = False
        self.route("POST", rf"{_SYNC}/scan$", self._scan)
        self.route("POST", rf"{_SYNC}/confirm$", self._confirm)
        self.route("GET", rf"{_SYNC}/status/(\d+)$", self._payout_status)
        self.route("POST", rf"{_ASYNC}/lote/autorizar$", self._authorize)
        self.route("GET", rf"{_ASYNC}/lote/estado/([0-9a-f-]+)$", self._batch_status)
        self.route("POST", rf"{_ASYNC}/bancos$", self._banks)

    def client(self, **kw: Any) -> PayoutsClient:
        return PayoutsClient(self.session(**kw))

    def async_client(self, **kw: Any) -> AsyncPayoutsClient:
        return AsyncPayoutsClient(self.async_session(**kw))

    # -- seeding / scenario controls ------------------------------------------------------

    def add_source_account(self, number: str, *, balance: Decimal | str = "1000") -> None:
        self.accounts[number] = Decimal(balance)

    def add_qr(
        self,
        *,
        amount: Decimal | str = "0",
        description: str = "",
        holder: str = "PERSONA NATURAL",
        account: str = "1311713043",
        bank_code: str = "1918",
    ) -> str:
        """Registers a payable QR and returns its text content."""
        reference = str(next(self._refs))
        text = f"MOCKQR{reference}"
        self.qrs[text] = {
            "importe": float(Decimal(amount)),
            "moneda": "BOB",
            "glosa": description,
            "numeroReferencia": reference,
            "cuentaDestino": account,
            "ciNitDestino": "2274887",
            "titularDestino": holder,
            "codigoBancoDestino": bank_code,
            "nombreBancoDestino": "",
            "fechaVencimiento": datetime.now(BOLIVIA_TZ).date().isoformat(),
        }
        return text

    def next_payment_unconfirmed(self) -> None:
        """The next ``pay_qr`` answers code 96 although the payout goes through."""
        self._unconfirmed = True

    def settle(self, process_id: str) -> list[BatchWebhookCall]:
        """Marks every pending transfer of a batch as paid; returns the webhooks."""
        batch = self.batches[process_id]
        calls = []
        for tx in batch["transfers"]:
            if tx["estado"] != "PENDIENTE":
                continue
            tx["estado"] = "PAGADO"
            tx["mensaje"] = "Transacción acreditada en cuenta destino"
            tx["fechaHoraTransaccion"] = _now()
            url, _, token = batch["webhookUrl"].partition("?token=")
            calls.append(
                BatchWebhookCall(
                    url, token, json.dumps({"nroLote": batch["nroLote"], **tx}, ensure_ascii=False)
                )
            )
        return calls

    # -- QR payouts --------------------------------------------------------------------

    def _scan(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        text = json.loads(request.content).get("imagen")
        if not text:
            return _code(400, "02", "El campo 'imagen' es obligatorio y no puede estar vacío")
        qr = self.qrs.get(text)
        if qr is None:
            return _code(200, "09", "El código QR no es válido.")
        return httpx.Response(200, json={"data": qr, "code": "00"})

    def _confirm(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        body = json.loads(request.content)
        qr = next(
            (q for q in self.qrs.values() if q["numeroReferencia"] == body["numeroReferencia"]),
            None,
        )
        if qr is None:
            return _code(200, "04", "Transacción no encontrada.")
        if body["cuentaOrigen"] not in self.accounts:
            return _code(200, "15", "Número de cuenta no encontrada.")
        amount = Decimal(str(qr["importe"])) or Decimal(str(body["importe"]))
        if self.accounts[body["cuentaOrigen"]] < amount:
            return _code(200, "13", "Saldo insuficiente.")
        self.accounts[body["cuentaOrigen"]] -= amount
        reference = str(next(self._refs))
        payout = {
            "numeroReferencia": reference,
            "transaccionId": body["transaccionId"],
            "fechaHoraTransaccion": _now(),
            "numOrdenAch": reference[-14:],
            "importe": float(amount),
            "moneda": "BOB",
            "estado": "APROBADA",
            "mensaje": "La transacción fue aprobada.",
            "glosa": qr["glosa"] or body.get("glosa"),
            "cuentaDestino": qr["cuentaDestino"],
            "titularDestino": qr["titularDestino"],
            "ciNitDestino": qr["ciNitDestino"],
            "nombreBancoDestino": qr["nombreBancoDestino"],
            "codigoBancoDestino": qr["codigoBancoDestino"],
            "cuentaOrigen": body["cuentaOrigen"],
            "titularOrigen": "alias comercio",
        }
        self.payouts[reference] = payout
        if self._unconfirmed:
            self._unconfirmed = False
            return _code(200, "96", "Error o resultado no confirmado en emisor \u2013 ATC")
        return httpx.Response(200, json={"data": payout, "code": "00"})

    def _payout_status(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        payout = self.payouts.get(match.group(1))
        if payout is None:
            return _code(200, "04", "Transacción no encontrada.")
        return httpx.Response(200, json={"data": payout, "code": "00"})

    # -- ACH batches -----------------------------------------------------------------------

    def _branch_ok(self, request: httpx.Request) -> bool:
        return bool(request.headers.get("branchCode"))

    def _authorize(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        if not self._branch_ok(request):
            return _code(400, "99", "Error interno del servidor", key="message")
        body = json.loads(request.content)
        lot = str(next(self._lots))
        transfers = []
        for tx in body["transacciones"]:
            if tx["codeBanco"] not in self.banks:
                transfers.append(
                    {
                        "transaccionId": tx["transaccionId"],
                        "estado": "ERROR",
                        "mensaje": "Codigo de banco no habilitado",
                    }
                )
                continue
            transfers.append(
                {
                    "transaccionId": tx["transaccionId"],
                    "numeroReferencia": str(next(self._refs)),
                    "estado": "PENDIENTE",
                    "mensaje": "Transacción en proceso.",
                    "cuentaOrigen": tx["cuentaOrigen"],
                    "cuentaDestino": tx["cuentaDestino"],
                    "ciCliente": tx["ciNitDestino"],
                    "nombreCliente": tx["titularDestino"],
                    "codigoBanco": tx["codeBanco"],
                    "nombreBanco": self.banks[tx["codeBanco"]],
                    "importe": tx["importe"],
                    "moneda": tx["tipoMoneda"],
                }
            )
        self.batches[body["processId"]] = {
            "nroLote": lot,
            "webhookUrl": body["webhookUrl"],
            "transfers": transfers,
        }
        summary = [
            {k: t[k] for k in ("transaccionId", "estado", "numeroReferencia", "mensaje") if k in t}
            for t in transfers
        ]
        return httpx.Response(
            200,
            json={
                "code": "00",
                "message": "success",
                "data": {"nroLote": lot, "processId": body["processId"], "transacciones": summary},
            },
        )

    def _batch_status(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        if not self._branch_ok(request):
            return _code(400, "99", "Error interno del servidor", key="message")
        batch = self.batches.get(match.group(1))
        lot, tx_id = request.url.params.get("nroLote"), request.url.params.get("transaccionId")
        if batch is None:
            return _code(200, "04", "Transacción no encontrada", key="message")
        rows = [
            t
            for t in batch["transfers"]
            if (lot and lot == batch["nroLote"]) or (tx_id and t["transaccionId"] == tx_id)
        ]
        if not rows:
            return _code(200, "04", "Transacción no encontrada", key="message")
        return httpx.Response(
            200,
            json={
                "code": "00",
                "message": "Operación procesada correctamente",
                "nroLote": batch["nroLote"],
                "transacciones": rows,
            },
        )

    def _banks(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        if not self._branch_ok(request):
            return _code(400, "99", "Error interno del servidor", key="message")
        data = [{"codigoBanco": code, "descripcion": name} for code, name in self.banks.items()]
        return httpx.Response(200, json={"data": data, "code": "00", "message": "success"})


def _now() -> str:
    return datetime.now(BOLIVIA_TZ).replace(tzinfo=None, microsecond=0).isoformat()


def _code(status: int, code: str, message: str, *, key: str = "errorMessage") -> httpx.Response:
    return httpx.Response(status, json={"code": code, key: message, "data": None})
