from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest

from openhub_bo.core import Webhook
from openhub_bo.qr.testing import MockOpenHub


@pytest.fixture(scope="session")
def fixtures_dir() -> Path:
    return Path(__file__).resolve().parents[4] / "fixtures" / "openhub"


@pytest.fixture
def webhook() -> Webhook:
    return Webhook(url="https://shop.example/qr/confirmed", value="46bc-b2a2-ea12258c99ab")


@pytest.fixture
def mock() -> MockOpenHub:
    return MockOpenHub()


@pytest.fixture
def qr_args(webhook: Webhook) -> dict[str, Any]:
    return {
        "amount": "10.50",
        "description": "Pedido 42",
        "reference": "4024",
        "establishment_id": 422717,
        "establishment_name": "Tienda Central",
        "expires_in": 900,
        "webhook": webhook,
    }
