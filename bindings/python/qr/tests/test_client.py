from __future__ import annotations

import json
from datetime import timedelta
from decimal import Decimal

import httpx
import pytest

from openhub_bo.core import (
    AmbiguousOutcomeError,
    ApiError,
    AuthenticationError,
    PaymentStatus,
    Session,
    TransportError,
    ValidationError,
)
from openhub_bo.qr import QrClient, QrKind, QrStatus, parse_webhook


def test_full_payment_flow(mock, qr_args, webhook):
    qr_client = mock.client()
    qr = qr_client.generate_qr(**qr_args)
    assert qr.status is PaymentStatus.PENDING
    assert qr.amount == Decimal("10.5")
    assert qr.merchant_reference == "4024"
    assert qr.expires_at is not None and qr.expires_at.tzinfo is not None
    assert qr.qr_png.startswith(b"\x89PNG")

    call = mock.pay(qr.reference)
    notification = parse_webhook(call.headers, call.body, webhook=webhook)
    assert notification.success and notification.status is PaymentStatus.PAID
    assert notification.reference == qr.reference

    status = qr_client.get_qr_status(qr.reference)
    assert status.status is QrStatus.PAID and status.status.is_final
    assert status.payer is not None and status.payer.name == "Juan Perez"
    assert status.payer_bank is not None and status.payer_bank.bank_name == "Banco Unión"


def test_wire_body_uses_spanish_contract(mock, qr_args, webhook):
    qr = mock.client().generate_qr(
        **{**qr_args, "expires_in": timedelta(minutes=2)}, kind=QrKind.MLD
    )
    assert qr.kind is QrKind.MLD
    request = mock.requests[-1]
    assert request.url.path == "/qr/mld/v2/generate"
    assert request.headers["client_id"] == mock.client_id
    assert request.headers["authorization"] == f"Bearer {request.headers['access_token']}"
    assert json.loads(request.content) == {
        "glosa": "Pedido 42",
        "moneda": "BOB",
        "monto": 10.5,
        "numeroReferencia": "4024",
        "vigencia": 120,
        "idEstablecimiento": 422717,
        "nombreEstablecimiento": "Tienda Central",
        "webhook": {"url": webhook.url, "key": "x-api-key", "value": webhook.value},
    }


def test_clients_share_the_session_token(mock, qr_args):
    session = mock.session()
    first, second = QrClient(session), QrClient(session)
    qr = first.generate_qr(**qr_args)
    second.get_qr_status(qr.reference)
    first.get_qr_status(qr.reference)
    assert mock.token_requests == 1


def test_rejected_token_is_refreshed_once(mock, qr_args):
    qr_client = mock.client()
    qr = qr_client.generate_qr(**qr_args)
    mock.revoke_tokens()
    assert qr_client.get_qr_status(qr.reference).status is PaymentStatus.PENDING
    assert mock.token_requests == 2


def test_bad_credentials(mock, qr_args):
    with pytest.raises(AuthenticationError) as exc:
        mock.client(client_secret="wrong").generate_qr(**qr_args)
    assert exc.value.status == 401


def test_unknown_reference_raises_api_error(mock):
    with pytest.raises(ApiError) as exc:
        mock.client().get_qr_status("999")
    assert exc.value.status == 404
    assert exc.value.code == "TRANSACCION_NO_ENCONTRADA"
    assert not exc.value.retryable


@pytest.mark.parametrize(
    ("override", "field"),
    [
        ({"amount": "10.505"}, "amount"),
        ({"amount": "0"}, "amount"),
        ({"reference": "../etc"}, "reference"),
        ({"reference": "ord-42"}, "reference"),
        ({"description": "  "}, "description"),
        ({"expires_in": 0}, "expires_in"),
    ],
)
def test_validation_happens_before_network(mock, qr_args, override, field):
    with pytest.raises(ValidationError) as exc:
        mock.client().generate_qr(**{**qr_args, **override})
    assert exc.value.field == field
    assert not exc.value.retryable
    assert all(not r.url.path.endswith("/generate") for r in mock.requests)


def test_floats_are_rejected(mock, qr_args):
    with pytest.raises(TypeError, match="floats"):
        mock.client().generate_qr(**{**qr_args, "amount": 10.5})


def test_cancel_pending_qr(mock, qr_args):
    qr_client = mock.client()
    qr = qr_client.generate_qr(**qr_args)
    cancelled = qr_client.cancel_qr(qr.reference)
    assert cancelled.reference == qr.reference
    assert cancelled.status is PaymentStatus.CANCELLED
    assert qr_client.get_qr_status(qr.reference).status is PaymentStatus.CANCELLED


def test_cancel_paid_qr_fails(mock, qr_args):
    qr_client = mock.client()
    qr = qr_client.generate_qr(**qr_args)
    mock.pay(qr.reference)
    with pytest.raises(ApiError) as exc:
        qr_client.cancel_qr(qr.reference)
    assert (exc.value.status, exc.value.code) == (409, "ESTADO_INVALIDO")


def _session_failing_on(mock, path_suffix, response_or_exc):
    def handler(request: httpx.Request) -> httpx.Response:
        if request.url.path.endswith(path_suffix):
            if isinstance(response_or_exc, Exception):
                raise response_or_exc
            return response_or_exc
        return mock.transport.handle_request(request)

    http = httpx.Client(transport=httpx.MockTransport(handler))
    return Session(mock.client_id, mock.client_secret, base_url=mock.BASE_URL, http_client=http)


def test_gateway_errors_are_retryable(mock, qr_args):
    session = _session_failing_on(
        mock, "/generate", httpx.Response(502, text="Error forwarding call")
    )
    with pytest.raises(ApiError) as exc:
        QrClient(session).generate_qr(**qr_args)
    assert exc.value.status == 502 and exc.value.retryable


def test_lost_generate_response_is_ambiguous(mock, qr_args):
    session = _session_failing_on(mock, "/generate", httpx.ReadTimeout("no response"))
    with pytest.raises(AmbiguousOutcomeError) as exc:
        QrClient(session).generate_qr(**qr_args)
    assert exc.value.operation == "qr.generate"


def test_lost_status_response_is_a_plain_transport_error(mock):
    session = _session_failing_on(mock, "/verify/1", httpx.ReadTimeout("no response"))
    with pytest.raises(TransportError) as exc:
        QrClient(session).get_qr_status("1")
    assert exc.value.retryable


def test_connection_refused_on_generate_is_safe_to_retry(mock, qr_args):
    session = _session_failing_on(mock, "/generate", httpx.ConnectError("refused"))
    with pytest.raises(TransportError) as exc:
        QrClient(session).generate_qr(**qr_args)
    assert not exc.value.maybe_sent
