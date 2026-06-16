"""waeasi.wasi.clocks — monotonic + wall-clock + sleep."""

from __future__ import annotations

import asyncio
import time
from typing import Callable, Optional

_MONO_BASE_NS: Optional[int] = None
_MONO_OVERRIDE: Optional[Callable[[], int]] = None
_WALL_OVERRIDE: Optional[Callable[[], int]] = None


def bind_host(
    *,
    mono_now: Optional[Callable[[], int]] = None,
    wall_now: Optional[Callable[[], int]] = None,
) -> None:
    global _MONO_OVERRIDE, _WALL_OVERRIDE
    _MONO_OVERRIDE = mono_now
    _WALL_OVERRIDE = wall_now


def monotonic_now() -> int:
    """Nanoseconds since instance start.  Strictly monotonic."""
    if _MONO_OVERRIDE is not None:
        return _MONO_OVERRIDE()
    global _MONO_BASE_NS
    cur = time.monotonic_ns()
    if _MONO_BASE_NS is None:
        _MONO_BASE_NS = cur
    return cur - _MONO_BASE_NS


def wall_now() -> int:
    """Unix-epoch nanoseconds."""
    if _WALL_OVERRIDE is not None:
        return _WALL_OVERRIDE()
    return time.time_ns()


async def sleep(ms: float) -> None:
    """Cooperative sleep — yields to the asyncio loop."""
    if ms <= 0:
        await asyncio.sleep(0)
        return
    await asyncio.sleep(ms / 1000.0)
