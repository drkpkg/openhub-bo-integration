"""Merchant-account operations declared against the native module."""

from __future__ import annotations

from collections.abc import Callable, Sequence
from datetime import date
from typing import Any, TypeVar

from openhub_bo.core import Native, Op

from . import _native
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

NATIVE = Native(_native)


T = TypeVar("T")


def _list_of(decode: Callable[[dict[str, Any]], T]) -> Callable[[list[dict[str, Any]]], list[T]]:
    return lambda rows: [decode(row) for row in rows]


GET = Op(NATIVE, "accounts.get", MerchantAccount._from)
LIST = Op(NATIVE, "accounts.list", MerchantAccounts._from)
CREATE = Op(NATIVE, "accounts.create", CreatedAccounts._from)
SET_STATUS: Op[list[StatusChanged]] = Op(
    NATIVE, "accounts.set_status", _list_of(StatusChanged._from)
)
RECONCILE = Op(NATIVE, "accounts.reconcile", Reconciliation._from)
CREDITS: Op[list[Movement]] = Op(NATIVE, "accounts.credits", _list_of(Movement._from))
DEBITS: Op[list[Movement]] = Op(NATIVE, "accounts.debits", _list_of(Movement._from))
BALANCES: Op[list[Balance]] = Op(NATIVE, "accounts.balances", _list_of(Balance._from))


def iso_date(value: date | str) -> str:
    return value.isoformat() if isinstance(value, date) else value


def account_ref(nit: str, account_number: str) -> dict[str, Any]:
    return {"nit": str(nit), "account_number": str(account_number)}


def merchant_ref(nit: str) -> dict[str, Any]:
    return {"nit": str(nit)}


def create_input(nit: str, establishment_id: int, accounts: Sequence[NewAccount]) -> dict[str, Any]:
    return {
        "nit": str(nit),
        "establishment_id": establishment_id,
        "accounts": [a.to_wire() for a in accounts],
    }


def status_input(nit: str, changes: Sequence[StatusChange]) -> dict[str, Any]:
    return {"nit": str(nit), "changes": [c.to_wire() for c in changes]}


def merchant_range(nit: str, date_from: date | str, date_to: date | str) -> dict[str, Any]:
    return {"nit": str(nit), "date_from": iso_date(date_from), "date_to": iso_date(date_to)}


def account_range(
    nit: str, account_number: str, date_from: date | str, date_to: date | str
) -> dict[str, Any]:
    return merchant_range(nit, date_from, date_to) | {"account_number": str(account_number)}


def balances_input(nit: str, account_numbers: Sequence[str]) -> dict[str, Any]:
    return {"nit": str(nit), "account_numbers": [str(n) for n in account_numbers]}
