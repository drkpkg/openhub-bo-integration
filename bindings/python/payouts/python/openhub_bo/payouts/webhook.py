from __future__ import annotations

from ._ops import PARSE_WEBHOOK
from .models import BatchNotification


def parse_batch_webhook(token: str, body: bytes | str, *, expected_token: str) -> BatchNotification:
    """Authenticates and parses one batch-transfer notification.

    ``token`` is the ``token`` query parameter of the incoming request;
    ``expected_token`` the ``webhook_token`` given to ``authorize_batch``.
    Answer with HTTP 200 and ``notification.ack()`` as the JSON body.
    """
    return PARSE_WEBHOOK(
        {
            "token": token,
            "body": body.decode("utf-8") if isinstance(body, bytes) else body,
            "expected_token": expected_token,
        }
    )
