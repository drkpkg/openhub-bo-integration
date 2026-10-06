"""QR operations declared against the native module. Each Op is checked
against the module's catalog at import time."""

from __future__ import annotations

from datetime import timedelta
from decimal import Decimal
from typing import Any

from openhub_bo.core import Handler, Native, Op, Webhook
from openhub_bo.core.models import to_amount, to_seconds

from . import _native
from .models import GeneratedQr, PaymentNotification, QrKind, QrStatusInfo

NATIVE = Native(_native)

GENERATE = Op(NATIVE, "qr.generate", GeneratedQr._from)
VERIFY = Op(NATIVE, "qr.verify", QrStatusInfo._from)
CANCEL = Op(NATIVE, "qr.cancel", QrStatusInfo._from)
PARSE_WEBHOOK = Handler(NATIVE, "qr.webhook.parse", PaymentNotification._from)


def generate_input(
    *,
    amount: Decimal | int | str,
    description: str,
    reference: str,
    establishment_id: int,
    establishment_name: str,
    expires_in: int | timedelta,
    webhook: Webhook,
    kind: QrKind | str,
) -> dict[str, Any]:
    return {
        "kind": QrKind(kind).value,
        "description": description,
        "amount": to_amount(amount),
        "reference": str(reference),
        "expires_in": to_seconds(expires_in),
        "establishment_id": establishment_id,
        "establishment_name": establishment_name,
        "webhook": webhook.to_wire(),
    }


def verify_input(reference: str, kind: QrKind | str) -> dict[str, Any]:
    return {"kind": QrKind(kind).value, "reference": str(reference)}


def cancel_input(reference: str) -> dict[str, Any]:
    return {"reference": str(reference)}
