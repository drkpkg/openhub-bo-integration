"""Credentials, token cache and execution of operations.

A session is shared by every product client (``QrClient(session)``,
``PixClient(session)``, ...) so they reuse one OAuth token.
"""

from __future__ import annotations

import asyncio
import threading
import time
from types import TracebackType
from typing import Any, TypeVar

import httpx

from . import _native
from ._bridge import Native
from .errors import AmbiguousOutcomeError, AuthenticationError, TransportError
from .models import Environment
from .ops import Op
from .transport import (
    AsyncHttpxTransport,
    AsyncTransport,
    HttpxTransport,
    Request,
    Response,
    Transport,
)

T = TypeVar("T")

NATIVE = Native(_native)
TOKEN: Op[dict[str, Any]] = Op(NATIVE, "token", lambda value: value)


class _SessionState:
    """Everything except I/O and locking, shared by both session flavours."""

    def __init__(
        self,
        client_id: str,
        client_secret: str | None,
        basic_token: str | None,
        environment: Environment | str,
        base_url: str | None,
    ) -> None:
        self.config: dict[str, Any] = {
            "client_id": client_id,
            "client_secret": client_secret,
            "basic_token": basic_token,
            "environment": Environment(environment).value,
            "base_url": base_url,
        }
        self._token: dict[str, Any] | None = None
        # Fail fast on missing credentials instead of at the first request.
        self.token_request()

    @property
    def environment(self) -> Environment:
        return Environment(self.config["environment"])

    def valid_token(self) -> dict[str, Any] | None:
        token = self._token
        return token if token and time.time() < token["expires_at"] else None

    def token_request(self) -> Request:
        return TOKEN.build(self.config, None, self._token_input())

    def accept_token(self, response: Response) -> dict[str, Any]:
        self._token = TOKEN.parse(self._token_input(), response)
        return self._token

    def invalidate(self, token: dict[str, Any]) -> None:
        """Drops ``token`` unless another caller already refreshed it."""
        if self._token is not None and self._token["access_token"] == token["access_token"]:
            self._token = None

    @staticmethod
    def classify(op: Op[Any], exc: TransportError) -> Exception:
        if exc.maybe_sent and not op.idempotent:
            return AmbiguousOutcomeError(op.name, exc)
        return exc

    @staticmethod
    def _token_input() -> dict[str, Any]:
        return {"now": int(time.time())}


class Session:
    """Synchronous session. Thread-safe; close it (or use ``with``) when done.

    >>> with Session("client-id", "client-secret") as session:
    ...     qr = QrClient(session)
    """

    def __init__(
        self,
        client_id: str,
        client_secret: str | None = None,
        *,
        basic_token: str | None = None,
        environment: Environment | str = Environment.SANDBOX,
        base_url: str | None = None,
        timeout: float = 30.0,
        http_client: httpx.Client | None = None,
        transport: Transport | None = None,
    ) -> None:
        self._state = _SessionState(client_id, client_secret, basic_token, environment, base_url)
        self._transport = transport or HttpxTransport(http_client, timeout=timeout)
        self._lock = threading.Lock()

    @property
    def environment(self) -> Environment:
        return self._state.environment

    def execute(self, op: Op[T], input: dict[str, Any]) -> T:
        """Runs ``op``: token, build, send, parse; refreshes a rejected token once."""
        for attempt in range(2):
            token = self._ensure_token()
            response = self._send(op, op.build(self._state.config, token, input))
            try:
                return op.parse(input, response)
            except AuthenticationError:
                if attempt:
                    raise
                self._state.invalidate(token)
        raise AssertionError("unreachable")

    def _ensure_token(self) -> dict[str, Any]:
        with self._lock:
            token = self._state.valid_token()
            if token is None:
                response = self._send(TOKEN, self._state.token_request())
                token = self._state.accept_token(response)
            return token

    def _send(self, op: Op[Any], request: Request) -> Response:
        try:
            return self._transport.send(request)
        except TransportError as exc:
            raise self._state.classify(op, exc) from exc

    def close(self) -> None:
        self._transport.close()

    def __enter__(self) -> Session:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        tb: TracebackType | None,
    ) -> None:
        self.close()


class AsyncSession:
    """Asyncio counterpart of :class:`Session`."""

    def __init__(
        self,
        client_id: str,
        client_secret: str | None = None,
        *,
        basic_token: str | None = None,
        environment: Environment | str = Environment.SANDBOX,
        base_url: str | None = None,
        timeout: float = 30.0,
        http_client: httpx.AsyncClient | None = None,
        transport: AsyncTransport | None = None,
    ) -> None:
        self._state = _SessionState(client_id, client_secret, basic_token, environment, base_url)
        self._transport = transport or AsyncHttpxTransport(http_client, timeout=timeout)
        # Created lazily so the session is not bound to the loop alive at __init__.
        self._lock: asyncio.Lock | None = None

    @property
    def environment(self) -> Environment:
        return self._state.environment

    async def execute(self, op: Op[T], input: dict[str, Any]) -> T:
        """Runs ``op``: token, build, send, parse; refreshes a rejected token once."""
        for attempt in range(2):
            token = await self._ensure_token()
            response = await self._send(op, op.build(self._state.config, token, input))
            try:
                return op.parse(input, response)
            except AuthenticationError:
                if attempt:
                    raise
                self._state.invalidate(token)
        raise AssertionError("unreachable")

    async def _ensure_token(self) -> dict[str, Any]:
        if self._lock is None:
            self._lock = asyncio.Lock()
        async with self._lock:
            token = self._state.valid_token()
            if token is None:
                response = await self._send(TOKEN, self._state.token_request())
                token = self._state.accept_token(response)
            return token

    async def _send(self, op: Op[Any], request: Request) -> Response:
        try:
            return await self._transport.send(request)
        except TransportError as exc:
            raise self._state.classify(op, exc) from exc

    async def aclose(self) -> None:
        await self._transport.aclose()

    async def __aenter__(self) -> AsyncSession:
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        tb: TracebackType | None,
    ) -> None:
        await self.aclose()
