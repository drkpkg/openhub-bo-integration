"""In-memory fake of the OpenHub QR APIs, for your test suite and ours.

>>> mock = MockOpenHub()
>>> qr_client = mock.client()
>>> qr = qr_client.generate_qr(...)
>>> call = mock.pay(qr.reference)       # simulate the payer scanning the QR
>>> notification = parse_webhook(call.headers, call.body, webhook=...)

Mirrors the contract verified against the sandbox (envelopes, validation and
cancel errors). It is not a substitute for certifying against the real sandbox.
"""

from __future__ import annotations

import base64
import itertools
import json
import re
import uuid
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import Any

import httpx

from openhub_bo.core import BOLIVIA_TZ
from openhub_bo.core.testing import MockGateway, envelope_error, envelope_ok

from .client import AsyncQrClient, QrClient

# 1x1 transparent PNG.
_PNG = base64.b64encode(
    bytes.fromhex(
        "89504e470d0a1a0a0000000d4948445200000001000000010806000000"
        "1f15c4890000000d49444154789c6300010000000500010d0a2db40000000049454e44ae426082"
    )
).decode()


@dataclass(frozen=True, slots=True)
class WebhookCall:
    """The HTTP call ATC would make to the integrator's webhook."""

    url: str
    headers: dict[str, str]
    body: str


class MockOpenHub(MockGateway):
    def __init__(self, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        self.qrs: dict[str, dict[str, Any]] = {}
        self._refs = itertools.count(153980)
        self.route("POST", r"/qr/(simple|mld)/v2/generate$", self._generate)
        self.route("GET", r"/qr/(simple|mld)/v2/verify/([^/]+)$", self._verify)
        self.route("POST", r"/qr/simple/v2/cancel/([^/]+)$", self._cancel)

    def client(self, **kwargs: Any) -> QrClient:
        return QrClient(self.session(**kwargs))

    def async_client(self, **kwargs: Any) -> AsyncQrClient:
        return AsyncQrClient(self.async_session(**kwargs))

    # -- scenario controls -----------------------------------------------------

    def pay(
        self,
        reference: str,
        *,
        payer_name: str = "Juan Perez",
        payer_document: str = "12345678",
        payer_account: str = "1234567890",
        bank_code: str = "101",
        bank_name: str = "Banco Unión",
    ) -> WebhookCall:
        """Marks the QR as paid; returns the webhook call ATC would send."""
        qr = self.qrs[reference]
        now = datetime.now(BOLIVIA_TZ).replace(tzinfo=None, microsecond=0).isoformat()
        ach = str(uuid.uuid4().int)[:9]
        qr["estado"] = "PAGADO"
        qr["mensaje"] = "QR pagado"
        qr["clienteOrigen"] = {
            "nombreCliente": payer_name,
            "numeroCuenta": payer_account,
            "ciNitCliente": payer_document,
        }
        qr["bancoOrigen"] = {
            "numeroOrdenAch": ach,
            "codigoBanco": bank_code,
            "nombreBanco": bank_name,
            "fechaTransaccion": now,
        }
        webhook = qr["webhook"]
        payload = {
            "detalleRespuesta": "Transacción procesada correctamente",
            "codigoRespuesta": "SUCCESS",
            "numeroReferencia": reference,
            "monto": qr["monto"],
            "fechaHoraTransaccion": now,
            "moneda": qr["moneda"],
            "clienteOrigen": {
                "ciCliente": payer_document,
                "nombreCliente": payer_name,
                "numeroCuenta": payer_account,
            },
            "bancoOrigen": {
                "codigoBanco": bank_code,
                "nombreBanco": bank_name,
                "numeroOrdenAch": ach,
            },
        }
        return WebhookCall(
            url=webhook["url"],
            headers={"Content-Type": "application/json", webhook["key"]: webhook["value"]},
            body=json.dumps(payload, ensure_ascii=False),
        )

    def expire(self, reference: str) -> None:
        self.qrs[reference]["estado"] = "EXPIRADO"

    # -- routes ------------------------------------------------------------------

    def _generate(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        body = json.loads(request.content)
        errors = []
        if not body.get("webhook"):
            errors.append(
                {
                    "field": "webhook",
                    "message": "La configuración de webhook es requerida",
                    "code": "REQUIRED_FIELD",
                }
            )
        if not str(body.get("numeroReferencia", "")).isdigit():
            errors.append(
                {
                    "field": "numeroReferencia",
                    "message": "El número de referencia debe contener solo dígitos numéricos",
                    "code": "INVALID_FORMAT",
                }
            )
        if errors:
            return envelope_error(400, "Error de validación", errors)
        reference = str(next(self._refs))
        expires = datetime.now(BOLIVIA_TZ) + timedelta(seconds=body["vigencia"])
        self.qrs[reference] = {
            **body,
            "kind": match.group(1),
            "estado": "PENDIENTE",
            "mensaje": "QR generado, esperando pago",
        }
        return envelope_ok(
            "QR generado exitosamente",
            {
                "numeroReferencia": reference,
                "estado": "PENDIENTE",
                "fechaExpiracion": expires.replace(tzinfo=None).isoformat(),
                "moneda": body["moneda"],
                "monto": body["monto"],
                "numeroReferenciaOriginante": body["numeroReferencia"],
                "qr": _PNG,
            },
        )

    def _verify(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        reference = match.group(2)
        qr = self.qrs.get(reference)
        if qr is None:
            return _not_found(reference, "Error al consultar estado de QR")
        empty_payer = {"nombreCliente": "", "numeroCuenta": "", "ciNitCliente": ""}
        empty_bank = {
            "numeroOrdenAch": "",
            "codigoBanco": "",
            "nombreBanco": "",
            "fechaTransaccion": "",
        }
        return envelope_ok(
            "Estado de QR consultado exitosamente",
            {
                "estado": qr["estado"],
                "mensaje": qr["mensaje"],
                "importe": qr["monto"],
                "moneda": qr["moneda"],
                "numeroReferenciaOriginante": qr["numeroReferencia"],
                "numeroReferencia": reference,
                "clienteOrigen": qr.get("clienteOrigen", empty_payer),
                "bancoOrigen": qr.get("bancoOrigen", empty_bank),
            },
        )

    def _cancel(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        reference = match.group(1)
        qr = self.qrs.get(reference)
        if qr is None:
            return _not_found(reference, "Error al anular QR")
        if qr["estado"] != "PENDIENTE":
            return envelope_error(
                409,
                "Error al anular QR",
                [
                    {
                        "field": "numeroReferencia",
                        "message": (
                            f"El QR ya está en estado {qr['estado']}, no se puede anular "
                            "(solo se anulan QR en estado PENDIENTE)"
                        ),
                        "code": "ESTADO_INVALIDO",
                    }
                ],
            )
        qr["estado"] = "CANCELADO"
        qr["mensaje"] = "Transacción cancelada"
        return envelope_ok(
            "Transacción cancelada",
            {
                "estado": "CANCELADO",
                "mensaje": "Transacción cancelada",
                "importe": qr["monto"],
                "moneda": qr["moneda"],
                "numeroReferenciaOriginante": qr["numeroReferencia"],
                "numeroReferencia": reference,
            },
        )


def _not_found(reference: str, message: str) -> httpx.Response:
    return envelope_error(
        404,
        message,
        [
            {
                "field": "numeroReferencia",
                "message": f"Transacción no encontrada con número de referencia: {reference}",
                "code": "TRANSACCION_NO_ENCONTRADA",
            }
        ],
    )
