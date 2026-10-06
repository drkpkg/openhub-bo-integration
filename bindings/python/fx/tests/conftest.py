from __future__ import annotations

from pathlib import Path

import pytest

from openhub_bo.core import Webhook
from openhub_bo.fx import Glosa
from openhub_bo.fx.testing import MockFx


@pytest.fixture(scope="session")
def fixtures_dir() -> Path:
    return Path(__file__).resolve().parents[4] / "fixtures" / "openhub" / "fx"


@pytest.fixture
def webhook() -> Webhook:
    return Webhook(url="https://shop.example/fx/confirmed", value="f4a1b6d9c3e8f1a2b3c4")


@pytest.fixture
def glosa() -> Glosa:
    return Glosa("1", "Tienda Central", "7011", "Pedido 42")


@pytest.fixture
def mock() -> MockFx:
    return MockFx()
