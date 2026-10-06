"""Minimal webhook receiver to capture real OpenHub QR notifications.

Records every call **raw** (headers + body) to a JSONL file so the real payload
format can be compared with the docs, authenticates it with ``parse_webhook``,
confirms the payment with a status query, and answers HTTP 200.

Routes: ``POST /qr/simple`` and ``POST /qr/mld`` (the kind is used to confirm the
status), ``GET /health``.

Environment: CLIENT_ID, CLIENT_SECRET, WEBHOOK_SECRET (header value set when
generating the QR), optional WEBHOOK_HEADER (default x-api-key), PORT (default
8000), CAPTURE_FILE (default captures/webhooks.jsonl), OPENHUB_ENVIRONMENT.

    set -a && . ../../.env && set +a
    WEBHOOK_SECRET=... uv run --no-sync python examples/webhook_receiver.py

Only stdlib + openhub-bo-qr, so it can also be deployed as-is (Procfile:
``web: python examples/webhook_receiver.py``).
"""

from __future__ import annotations

import json
import os
import sys
import threading
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

from openhub_bo.core import Environment, OpenHubError, Session, Webhook, WebhookAuthError
from openhub_bo.qr import QrClient, QrKind, parse_webhook

ROUTES = {"/qr/simple": QrKind.SIMPLE, "/qr/mld": QrKind.MLD}

WEBHOOK = Webhook(
    url="(receiver)",
    value=os.environ["WEBHOOK_SECRET"],
    key=os.environ.get("WEBHOOK_HEADER", "x-api-key"),
)
CAPTURE = Path(os.environ.get("CAPTURE_FILE", "captures/webhooks.jsonl"))
SESSION = Session(
    os.environ["CLIENT_ID"],
    os.environ["CLIENT_SECRET"],
    environment=os.environ.get("OPENHUB_ENVIRONMENT", Environment.SANDBOX),
)
QR = QrClient(SESSION)
_write_lock = threading.Lock()


def _record(entry: dict[str, Any]) -> None:
    CAPTURE.parent.mkdir(parents=True, exist_ok=True)
    with _write_lock, CAPTURE.open("a", encoding="utf-8") as f:
        f.write(json.dumps(entry, ensure_ascii=False, default=str) + "\n")


class Handler(BaseHTTPRequestHandler):
    server_version = "openhub-webhook-receiver"

    def do_GET(self) -> None:
        if self.path == "/health":
            self._reply(200, {"ok": True})
        else:
            self._reply(404, {"error": "not found"})

    def do_POST(self) -> None:
        body = self.rfile.read(int(self.headers.get("Content-Length") or 0))
        headers = dict(self.headers.items())
        entry: dict[str, Any] = {
            "received_at": datetime.now(timezone.utc).isoformat(),
            "path": self.path,
            "client": self.client_address[0],
            # Never store the shared secret; only whether it matched.
            "headers": {
                k: ("<secret>" if k.lower() == WEBHOOK.key.lower() else v)
                for k, v in headers.items()
            },
            "body": body.decode("utf-8", errors="replace"),
        }
        kind = ROUTES.get(self.path.split("?", 1)[0])
        status, reply = 200, {"ok": True}
        try:
            notification = parse_webhook(headers, body, webhook=WEBHOOK)
            entry["parsed"] = {
                "reference": notification.reference,
                "status": notification.status.value,
                "response_code": notification.response_code,
                "amount": str(notification.amount),
            }
            if kind is not None:
                confirmed = QR.get_qr_status(notification.reference, kind=kind)
                entry["confirmed_status"] = confirmed.status.value
                entry["confirmed_amount"] = str(confirmed.amount)
        except WebhookAuthError as exc:
            entry["error"] = f"auth: {exc}"
            status, reply = 401, {"error": "unauthorized"}
        except OpenHubError as exc:
            # Unknown payload shape: keep it (that's the point) and still ack.
            entry["error"] = f"{type(exc).__name__}: {exc}"
        _record(entry)
        outcome = entry.get("parsed") or entry.get("error")
        print(
            f"[{entry['received_at']}] POST {self.path} -> {status} {outcome} "
            f"confirmed={entry.get('confirmed_status')}",
            flush=True,
        )
        self._reply(status, reply)

    def _reply(self, status: int, payload: dict[str, Any]) -> None:
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, format: str, *args: Any) -> None:  # quiet default logging
        pass


def main() -> int:
    port = int(os.environ.get("PORT", "8000"))
    print(f"listening on :{port}, capturing to {CAPTURE}", flush=True)
    ThreadingHTTPServer(("0.0.0.0", port), Handler).serve_forever()
    return 0


if __name__ == "__main__":
    sys.exit(main())
