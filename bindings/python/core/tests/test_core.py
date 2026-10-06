from __future__ import annotations

import json
import types

import httpx
import pytest

from openhub_bo.core import (
    AmbiguousOutcomeError,
    ApiError,
    AuthenticationError,
    Environment,
    HttpxTransport,
    Native,
    Op,
    Session,
    TransportError,
    ValidationError,
    Webhook,
    __version__,
)
from openhub_bo.core.errors import error_from_core
from openhub_bo.core.session import NATIVE
from openhub_bo.core.testing import MockGateway


def test_native_catalog_and_version():
    assert __version__
    assert NATIVE.operations["token"] == {"kind": "operation", "name": "token", "idempotent": True}


def test_protocol_mismatch_is_an_import_error():
    fake = types.SimpleNamespace(__name__="fake._native", PROTOCOL_VERSION=1, __version__="0")
    with pytest.raises(ImportError, match="protocol 1"):
        Native(fake)  # type: ignore[arg-type]


def test_declaring_unknown_operation_fails_at_import_time():
    with pytest.raises(ImportError, match=r"qr\.nope"):
        Op(NATIVE, "qr.nope", lambda v: v)


def test_missing_credentials_fail_at_construction():
    with pytest.raises(ValidationError) as exc:
        Session("client-id")
    assert exc.value.field == "client_secret"


def test_environment_defaults_to_sandbox():
    session = Session("id", "secret")
    assert session.environment is Environment.SANDBOX
    session.close()


def test_token_is_fetched_and_cached():
    mock = MockGateway()
    with mock.session() as session:
        first = session._ensure_token()
        assert session._ensure_token() is first
    assert mock.token_requests == 1
    assert first["token_type"] == "access_token"


def test_bad_credentials_raise_authentication_error():
    mock = MockGateway()
    with mock.session(client_secret="wrong") as session, pytest.raises(AuthenticationError):
        session._ensure_token()


@pytest.mark.parametrize(
    ("exc", "maybe_sent"),
    [
        (httpx.ConnectError("refused"), False),
        (httpx.ConnectTimeout("slow connect"), False),
        (httpx.ReadTimeout("slow response"), True),
        (httpx.RemoteProtocolError("dropped"), True),
    ],
)
def test_transport_classifies_whether_request_was_sent(exc, maybe_sent):
    def boom(request: httpx.Request) -> httpx.Response:
        raise exc

    transport = HttpxTransport(httpx.Client(transport=httpx.MockTransport(boom)))
    with pytest.raises(TransportError) as raised:
        transport.send({"method": "GET", "url": "https://x.bo", "headers": {}, "body": None})
    assert raised.value.maybe_sent is maybe_sent
    assert raised.value.retryable


def test_ambiguous_outcome_is_not_a_transport_error():
    # Generic `except TransportError: retry` loops must not repeat it blindly.
    error = AmbiguousOutcomeError("qr.generate", TransportError("lost", maybe_sent=True))
    assert not isinstance(error, TransportError)
    assert not error.retryable


def test_api_errors_carry_retryable_and_code():
    error = error_from_core(
        {
            "kind": "api",
            "status": 502,
            "message": "down",
            "errors": [{"message": "m", "code": "X"}],
            "retryable": True,
        }
    )
    assert isinstance(error, ApiError)
    assert error.retryable and error.code == "X"


def test_native_errors_round_trip_through_json():
    with pytest.raises(ValidationError):
        NATIVE.invoke(
            "token.build", {"config": json.loads('{"client_id": ""}'), "input": {"now": 0}}
        )


def test_webhook_secret_is_not_in_repr():
    assert "s3cret" not in repr(Webhook(url="https://a.bo/h", value="s3cret"))


def test_ambiguous_core_errors_name_the_operation():
    error = error_from_core(
        {
            "kind": "ambiguous",
            "message": "no confirmado",
            "errors": [{"message": "m", "code": "96"}],
        },
        "payouts.pay.parse",
    )
    assert isinstance(error, AmbiguousOutcomeError)
    assert (error.operation, error.code, error.retryable) == ("payouts.pay", "96", False)


def test_op_timeout_reaches_the_transport():
    seen = {}

    def handler(request: httpx.Request) -> httpx.Response:
        seen["timeout"] = request.extensions.get("timeout")
        return httpx.Response(200, text="{}")

    transport = HttpxTransport(httpx.Client(transport=httpx.MockTransport(handler)))
    transport.send(
        {"method": "GET", "url": "https://x.bo", "headers": {}, "body": None, "timeout": 90.0}
    )
    assert seen["timeout"]["read"] == 90.0
