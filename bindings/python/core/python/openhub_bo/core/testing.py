"""In-memory fake of the OpenHub gateway (Sensedia) for tests.

Handles the OAuth token and the ``access_token``/``client_id`` check like the
real gateway; product packages register their routes on top (see
``openhub_bo.qr.testing.MockOpenHub``).
"""

from __future__ import annotations

import base64
import re
import uuid
from collections.abc import Callable
from typing import Any

import httpx

from .session import AsyncSession, Session

RouteHandler = Callable[[httpx.Request, "re.Match[str]"], httpx.Response]

_TOKEN = "/oauth-client-credentials/access-token"


class MockGateway:
    BASE_URL = "https://openhub.mock"

    def __init__(
        self,
        *,
        client_id: str = "test-client",
        client_secret: str = "test-secret",
        token_ttl: int = 3600,
    ) -> None:
        self.client_id = client_id
        self.client_secret = client_secret
        self.token_ttl = token_ttl
        self.token_requests = 0
        self.requests: list[httpx.Request] = []
        self._tokens: set[str] = set()
        self._routes: list[tuple[str, re.Pattern[str], RouteHandler]] = []
        self.transport = httpx.MockTransport(self._handle)

    def route(self, method: str, pattern: str, handler: RouteHandler) -> None:
        """Registers ``handler`` for authenticated calls whose path matches ``pattern``."""
        self._routes.append((method.upper(), re.compile(pattern), handler))

    def session(self, **kwargs: Any) -> Session:
        return Session(
            self.client_id,
            kwargs.pop("client_secret", self.client_secret),
            base_url=self.BASE_URL,
            http_client=httpx.Client(transport=self.transport),
            **kwargs,
        )

    def async_session(self, **kwargs: Any) -> AsyncSession:
        return AsyncSession(
            self.client_id,
            kwargs.pop("client_secret", self.client_secret),
            base_url=self.BASE_URL,
            http_client=httpx.AsyncClient(transport=self.transport),
            **kwargs,
        )

    def revoke_tokens(self) -> None:
        """Invalidates every issued token, as if they had expired server-side."""
        self._tokens.clear()

    def _handle(self, request: httpx.Request) -> httpx.Response:
        self.requests.append(request)
        path = request.url.path
        if path.endswith(_TOKEN):
            return self._token(request)
        if not self._authorized(request):
            return httpx.Response(
                401,
                text="Access Token in the request, identified by HEADER access_token, is invalid.",
            )
        for method, pattern, handler in self._routes:
            if request.method == method and (match := pattern.search(path)):
                return handler(request, match)
        return httpx.Response(404, json={"success": False, "message": "Not found", "errors": []})

    def _token(self, request: httpx.Request) -> httpx.Response:
        self.token_requests += 1
        expected = base64.b64encode(f"{self.client_id}:{self.client_secret}".encode()).decode()
        if request.headers.get("authorization") != f"Basic {expected}":
            return httpx.Response(
                401, json={"error": "invalid_client", "error_description": "Bad credentials"}
            )
        token = str(uuid.uuid4())
        self._tokens.add(token)
        # Sandbox answers 201 with token_type "access_token" and no scope.
        return httpx.Response(
            201,
            json={
                "access_token": token,
                "token_type": "access_token",
                "expires_in": self.token_ttl,
            },
        )

    def _authorized(self, request: httpx.Request) -> bool:
        # Like Sensedia: the token travels in the `access_token` header.
        return (
            request.headers.get("access_token") in self._tokens
            and request.headers.get("client_id") == self.client_id
        )


def envelope_ok(message: str, data: dict[str, Any]) -> httpx.Response:
    """``{success: true, message, data}`` as used by the QR family."""
    return httpx.Response(200, json={"success": True, "message": message, "data": data})


def envelope_error(
    status: int, message: str, errors: list[dict[str, Any]] | None = None
) -> httpx.Response:
    """``{success: false, message, errors}`` as used by the QR family."""
    return httpx.Response(
        status, json={"success": False, "message": message, "errors": errors or []}
    )
