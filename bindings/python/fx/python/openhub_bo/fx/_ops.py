"""FX operations declared against the native module (checked at import)."""

from __future__ import annotations

from datetime import timedelta
from decimal import Decimal
from typing import Any

from openhub_bo.core import Handler, Native, Op, Webhook
from openhub_bo.core.models import to_amount, to_seconds

from . import _native
from .models import (
    BinanceNotification,
    Currency,
    FxNotification,
    FxQr,
    FxStatusInfo,
    Glosa,
    VirtualAsset,
    glosa_wire,
)

NATIVE = Native(_native)

PIX_GENERATE = Op(NATIVE, "pix.generate", FxQr._from)
PIX_VERIFY = Op(NATIVE, "pix.verify", FxStatusInfo._from)
PIX_CANCEL = Op(NATIVE, "pix.cancel", FxStatusInfo._from)
CRYPTO_GENERATE = Op(NATIVE, "crypto.generate", FxQr._from)
CRYPTO_VERIFY = Op(NATIVE, "crypto.verify", FxStatusInfo._from)
BINANCE_GENERATE = Op(NATIVE, "binance.generate", FxQr._from)
BINANCE_VERIFY = Op(NATIVE, "binance.verify", FxStatusInfo._from)
PARSE_BINANCE_WEBHOOK = Handler(NATIVE, "binance.webhook.parse", BinanceNotification._from)
PARSE_FX_WEBHOOK = Handler(NATIVE, "fx.webhook.parse", FxNotification._from)


def _seconds_or_none(value: int | timedelta | None) -> int | None:
    return None if value is None else to_seconds(value)


def _common(
    amount: Decimal | int | str,
    glosa: Glosa | str,
    reference: str,
    webhook: Webhook,
    currency: Currency | str,
    channel: str,
) -> dict[str, Any]:
    return {
        "reference": str(reference),
        "glosa": glosa_wire(glosa),
        "amount": to_amount(amount),
        "currency": Currency(currency).value,
        "channel": channel,
        "webhook": webhook.to_wire(),
    }


def pix_input(
    *,
    amount: Decimal | int | str,
    glosa: Glosa | str,
    reference: str,
    payer_cpf: str,
    payer_phone: str,
    webhook: Webhook,
    currency: Currency | str,
    channel: str,
    expires_in: int | timedelta | None,
    payer_email: str | None,
    extra: str | None,
) -> dict[str, Any]:
    return _common(amount, glosa, reference, webhook, currency, channel) | {
        "payer_cpf": payer_cpf,
        "payer_phone": payer_phone,
        "expires_in": _seconds_or_none(expires_in),
        "payer_email": payer_email,
        "extra": extra,
    }


def crypto_input(
    *,
    amount: Decimal | int | str,
    glosa: Glosa | str,
    reference: str,
    asset: VirtualAsset | str,
    webhook: Webhook,
    currency: Currency | str,
    channel: str,
    expires_in: int | timedelta,
    extra: str | None,
) -> dict[str, Any]:
    return _common(amount, glosa, reference, webhook, currency, channel) | {
        "asset": VirtualAsset(asset).value,
        "expires_in": to_seconds(expires_in),
        "extra": extra,
    }


def binance_input(
    *,
    amount: Decimal | int | str,
    glosa: Glosa | str,
    reference: str,
    webhook: Webhook,
    currency: Currency | str,
    channel: str,
    expires_in: int | timedelta | None,
    extra: str | None,
) -> dict[str, Any]:
    return _common(amount, glosa, reference, webhook, currency, channel) | {
        "expires_in": _seconds_or_none(expires_in),
        "extra": extra,
    }


def ref_input(reference: str) -> dict[str, Any]:
    return {"reference": str(reference)}
