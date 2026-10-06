"""QR Simple / MLD-BCB data types."""

from __future__ import annotations

import base64
from dataclasses import dataclass, field
from datetime import datetime
from decimal import Decimal
from enum import Enum
from typing import Any

from openhub_bo.core import PaymentStatus
from openhub_bo.core.models import decimal_or_none, local_datetime

QrStatus = PaymentStatus
"""Alias kept for QR-centric code."""


class QrKind(str, Enum):
    SIMPLE = "simple"
    """Interbank QR Simple."""
    MLD = "mld"
    """QR under the Banco Central de Bolivia standard (MLD-BCB)."""


@dataclass(frozen=True, slots=True)
class Payer:
    name: str | None
    account_number: str | None
    document_id: str | None
    """CI or NIT."""

    @classmethod
    def _from(cls, data: dict[str, Any] | None) -> Payer | None:
        if not data:
            return None
        return cls(data.get("name"), data.get("account_number"), data.get("document_id"))


@dataclass(frozen=True, slots=True)
class PayerBank:
    bank_code: str | None
    bank_name: str | None
    ach_order_number: str | None
    transaction_date: str | None

    @classmethod
    def _from(cls, data: dict[str, Any] | None) -> PayerBank | None:
        if not data:
            return None
        return cls(
            data.get("bank_code"),
            data.get("bank_name"),
            data.get("ach_order_number"),
            data.get("transaction_date"),
        )


@dataclass(frozen=True, slots=True)
class GeneratedQr:
    kind: QrKind
    """Simple and MLD-BCB references are separate: keep it with the reference."""
    reference: str
    """ATC's reference. Store it: status queries and webhooks use it."""
    merchant_reference: str | None
    """The reference you sent (``numeroReferenciaOriginante``)."""
    status: PaymentStatus
    raw_status: str
    expires_at: datetime | None
    currency: str
    amount: Decimal
    qr_image_base64: str = field(repr=False)

    @property
    def qr_png(self) -> bytes:
        return base64.b64decode(self.qr_image_base64)

    @property
    def qr_data_uri(self) -> str:
        """Ready for ``<img src="...">``."""
        return f"data:image/png;base64,{self.qr_image_base64}"

    @classmethod
    def _from(cls, data: dict[str, Any]) -> GeneratedQr:
        return cls(
            kind=QrKind(data["kind"]),
            reference=data["reference"],
            merchant_reference=data.get("merchant_reference"),
            status=PaymentStatus(data["status"]),
            raw_status=data["raw_status"],
            expires_at=local_datetime(data.get("expires_at")),
            currency=data["currency"],
            amount=Decimal(data["amount"]),
            qr_image_base64=data["qr_image_base64"],
        )


@dataclass(frozen=True, slots=True)
class QrStatusInfo:
    kind: QrKind
    reference: str
    merchant_reference: str | None
    status: PaymentStatus
    raw_status: str
    message: str | None
    amount: Decimal | None
    currency: str | None
    payer: Payer | None
    payer_bank: PayerBank | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> QrStatusInfo:
        return cls(
            kind=QrKind(data["kind"]),
            reference=data["reference"],
            merchant_reference=data.get("merchant_reference"),
            status=PaymentStatus(data["status"]),
            raw_status=data["raw_status"],
            message=data.get("message"),
            amount=decimal_or_none(data.get("amount")),
            currency=data.get("currency"),
            payer=Payer._from(data.get("payer")),
            payer_bank=PayerBank._from(data.get("payer_bank")),
        )


@dataclass(frozen=True, slots=True)
class PaymentNotification:
    reference: str
    """ATC's reference, as returned by ``generate_qr``."""
    amount: Decimal
    currency: str | None
    status: PaymentStatus
    success: bool
    response_code: str
    response_detail: str | None
    transaction_at: datetime | None
    payer: Payer | None
    payer_bank: PayerBank | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> PaymentNotification:
        return cls(
            reference=data["reference"],
            amount=Decimal(data["amount"]),
            currency=data.get("currency"),
            status=PaymentStatus(data["status"]),
            success=data["success"],
            response_code=data["response_code"],
            response_detail=data.get("response_detail"),
            transaction_at=local_datetime(data.get("transaction_at")),
            payer=Payer._from(data.get("payer")),
            payer_bank=PayerBank._from(data.get("payer_bank")),
        )
