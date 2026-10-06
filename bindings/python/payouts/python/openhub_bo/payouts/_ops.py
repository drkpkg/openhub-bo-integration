"""Payout operations declared against the native module."""

from __future__ import annotations

import uuid
from collections.abc import Callable, Sequence
from decimal import Decimal
from typing import Any, TypeVar

from openhub_bo.core import Handler, Native, Op
from openhub_bo.core.models import to_amount

from . import _native
from .models import (
    Bank,
    BatchAuthorization,
    BatchNotification,
    BatchStatus,
    BatchTransfer,
    Payout,
    ScannedQr,
)

NATIVE = Native(_native)
T = TypeVar("T")


def _list_of(decode: Callable[[dict[str, Any]], T]) -> Callable[[list[dict[str, Any]]], list[T]]:
    return lambda rows: [decode(row) for row in rows]


SCAN = Op(NATIVE, "payouts.scan", ScannedQr._from)
PAY = Op(NATIVE, "payouts.pay", Payout._from)
STATUS = Op(NATIVE, "payouts.status", Payout._from)
AUTHORIZE = Op(NATIVE, "batch.authorize", BatchAuthorization._from)
BATCH_STATUS = Op(NATIVE, "batch.status", BatchStatus._from)
BANKS: Op[list[Bank]] = Op(NATIVE, "batch.banks", _list_of(Bank._from))
PARSE_WEBHOOK = Handler(NATIVE, "batch.webhook.parse", BatchNotification._from)


def pay_input(
    scanned: ScannedQr,
    source_account: str,
    transaction_id: str,
    amount: Decimal | int | str | None,
    description: str | None,
) -> dict[str, Any]:
    return {
        "scanned": scanned.to_wire(),
        "source_account": str(source_account),
        "transaction_id": transaction_id,
        "amount": None if amount is None else to_amount(amount),
        "description": description,
    }


def authorize_input(
    branch_code: str,
    transfers: Sequence[BatchTransfer],
    webhook_url: str,
    webhook_token: str,
    process_id: str | None,
) -> dict[str, Any]:
    return {
        "branch_code": str(branch_code),
        "process_id": process_id or str(uuid.uuid4()),
        "webhook_url": webhook_url,
        "webhook_token": webhook_token,
        "transfers": [t.to_wire() for t in transfers],
    }


def batch_query(
    branch_code: str, process_id: str, batch_number: str | None, transaction_id: str | None
) -> dict[str, Any]:
    return {
        "branch_code": str(branch_code),
        "process_id": process_id,
        "batch_number": batch_number,
        "transaction_id": transaction_id,
    }
