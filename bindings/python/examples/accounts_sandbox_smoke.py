"""Read-only check of the merchant-accounts API against the real sandbox.

Reads CLIENT_ID / CLIENT_SECRET and, optionally, OPENHUB_NIT (your merchant's NIT)
from the environment (from bindings/python):

    set -a && . ../../.env && set +a
    uv run --no-sync python examples/accounts_sandbox_smoke.py

Without OPENHUB_NIT it uses a made-up NIT and expects "merchant not found".
Never creates accounts or changes their status.
"""

from __future__ import annotations

import os
import sys
from datetime import date, timedelta

from openhub_bo.accounts import AccountsClient
from openhub_bo.core import ApiError, Session

FAKE_NIT = "1234567"


def main() -> int:
    nit = os.environ.get("OPENHUB_NIT", FAKE_NIT)
    today = date.today()
    with Session(os.environ["CLIENT_ID"], os.environ["CLIENT_SECRET"]) as session:
        client = AccountsClient(session)
        try:
            listing = client.list_accounts(nit)
        except ApiError as exc:
            print(f"list_accounts({nit}) -> {exc.code}: {exc}")
            return 0 if nit == FAKE_NIT and exc.code == "17" else 1
        numbers = [a.number for a in listing.accounts]
        print(f"{listing.merchant_name}: {len(numbers)} accounts")
        for balance in client.balances(nit, numbers[:10]) if numbers else []:
            print(
                f"  {balance.account_number} {balance.status.value} available={balance.available}"
            )
        rec = client.reconcile(nit, date_from=today - timedelta(days=7), date_to=today)
        print(f"reconcile 7d: {len(rec.movements)} movements")
        if numbers:
            credits = client.credits(
                nit, numbers[0], date_from=today - timedelta(days=30), date_to=today
            )
            print(f"credits 30d on {numbers[0]}: {len(credits)}")
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
