"""Merchant accounts at ATC: lookup, creation, status, balances and movements.

Dates accept ``datetime.date`` or ``"yyyy-mm-dd"``. Limits observed in the
sandbox: reconciliation spans at most 7 days, credits/debits 31 days, balances
up to 10 accounts.
"""

from __future__ import annotations

from collections.abc import Sequence
from datetime import date

from openhub_bo.core import AsyncSession, Session

from . import _ops
from .models import (
    Balance,
    CreatedAccounts,
    MerchantAccount,
    MerchantAccounts,
    Movement,
    NewAccount,
    Reconciliation,
    StatusChange,
    StatusChanged,
)


class AccountsClient:
    def __init__(self, session: Session) -> None:
        self.session = session

    def get_account(self, nit: str, account_number: str) -> MerchantAccount:
        return self.session.execute(_ops.GET, _ops.account_ref(nit, account_number))

    def list_accounts(self, nit: str) -> MerchantAccounts:
        return self.session.execute(_ops.LIST, _ops.merchant_ref(nit))

    def create_accounts(
        self, nit: str, *, establishment_id: int, accounts: Sequence[NewAccount]
    ) -> CreatedAccounts:
        """Creates 1..1000 accounts (BOB). Not idempotent: a lost response raises
        ``AmbiguousOutcomeError``; check :meth:`list_accounts` before retrying."""
        return self.session.execute(_ops.CREATE, _ops.create_input(nit, establishment_id, accounts))

    def set_account_status(self, nit: str, changes: Sequence[StatusChange]) -> list[StatusChanged]:
        """Changing to ``CLOSED`` requires zero balance and cannot be undone."""
        return self.session.execute(_ops.SET_STATUS, _ops.status_input(nit, changes))

    def reconcile(self, nit: str, *, date_from: date | str, date_to: date | str) -> Reconciliation:
        """Movements and balances of every account of the merchant (max 7 days)."""
        return self.session.execute(_ops.RECONCILE, _ops.merchant_range(nit, date_from, date_to))

    def credits(
        self, nit: str, account_number: str, *, date_from: date | str, date_to: date | str
    ) -> list[Movement]:
        """Incoming movements (QR payins, deposits) of one account (max 31 days)."""
        return self.session.execute(
            _ops.CREDITS, _ops.account_range(nit, account_number, date_from, date_to)
        )

    def debits(
        self, nit: str, account_number: str, *, date_from: date | str, date_to: date | str
    ) -> list[Movement]:
        """Outgoing movements (payouts) of one account (max 31 days)."""
        return self.session.execute(
            _ops.DEBITS, _ops.account_range(nit, account_number, date_from, date_to)
        )

    def balances(self, nit: str, account_numbers: Sequence[str]) -> list[Balance]:
        """Current balances of up to 10 accounts."""
        return self.session.execute(_ops.BALANCES, _ops.balances_input(nit, account_numbers))


class AsyncAccountsClient:
    """Asyncio counterpart of :class:`AccountsClient`."""

    def __init__(self, session: AsyncSession) -> None:
        self.session = session

    async def get_account(self, nit: str, account_number: str) -> MerchantAccount:
        return await self.session.execute(_ops.GET, _ops.account_ref(nit, account_number))

    async def list_accounts(self, nit: str) -> MerchantAccounts:
        return await self.session.execute(_ops.LIST, _ops.merchant_ref(nit))

    async def create_accounts(
        self, nit: str, *, establishment_id: int, accounts: Sequence[NewAccount]
    ) -> CreatedAccounts:
        """See :meth:`AccountsClient.create_accounts`."""
        return await self.session.execute(
            _ops.CREATE, _ops.create_input(nit, establishment_id, accounts)
        )

    async def set_account_status(
        self, nit: str, changes: Sequence[StatusChange]
    ) -> list[StatusChanged]:
        """See :meth:`AccountsClient.set_account_status`."""
        return await self.session.execute(_ops.SET_STATUS, _ops.status_input(nit, changes))

    async def reconcile(
        self, nit: str, *, date_from: date | str, date_to: date | str
    ) -> Reconciliation:
        return await self.session.execute(
            _ops.RECONCILE, _ops.merchant_range(nit, date_from, date_to)
        )

    async def credits(
        self, nit: str, account_number: str, *, date_from: date | str, date_to: date | str
    ) -> list[Movement]:
        return await self.session.execute(
            _ops.CREDITS, _ops.account_range(nit, account_number, date_from, date_to)
        )

    async def debits(
        self, nit: str, account_number: str, *, date_from: date | str, date_to: date | str
    ) -> list[Movement]:
        return await self.session.execute(
            _ops.DEBITS, _ops.account_range(nit, account_number, date_from, date_to)
        )

    async def balances(self, nit: str, account_numbers: Sequence[str]) -> list[Balance]:
        return await self.session.execute(_ops.BALANCES, _ops.balances_input(nit, account_numbers))
