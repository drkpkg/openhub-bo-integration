"""Merchant accounts, balances and movements."""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal
from enum import Enum
from typing import Any

from openhub_bo.core import PaymentStatus
from openhub_bo.core.models import decimal_or_none


class AccountStatus(str, Enum):
    ACTIVE = "active"
    BLOCKED = "blocked"
    SUSPENDED = "suspended"
    CLOSED = "closed"
    """Final; requires zero balance."""
    UNKNOWN = "unknown"


# -- inputs ------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class NewAccount:
    alias: str
    """What the account is used for (max 45 characters)."""
    category: str
    """``rubro`` (max 45 characters)."""

    def to_wire(self) -> dict[str, str]:
        return {"alias": self.alias, "category": self.category}


@dataclass(frozen=True, slots=True)
class StatusChange:
    account_number: str
    status: AccountStatus
    reason: str
    """``descripcionMotivo`` (max 50 characters)."""

    def to_wire(self) -> dict[str, str]:
        return {
            "account_number": self.account_number,
            "status": AccountStatus(self.status).value,
            "reason": self.reason,
        }


# -- outputs -----------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Account:
    number: str
    alias: str | None
    status: AccountStatus
    raw_status: str | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> Account:
        return cls(
            data["number"], data.get("alias"), AccountStatus(data["status"]), data.get("raw_status")
        )


@dataclass(frozen=True, slots=True)
class MerchantAccount:
    nit: str
    merchant_name: str | None
    establishment_id: int | None
    establishment_name: str | None
    account: Account

    @classmethod
    def _from(cls, data: dict[str, Any]) -> MerchantAccount:
        return cls(
            nit=data["nit"],
            merchant_name=data.get("merchant_name"),
            establishment_id=data.get("establishment_id"),
            establishment_name=data.get("establishment_name"),
            account=Account._from(data["account"]),
        )


@dataclass(frozen=True, slots=True)
class Establishment:
    id: int
    name: str | None
    accounts: list[Account]

    @classmethod
    def _from(cls, data: dict[str, Any]) -> Establishment:
        return cls(data["id"], data.get("name"), [Account._from(a) for a in data["accounts"]])


@dataclass(frozen=True, slots=True)
class MerchantAccounts:
    nit: str
    merchant_name: str | None
    establishments: list[Establishment]

    @property
    def accounts(self) -> list[Account]:
        return [a for e in self.establishments for a in e.accounts]

    @classmethod
    def _from(cls, data: dict[str, Any]) -> MerchantAccounts:
        return cls(
            data["nit"],
            data.get("merchant_name"),
            [Establishment._from(e) for e in data["establishments"]],
        )


@dataclass(frozen=True, slots=True)
class CreatedAccounts:
    nit: str
    merchant_name: str | None
    establishment_id: int
    establishment_name: str | None
    accounts: list[Account]

    @classmethod
    def _from(cls, data: dict[str, Any]) -> CreatedAccounts:
        return cls(
            nit=data["nit"],
            merchant_name=data.get("merchant_name"),
            establishment_id=data["establishment_id"],
            establishment_name=data.get("establishment_name"),
            accounts=[Account._from(a) for a in data["accounts"]],
        )


@dataclass(frozen=True, slots=True)
class StatusChanged:
    account_number: str
    status: AccountStatus
    raw_status: str

    @classmethod
    def _from(cls, data: dict[str, Any]) -> StatusChanged:
        return cls(data["account_number"], AccountStatus(data["status"]), data["raw_status"])


@dataclass(frozen=True, slots=True)
class Party:
    account: str | None
    document_id: str | None
    name: str | None
    bank_code: str | None
    bank_name: str | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> Party:
        return cls(
            data.get("account"),
            data.get("document_id"),
            data.get("name"),
            data.get("bank_code"),
            data.get("bank_name"),
        )


@dataclass(frozen=True, slots=True)
class Movement:
    transaction_id: str | None
    """Payouts only: the id you sent when authorising it."""
    operation_type: str
    """``PAYIN QR``, ``PAYIN ACH``, ``PAYOUT ACH``..."""
    status: PaymentStatus
    raw_status: str
    message: str | None
    transaction_at: str | None
    amount: Decimal
    fee: Decimal | None
    total: Decimal | None
    currency: str | None
    origin: Party
    destination: Party
    reference: str | None
    ach_order_number: str | None
    recipient_order_number: str | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> Movement:
        return cls(
            transaction_id=data.get("transaction_id"),
            operation_type=data["operation_type"],
            status=PaymentStatus(data["status"]),
            raw_status=data["raw_status"],
            message=data.get("message"),
            transaction_at=data.get("transaction_at"),
            amount=Decimal(data["amount"]),
            fee=decimal_or_none(data.get("fee")),
            total=decimal_or_none(data.get("total")),
            currency=data.get("currency"),
            origin=Party._from(data["origin"]),
            destination=Party._from(data["destination"]),
            reference=data.get("reference"),
            ach_order_number=data.get("ach_order_number"),
            recipient_order_number=data.get("recipient_order_number"),
        )


@dataclass(frozen=True, slots=True)
class Balance:
    account_number: str
    status: AccountStatus
    raw_status: str | None
    currency: str | None
    available: Decimal
    booked: Decimal
    """``saldoContable``: total booked balance."""
    held: Decimal
    """``saldoRetenido``."""
    last_credit_at: str | None
    last_debit_at: str | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> Balance:
        return cls(
            account_number=data["account_number"],
            status=AccountStatus(data["status"]),
            raw_status=data.get("raw_status"),
            currency=data.get("currency"),
            available=Decimal(data["available"]),
            booked=Decimal(data["booked"]),
            held=Decimal(data["held"]),
            last_credit_at=data.get("last_credit_at"),
            last_debit_at=data.get("last_debit_at"),
        )


@dataclass(frozen=True, slots=True)
class Reconciliation:
    movements: list[Movement]
    balances: list[Balance]

    @classmethod
    def _from(cls, data: dict[str, Any]) -> Reconciliation:
        return cls(
            [Movement._from(m) for m in data["movements"]],
            [Balance._from(b) for b in data["balances"]],
        )
