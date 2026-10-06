"""PIX / virtual assets (Koibanx) / Binance data types."""

from __future__ import annotations

import base64
from dataclasses import dataclass, field
from decimal import Decimal
from enum import Enum
from typing import Any

from openhub_bo.core import PaymentStatus
from openhub_bo.core.models import decimal_or_none


class Currency(str, Enum):
    """Currency the merchant charges in (the payer pays the converted amount)."""

    BOB = "BOB"
    USD = "USD"


class VirtualAsset(str, Enum):
    """Settlement asset for Koibanx QRs."""

    USDT = "usdt"
    USDC = "usdc"
    BK = "bk"


@dataclass(frozen=True, slots=True)
class Glosa:
    """``glosa`` in the ``branch_code|branch_name|category|description`` format
    the FX APIs require (``category`` is e.g. an MCC like ``7011`` or ``MISCELANEAS``)."""

    branch_code: str
    branch_name: str
    category: str
    description: str

    def to_wire(self) -> str:
        return "|".join((self.branch_code, self.branch_name, self.category, self.description))


def glosa_wire(glosa: Glosa | str) -> str:
    return glosa.to_wire() if isinstance(glosa, Glosa) else glosa


@dataclass(frozen=True, slots=True)
class FxQr:
    reference: str
    """ATC's reference; use it for status queries."""
    merchant_reference: str | None
    status: PaymentStatus
    raw_status: str
    detail: str | None
    amount: Decimal
    currency: str
    converted_amount: Decimal | None
    """What the payer pays, in ``converted_currency`` (BRL, USDT, USDC...)."""
    converted_currency: str | None
    exchange_rate: Decimal | None
    expires_at: str | None
    """``qrExpiracion`` as sent; its format differs per product."""
    image_mime: str
    image_base64: str = field(repr=False)

    @property
    def image(self) -> bytes:
        return base64.b64decode(self.image_base64)

    @property
    def data_uri(self) -> str:
        """Ready for ``<img src="...">`` (PNG for PIX/Koibanx, JPEG for Binance)."""
        return f"data:{self.image_mime};base64,{self.image_base64}"

    @classmethod
    def _from(cls, data: dict[str, Any]) -> FxQr:
        return cls(
            reference=data["reference"],
            merchant_reference=data.get("merchant_reference"),
            status=PaymentStatus(data["status"]),
            raw_status=data["raw_status"],
            detail=data.get("detail"),
            amount=Decimal(data["amount"]),
            currency=data["currency"],
            converted_amount=decimal_or_none(data.get("converted_amount")),
            converted_currency=data.get("converted_currency"),
            exchange_rate=decimal_or_none(data.get("exchange_rate")),
            expires_at=data.get("expires_at"),
            image_mime=data["image_mime"],
            image_base64=data["image_base64"],
        )


@dataclass(frozen=True, slots=True)
class FxStatusInfo:
    reference: str
    status: PaymentStatus
    raw_status: str
    detail: str | None
    amount: Decimal | None
    currency: str | None
    converted_amount: Decimal | None
    converted_currency: str | None
    exchange_rate: Decimal | None
    reversal: dict[str, Any] | None
    """PIX: reversal/chargeback details, if any."""
    requested_at: str | None
    """PIX cancel: when the request was processed."""

    @classmethod
    def _from(cls, data: dict[str, Any]) -> FxStatusInfo:
        return cls(
            reference=data["reference"],
            status=PaymentStatus(data["status"]),
            raw_status=data["raw_status"],
            detail=data.get("detail"),
            amount=decimal_or_none(data.get("amount")),
            currency=data.get("currency"),
            converted_amount=decimal_or_none(data.get("converted_amount")),
            converted_currency=data.get("converted_currency"),
            exchange_rate=decimal_or_none(data.get("exchange_rate")),
            reversal=data.get("reversal"),
            requested_at=data.get("requested_at"),
        )


@dataclass(frozen=True, slots=True)
class BinanceNotification:
    reference: str
    status: PaymentStatus
    raw_status: str
    amount: Decimal | None
    currency: str | None
    transaction_at: str | None
    payer_name: str | None
    payer_document_id: str | None
    ack: dict[str, Any]
    """Answer the webhook with this JSON and HTTP 200; ATC requires it."""

    @classmethod
    def _from(cls, data: dict[str, Any]) -> BinanceNotification:
        return cls(
            reference=data["reference"],
            status=PaymentStatus(data["status"]),
            raw_status=data["raw_status"],
            amount=decimal_or_none(data.get("amount")),
            currency=data.get("currency"),
            transaction_at=data.get("transaction_at"),
            payer_name=data.get("payer_name"),
            payer_document_id=data.get("payer_document_id"),
            ack=data["ack"],
        )


@dataclass(frozen=True, slots=True)
class FxNotification:
    """PIX / Koibanx notification. Their payload is undocumented: fields are a
    best-effort reading and the full body is kept in ``payload``."""

    reference: str | None
    status: PaymentStatus
    raw_status: str | None
    amount: Decimal | None
    currency: str | None
    payload: dict[str, Any]

    @classmethod
    def _from(cls, data: dict[str, Any]) -> FxNotification:
        return cls(
            reference=data.get("reference"),
            status=PaymentStatus(data["status"]),
            raw_status=data.get("raw_status"),
            amount=decimal_or_none(data.get("amount")),
            currency=data.get("currency"),
            payload=data["payload"],
        )
