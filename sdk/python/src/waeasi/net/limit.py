"""waeasi.net.limit — concurrency / connection limits.

In a hot LLM gateway you typically want to cap concurrent fan-out to
upstream providers (so a 200-RPS spike doesn't melt OpenAI's quota).
This module ships a small ``Semaphore``-like primitive plus a wrapper
that adapts any async coroutine into a limited variant.
"""

from __future__ import annotations

import asyncio
from typing import Awaitable, Callable, TypeVar

T = TypeVar("T")


class Limit:
    """Bounded concurrency limit.  Reentrant from the same task."""

    __slots__ = ("_sem", "_max")

    def __init__(self, max_concurrent: int) -> None:
        if max_concurrent < 1:
            raise ValueError("max_concurrent must be ≥ 1")
        self._max = max_concurrent
        self._sem = asyncio.Semaphore(max_concurrent)

    @property
    def max_concurrent(self) -> int: return self._max

    async def __aenter__(self) -> "Limit":
        await self._sem.acquire()
        return self

    async def __aexit__(self, *exc) -> None:  # type: ignore[no-untyped-def]
        self._sem.release()

    async def run(self, fn: Callable[[], Awaitable[T]]) -> T:
        async with self:
            return await fn()


def limited(max_concurrent: int) -> Callable[[Callable[..., Awaitable[T]]], Callable[..., Awaitable[T]]]:
    """Decorator: cap concurrent invocations of an async function."""
    sem = Limit(max_concurrent)

    def deco(fn: Callable[..., Awaitable[T]]) -> Callable[..., Awaitable[T]]:
        async def wrapped(*args, **kwargs):  # type: ignore[no-untyped-def]
            async with sem:
                return await fn(*args, **kwargs)
        wrapped.__name__ = getattr(fn, "__name__", "limited")
        return wrapped
    return deco


class Timeout:
    """Async context manager / wrapper applying an absolute deadline."""

    def __init__(self, seconds: float) -> None:
        self._sec = seconds

    async def run(self, awaitable: Awaitable[T]) -> T:
        return await asyncio.wait_for(awaitable, timeout=self._sec)


async def gather_limited(
    coros: list[Callable[[], Awaitable[T]]],
    max_concurrent: int,
) -> list[T]:
    """Run a batch of coroutines with bounded concurrency, preserving order."""
    sem = asyncio.Semaphore(max_concurrent)
    results: list[T] = [None] * len(coros)  # type: ignore[list-item]

    async def runner(i: int, fn: Callable[[], Awaitable[T]]) -> None:
        async with sem:
            results[i] = await fn()

    await asyncio.gather(*(runner(i, c) for i, c in enumerate(coros)))
    return results
