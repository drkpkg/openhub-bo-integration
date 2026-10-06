from __future__ import annotations

from collections.abc import Mapping
from typing import Any

from openhub_bo.core import Webhook

from ._ops import PARSE_BINANCE_WEBHOOK, PARSE_FX_WEBHOOK
from .models import BinanceNotification, FxNotification


def _call(headers: Mapping[str, str], body: bytes | str, webhook: Webhook) -> dict[str, Any]:
    return {
        "headers": {str(k): str(v) for k, v in headers.items()},
        "body": body.decode("utf-8") if isinstance(body, bytes) else body,
        "key": webhook.key,
        "value": webhook.value,
    }


def parse_binance_webhook(
    headers: Mapping[str, str], body: bytes | str, *, webhook: Webhook
) -> BinanceNotification:
    """Authenticates and parses a Binance Pay notification.

    Answer with HTTP 200 and ``notification.ack`` as the JSON body; ATC requires
    it. Confirm with ``BinanceClient.get_status`` before releasing goods.
    """
    return PARSE_BINANCE_WEBHOOK(_call(headers, body, webhook))


def parse_fx_webhook(
    headers: Mapping[str, str], body: bytes | str, *, webhook: Webhook
) -> FxNotification:
    """Authenticates and reads a PIX / Koibanx notification.

    Their payload is undocumented: fields are best-effort and the full body is in
    ``notification.payload``. Always confirm with ``get_status``.
    """
    return PARSE_FX_WEBHOOK(_call(headers, body, webhook))
