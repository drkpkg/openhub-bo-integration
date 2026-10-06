"""QR Simple / MLD-BCB clients. Thin by design: inputs are built in ``_ops``
and executed by the shared :class:`~openhub_bo.core.Session`."""

from __future__ import annotations

from datetime import timedelta
from decimal import Decimal

from openhub_bo.core import AsyncSession, Session, Webhook

from ._ops import CANCEL, GENERATE, VERIFY, cancel_input, generate_input, verify_input
from .models import GeneratedQr, QrKind, QrStatusInfo


class QrClient:
    """Collect payments with QR Simple or QR MLD-BCB.

    >>> with Session("client-id", "client-secret") as session:
    ...     qr = QrClient(session).generate_qr(amount="10.50", ...)
    """

    def __init__(self, session: Session) -> None:
        self.session = session

    def generate_qr(
        self,
        *,
        amount: Decimal | int | str,
        description: str,
        reference: str,
        establishment_id: int,
        establishment_name: str,
        expires_in: int | timedelta,
        webhook: Webhook,
        kind: QrKind | str = QrKind.SIMPLE,
    ) -> GeneratedQr:
        """Creates a collection QR.

        ``reference`` must be numeric and ``expires_in`` is in seconds. OpenHub
        requires a ``webhook`` for every QR. A lost response raises
        :class:`~openhub_bo.core.AmbiguousOutcomeError`: the QR may exist.
        """
        return self.session.execute(
            GENERATE,
            generate_input(
                amount=amount,
                description=description,
                reference=reference,
                establishment_id=establishment_id,
                establishment_name=establishment_name,
                expires_in=expires_in,
                webhook=webhook,
                kind=kind,
            ),
        )

    def get_qr_status(self, reference: str, *, kind: QrKind | str = QrKind.SIMPLE) -> QrStatusInfo:
        """``reference`` is :attr:`GeneratedQr.reference` (ATC's reference) and ``kind``
        must match :attr:`GeneratedQr.kind`: a MLD reference is not found as Simple."""
        return self.session.execute(VERIFY, verify_input(reference, kind))

    def cancel_qr(self, reference: str) -> QrStatusInfo:
        """Cancels a pending QR Simple and returns its new status.

        Only ``PENDING`` QRs can be cancelled; otherwise ``ApiError`` with
        ``code == "ESTADO_INVALIDO"`` (HTTP 409). MLD-BCB has no cancel operation
        (the gateway has no such route); MLD QRs simply expire.
        """
        return self.session.execute(CANCEL, cancel_input(reference))


class AsyncQrClient:
    """Asyncio counterpart of :class:`QrClient`."""

    def __init__(self, session: AsyncSession) -> None:
        self.session = session

    async def generate_qr(
        self,
        *,
        amount: Decimal | int | str,
        description: str,
        reference: str,
        establishment_id: int,
        establishment_name: str,
        expires_in: int | timedelta,
        webhook: Webhook,
        kind: QrKind | str = QrKind.SIMPLE,
    ) -> GeneratedQr:
        """See :meth:`QrClient.generate_qr`."""
        return await self.session.execute(
            GENERATE,
            generate_input(
                amount=amount,
                description=description,
                reference=reference,
                establishment_id=establishment_id,
                establishment_name=establishment_name,
                expires_in=expires_in,
                webhook=webhook,
                kind=kind,
            ),
        )

    async def get_qr_status(
        self, reference: str, *, kind: QrKind | str = QrKind.SIMPLE
    ) -> QrStatusInfo:
        """See :meth:`QrClient.get_qr_status`."""
        return await self.session.execute(VERIFY, verify_input(reference, kind))

    async def cancel_qr(self, reference: str) -> QrStatusInfo:
        """See :meth:`QrClient.cancel_qr`."""
        return await self.session.execute(CANCEL, cancel_input(reference))
