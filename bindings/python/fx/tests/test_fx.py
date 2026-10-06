from __future__ import annotations

import asyncio
import json
from decimal import Decimal

import pytest

from openhub_bo.core import ApiError, PaymentStatus, ValidationError, WebhookAuthError
from openhub_bo.fx import (
    BinanceClient,
    Currency,
    PixClient,
    VirtualAsset,
    parse_binance_webhook,
    parse_fx_webhook,
)


@pytest.fixture
def pix_args(glosa, webhook):
    return {
        "amount": "145.00",
        "glosa": glosa,
        "reference": "311113",
        "payer_cpf": "12345678901",
        "payer_phone": "+5511999999999",
        "webhook": webhook,
        "expires_in": 120,
    }


def test_pix_flow_pay_then_refund(mock, pix_args, webhook):
    pix = mock.pix_client()
    qr = pix.generate_qr(**pix_args)
    assert qr.status is PaymentStatus.PENDING
    assert qr.converted_currency == "BRL" and qr.converted_amount == Decimal("116")
    assert qr.image_mime == "image/png" and qr.image.startswith(b"\x89PNG")
    assert qr.merchant_reference == "311113"

    body = json.loads(mock.requests[-1].content)
    assert body["glosa"] == "1|Tienda Central|7011|Pedido 42"
    assert body["tiempoQr"] == "00:02:00"

    with pytest.raises(ApiError) as exc:  # sandbox: only paid QRs can be "cancelled"
        pix.cancel(qr.reference)
    assert (exc.value.status, exc.value.code, exc.value.retryable) == (500, "EG-00001", False)

    call = mock.pay(qr.reference)
    notification = parse_fx_webhook(call.headers, call.body, webhook=webhook)
    assert notification.reference == qr.reference and notification.status is PaymentStatus.PAID
    assert pix.get_status(qr.reference).status is PaymentStatus.PAID

    refunded = pix.cancel(qr.reference)
    assert refunded.status is PaymentStatus.CANCELLED
    assert refunded.reference == qr.reference and refunded.requested_at


def test_pix_merchant_not_enabled(mock, pix_args):
    mock.disable("pix")
    with pytest.raises(ApiError) as exc:
        mock.pix_client().generate_qr(**pix_args)
    assert exc.value.code == "GQ-00005"
    assert not exc.value.retryable


def test_pix_unknown_reference_is_not_retryable_despite_http_500(mock):
    with pytest.raises(ApiError) as exc:
        mock.pix_client().get_status("1")
    assert (exc.value.status, exc.value.retryable) == (500, False)


@pytest.mark.parametrize(
    ("override", "field"),
    [
        ({"glosa": "Pedido 42"}, "glosa"),
        ({"payer_phone": "+59171234567"}, "payer_phone"),
        ({"payer_cpf": "123"}, "payer_cpf"),
        ({"reference": "ord-1"}, "reference"),
        ({"amount": "1.001"}, "amount"),
        ({"expires_in": 0}, "expires_in"),
    ],
)
def test_pix_validation_before_network(mock, pix_args, override, field):
    with pytest.raises(ValidationError) as exc:
        mock.pix_client().generate_qr(**{**pix_args, **override})
    assert exc.value.field == field
    assert not any(r.url.path.endswith("/generar") for r in mock.requests)


def test_virtual_assets_flow(mock, glosa, webhook):
    client = mock.virtual_assets_client()
    qr = client.generate_qr(
        amount="63.60", glosa=glosa, reference="321", asset=VirtualAsset.USDC, webhook=webhook
    )
    body = json.loads(mock.requests[-1].content)
    assert (body["activoVirtual"], body["tiempoVencimientoQR"]) == ("UP", 180)
    assert qr.converted_currency == "USDC" and qr.converted_amount == Decimal("5")
    mock.pay(qr.reference)
    assert client.get_status(qr.reference).status is PaymentStatus.PAID


@pytest.mark.parametrize(
    ("override", "field"),
    [({"amount": "49.99"}, "amount"), ({"expires_in": 60}, "expires_in")],
)
def test_virtual_assets_sandbox_rules(mock, glosa, webhook, override, field):
    args = {"amount": "50", "glosa": glosa, "reference": "1", "asset": "usdt", "webhook": webhook}
    with pytest.raises(ValidationError) as exc:
        mock.virtual_assets_client().generate_qr(**{**args, **override})
    assert exc.value.field == field


def test_virtual_assets_usd_has_no_bob_minimum(mock, glosa, webhook):
    qr = mock.virtual_assets_client().generate_qr(
        amount="10",
        currency=Currency.USD,
        glosa=glosa,
        reference="2",
        asset="usdt",
        webhook=webhook,
    )
    assert qr.currency == "USD"


def test_binance_flow_and_ack(mock, glosa, webhook):
    client = mock.binance_client()
    qr = client.generate_qr(
        amount="12.04", glosa=glosa, reference="200397", webhook=webhook, expires_in=300
    )
    assert qr.image_mime == "image/jpeg"
    assert qr.converted_currency == "USDT" and qr.converted_amount == Decimal("1")
    assert json.loads(mock.requests[-1].content)["tiempoQr"] == "00:05:00"

    call = mock.pay(qr.reference)
    notification = parse_binance_webhook(call.headers, call.body, webhook=webhook)
    assert notification.status is PaymentStatus.PAID
    assert notification.ack == {
        "numeroReferencia": "200397",
        "codigoRespuesta": "00",
        "detalleRespuesta": None,
    }
    assert client.get_status(qr.reference).status is PaymentStatus.PAID


def test_binance_rules_and_disabled_merchant(mock, glosa, webhook):
    client = mock.binance_client()
    with pytest.raises(ValidationError):
        client.generate_qr(amount="1", glosa=glosa, reference="12345678901", webhook=webhook)
    with pytest.raises(ValidationError):
        client.generate_qr(amount="1", glosa=glosa, reference="1", webhook=webhook, expires_in=301)
    mock.disable("binance")
    with pytest.raises(ApiError) as exc:
        client.generate_qr(amount="1", glosa=glosa, reference="1", webhook=webhook)
    assert exc.value.code == "GQ-00000"


def test_documented_binance_webhook(fixtures_dir, webhook):
    body = (fixtures_dir / "binance_webhook.json").read_bytes()
    notification = parse_binance_webhook({"x-api-key": webhook.value}, body, webhook=webhook)
    assert (notification.reference, notification.amount) == ("4221", Decimal("1.00"))
    with pytest.raises(WebhookAuthError):
        parse_binance_webhook({"x-api-key": "nope"}, body, webhook=webhook)


def test_clients_share_one_token(mock, pix_args, glosa, webhook):
    session = mock.session()
    PixClient(session).generate_qr(**pix_args)
    BinanceClient(session).generate_qr(amount="1", glosa=glosa, reference="7", webhook=webhook)
    assert mock.token_requests == 1


def test_async_clients(mock, pix_args, glosa, webhook):
    async def scenario() -> tuple[PaymentStatus, PaymentStatus]:
        pix = mock.async_pix_client()
        binance = mock.async_binance_client()
        pix_qr, binance_qr = await asyncio.gather(
            pix.generate_qr(**pix_args),
            binance.generate_qr(amount="1", glosa=glosa, reference="8", webhook=webhook),
        )
        mock.pay(binance_qr.reference)
        pix_status, binance_status = await asyncio.gather(
            pix.get_status(pix_qr.reference), binance.get_status(binance_qr.reference)
        )
        return pix_status.status, binance_status.status

    assert asyncio.run(scenario()) == (PaymentStatus.PENDING, PaymentStatus.PAID)
