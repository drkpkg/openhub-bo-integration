"""Payouts: pay third-party QRs and send ACH transfer batches from your ATC
account. **These move real money in production.**

QR flow: ``scan_qr`` (show the recipient to the user) → ``pay_qr`` →
``get_payout``. ``pay_qr`` raises ``AmbiguousOutcomeError`` when OpenHub cannot
confirm the result (codes 94/96) or the response is lost: query ``get_payout``
before retrying.
"""

from __future__ import annotations

from collections.abc import Sequence
from decimal import Decimal

from openhub_bo.core import AsyncSession, Session

from . import _ops
from .models import Bank, BatchAuthorization, BatchStatus, BatchTransfer, Payout, ScannedQr


class PayoutsClient:
    def __init__(self, session: Session) -> None:
        self.session = session

    # -- QR payouts --------------------------------------------------------------------

    def scan_qr(self, qr_text: str) -> ScannedQr:
        """Decodes a QR from its **text content** (what a QR reader returns, not
        the image): recipient, amount and description."""
        return self.session.execute(_ops.SCAN, {"qr_text": qr_text})

    def pay_qr(
        self,
        scanned: ScannedQr,
        *,
        source_account: str,
        transaction_id: str,
        amount: Decimal | int | str | None = None,
        description: str | None = None,
    ) -> Payout:
        """Pays a scanned QR from ``source_account`` (your ATC account).

        ``amount`` only for open-amount QRs; ``description`` only when the QR has
        none. ``transaction_id`` is your unique id (≤ 32 characters).
        """
        return self.session.execute(
            _ops.PAY, _ops.pay_input(scanned, source_account, transaction_id, amount, description)
        )

    def get_payout(self, reference: str) -> Payout:
        return self.session.execute(_ops.STATUS, {"reference": str(reference)})

    # -- ACH batches ---------------------------------------------------------------------

    def authorize_batch(
        self,
        branch_code: str,
        transfers: Sequence[BatchTransfer],
        *,
        webhook_url: str,
        webhook_token: str,
        process_id: str | None = None,
    ) -> BatchAuthorization:
        """Submits a batch. ATC POSTs one notification per transfer to
        ``webhook_url?token=<webhook_token>`` (see ``parse_batch_webhook``).
        ``process_id`` defaults to a new UUID; keep it to query the batch."""
        return self.session.execute(
            _ops.AUTHORIZE,
            _ops.authorize_input(branch_code, transfers, webhook_url, webhook_token, process_id),
        )

    def get_batch_status(
        self,
        branch_code: str,
        process_id: str,
        *,
        batch_number: str | None = None,
        transaction_id: str | None = None,
    ) -> BatchStatus:
        """Give exactly one of ``batch_number`` or ``transaction_id``."""
        return self.session.execute(
            _ops.BATCH_STATUS,
            _ops.batch_query(branch_code, process_id, batch_number, transaction_id),
        )

    def list_banks(self, branch_code: str) -> list[Bank]:
        return self.session.execute(_ops.BANKS, {"branch_code": str(branch_code)})


class AsyncPayoutsClient:
    """Asyncio counterpart of :class:`PayoutsClient`."""

    def __init__(self, session: AsyncSession) -> None:
        self.session = session

    async def scan_qr(self, qr_text: str) -> ScannedQr:
        return await self.session.execute(_ops.SCAN, {"qr_text": qr_text})

    async def pay_qr(
        self,
        scanned: ScannedQr,
        *,
        source_account: str,
        transaction_id: str,
        amount: Decimal | int | str | None = None,
        description: str | None = None,
    ) -> Payout:
        """See :meth:`PayoutsClient.pay_qr`."""
        return await self.session.execute(
            _ops.PAY, _ops.pay_input(scanned, source_account, transaction_id, amount, description)
        )

    async def get_payout(self, reference: str) -> Payout:
        return await self.session.execute(_ops.STATUS, {"reference": str(reference)})

    async def authorize_batch(
        self,
        branch_code: str,
        transfers: Sequence[BatchTransfer],
        *,
        webhook_url: str,
        webhook_token: str,
        process_id: str | None = None,
    ) -> BatchAuthorization:
        """See :meth:`PayoutsClient.authorize_batch`."""
        return await self.session.execute(
            _ops.AUTHORIZE,
            _ops.authorize_input(branch_code, transfers, webhook_url, webhook_token, process_id),
        )

    async def get_batch_status(
        self,
        branch_code: str,
        process_id: str,
        *,
        batch_number: str | None = None,
        transaction_id: str | None = None,
    ) -> BatchStatus:
        return await self.session.execute(
            _ops.BATCH_STATUS,
            _ops.batch_query(branch_code, process_id, batch_number, transaction_id),
        )

    async def list_banks(self, branch_code: str) -> list[Bank]:
        return await self.session.execute(_ops.BANKS, {"branch_code": str(branch_code)})
