"""Declarative operations: one :class:`Op` per endpoint of a native module."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Any, Generic, TypeVar

from ._bridge import Native
from .transport import Request, Response

T = TypeVar("T")


@dataclass(frozen=True, slots=True)
class Op(Generic[T]):
    """An endpoint implemented by ``native``.

    Declaring an Op fails at import time if the native module does not know
    ``name``, so packages cannot drift from their native code.
    """

    native: Native
    name: str
    decode: Callable[[Any], T]

    def __post_init__(self) -> None:
        info = self.native.operations.get(self.name)
        if info is None or info.get("kind") != "operation":
            raise ImportError(f"native module has no operation `{self.name}`")

    @property
    def idempotent(self) -> bool:
        return bool(self.native.operations[self.name]["idempotent"])

    @property
    def timeout(self) -> float | None:
        """Client timeout ATC recommends for this endpoint, if any."""
        secs = self.native.operations[self.name].get("timeout_secs")
        return None if secs is None else float(secs)

    def build(
        self, config: dict[str, Any], token: dict[str, Any] | None, input: dict[str, Any]
    ) -> Request:
        request: Request = self.native.invoke(
            f"{self.name}.build", {"config": config, "token": token, "input": input}
        )
        if self.timeout is not None:
            request["timeout"] = self.timeout
        return request

    def parse(self, input: dict[str, Any], response: Response) -> T:
        value = self.native.invoke(
            f"{self.name}.parse", {"input": input, "response": response.to_wire()}
        )
        return self.decode(value)


@dataclass(frozen=True, slots=True)
class Handler(Generic[T]):
    """A pure native function (no HTTP), e.g. webhook parsing."""

    native: Native
    name: str
    decode: Callable[[Any], T]

    def __post_init__(self) -> None:
        info = self.native.operations.get(self.name)
        if info is None or info.get("kind") != "handler":
            raise ImportError(f"native module has no handler `{self.name}`")

    def __call__(self, input: dict[str, Any]) -> T:
        return self.decode(self.native.invoke(self.name, input))
