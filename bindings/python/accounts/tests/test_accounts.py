from __future__ import annotations

import asyncio
import json
from datetime import date
from decimal import Decimal

import pytest

from openhub_bo.accounts import AccountStatus, NewAccount, StatusChange
from openhub_bo.accounts.testing import MockAccounts
from openhub_bo.core import ApiError, PaymentStatus, ValidationError

NIT, EST, ACC = "1000000019", 111369, "7014227171"


@pytest.fixture
def mock() -> MockAccounts:
    m = MockAccounts()
    m.add_account(NIT, EST, ACC, alias="CAJA 1 - COMERCIALES", available="50.00")
    m.add_account(NIT, EST, "7014227172", alias="CAJA 2 - COMERCIALES")
    m.add_movement(NIT, ACC, "PAYIN QR", "100.00", at="2026-10-02T10:00:00")
    m.add_movement(NIT, ACC, "PAYOUT ACH", "30.00", at="2026-10-03T11:00:00")
    return m


def test_lookup_and_list(mock):
    client = mock.client()
    detail = client.get_account(NIT, ACC)
    assert (detail.establishment_id, detail.account.status) == (EST, AccountStatus.ACTIVE)
    listing = client.list_accounts(NIT)
    assert [a.number for a in listing.accounts] == [ACC, "7014227172"]
    assert mock.requests[-1].url.path == f"/cuentas-comercios/v1/cuentas/{NIT}"


def test_balances_and_movements(mock):
    client = mock.client()
    balance = client.balances(NIT, [ACC])[0]
    assert (balance.available, balance.currency) == (Decimal("120"), "BOB")
    assert balance.last_credit_at == "2026-10-02T10:00:00"

    credits = client.credits(NIT, ACC, date_from=date(2026, 9, 10), date_to=date(2026, 10, 6))
    assert [(m.operation_type, m.amount, m.reference) for m in credits] == [
        ("PAYIN QR", Decimal("100"), None)
    ]
    debits = client.debits(NIT, ACC, date_from="2026-10-01", date_to="2026-10-06")
    assert debits[0].status is PaymentStatus.PAID and debits[0].transaction_id

    rec = client.reconcile(NIT, date_from=date(2026, 10, 1), date_to=date(2026, 10, 6))
    assert len(rec.movements) == 2 and len(rec.balances) == 2
    body = json.loads(mock.requests[-1].content)
    assert body == {"nit": NIT, "fechaInicio": "2026-10-01", "fechaFin": "2026-10-06"}


def test_create_and_change_status(mock):
    client = mock.client()
    created = client.create_accounts(
        NIT, establishment_id=EST, accounts=[NewAccount("CAJA 5", "COMERCIALES")]
    )
    new = created.accounts[0]
    assert new.status is AccountStatus.ACTIVE and new.alias == "CAJA 5 - COMERCIALES"

    changed = client.set_account_status(
        NIT, [StatusChange(new.number, AccountStatus.BLOCKED, "Sospecha de fraude")]
    )
    assert changed[0].status is AccountStatus.BLOCKED
    request = mock.requests[-1]
    assert request.method == "PATCH"
    assert json.loads(request.content)["cuentas"][0]["estado"] == "BLOQUEADA"
    assert client.get_account(NIT, new.number).account.status is AccountStatus.BLOCKED


def test_closing_requires_zero_balance(mock):
    with pytest.raises(ApiError) as exc:
        mock.client().set_account_status(NIT, [StatusChange(ACC, AccountStatus.CLOSED, "Cierre")])
    assert exc.value.code == "05"


@pytest.mark.parametrize(
    ("call", "field"),
    [
        (lambda c: c.reconcile(NIT, date_from="2026-09-28", date_to="2026-10-06"), "date_to"),
        (lambda c: c.credits(NIT, ACC, date_from="2026-08-01", date_to="2026-10-06"), "date_to"),
        (lambda c: c.balances(NIT, [str(7014227100 + i) for i in range(11)]), "account_numbers"),
        (lambda c: c.list_accounts("12-3"), "nit"),
        (lambda c: c.reconcile(NIT, date_from="06/10/2026", date_to="2026-10-06"), "date_from"),
    ],
)
def test_sandbox_limits_are_validated_locally(mock, call, field):
    with pytest.raises(ValidationError) as exc:
        call(mock.client())
    assert exc.value.field == field


@pytest.mark.parametrize(
    ("call", "code"),
    [
        (lambda c: c.list_accounts("1234567"), "17"),
        (lambda c: c.get_account(NIT, "7099999999"), "15"),
        (lambda c: c.credits("1234567", ACC, date_from="2026-10-01", date_to="2026-10-06"), "99"),
    ],
)
def test_sandbox_error_codes(mock, call, code):
    with pytest.raises(ApiError) as exc:
        call(mock.client())
    assert (exc.value.code, exc.value.retryable) == (code, False)


def test_async_client(mock):
    async def scenario() -> Decimal:
        client = mock.async_client()
        balances, listing = await asyncio.gather(
            client.balances(NIT, [ACC]), client.list_accounts(NIT)
        )
        assert len(listing.accounts) == 2
        return balances[0].available

    assert asyncio.run(scenario()) == Decimal("120")
