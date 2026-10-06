"""How requests built by the native core reach OpenHub.

The core never does I/O: it returns a request dict, a :class:`Transport`
sends it. Swap the transport to add proxies, logging, retries or fakes.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Protocol

import httpx

from .errors import TransportError

Request = dict[str, Any]
"""``{"method", "url", "headers", "body"}`` as built by the core, plus an
optional ``timeout`` (seconds) when the endpoint has a recommended one."""


@dataclass(frozen=True, slots=True)
class Response:
    status: int
    body: str

    def to_wire(self) -> dict[str, Any]:
        return {"status": self.status, "body": self.body}


class Transport(Protocol):
    def send(self, request: Request) -> Response: ...

    def close(self) -> None: ...


class AsyncTransport(Protocol):
    async def send(self, request: Request) -> Response: ...

    async def aclose(self) -> None: ...


# Failures where the request provably never reached the server.
_NOT_SENT = (httpx.ConnectError, httpx.ConnectTimeout, httpx.PoolTimeout)


def _transport_error(exc: httpx.HTTPError) -> TransportError:
    return TransportError(f"{type(exc).__name__}: {exc}", maybe_sent=not isinstance(exc, _NOT_SENT))


def _timeout(request: Request) -> dict[str, Any]:
    timeout = request.get("timeout")
    return {} if timeout is None else {"timeout": timeout}


class HttpxTransport:
    def __init__(self, client: httpx.Client | None = None, *, timeout: float = 30.0) -> None:
        self._client = client or httpx.Client(timeout=timeout)
        self._owns_client = client is None

    def send(self, request: Request) -> Response:
        try:
            r = self._client.request(
                request["method"],
                request["url"],
                headers=request["headers"],
                content=request["body"],
                **_timeout(request),
            )
        except httpx.HTTPError as exc:
            raise _transport_error(exc) from exc
        return Response(r.status_code, r.text)

    def close(self) -> None:
        if self._owns_client:
            self._client.close()


class AsyncHttpxTransport:
    def __init__(self, client: httpx.AsyncClient | None = None, *, timeout: float = 30.0) -> None:
        self._client = client or httpx.AsyncClient(timeout=timeout)
        self._owns_client = client is None

    async def send(self, request: Request) -> Response:
        try:
            r = await self._client.request(
                request["method"],
                request["url"],
                headers=request["headers"],
                content=request["body"],
                **_timeout(request),
            )
        except httpx.HTTPError as exc:
            raise _transport_error(exc) from exc
        return Response(r.status_code, r.text)

    async def aclose(self) -> None:
        if self._owns_client:
            await self._client.aclose()
