"""Payouts from your Red Enlace (ATC) OpenHub account: pay third-party
interoperable QRs and send ACH transfer batches. Moves real money in production."""

from ._ops import NATIVE
from .client import AsyncPayoutsClient, PayoutsClient
from .models import (
    Bank,
    BatchAuthorization,
    BatchNotification,
    BatchStatus,
    BatchTransfer,
    BatchTransferResult,
    BatchTransferStatus,
    Payout,
    Recipient,
    ScannedQr,
)
from .webhook import parse_batch_webhook

__version__ = NATIVE.version

__all__ = [
    "AsyncPayoutsClient",
    "Bank",
    "BatchAuthorization",
    "BatchNotification",
    "BatchStatus",
    "BatchTransfer",
    "BatchTransferResult",
    "BatchTransferStatus",
    "Payout",
    "PayoutsClient",
    "Recipient",
    "ScannedQr",
    "__version__",
    "parse_batch_webhook",
]
