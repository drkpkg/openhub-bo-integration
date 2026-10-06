"""Crossing point into a package's native module."""

from __future__ import annotations

import json
from types import ModuleType
from typing import Any

from .errors import error_from_core

SUPPORTED_PROTOCOL = 2
"""Core FFI protocol this Python code speaks (see ``openhub_core::ffi``)."""


class Native:
    """A native module exposing ``call(op, payload) -> str``.

    Checks the protocol version on load and caches the operation catalog from
    ``describe``, so declared operations can be validated at import time.
    """

    def __init__(self, module: ModuleType) -> None:
        protocol = getattr(module, "PROTOCOL_VERSION", None)
        if protocol != SUPPORTED_PROTOCOL:
            raise ImportError(
                f"{module.__name__} speaks core protocol {protocol}, "
                f"this package expects {SUPPORTED_PROTOCOL}; reinstall matching versions"
            )
        self._module = module
        self.version: str = module.__version__
        self.operations: dict[str, dict[str, Any]] = {
            entry["name"]: entry for entry in self.invoke("describe", {})
        }

    def invoke(self, op: str, payload: dict[str, Any]) -> Any:
        raw = self._module.call(op, json.dumps(payload, separators=(",", ":")))
        result = json.loads(raw)
        if result["ok"]:
            return result["value"]
        raise error_from_core(result["error"], op)
