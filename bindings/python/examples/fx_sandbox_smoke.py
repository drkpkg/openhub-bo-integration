"""Checks PIX, virtual assets (Koibanx) and Binance against the real sandbox.

Reads CLIENT_ID / CLIENT_SECRET from the environment (from bindings/python):

    set -a && . ../../.env && set +a
    uv run --no-sync python examples/fx_sandbox_smoke.py

For each product: generate a small QR and query it. If the merchant is not
enabled for a product, OpenHub answers a `GQ-` code; that is reported as
"not enabled" rather than a failure. Never prints credentials or tokens.
"""

from __future__ import annotations

import os
import sys
import time
from collections.abc import Callable

from openhub_bo.core import ApiError, Session, Webhook
from openhub_bo.fx import BinanceClient, FxQr, Glosa, PixClient, VirtualAsset, VirtualAssetsClient

NOT_ENABLED = {"GQ-00000", "GQ-00005", "GQ-00006"}


def check(name: str, generate: Callable[[], FxQr], verify: Callable[[str], object]) -> bool:
    try:
        qr = generate()
    except ApiError as exc:
        if exc.code in NOT_ENABLED:
            print(f"{name:8} not enabled for this merchant ({exc.code}): {exc}")
            return True
        print(f"{name:8} FAILED: {exc} code={exc.code}")
        return False
    print(
        f"{name:8} generated ref={qr.reference} {qr.amount} {qr.currency} -> "
        f"{qr.converted_amount} {qr.converted_currency} ({qr.image_mime})"
    )
    print(f"{name:8} status {verify(qr.reference)}")
    return True


def main() -> int:
    webhook = Webhook(url="https://example.com/openhub/fx", value="sandbox-smoke-test")
    glosa = Glosa("1", "Prueba", "7011", "Prueba openhub-bo")
    ref = str(int(time.time()))[-9:]
    with Session(os.environ["CLIENT_ID"], os.environ["CLIENT_SECRET"]) as session:
        pix, crypto, binance = (
            PixClient(session),
            VirtualAssetsClient(session),
            BinanceClient(session),
        )
        results = [
            check(
                "pix",
                lambda: pix.generate_qr(
                    amount="1.00",
                    glosa=glosa,
                    reference=ref,
                    payer_cpf="12345678901",
                    payer_phone="+5511999999999",
                    webhook=webhook,
                    expires_in=120,
                ),
                pix.get_status,
            ),
            check(
                "crypto",
                lambda: crypto.generate_qr(
                    amount="50",
                    glosa=glosa,
                    reference=ref,
                    asset=VirtualAsset.USDC,
                    webhook=webhook,
                    expires_in=180,
                ),
                crypto.get_status,
            ),
            check(
                "binance",
                lambda: binance.generate_qr(
                    amount="1.00",
                    glosa=glosa,
                    reference=ref,
                    webhook=webhook,
                    expires_in=120,
                ),
                binance.get_status,
            ),
        ]
        for name, client in (("pix", pix), ("crypto", crypto), ("binance", binance)):
            try:
                client.get_status("1")
            except ApiError as exc:
                detail = f"HTTP {exc.status} {exc.code} retryable={exc.retryable}"
                print(f"{name:8} unknown ref -> {detail}")
    print("OK" if all(results) else "FAILED")
    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(main())
