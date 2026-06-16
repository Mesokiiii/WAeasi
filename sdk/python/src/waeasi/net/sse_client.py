"""waeasi.net.sse_client — consume Server-Sent Events from upstream APIs.

OpenAI ``/chat/completions?stream=true``, Anthropic ``/messages``
streaming, and most LLM providers emit text/event-stream.  This module
parses the wire format into ``Event`` objects from any iterator of
bytes (e.g. ``fetch_stream(...).iter_bytes()``).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import AsyncIterable, AsyncIterator, Optional


@dataclass
class Event:
    data: str
    event: Optional[str] = None
    id: Optional[str] = None
    retry_ms: Optional[int] = None


async def parse(byte_stream: AsyncIterable[bytes]) -> AsyncIterator[Event]:
    """Async-iterate complete SSE Events from a byte stream."""
    buf = b""
    fields: dict[str, list[str]] = {}
    last_id: Optional[str] = None

    async for chunk in byte_stream:
        buf += chunk
        while True:
            nl = buf.find(b"\n")
            if nl < 0:
                break
            raw = buf[:nl]
            buf = buf[nl + 1:]
            if raw.endswith(b"\r"):
                raw = raw[:-1]

            if not raw:
                ev = _flush(fields, last_id)
                if ev is not None:
                    if ev.id is not None:
                        last_id = ev.id
                    yield ev
                fields = {}
                continue

            if raw.startswith(b":"):
                continue  # comment

            sep = raw.find(b":")
            if sep == -1:
                # Field with empty value — RFC permits it.
                fields.setdefault(raw.decode("utf-8", errors="replace"), []).append("")
                continue
            name = raw[:sep].decode("utf-8", errors="replace")
            value = raw[sep + 1:]
            if value.startswith(b" "):
                value = value[1:]
            fields.setdefault(name, []).append(value.decode("utf-8", errors="replace"))

    # Stream ended without trailing blank line — flush whatever's pending.
    ev = _flush(fields, last_id)
    if ev is not None:
        yield ev


def _flush(fields: dict[str, list[str]], last_id: Optional[str]) -> Optional[Event]:
    if "data" not in fields and "event" not in fields:
        return None
    data = "\n".join(fields.get("data", []))
    name = fields.get("event", [None])[-1]
    new_id = fields.get("id", [None])[-1]
    retry_raw = fields.get("retry", [None])[-1]
    retry_ms: Optional[int] = None
    if retry_raw is not None:
        try:
            retry_ms = int(retry_raw)
        except ValueError:
            retry_ms = None
    return Event(
        data=data,
        event=name,
        id=new_id if new_id is not None else last_id,
        retry_ms=retry_ms,
    )


async def parse_data_only(byte_stream: AsyncIterable[bytes]) -> AsyncIterator[str]:
    """Yield only the ``data`` payloads (skipping [DONE] sentinels)."""
    async for ev in parse(byte_stream):
        if ev.data == "[DONE]":
            return
        if ev.data:
            yield ev.data
