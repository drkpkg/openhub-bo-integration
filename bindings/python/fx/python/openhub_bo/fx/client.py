"""PIX, virtual-asset (Koibanx) and Binance clients. Each takes the shared
:class:`~openhub_bo.core.Session`; inputs are built in ``_ops``."""

from __future__ import annotations

from datetime import timedelta
from decimal import Decimal

from openhub_bo.core import AsyncSession, Session, Webhook

from . import _ops
from .models import Currency, FxQr, FxStatusInfo, Glosa, VirtualAsset


class PixClient:
    """Charge Brazilian payers via PIX: they pay BRL, you receive BOB/USD."""

    def __init__(self, session: Session) -> None:
        self.session = session

    def generate_qr(
        self,
        *,
        amount: Decimal | int | str,
        glosa: Glosa | str,
        reference: str,
        payer_cpf: str,
        payer_phone: str,
        webhook: Webhook,
        currency: Currency | str = Currency.BOB,
        channel: str = "WEB",
        expires_in: int | timedelta | None = None,
        payer_email: str | None = None,
        extra: str | None = None,
    ) -> FxQr:
        """``payer_phone`` is ``+`` and 13 digits; ``expires_in`` defaults to 100 s on
        OpenHub's side. Requires PIX to be enabled for your merchant (``GQ-00005``)."""
        return self.session.execute(
            _ops.PIX_GENERATE,
            _ops.pix_input(
                amount=amount,
                glosa=glosa,
                reference=reference,
                payer_cpf=payer_cpf,
                payer_phone=payer_phone,
                webhook=webhook,
                currency=currency,
                channel=channel,
                expires_in=expires_in,
                payer_email=payer_email,
                extra=extra,
            ),
        )

    def get_status(self, reference: str) -> FxStatusInfo:
        return self.session.execute(_ops.PIX_VERIFY, _ops.ref_input(reference))

    def cancel(self, reference: str) -> FxStatusInfo:
        """OpenHub's "cancela". The sandbox only allows it on paid ("aprobado")
        QRs, i.e. it acts as a refund request; unpaid ones answer ``EG-00001``."""
        return self.session.execute(_ops.PIX_CANCEL, _ops.ref_input(reference))


class VirtualAssetsClient:
    """Charge in BOB/USD, settle in USDT/USDC through Koibanx."""

    def __init__(self, session: Session) -> None:
        self.session = session

    def generate_qr(
        self,
        *,
        amount: Decimal | int | str,
        glosa: Glosa | str,
        reference: str,
        asset: VirtualAsset | str,
        webhook: Webhook,
        currency: Currency | str = Currency.BOB,
        channel: str = "WEB",
        expires_in: int | timedelta = 180,
        extra: str | None = None,
    ) -> FxQr:
        """Minimum Bs 50 in BOB; ``expires_in`` 180..600 s (sandbox rules)."""
        return self.session.execute(
            _ops.CRYPTO_GENERATE,
            _ops.crypto_input(
                amount=amount,
                glosa=glosa,
                reference=reference,
                asset=asset,
                webhook=webhook,
                currency=currency,
                channel=channel,
                expires_in=expires_in,
                extra=extra,
            ),
        )

    def get_status(self, reference: str) -> FxStatusInfo:
        return self.session.execute(_ops.CRYPTO_VERIFY, _ops.ref_input(reference))


class BinanceClient:
    """Charge through Binance Pay QR."""

    def __init__(self, session: Session) -> None:
        self.session = session

    def generate_qr(
        self,
        *,
        amount: Decimal | int | str,
        glosa: Glosa | str,
        reference: str,
        webhook: Webhook,
        currency: Currency | str = Currency.BOB,
        channel: str = "WEB",
        expires_in: int | timedelta | None = None,
        extra: str | None = None,
    ) -> FxQr:
        """``reference`` at most 10 digits; ``expires_in`` at most 300 s."""
        return self.session.execute(
            _ops.BINANCE_GENERATE,
            _ops.binance_input(
                amount=amount,
                glosa=glosa,
                reference=reference,
                webhook=webhook,
                currency=currency,
                channel=channel,
                expires_in=expires_in,
                extra=extra,
            ),
        )

    def get_status(self, reference: str) -> FxStatusInfo:
        return self.session.execute(_ops.BINANCE_VERIFY, _ops.ref_input(reference))


class AsyncPixClient:
    """Asyncio counterpart of :class:`PixClient`."""

    def __init__(self, session: AsyncSession) -> None:
        self.session = session

    async def generate_qr(
        self,
        *,
        amount: Decimal | int | str,
        glosa: Glosa | str,
        reference: str,
        payer_cpf: str,
        payer_phone: str,
        webhook: Webhook,
        currency: Currency | str = Currency.BOB,
        channel: str = "WEB",
        expires_in: int | timedelta | None = None,
        payer_email: str | None = None,
        extra: str | None = None,
    ) -> FxQr:
        """See :meth:`PixClient.generate_qr`."""
        return await self.session.execute(
            _ops.PIX_GENERATE,
            _ops.pix_input(
                amount=amount,
                glosa=glosa,
                reference=reference,
                payer_cpf=payer_cpf,
                payer_phone=payer_phone,
                webhook=webhook,
                currency=currency,
                channel=channel,
                expires_in=expires_in,
                payer_email=payer_email,
                extra=extra,
            ),
        )

    async def get_status(self, reference: str) -> FxStatusInfo:
        return await self.session.execute(_ops.PIX_VERIFY, _ops.ref_input(reference))

    async def cancel(self, reference: str) -> FxStatusInfo:
        """See :meth:`PixClient.cancel`."""
        return await self.session.execute(_ops.PIX_CANCEL, _ops.ref_input(reference))


class AsyncVirtualAssetsClient:
    """Asyncio counterpart of :class:`VirtualAssetsClient`."""

    def __init__(self, session: AsyncSession) -> None:
        self.session = session

    async def generate_qr(
        self,
        *,
        amount: Decimal | int | str,
        glosa: Glosa | str,
        reference: str,
        asset: VirtualAsset | str,
        webhook: Webhook,
        currency: Currency | str = Currency.BOB,
        channel: str = "WEB",
        expires_in: int | timedelta = 180,
        extra: str | None = None,
    ) -> FxQr:
        """See :meth:`VirtualAssetsClient.generate_qr`."""
        return await self.session.execute(
            _ops.CRYPTO_GENERATE,
            _ops.crypto_input(
                amount=amount,
                glosa=glosa,
                reference=reference,
                asset=asset,
                webhook=webhook,
                currency=currency,
                channel=channel,
                expires_in=expires_in,
                extra=extra,
            ),
        )

    async def get_status(self, reference: str) -> FxStatusInfo:
        return await self.session.execute(_ops.CRYPTO_VERIFY, _ops.ref_input(reference))


class AsyncBinanceClient:
    """Asyncio counterpart of :class:`BinanceClient`."""

    def __init__(self, session: AsyncSession) -> None:
        self.session = session

    async def generate_qr(
        self,
        *,
        amount: Decimal | int | str,
        glosa: Glosa | str,
        reference: str,
        webhook: Webhook,
        currency: Currency | str = Currency.BOB,
        channel: str = "WEB",
        expires_in: int | timedelta | None = None,
        extra: str | None = None,
    ) -> FxQr:
        """See :meth:`BinanceClient.generate_qr`."""
        return await self.session.execute(
            _ops.BINANCE_GENERATE,
            _ops.binance_input(
                amount=amount,
                glosa=glosa,
                reference=reference,
                webhook=webhook,
                currency=currency,
                channel=channel,
                expires_in=expires_in,
                extra=extra,
            ),
        )

    async def get_status(self, reference: str) -> FxStatusInfo:
        return await self.session.execute(_ops.BINANCE_VERIFY, _ops.ref_input(reference))
