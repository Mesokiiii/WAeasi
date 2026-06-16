"""waeasi.web.sse — Server-Sent Events response.

For LLM streaming UIs.  Build an ``SSEResponse`` from any async
generator of ``Event`` objects (or simple strings).  The wire format
is the standard text/event-stream (RFC 8895):

    event: token
    data: {"text":"hello"}
    id: 42

    data: ...

This Response is constructed with a streaming body that the wasi:io
output stream drains chunk by chunk; back-pressure is honoured.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import AsyncIterable, AsyncIterator, Optional, Union

from ..runtime.body import Body
from ..runtime.headers import Headers
from ..runtime.response import Response


@dataclass
class Event:
    data: str
    event: Optional[str] = None
    id: Optional[str] = None
    retry_ms: Optional[int] = None
    # extra fields (CR-separated comments) for OTel propagation, etc.
    comments: tuple[str, ...] = field(default_factory=tuple)


# An SSE producer either yields strings (data-only events) or Event objs.
SSEItem = Union[str, Event]
SSEStream = AsyncIterable[SSEItem]


def encode(ev: SSEItem) -> bytes:
    """Encode a single SSE record (terminated by blank line)."""
    if isinstance(ev, str):
        return _encode_data(ev)
    parts: list[str] = []
    for c in ev.comments:
        parts.append(f": {c}")
    if ev.event is not None:
        parts.append(f"event: {ev.event}")
    if ev.id is not None:
        parts.append(f"id: {ev.id}")
    if ev.retry_ms is not None:
        parts.append(f"retry: {int(ev.retry_ms)}")
    parts.extend(_data_lines(ev.data))
    parts.append("")  # blank line terminator
    return ("\n".join(parts) + "\n").encode("utf-8")


def _encode_data(text: str) -> bytes:
    return ("\n".join(_data_lines(text)) + "\n\n").encode("utf-8")


def _data_lines(text: str) -> list[str]:
    # SSE requires each \n in the payload to be sent as its own data: line.
    return [f"data: {line}" for line in text.split("\n")]


class SSEResponse(Response):
    """Streaming SSE response.  Pass an async iterable of ``Event`` or ``str``."""

    def __init__(
        self,
        stream: SSEStream,
        *,
        headers: Optional[Headers] = None,
        keep_alive_s: Optional[float] = 20.0,
    ) -> None:
        h = Headers(headers)
        h.set("content-type", "text/event-stream; charset=utf-8")
        h.set("cache-control", "no-cache, no-transform")
        h.set("x-accel-buffering", "no")
        super().__init__(None, status=200, headers=h)
        # Replace internal body with an async iterator producing wire bytes.
        self._body = Body(_sse_async_gen(stream, keep_alive_s))


async def _sse_async_gen(stream: SSEStream, keep_alive_s: Optional[float]) -> AsyncIterator[bytes]:
    import asyncio
    it = stream.__aiter__()
    sentinel = object()
    next_task: Optional[asyncio.Task] = None
    try:
        while True:
            if next_task is None:
                next_task = asyncio.ensure_future(_safe_next(it, sentinel))
            timeout = keep_alive_s if keep_alive_s and keep_alive_s > 0 else None
            try:
                done, _ = await asyncio.wait({next_task}, timeout=timeout)
            except asyncio.CancelledError:
                next_task.cancel()
                raise
            if not done:
                # Heartbeat — keeps proxies alive without polluting the stream.
                yield b": keep-alive\n\n"
                continue
            ev = next_task.result()
            next_task = None
            if ev is sentinel:
                return
            yield encode(ev)  # type: ignore[arg-type]
    finally:
        if next_task is not None:
            next_task.cancel()


async def _safe_next(it: AsyncIterator[SSEItem], sentinel: object) -> object:
    try:
        return await it.__anext__()
    except StopAsyncIteration:
        return sentinel


# Convenience: a "done" event that mirrors the OpenAI [DONE] sentinel.
def done_event() -> Event:
    return Event(data="[DONE]")
