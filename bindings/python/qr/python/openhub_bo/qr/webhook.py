from __future__ import annotations

from collections.abc import Mapping

from openhub_bo.core import Webhook

from ._ops import PARSE_WEBHOOK
from .models import PaymentNotification


def parse_webhook(
    headers: Mapping[str, str],
    body: bytes | str,
    *,
    webhook: Webhook,
) -> PaymentNotification:
    """Authenticates and parses a payment notification sent by ATC.

    ``webhook`` must be the same :class:`~openhub_bo.core.Webhook` passed to
    ``generate_qr``. Raises :class:`~openhub_bo.core.WebhookAuthError` if the
    auth header is missing or wrong.

    ATC does not sign payloads, so before releasing goods confirm the payment
    with ``get_qr_status(notification.reference)`` and check the amount.
    """
    text = body.decode("utf-8") if isinstance(body, bytes) else body
    return PARSE_WEBHOOK(
        {
            "headers": {str(k): str(v) for k, v in headers.items()},
            "body": text,
            "key": webhook.key,
            "value": webhook.value,
        }
    )
