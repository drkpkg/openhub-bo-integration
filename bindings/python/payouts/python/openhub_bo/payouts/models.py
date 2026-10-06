"""Payout data types: QR payouts and ACH transfer batches."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import date
from decimal import Decimal
from typing import Any

from openhub_bo.core import PaymentStatus
from openhub_bo.core.models import decimal_or_none, to_amount


@dataclass(frozen=True, slots=True)
class Recipient:
    account: str | None
    document_id: str | None
    holder: str | None
    bank_code: str | None
    bank_name: str | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> Recipient:
        return cls(
            data.get("account"),
            data.get("document_id"),
            data.get("holder"),
            data.get("bank_code"),
            data.get("bank_name"),
        )

    def to_wire(self) -> dict[str, Any]:
        return {
            "account": self.account,
            "document_id": self.document_id,
            "holder": self.holder,
            "bank_code": self.bank_code,
            "bank_name": self.bank_name,
        }


@dataclass(frozen=True, slots=True)
class ScannedQr:
    """A decoded QR: show it to the user, then pass it to ``pay_qr``."""

    reference: str
    amount: Decimal
    """Zero for open-amount QRs (the payer chooses the amount)."""
    currency: str | None
    description: str | None
    recipient: Recipient
    expires_on: str | None

    @property
    def open_amount(self) -> bool:
        return self.amount == 0

    @classmethod
    def _from(cls, data: dict[str, Any]) -> ScannedQr:
        return cls(
            reference=data["reference"],
            amount=Decimal(data["amount"]),
            currency=data.get("currency"),
            description=data.get("description"),
            recipient=Recipient._from(data["recipient"]),
            expires_on=data.get("expires_on"),
        )

    def to_wire(self) -> dict[str, Any]:
        return {
            "reference": self.reference,
            "amount": str(self.amount),
            "currency": self.currency,
            "description": self.description,
            "recipient": self.recipient.to_wire(),
            "expires_on": self.expires_on,
        }


@dataclass(frozen=True, slots=True)
class Payout:
    reference: str
    transaction_id: str | None
    transaction_at: str | None
    status: PaymentStatus
    raw_status: str
    message: str | None
    amount: Decimal | None
    currency: str | None
    description: str | None
    source_account: str | None
    source_holder: str | None
    recipient: Recipient
    ach_order_number: str | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> Payout:
        return cls(
            reference=data["reference"],
            transaction_id=data.get("transaction_id"),
            transaction_at=data.get("transaction_at"),
            status=PaymentStatus(data["status"]),
            raw_status=data["raw_status"],
            message=data.get("message"),
            amount=decimal_or_none(data.get("amount")),
            currency=data.get("currency"),
            description=data.get("description"),
            source_account=data.get("source_account"),
            source_holder=data.get("source_holder"),
            recipient=Recipient._from(data["recipient"]),
            ach_order_number=data.get("ach_order_number"),
        )


@dataclass(frozen=True, slots=True)
class BatchTransfer:
    """One ACH transfer of a batch."""

    transaction_id: str
    """Your id, 3 to 14 characters, unique within the batch."""
    amount: Decimal | int | str
    source_account: str
    """Your ATC virtual account."""
    destination_account: str
    bank_code: str
    """See ``list_banks``."""
    branch_city: str
    """City of the source account: CBB, COB, LPZ, ORU, POT, SCZ, SUC, TJA, TRI."""
    description: str
    recipient_document_id: str
    recipient_name: str
    currency: str = "BOB"
    date: date | str = field(default_factory=date.today)
    """Today or later."""

    def to_wire(self) -> dict[str, Any]:
        return {
            "transaction_id": self.transaction_id,
            "amount": to_amount(self.amount),
            "date": self.date.isoformat() if isinstance(self.date, date) else self.date,
            "source_account": self.source_account,
            "destination_account": self.destination_account,
            "bank_code": self.bank_code,
            "branch_city": self.branch_city,
            "description": self.description,
            "recipient_document_id": self.recipient_document_id,
            "recipient_name": self.recipient_name,
            "currency": self.currency,
        }


@dataclass(frozen=True, slots=True)
class BatchTransferResult:
    transaction_id: str
    status: PaymentStatus
    """``ERROR`` for transfers rejected up front (e.g. bank not enabled)."""
    raw_status: str
    reference: str | None
    message: str | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> BatchTransferResult:
        return cls(
            data["transaction_id"],
            PaymentStatus(data["status"]),
            data["raw_status"],
            data.get("reference"),
            data.get("message"),
        )


@dataclass(frozen=True, slots=True)
class BatchAuthorization:
    batch_number: str
    process_id: str
    transfers: list[BatchTransferResult]

    @property
    def rejected(self) -> list[BatchTransferResult]:
        return [t for t in self.transfers if t.status is PaymentStatus.ERROR]

    @classmethod
    def _from(cls, data: dict[str, Any]) -> BatchAuthorization:
        return cls(
            data["batch_number"],
            data["process_id"],
            [BatchTransferResult._from(t) for t in data["transfers"]],
        )


@dataclass(frozen=True, slots=True)
class BatchTransferStatus:
    transaction_id: str | None
    reference: str | None
    status: PaymentStatus
    raw_status: str
    message: str | None
    source_account: str | None
    destination_account: str | None
    ach_number: str | None
    recipient_number: str | None
    recipient_document_id: str | None
    recipient_name: str | None
    transaction_at: str | None
    bank_code: str | None
    bank_name: str | None
    amount: Decimal | None
    currency: str | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> BatchTransferStatus:
        return cls(
            transaction_id=data.get("transaction_id"),
            reference=data.get("reference"),
            status=PaymentStatus(data["status"]),
            raw_status=data["raw_status"],
            message=data.get("message"),
            source_account=data.get("source_account"),
            destination_account=data.get("destination_account"),
            ach_number=data.get("ach_number"),
            recipient_number=data.get("recipient_number"),
            recipient_document_id=data.get("recipient_document_id"),
            recipient_name=data.get("recipient_name"),
            transaction_at=data.get("transaction_at"),
            bank_code=data.get("bank_code"),
            bank_name=data.get("bank_name"),
            amount=decimal_or_none(data.get("amount")),
            currency=data.get("currency"),
        )


@dataclass(frozen=True, slots=True)
class BatchStatus:
    batch_number: str | None
    transfers: list[BatchTransferStatus]

    @classmethod
    def _from(cls, data: dict[str, Any]) -> BatchStatus:
        return cls(
            data.get("batch_number"), [BatchTransferStatus._from(t) for t in data["transfers"]]
        )


@dataclass(frozen=True, slots=True)
class Bank:
    code: str
    name: str | None

    @classmethod
    def _from(cls, data: dict[str, Any]) -> Bank:
        return cls(data["code"], data.get("name"))


@dataclass(frozen=True, slots=True)
class BatchNotification:
    batch_number: str | None
    transfer: BatchTransferStatus

    def ack(self, *, processed: bool = True, detail: str | None = None) -> dict[str, Any]:
        """JSON body to answer the webhook with (HTTP 200)."""
        return {
            "nroLote": self.batch_number,
            "numeroReferencia": self.transfer.reference,
            "codigoRespuesta": "EXITOSO" if processed else "FALLIDO",
            "detalleRespuesta": detail,
        }

    @classmethod
    def _from(cls, data: dict[str, Any]) -> BatchNotification:
        return cls(data.get("batch_number"), BatchTransferStatus._from(data["transfer"]))
