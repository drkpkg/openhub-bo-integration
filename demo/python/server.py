"""OpenHub QR demo service — Python (openhub-bo-qr).

Serves its demo app (demo/apps/tienda) and the demo API shared by the three language
services. Environment: CLIENT_ID, CLIENT_SECRET, DEMO_PASSWORD, PORT (8101),
PUBLIC_URL (optional; else derived from the Host header), DEMO_LINKS (optional
"Python=https://...,TypeScript=...,Ruby=...").
"""

from __future__ import annotations

import base64
import json
import os
import re
import secrets
import threading
import time
from collections import deque
from datetime import datetime, timezone
from http.cookies import SimpleCookie
from decimal import Decimal, InvalidOperation
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from importlib.metadata import version
from pathlib import Path
from typing import Any
from urllib.parse import parse_qs

from openhub_bo.core import (
    ApiError,
    Environment,
    OpenHubError,
    Session,
    ValidationError,
    Webhook,
    WebhookAuthError,
)
from openhub_bo.qr import QrClient, QrKind, parse_webhook

LANGUAGE = "Python"
APP = "tienda"
COOKIE = f"demo_{APP}"  # per app: browsers share cookies across localhost ports
MAX_AMOUNT = Decimal("10")
WEB = Path(__file__).resolve().parents[1] / "web"
INDEX = (WEB.parent / "apps" / APP / "index.html").read_bytes()
LOGIN = (WEB / "login.html").read_bytes()
SESSIONS: set[str] = set()
PASSWORD = os.environ["DEMO_PASSWORD"]
WEBHOOK_SECRET = secrets.token_urlsafe(32)
SESSION = Session(os.environ["CLIENT_ID"], os.environ["CLIENT_SECRET"], environment=Environment.SANDBOX)
QR = QrClient(SESSION)
EVENTS: deque[dict[str, Any]] = deque(maxlen=50)
PAYMENTS: dict[str, dict[str, Any]] = {}  # "kind:reference" -> record shown by the app
LOCK = threading.Lock()


def links() -> list[dict[str, str]]:
    raw = os.environ.get("DEMO_LINKS", "")
    return [dict(zip(("language", "url"), part.split("=", 1), strict=True)) for part in raw.split(",") if "=" in part]


def new_reference() -> str:
    # OpenHub stores the merchant reference as a 32-bit integer.
    return str(int(time.time() * 1000) % 2_147_483_647)


def track(kind: str, reference: str, **fields: Any) -> None:
    """Keeps the app's payment list in sync with what OpenHub reports."""
    with LOCK:
        record = PAYMENTS.get(f"{kind}:{reference}")
        if record is not None:
            record.update(fields, updatedAt=datetime.now(timezone.utc).isoformat())


def qr_json(qr: Any, webhook_url: str) -> dict[str, Any]:
    return {
        "kind": qr.kind.value, "reference": qr.reference, "merchantReference": qr.merchant_reference,
        "status": qr.status.value, "rawStatus": qr.raw_status, "amount": str(qr.amount),
        "currency": qr.currency, "expiresAt": qr.expires_at.isoformat() if qr.expires_at else None,
        "qrDataUri": qr.qr_data_uri, "webhookUrl": webhook_url,
    }


def status_json(st: Any) -> dict[str, Any]:
    return {
        "kind": st.kind.value, "reference": st.reference, "status": st.status.value,
        "rawStatus": st.raw_status, "message": st.message,
        "amount": None if st.amount is None else str(st.amount),
        "payer": st.payer.name if st.payer else None,
        "payerBank": st.payer_bank.bank_name if st.payer_bank else None,
    }


class Handler(BaseHTTPRequestHandler):
    server_version = "openhub-demo-python"

    # -- routing -----------------------------------------------------------------------

    def do_GET(self) -> None:
        path = self.path.split("?", 1)[0]
        if path == "/health":
            return self._json(200, {"ok": True, "language": LANGUAGE})
        if path == "/login":
            return self._send(200, LOGIN, "text/html; charset=utf-8")
        if not self._authorized():
            return None
        if path == "/":
            return self._send(200, INDEX, "text/html; charset=utf-8")
        if path == "/api/info":
            return self._json(200, {"language": LANGUAGE, "app": APP, "library": "openhub-bo-qr",
                                    "version": version("openhub-bo-qr"), "environment": "sandbox",
                                    "links": links()})
        if path == "/api/payments":
            with LOCK:
                return self._json(200, sorted(PAYMENTS.values(), key=lambda r: r["createdAt"], reverse=True))
        if path == "/api/events":
            with LOCK:
                return self._json(200, list(reversed(EVENTS)))
        if m := re.fullmatch(r"/api/qr/(simple|mld)/(\d+)", path):
            return self._call(lambda: self._status(QrKind(m.group(1)), m.group(2)))
        return self._json(404, {"error": "not found"})

    def do_POST(self) -> None:
        path = self.path.split("?", 1)[0]
        body = self.rfile.read(int(self.headers.get("Content-Length") or 0))
        if m := re.fullmatch(r"/webhooks/qr/(simple|mld)", path):
            return self._webhook(QrKind(m.group(1)), body)
        if path == "/login":
            return self._login(body)
        if not self._authorized():
            return None
        if path == "/api/qr":
            return self._call(lambda: self._generate(json.loads(body or b"{}")))
        if m := re.fullmatch(r"/api/qr/simple/(\d+)/cancel", path):
            return self._call(lambda: self._cancel(m.group(1)))
        return self._json(404, {"error": "not found"})

    # -- handlers ----------------------------------------------------------------------

    def _generate(self, form: dict[str, Any]) -> dict[str, Any]:
        try:
            amount = Decimal(str(form.get("amount", "")))
        except InvalidOperation:
            raise ValidationError("must be a decimal number", field="amount") from None
        if amount > MAX_AMOUNT:
            raise ValidationError(f"the demo allows at most Bs {MAX_AMOUNT}", field="amount")
        kind = QrKind(form.get("kind", "simple"))
        webhook_url = f"{self._public_url()}/webhooks/qr/{kind.value}"
        qr = QR.generate_qr(
            amount=amount, description=str(form.get("description", "Prueba demo")),
            reference=new_reference(), establishment_id=1, establishment_name="Demo openhub bo",
            expires_in=int(form.get("expiresIn", 600)),
            webhook=Webhook(url=webhook_url, value=WEBHOOK_SECRET), kind=kind,
        )
        result = qr_json(qr, webhook_url)
        now = datetime.now(timezone.utc).isoformat()
        with LOCK:
            PAYMENTS[f"{qr.kind.value}:{qr.reference}"] = {
                "kind": qr.kind.value, "reference": qr.reference,
                "merchantReference": qr.merchant_reference,
                "label": str(form.get("label") or form.get("description") or ""),
                "amount": str(qr.amount), "currency": qr.currency, "status": qr.status.value,
                "expiresAt": result["expiresAt"], "createdAt": now, "updatedAt": now,
            }
        return result

    def _status(self, kind: QrKind, reference: str) -> dict[str, Any]:
        result = status_json(QR.get_qr_status(reference, kind=kind))
        track(kind.value, reference, status=result["status"], payer=result["payer"],
              payerBank=result["payerBank"])
        return result

    def _cancel(self, reference: str) -> dict[str, Any]:
        result = status_json(QR.cancel_qr(reference))
        track("simple", reference, status=result["status"])
        return result

    def _webhook(self, kind: QrKind, body: bytes) -> None:
        event: dict[str, Any] = {"receivedAt": datetime.now(timezone.utc).isoformat(), "kind": kind.value}
        status = 200
        try:
            n = parse_webhook(dict(self.headers.items()), body,
                              webhook=Webhook(url="", value=WEBHOOK_SECRET))
            event.update(reference=n.reference, status=n.status.value, amount=str(n.amount))
            event["confirmedStatus"] = QR.get_qr_status(n.reference, kind=kind).status.value
            track(kind.value, n.reference, status=event["confirmedStatus"], viaWebhook=True)
        except WebhookAuthError as exc:
            status, event["error"] = 401, f"rechazado: {exc}"
        except OpenHubError as exc:
            event["error"] = f"{type(exc).__name__}: {exc} · body={body[:300].decode(errors='replace')}"
        with LOCK:
            EVENTS.append(event)
        print(f"webhook {kind.value} -> {status} {event}", flush=True)
        self._json(status, {"ok": status == 200})

    # -- helpers -----------------------------------------------------------------------

    def _call(self, fn: Any) -> None:
        try:
            self._json(200, fn())
        except ValidationError as exc:
            self._json(400, {"error": str(exc), "field": exc.field})
        except ApiError as exc:
            self._json(422, {"error": str(exc), "code": exc.code, "retryable": exc.retryable})
        except (OpenHubError, ValueError) as exc:
            self._json(422, {"error": f"{type(exc).__name__}: {exc}"})

    def _public_url(self) -> str:
        return os.environ.get("PUBLIC_URL") or f"https://{self.headers.get('Host', 'localhost')}"

    def _authorized(self) -> bool:
        """Session cookie (browser) or HTTP Basic (scripts). Never triggers the
        browser's Basic-auth dialog: pages redirect to /login, APIs get 401."""
        expected = "Basic " + base64.b64encode(f"demo:{PASSWORD}".encode()).decode()
        if secrets.compare_digest(self.headers.get("Authorization", ""), expected):
            return True
        cookie = SimpleCookie(self.headers.get("Cookie", ""))
        if COOKIE in cookie and cookie[COOKIE].value in SESSIONS:
            return True
        if self.path.split("?", 1)[0] == "/":
            self._redirect("/login")
        else:
            self._json(401, {"error": "unauthorized"})
        return False

    def _login(self, body: bytes) -> None:
        password = parse_qs(body.decode()).get("password", [""])[0]
        if not secrets.compare_digest(password.encode(), PASSWORD.encode()):
            return self._redirect("/login?error=1")
        token = secrets.token_urlsafe(24)
        SESSIONS.add(token)
        secure = "; Secure" if self.headers.get("X-Forwarded-Proto") == "https" else ""
        self._redirect("/", f"{COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax{secure}")

    def _redirect(self, location: str, cookie: str | None = None) -> None:
        self.send_response(303)
        self.send_header("Location", location)
        if cookie:
            self.send_header("Set-Cookie", cookie)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def _json(self, status: int, payload: Any) -> None:
        self._send(status, json.dumps(payload).encode(), "application/json")

    def _send(self, status: int, data: bytes, content_type: str) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, format: str, *args: Any) -> None:
        pass


if __name__ == "__main__":
    port = int(os.environ.get("PORT", "8101"))
    print(f"{LANGUAGE} demo on :{port}", flush=True)
    ThreadingHTTPServer(("0.0.0.0", port), Handler).serve_forever()
