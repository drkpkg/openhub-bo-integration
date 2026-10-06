from __future__ import annotations

import asyncio
import json
from decimal import Decimal

import httpx
import pytest

from openhub_bo.core import (
    AmbiguousOutcomeError,
    ApiError,
    PaymentStatus,
    ValidationError,
    WebhookAuthError,
)
from openhub_bo.payouts import BatchTransfer, parse_batch_webhook
from openhub_bo.payouts.testing import MockPayouts

SOURCE = "7010123451"
BRANCH = "455544"
TOKEN = "tok_0123456789abcdef"


@pytest.fixture
def mock() -> MockPayouts:
    m = MockPayouts()
    m.add_source_account(SOURCE, balance="500")
    return m


def transfer(tx_id: str = "001002", bank: str = "1018") -> BatchTransfer:
    return BatchTransfer(
        tx_id,
        "50.00",
        "484811311404044",
        "1311404044",
        bank,
        "lpz",
        "Pago proveedor",
        "5452452",
        "PROVEEDOR SRL",
    )


def test_fixed_amount_qr_flow(mock):
    client = mock.client()
    scanned = client.scan_qr(
        mock.add_qr(amount="100.50", description="Factura 7", holder="PROVEEDOR SRL")
    )
    assert (scanned.amount, scanned.open_amount, scanned.recipient.holder) == (
        Decimal("100.5"),
        False,
        "PROVEEDOR SRL",
    )
    payout = client.pay_qr(scanned, source_account=SOURCE, transaction_id="PAY-1")
    body = json.loads(mock.requests[-1].content)
    assert body["importe"] == 0 and "glosa" not in body  # docs: 0.00 for fixed-amount QRs
    assert payout.status is PaymentStatus.PAID
    assert mock.accounts[SOURCE] == Decimal("399.50")
    assert client.get_payout(payout.reference).transaction_id == "PAY-1"


def test_open_amount_qr_needs_amount_and_description(mock):
    client = mock.client()
    scanned = client.scan_qr(mock.add_qr())
    assert scanned.open_amount
    with pytest.raises(ValidationError) as exc:
        client.pay_qr(scanned, source_account=SOURCE, transaction_id="PAY-2")
    assert exc.value.field == "amount"
    with pytest.raises(ValidationError) as exc:
        client.pay_qr(scanned, source_account=SOURCE, transaction_id="PAY-2", amount="20")
    assert exc.value.field == "description"
    payout = client.pay_qr(
        scanned, source_account=SOURCE, transaction_id="PAY-2", amount="20", description="Pago"
    )
    assert payout.amount == Decimal("20")


def test_unconfirmed_payment_is_ambiguous_and_status_resolves_it(mock):
    client = mock.client()
    scanned = client.scan_qr(mock.add_qr(amount="10", description="x"))
    mock.next_payment_unconfirmed()
    with pytest.raises(AmbiguousOutcomeError) as exc:
        client.pay_qr(scanned, source_account=SOURCE, transaction_id="PAY-3")
    assert (exc.value.operation, exc.value.code, exc.value.retryable) == (
        "payouts.pay",
        "96",
        False,
    )
    assert mock.accounts[SOURCE] == Decimal("490")  # it went through anyway


def test_payout_errors(mock):
    client = mock.client()
    with pytest.raises(ApiError) as exc:
        client.scan_qr("not-a-qr")
    assert exc.value.code == "09"
    scanned = client.scan_qr(mock.add_qr(amount="9999", description="x"))
    with pytest.raises(ApiError) as exc:
        client.pay_qr(scanned, source_account=SOURCE, transaction_id="PAY-4")
    assert exc.value.code == "13"
    with pytest.raises(ApiError) as exc:
        client.get_payout("1")
    assert exc.value.code == "04"


def test_payout_ops_use_ats_timeouts(mock):
    seen: list[object] = []

    def handler(request: httpx.Request) -> httpx.Response:
        seen.append(request.extensions["timeout"]["read"])
        return mock.transport.handle_request(request)

    from openhub_bo.core import Session
    from openhub_bo.payouts import PayoutsClient

    session = Session(
        mock.client_id,
        mock.client_secret,
        base_url=mock.BASE_URL,
        http_client=httpx.Client(transport=httpx.MockTransport(handler), timeout=5),
    )
    client = PayoutsClient(session)
    scanned = client.scan_qr(mock.add_qr(amount="1", description="x"))
    client.pay_qr(scanned, source_account=SOURCE, transaction_id="PAY-5")
    assert seen == [5, 40.0, 90.0]  # token (client default), scan, pay


def test_batch_flow_with_webhooks(mock):
    client = mock.client()
    assert [b.code for b in client.list_banks(BRANCH)] == ["1005", "1014", "1018"]
    auth = client.authorize_batch(
        BRANCH,
        [transfer(), transfer("001003", bank="9999")],
        webhook_url="https://shop.example/ach",
        webhook_token=TOKEN,
    )
    request = mock.requests[-1]
    assert request.headers["branchCode"] == BRANCH
    assert json.loads(request.content)["webhookUrl"] == f"https://shop.example/ach?token={TOKEN}"
    assert [t.status for t in auth.transfers] == [PaymentStatus.PENDING, PaymentStatus.ERROR]
    assert [t.transaction_id for t in auth.rejected] == ["001003"]

    calls = mock.settle(auth.process_id)
    notification = parse_batch_webhook(calls[0].token, calls[0].body, expected_token=TOKEN)
    assert notification.transfer.status is PaymentStatus.PAID
    assert notification.ack()["codigoRespuesta"] == "EXITOSO"
    assert notification.ack(processed=False, detail="dup")["codigoRespuesta"] == "FALLIDO"
    with pytest.raises(WebhookAuthError):
        parse_batch_webhook("wrong", calls[0].body, expected_token=TOKEN)

    status = client.get_batch_status(BRANCH, auth.process_id, batch_number=auth.batch_number)
    assert status.transfers[0].status is PaymentStatus.PAID
    by_tx = client.get_batch_status(BRANCH, auth.process_id, transaction_id="001002")
    assert by_tx.transfers[0].amount == Decimal("50")


def test_batch_validation(mock):
    client = mock.client()
    with pytest.raises(ValidationError) as exc:
        client.authorize_batch(
            BRANCH, [transfer(), transfer()], webhook_url="https://a.bo", webhook_token=TOKEN
        )
    assert exc.value.field == "transfers.transaction_id"
    with pytest.raises(ValidationError) as exc:
        client.authorize_batch(
            BRANCH, [transfer()], webhook_url="https://a.bo", webhook_token="short"
        )
    assert exc.value.field == "webhook_token"
    with pytest.raises(ValidationError):
        client.get_batch_status(BRANCH, "not-a-uuid", batch_number="1")


def test_async_payouts(mock):
    async def scenario() -> PaymentStatus:
        client = mock.async_client()
        scanned = await client.scan_qr(mock.add_qr(amount="5", description="x"))
        payout = await client.pay_qr(scanned, source_account=SOURCE, transaction_id="PAY-A")
        return (await client.get_payout(payout.reference)).status

    assert asyncio.run(scenario()) is PaymentStatus.PAID
