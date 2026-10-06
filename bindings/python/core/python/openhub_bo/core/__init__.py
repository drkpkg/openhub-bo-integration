"""Shared foundation of the openhub-bo packages: session, transport, errors and
common types for the Red Enlace (ATC) OpenHub APIs."""

from ._bridge import SUPPORTED_PROTOCOL, Native
from .errors import (
    AmbiguousOutcomeError,
    ApiError,
    ApiFieldError,
    AuthenticationError,
    CoreError,
    DecodeError,
    OpenHubError,
    TransportError,
    ValidationError,
    WebhookAuthError,
)
from .models import BOLIVIA_TZ, Environment, PaymentStatus, Webhook
from .ops import Handler, Op
from .session import NATIVE, AsyncSession, Session
from .transport import (
    AsyncHttpxTransport,
    AsyncTransport,
    HttpxTransport,
    Request,
    Response,
    Transport,
)

__version__ = NATIVE.version

__all__ = [
    "BOLIVIA_TZ",
    "SUPPORTED_PROTOCOL",
    "AmbiguousOutcomeError",
    "ApiError",
    "ApiFieldError",
    "AsyncHttpxTransport",
    "AsyncSession",
    "AsyncTransport",
    "AuthenticationError",
    "CoreError",
    "DecodeError",
    "Environment",
    "Handler",
    "HttpxTransport",
    "Native",
    "Op",
    "OpenHubError",
    "PaymentStatus",
    "Request",
    "Response",
    "Session",
    "Transport",
    "TransportError",
    "ValidationError",
    "Webhook",
    "WebhookAuthError",
    "__version__",
]
