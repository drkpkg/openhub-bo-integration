"""In-memory fake of the OpenHub FX QR APIs (PIX, Koibanx, Binance).

Mirrors the rules and error shapes observed in the sandbox (2026-10-06) and the
documented success payloads. PIX/Koibanx webhook payloads are undocumented, so
the ones :meth:`MockFx.pay` returns for them are an assumption.

>>> mock = MockFx()
>>> pix = mock.pix_client()
>>> qr = pix.generate_qr(...)
>>> call = mock.pay(qr.reference)
"""

from __future__ import annotations

import itertools
import json
import re
from dataclasses import dataclass
from datetime import datetime, timedelta
from decimal import ROUND_HALF_UP, Decimal
from typing import Any

import httpx

from openhub_bo.core import BOLIVIA_TZ
from openhub_bo.core.testing import MockGateway, RouteHandler

from .client import (
    AsyncBinanceClient,
    AsyncPixClient,
    AsyncVirtualAssetsClient,
    BinanceClient,
    PixClient,
    VirtualAssetsClient,
)

_PNG = (
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGMAAQAABQABDQottAAAAABJRU5ErkJggg=="
)
_JPEG = "/9j/4AAQSkZJRgABAgAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8U"


@dataclass(frozen=True, slots=True)
class _Product:
    name: str
    prefix: str
    verify: str
    paid_code: str
    disabled_detail: str
    converted_currency: str
    rate: Decimal  # merchant currency units per converted unit
    image: str


_PRODUCTS = {
    "pix": _Product(
        "pix",
        "/qr/pix/v2",
        "verifica",
        "PAID",
        "El ID de comercio no está habilitado para utilizar el servicio PIX - GQ-00005",
        "BRL",
        Decimal("1.25"),
        _PNG,
    ),
    "koibanx": _Product(
        "koibanx",
        "/qr/koibanx/v2",
        "estado",
        "SUCCESS",
        "Error interno al generar el QR KOIBANX - GQ-00006",
        "usdc",
        Decimal("12.72"),
        _PNG,
    ),
    "binance": _Product(
        "binance",
        "/qr/binance/v2",
        "verificar",
        "SUCCESS",
        "Error interno al resolver el comercio BINANCE - GQ-00000",
        "USDT",
        Decimal("12.04"),
        _JPEG,
    ),
}

_DETAILS = {
    "PENDING": "Estado en espera de la confirmación pago QR",
    "PAID": "Transacción pagada",
    "SUCCESS": "Transacción completada",
    "CANCELLED": "Transaccion cancelada",
    "EXPIRED": "Transacción expirada",
}


@dataclass(frozen=True, slots=True)
class WebhookCall:
    """The HTTP call ATC would make to the integrator's webhook."""

    url: str
    headers: dict[str, str]
    body: str


class MockFx(MockGateway):
    def __init__(self, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        self.qrs: dict[str, dict[str, Any]] = {}
        self.disabled: set[str] = set()
        self._refs = itertools.count(11195900)
        for product in _PRODUCTS.values():
            self.route("POST", rf"{product.prefix}/generar$", self._generate(product))
            self.route("GET", rf"{product.prefix}/{product.verify}/([^/]+)$", self._verify(product))
        self.route("GET", r"/qr/pix/v2/cancela/([^/]+)$", self._pix_cancel)

    # -- factories -----------------------------------------------------------------

    def pix_client(self, **kw: Any) -> PixClient:
        return PixClient(self.session(**kw))

    def virtual_assets_client(self, **kw: Any) -> VirtualAssetsClient:
        return VirtualAssetsClient(self.session(**kw))

    def binance_client(self, **kw: Any) -> BinanceClient:
        return BinanceClient(self.session(**kw))

    def async_pix_client(self, **kw: Any) -> AsyncPixClient:
        return AsyncPixClient(self.async_session(**kw))

    def async_virtual_assets_client(self, **kw: Any) -> AsyncVirtualAssetsClient:
        return AsyncVirtualAssetsClient(self.async_session(**kw))

    def async_binance_client(self, **kw: Any) -> AsyncBinanceClient:
        return AsyncBinanceClient(self.async_session(**kw))

    # -- scenario controls -----------------------------------------------------------

    def disable(self, product: str) -> None:
        """Simulates a merchant not enabled for ``product`` (``pix``, ``koibanx``, ``binance``)."""
        self.disabled.add(product)

    def pay(self, reference: str) -> WebhookCall:
        """Marks the QR as paid; returns the webhook call ATC would send."""
        qr = self.qrs[reference]
        product = _PRODUCTS[qr["product"]]
        qr["code"] = product.paid_code
        now = datetime.now(BOLIVIA_TZ).replace(tzinfo=None, microsecond=0).isoformat()
        payload: dict[str, Any]
        if product.name == "binance":
            payload = {
                "numeroReferencia": qr["body"]["numeroReferencia"],
                "estado": "00",
                "transacciones": {
                    "monto": qr["body"]["monto"],
                    "moneda": qr["body"]["moneda"],
                    "fechaHoraTransaccion": now,
                    "cliente": {"nombreCliente": "", "ciCliente": ""},
                },
            }
        else:  # assumption: undocumented
            payload = {
                "numeroReferencia": reference,
                "codigoRespuesta": product.paid_code,
                "monto": qr["body"]["monto"],
                "moneda": qr["body"]["moneda"],
            }
        webhook = qr["body"]["webhook"]
        return WebhookCall(
            url=webhook["url"],
            headers={"Content-Type": "application/json", webhook["key"]: webhook["value"]},
            body=json.dumps(payload, ensure_ascii=False),
        )

    def expire(self, reference: str) -> None:
        self.qrs[reference]["code"] = "EXPIRED"

    # -- routes ------------------------------------------------------------------------

    def _generate(self, product: _Product) -> RouteHandler:
        def handler(request: httpx.Request, match: re.Match[str]) -> httpx.Response:
            body = json.loads(request.content)
            if product.name == "koibanx":
                missing = [
                    f for f in ("webhook", "moneda", "monto", "numeroReferencia") if not body.get(f)
                ]
                if missing:
                    return _validation(
                        [
                            {"field": f, "message": f"{f} es requerido", "code": "REQUIRED_FIELD"}
                            for f in missing
                        ]
                    )
                if body["moneda"] == "BOB" and Decimal(str(body["monto"])) < 50:
                    return _validation(
                        [
                            {
                                "field": "monto",
                                "message": "El monto mínimo permitido es 50",
                                "code": "INVALID_VALUE",
                            }
                        ]
                    )
                if not 180 <= int(body.get("tiempoVencimientoQR", 0)) <= 600:
                    return _flat_error(
                        body, "tiempoVencimientoQR debe estar entre 180 y 600 segundos"
                    )
            else:
                required = ["glosa", "moneda", "webhook", "monto", "canal", "numeroReferencia"]
                if product.name == "pix":
                    required.append("cpf")
                missing = [f for f in required if not body.get(f)]
                if missing:
                    return _gateway_error(
                        400,
                        "EG-00002",
                        "Errores de validación",
                        [f"{f} es requerido" for f in missing],
                    )
            if product.name in self.disabled:
                return _flat_error(body, product.disabled_detail)

            reference = str(next(self._refs))
            self.qrs[reference] = {"product": product.name, "code": "PENDING", "body": body}
            amount = Decimal(str(body["monto"]))
            expires = datetime.now(BOLIVIA_TZ) + timedelta(minutes=5)
            return httpx.Response(
                200,
                json={
                    "codigoRespuesta": "PENDING",
                    "detalleRespuesta": "Se generó cobro QR con éxito",
                    "moneda": body["moneda"],
                    "monto": body["monto"],
                    "numeroReferencia": reference,
                    "origenNumeroReferencia": body["numeroReferencia"],
                    "imagen": product.image,
                    "montoConversion": float(_converted(amount, product)),
                    "monedaConversion": product.converted_currency,
                    "tipoCambio": float(product.rate),
                    "qrExpiracion": expires.replace(tzinfo=None).isoformat(timespec="milliseconds"),
                },
            )

        return handler

    def _verify(self, product: _Product) -> RouteHandler:
        def handler(request: httpx.Request, match: re.Match[str]) -> httpx.Response:
            reference = match.group(1)
            qr = self.qrs.get(reference)
            if qr is None or qr["product"] != product.name:
                if product.name == "koibanx":
                    return httpx.Response(
                        200,
                        json={
                            "codigoRespuesta": "ERROR",
                            "detalleRespuesta": "Transacción no encontrada",
                            "data": None,
                        },
                    )
                return _gateway_error(500, "EG-00001", "Transacción no encontrada")
            amount = Decimal(str(qr["body"]["monto"]))
            return httpx.Response(
                200,
                json={
                    "codigoRespuesta": qr["code"],
                    "detalleRespuesta": _DETAILS[qr["code"]],
                    "data": {
                        "numeroReferencia": reference,
                        "monto": qr["body"]["monto"],
                        "moneda": qr["body"]["moneda"],
                        "montoConversion": float(_converted(amount, product)),
                        "monedaConversion": product.converted_currency,
                        "tipoCambio": float(product.rate),
                        **({"reversa": None} if product.name == "pix" else {}),
                    },
                },
            )

        return handler

    def _pix_cancel(self, request: httpx.Request, match: re.Match[str]) -> httpx.Response:
        reference = match.group(1)
        qr = self.qrs.get(reference)
        if qr is None or qr["product"] != "pix":
            return _gateway_error(500, "EG-00001", "Transacción no encontrada")
        if qr["code"] != "PAID":
            return _gateway_error(
                500,
                "EG-00001",
                "Cancelación no permitida: el estado actual del QR no es 'aprobado'.",
            )
        qr["code"] = "CANCELLED"
        amount = Decimal(str(qr["body"]["monto"]))
        return httpx.Response(
            200,
            json={
                "codigoRespuesta": "CANCELLED",
                "detalleRespuesta": "Transaccion cancelada",
                "data": {
                    "monto": qr["body"]["monto"],
                    "moneda": qr["body"]["moneda"],
                    "montoConversion": str(_converted(amount, _PRODUCTS["pix"])),
                    "monedaConversion": "BRL",
                    "fechaSolicitud": datetime.now(BOLIVIA_TZ).isoformat(timespec="milliseconds"),
                },
            },
        )


def _converted(amount: Decimal, product: _Product) -> Decimal:
    return (amount / product.rate).quantize(Decimal("0.00000001"), rounding=ROUND_HALF_UP)


def _validation(errors: list[dict[str, str]]) -> httpx.Response:
    return httpx.Response(
        400, json={"success": False, "message": "Error de validación", "errors": errors}
    )


def _gateway_error(
    status: int, code: str, message: str, data: list[str] | None = None
) -> httpx.Response:
    return httpx.Response(
        status, json={"data": data, "error": True, "code": code, "message": message}
    )


def _flat_error(body: dict[str, Any], detail: str) -> httpx.Response:
    return httpx.Response(
        200,
        json={
            "codigoRespuesta": "ERROR",
            "detalleRespuesta": detail,
            "moneda": body.get("moneda"),
            "monto": body.get("monto"),
            "numeroReferencia": None,
            "origenNumeroReferencia": None,
            "imagen": "",
            "montoConversion": 0,
            "monedaConversion": "",
            "tipoCambio": 0,
            "qrExpiracion": None,
        },
    )
