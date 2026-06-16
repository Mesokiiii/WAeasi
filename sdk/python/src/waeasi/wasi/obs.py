"""waeasi.wasi.obs — log / metrics binding stubs.

The host overrides these on import; in dev mode we route logs to
stderr and accumulate metrics in-memory for inspection.
"""

from __future__ import annotations

import sys
from typing import Callable, Optional

LogLevel = str

_log_emit: Optional[Callable[[LogLevel, str, str], None]] = None
_log_enabled: Optional[Callable[[LogLevel, str], bool]] = None
_register_counter: Optional[Callable[[str], int]] = None
_register_gauge: Optional[Callable[[str], int]] = None
_register_histogram: Optional[Callable[[str, list[float]], int]] = None
_counter_add: Optional[Callable[[int, int], None]] = None
_gauge_set: Optional[Callable[[int, int], None]] = None
_histogram_observe: Optional[Callable[[int, float], None]] = None

_C_CACHE: dict[str, int] = {}
_G_CACHE: dict[str, int] = {}
_H_CACHE: dict[str, int] = {}

# Dev-mode in-memory snapshots, exposed for tests.
COUNTERS: dict[int, int] = {}
GAUGES: dict[int, int] = {}
HISTOGRAMS: dict[int, list[float]] = {}


def bind_host(**fns) -> None:  # type: ignore[no-untyped-def]
    """Override any of the host functions.  Intended for componentize-py."""
    g = globals()
    for k, v in fns.items():
        if k in g:
            g[f"_{k}"] = v


def log_emit(level: LogLevel, target: str, msg: str) -> None:
    if _log_emit is not None:
        _log_emit(level, target, msg)
    else:
        sys.stderr.write(f"[{level}] {target}: {msg}\n")


def log_enabled(level: LogLevel, target: str) -> bool:
    return _log_enabled(level, target) if _log_enabled else True


def register_counter(name: str) -> int:
    h = _C_CACHE.get(name)
    if h is None:
        h = _register_counter(name) if _register_counter else len(_C_CACHE) + 1
        _C_CACHE[name] = h
        COUNTERS.setdefault(h, 0)
    return h


def register_gauge(name: str) -> int:
    h = _G_CACHE.get(name)
    if h is None:
        h = _register_gauge(name) if _register_gauge else len(_G_CACHE) + 1
        _G_CACHE[name] = h
        GAUGES.setdefault(h, 0)
    return h


def register_histogram(name: str, buckets: list[float]) -> int:
    h = _H_CACHE.get(name)
    if h is None:
        h = (_register_histogram(name, buckets)
             if _register_histogram else len(_H_CACHE) + 1)
        _H_CACHE[name] = h
        HISTOGRAMS.setdefault(h, [])
    return h


def counter_add(h: int, v: int) -> None:
    if _counter_add is not None:
        _counter_add(h, v)
    else:
        COUNTERS[h] = COUNTERS.get(h, 0) + v


def gauge_set(h: int, v: int) -> None:
    if _gauge_set is not None:
        _gauge_set(h, v)
    else:
        GAUGES[h] = v


def histogram_observe(h: int, v: float) -> None:
    if _histogram_observe is not None:
        _histogram_observe(h, v)
    else:
        HISTOGRAMS.setdefault(h, []).append(v)
