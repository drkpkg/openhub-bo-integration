"""Read-only check of the payout APIs against the real sandbox. Never pays a QR
and never authorises a batch.

Generates a QR Simple of your own, decodes it, scans it through the payout API
(without paying), cancels it; lists banks; queries unknown payouts/batches.

    set -a && . ../../.env && set +a
    uv run --no-sync --with zxing-cpp --with pillow python examples/payouts_sandbox_smoke.py

OPENHUB_BRANCH_CODE (your `branchCode`) is optional for the bank list.
"""

from __future__ import annotations

import io
import os
import sys
import time
import uuid

import zxingcpp  # type: ignore[import-not-found]
from PIL import Image  # type: ignore[import-not-found]

from openhub_bo.core import ApiError, Session, Webhook
from openhub_bo.payouts import PayoutsClient
from openhub_bo.qr import QrClient


def main() -> int:
    branch = os.environ.get("OPENHUB_BRANCH_CODE", "1")
    with Session(os.environ["CLIENT_ID"], os.environ["CLIENT_SECRET"]) as session:
        qr_client, payouts = QrClient(session), PayoutsClient(session)
        qr = qr_client.generate_qr(
            amount="1.00",
            description="Prueba scan openhub-bo",
            reference=str(int(time.time())),
            establishment_id=1,
            establishment_name="Prueba",
            expires_in=120,
            webhook=Webhook(url="https://example.com/hook", value="sandbox-smoke-test"),
        )
        text = zxingcpp.read_barcodes(Image.open(io.BytesIO(qr.qr_png)))[0].text
        scanned = payouts.scan_qr(text)
        print(
            f"scan       ref={scanned.reference} amount={scanned.amount} "
            f"holder={scanned.recipient.holder} bank={scanned.recipient.bank_name}"
        )
        print(f"cancel own QR -> {qr_client.cancel_qr(qr.reference).status.value}")

        banks = payouts.list_banks(branch)
        print(f"banks      {len(banks)} (first: {[b.code for b in banks[:5]]})")

        for label, call in (
            ("payout", lambda: payouts.get_payout("12345678901234567890")),
            (
                "batch",
                lambda: payouts.get_batch_status(branch, str(uuid.uuid4()), batch_number="1"),
            ),
        ):
            try:
                call()
            except ApiError as exc:
                print(f"{label:10} unknown -> HTTP {exc.status} code={exc.code}")
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
