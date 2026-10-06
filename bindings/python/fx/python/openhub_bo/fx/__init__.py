"""Cross-currency QR collections for the Red Enlace (ATC) OpenHub APIs:
PIX (payers in Brazil), virtual assets via Koibanx (USDT/USDC) and Binance Pay."""

from ._ops import NATIVE
from .client import (
    AsyncBinanceClient,
    AsyncPixClient,
    AsyncVirtualAssetsClient,
    BinanceClient,
    PixClient,
    VirtualAssetsClient,
)
from .models import (
    BinanceNotification,
    Currency,
    FxNotification,
    FxQr,
    FxStatusInfo,
    Glosa,
    VirtualAsset,
)
from .webhook import parse_binance_webhook, parse_fx_webhook

__version__ = NATIVE.version

__all__ = [
    "AsyncBinanceClient",
    "AsyncPixClient",
    "AsyncVirtualAssetsClient",
    "BinanceClient",
    "BinanceNotification",
    "Currency",
    "FxNotification",
    "FxQr",
    "FxStatusInfo",
    "Glosa",
    "PixClient",
    "VirtualAsset",
    "VirtualAssetsClient",
    "__version__",
    "parse_binance_webhook",
    "parse_fx_webhook",
]
