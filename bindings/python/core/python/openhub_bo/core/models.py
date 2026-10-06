"""Types shared by every product package."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from enum import Enum

BOLIVIA_TZ = timezone(timedelta(hours=-4), "BOT")
"""OpenHub timestamps carry no offset; they are Bolivia time (verified in sandbox)."""


class Environment(str, Enum):
    DEVELOPMENT = "development"
    SANDBOX = "sandbox"
    PRODUCTION = "production"


class PaymentStatus(str, Enum):
    """Status shared by every product family; ``raw_status`` keeps the original."""

    PENDING = "pending"
    PROCESSING = "processing"
    PAID = "paid"
    CANCELLED = "cancelled"
    EXPIRED = "expired"
    REJECTED = "rejected"
    REVERSED = "reversed"
    ERROR = "error"
    UNKNOWN = "unknown"

    @property
    def is_final(self) -> bool:
        return self not in (
            PaymentStatus.PENDING,
            PaymentStatus.PROCESSING,
            PaymentStatus.UNKNOWN,
        )


@dataclass(frozen=True, slots=True)
class Webhook:
    """Where ATC must POST notifications.

    ATC sends ``key: value`` as a request header; it is the only way to
    authenticate the call, so use a long random ``value``.
    """

    url: str
    value: str = field(repr=False)
    key: str = "x-api-key"

    def to_wire(self) -> dict[str, str]:
        return {"url": self.url, "key": self.key, "value": self.value}


# -- conversion helpers for product packages ---------------------------------


def to_amount(amount: Decimal | int | str) -> str:
    """Amounts cross the native boundary as exact decimal strings."""
    if isinstance(amount, bool) or not isinstance(amount, (Decimal, int, str)):
        raise TypeError(
            f"amount must be Decimal, int or str, not {type(amount).__name__} "
            "(floats are rejected to avoid rounding errors)"
        )
    return str(amount)


def to_seconds(value: int | timedelta) -> int:
    if isinstance(value, timedelta):
        return int(value.total_seconds())
    return int(value)


def decimal_or_none(value: str | None) -> Decimal | None:
    return None if value is None else Decimal(value)


def local_datetime(value: str | None) -> datetime | None:
    """Parses an OpenHub timestamp, assuming Bolivia time when it has no offset."""
    if not value:
        return None
    parsed = datetime.fromisoformat(value)
    return parsed if parsed.tzinfo else parsed.replace(tzinfo=BOLIVIA_TZ)
