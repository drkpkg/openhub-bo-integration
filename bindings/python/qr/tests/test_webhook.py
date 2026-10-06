from __future__ import annotations

import json
from datetime import datetime
from decimal import Decimal

import pytest

from openhub_bo.core import BOLIVIA_TZ, DecodeError, PaymentStatus, WebhookAuthError
from openhub_bo.qr import parse_webhook


def test_parses_documented_payload(fixtures_dir, webhook):
    payload = (fixtures_dir / "webhook_payment.json").read_bytes()
    notification = parse_webhook({"X-Api-Key": webhook.value}, payload, webhook=webhook)
    assert notification.success and notification.status is PaymentStatus.PAID
    assert notification.reference == "233324"
    assert notification.amount == Decimal("10.5")
    assert notification.transaction_at == datetime(2026, 5, 26, 14, 35, 20, tzinfo=BOLIVIA_TZ)
    assert notification.payer is not None and notification.payer.document_id == "12345678"


@pytest.mark.parametrize("headers", [{}, {"x-api-key": "wrong"}, {"authorization": "x"}])
def test_rejects_unauthenticated_calls(fixtures_dir, webhook, headers):
    payload = (fixtures_dir / "webhook_payment.json").read_bytes()
    with pytest.raises(WebhookAuthError):
        parse_webhook(headers, payload, webhook=webhook)


def test_rejects_garbage_body(webhook):
    with pytest.raises(DecodeError):
        parse_webhook({"x-api-key": webhook.value}, b"<html>", webhook=webhook)


def test_accepts_status_shaped_payload(fixtures_dir, webhook):
    data = json.loads((fixtures_dir / "verify_pending_response.json").read_text())["data"]
    body = json.dumps(data | {"estado": "PAGADO"})
    notification = parse_webhook({"x-api-key": webhook.value}, body, webhook=webhook)
    assert notification.success
    assert notification.response_code == "PAGADO"
    assert notification.amount == Decimal("10.5")
