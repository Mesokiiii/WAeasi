"""waeasi.runtime.context — per-invocation execution context.

Exposes a structured logger, monotonic / wall clocks, metric handles
and a ``wait_until`` lifetime extension primitive equivalent to
Cloudflare Workers' ``ExecutionContext.waitUntil``.
"""

from __future__ import annotations

import asyncio
from typing import Any, Awaitable, Callable, Mapping, Optional

from ..wasi import clocks as _clk
from ..wasi import obs as _obs


LogLevel = str  # one of "trace" | "debug" | "info" | "warn" | "error"


class Logger:
    __slots__ = ("_target",)

    def __init__(self, target: str) -> None:
        self._target = target

    def _emit(self, level: LogLevel, msg: str, fields: Optional[Mapping[str, Any]]) -> None:
        if not _obs.log_enabled(level, self._target):
            return
        if fields:
            import json
            msg = msg + " " + json.dumps(fields, default=str, ensure_ascii=False)
        _obs.log_emit(level, self._target, msg)

    def trace(self, msg: str, fields: Optional[Mapping[str, Any]] = None) -> None:
        self._emit("trace", msg, fields)

    def debug(self, msg: str, fields: Optional[Mapping[str, Any]] = None) -> None:
        self._emit("debug", msg, fields)

    def info(self, msg: str, fields: Optional[Mapping[str, Any]] = None) -> None:
        self._emit("info", msg, fields)

    def warn(self, msg: str, fields: Optional[Mapping[str, Any]] = None) -> None:
        self._emit("warn", msg, fields)

    def error(self, msg: str, fields: Optional[Mapping[str, Any]] = None) -> None:
        self._emit("error", msg, fields)


class Counter:
    __slots__ = ("_h",)
    def __init__(self, h: int) -> None: self._h = h
    def inc(self, by: int = 1) -> None: _obs.counter_add(self._h, by)


class Gauge:
    __slots__ = ("_h",)
    def __init__(self, h: int) -> None: self._h = h
    def set(self, v: float) -> None: _obs.gauge_set(self._h, int(v))


class Histogram:
    __slots__ = ("_h",)
    def __init__(self, h: int) -> None: self._h = h
    def observe(self, v: float) -> None: _obs.histogram_observe(self._h, float(v))


class ExecutionContext:
    __slots__ = ("target", "log", "_pending", "_aborted", "_cb")

    def __init__(self, target: str = "handler") -> None:
        self.target = target
        self.log = Logger(target)
        self._pending: list[Awaitable[Any]] = []
        self._aborted = False
        self._cb: Optional[Callable[[], None]] = None

    def wait_until(self, awaitable: Awaitable[Any]) -> None:
        async def shielded() -> None:
            try:
                await awaitable
            except Exception as e:  # noqa: BLE001
                self.log.error("wait_until rejected", {"err": str(e)})
        self._pending.append(asyncio.ensure_future(shielded()))

    async def drain(self) -> None:
        if not self._pending:
            return
        await asyncio.gather(*self._pending, return_exceptions=True)

    def abort(self) -> None:
        self._aborted = True
        if self._cb is not None:
            try: self._cb()
            except Exception:  # noqa: BLE001
                pass

    def on_abort(self, cb: Callable[[], None]) -> None:
        self._cb = cb

    @property
    def aborted(self) -> bool:
        return self._aborted

    def now(self) -> int:
        return _clk.monotonic_now()

    def wall_ns(self) -> int:
        return _clk.wall_now()

    def counter(self, name: str) -> Counter:
        return Counter(_obs.register_counter(name))

    def gauge(self, name: str) -> Gauge:
        return Gauge(_obs.register_gauge(name))

    def histogram(self, name: str, buckets: list[float]) -> Histogram:
        return Histogram(_obs.register_histogram(name, buckets))
