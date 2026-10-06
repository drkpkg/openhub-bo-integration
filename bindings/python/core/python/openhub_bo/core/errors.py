"""Exceptions raised by openhub-bo packages. All inherit from :class:`OpenHubError`."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True, slots=True)
class ApiFieldError:
    message: str
    field: str | None = None
    code: str | None = None


class OpenHubError(Exception):
    """Base class for every error raised by openhub-bo."""

    retryable: bool = False
    """Whether repeating the same call later may succeed."""


class ValidationError(OpenHubError, ValueError):
    """Input rejected locally, before calling OpenHub."""

    def __init__(self, message: str, *, field: str) -> None:
        super().__init__(f"invalid `{field}`: {message}")
        self.field = field


class AuthenticationError(OpenHubError):
    """Credentials or access token rejected by OpenHub."""

    def __init__(self, message: str, *, status: int) -> None:
        super().__init__(f"authentication failed (HTTP {status}): {message}")
        self.status = status


class ApiError(OpenHubError):
    """OpenHub answered with an error (``success: false``, non-2xx, ...)."""

    def __init__(
        self, message: str, *, status: int, errors: list[ApiFieldError], retryable: bool = False
    ) -> None:
        super().__init__(f"OpenHub error (HTTP {status}): {message}")
        self.status = status
        self.errors = errors
        self.retryable = retryable

    @property
    def code(self) -> str | None:
        """Code of the first field error, e.g. ``TRANSACCION_NO_ENCONTRADA``."""
        return next((e.code for e in self.errors if e.code), None)


class DecodeError(OpenHubError):
    """The response could not be understood."""


class WebhookAuthError(OpenHubError):
    """Incoming webhook did not carry the expected auth header."""


class TransportError(OpenHubError):
    """Network failure while talking to OpenHub.

    ``maybe_sent`` is False when the request never left (connection refused,
    connect timeout); retrying is then always safe.
    """

    retryable = True

    def __init__(self, message: str, *, maybe_sent: bool) -> None:
        super().__init__(message)
        self.maybe_sent = maybe_sent


class AmbiguousOutcomeError(OpenHubError):
    """A non-idempotent call (generating a QR, paying a payout...) may or may
    not have been applied: the response was lost, or OpenHub reported the
    result as unconfirmed (payout codes ``94``/``96``).

    Not a :class:`TransportError` on purpose, so generic retry loops do not
    repeat it blindly. Query the status or reconcile before trying again.
    """

    def __init__(
        self, operation: str, cause: Exception | str, *, errors: list[ApiFieldError] | None = None
    ) -> None:
        super().__init__(f"outcome of `{operation}` is unknown: {cause}")
        self.operation = operation
        self.errors = errors or []

    @property
    def code(self) -> str | None:
        return next((e.code for e in self.errors if e.code), None)


class CoreError(OpenHubError):
    """Contract violation between a package and its native module (a bug)."""


def _field_errors(data: dict[str, Any]) -> list[ApiFieldError]:
    return [
        ApiFieldError(message=e["message"], field=e.get("field"), code=e.get("code"))
        for e in data.get("errors", [])
    ]


def error_from_core(data: dict[str, Any], op: str = "") -> OpenHubError:
    """Maps a core error; ``op`` is the native op name (``payouts.pay.parse``)."""
    kind = data.get("kind")
    message = data.get("message", "unknown error")
    if kind == "ambiguous":
        operation = op.rsplit(".", 1)[0] if op.endswith((".build", ".parse")) else op
        return AmbiguousOutcomeError(operation, message, errors=_field_errors(data))
    if kind == "validation":
        return ValidationError(message, field=data["field"])
    if kind == "authentication":
        return AuthenticationError(message, status=data["status"])
    if kind == "api":
        return ApiError(
            message,
            status=data["status"],
            errors=_field_errors(data),
            retryable=bool(data.get("retryable", False)),
        )
    if kind == "decode":
        return DecodeError(message)
    if kind == "webhook_auth":
        return WebhookAuthError(message)
    return CoreError(message)
