"""End-to-end check against the real OpenHub sandbox: generate, query, cancel.

Reads CLIENT_ID / CLIENT_SECRET from the environment (e.g. the repo's .env):

    set -a && . ../../.env && set +a
    uv run --no-sync python examples/sandbox_smoke.py   (from bindings/python)

Creates a Bs 1.00 QR in the sandbox (valid 2 minutes) and cancels it.
Never prints credentials or tokens.
"""

from __future__ import annotations

import os
import sys
import time
from decimal import Decimal

from openhub_bo.core import ApiError, Environment, PaymentStatus, Session, Webhook
from openhub_bo.qr import QrClient


def main() -> int:
    session = Session(
        os.environ["CLIENT_ID"],
        os.environ["CLIENT_SECRET"],
        environment=os.environ.get("OPENHUB_ENVIRONMENT", Environment.SANDBOX),
    )
    webhook = Webhook(
        url=os.environ.get("OPENHUB_WEBHOOK_URL", "https://example.com/openhub/webhook"),
        value=os.environ.get("OPENHUB_WEBHOOK_SECRET", "sandbox-smoke-test"),
    )
    with session:
        client = QrClient(session)
        qr = client.generate_qr(
            amount=Decimal("1.00"),
            description="Prueba openhub-bo",
            reference=str(int(time.time())),
            establishment_id=int(os.environ.get("OPENHUB_ESTABLISHMENT_ID", "1")),
            establishment_name=os.environ.get("OPENHUB_ESTABLISHMENT_NAME", "Prueba"),
            expires_in=120,
            webhook=webhook,
        )
        print(f"generated  ref={qr.reference} status={qr.status.value} expires_at={qr.expires_at}")
        print(f"           png={len(qr.qr_png)} bytes, amount={qr.amount} {qr.currency}")

        status = client.get_qr_status(qr.reference)
        print(f"status     {status.status.value} ({status.message})")
        assert status.status is PaymentStatus.PENDING

        cancelled = client.cancel_qr(qr.reference)
        print(f"cancelled  {cancelled.status.value} ({cancelled.message})")
        assert cancelled.status is PaymentStatus.CANCELLED

        try:
            client.cancel_qr(qr.reference)
        except ApiError as exc:
            print(f"re-cancel  rejected as expected: HTTP {exc.status} {exc.code}")
        else:
            print("re-cancel  unexpectedly succeeded")
            return 1
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
