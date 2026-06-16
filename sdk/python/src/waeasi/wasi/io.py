"""waeasi.wasi.io — wasi:io/streams facade.

Async stream interface mirroring the TS version.  At componentize-py
build time the host-import side is replaced with direct calls into
``wasi:io/streams@0.2.0``.  In dev mode (``pytest``) the helpers
:func:`buffer_stream` and :func:`capture_stream` provide synthetic
test doubles.
"""

from __future__ import annotations

import asyncio
from typing import Optional, Protocol


class InputStream(Protocol):
    async def read(self, n: int) -> Optional[bytes]: ...


class OutputStream(Protocol):
    async def write_all(self, chunk: bytes) -> None: ...
    def close(self) -> None: ...


class _BufferStream:
    __slots__ = ("_data", "_off")

    def __init__(self, data: bytes) -> None:
        self._data = data
        self._off = 0

    async def read(self, n: int) -> Optional[bytes]:
        if self._off >= len(self._data):
            return None
        end = min(self._off + n, len(self._data))
        chunk = self._data[self._off:end]
        self._off = end
        return chunk


class _CaptureStream:
    __slots__ = ("_chunks", "_closed")

    def __init__(self) -> None:
        self._chunks: list[bytes] = []
        self._closed = False

    async def write_all(self, chunk: bytes) -> None:
        if self._closed:
            raise RuntimeError("stream closed")
        if chunk:
            self._chunks.append(chunk)

    def close(self) -> None:
        self._closed = True

    def drain(self) -> bytes:
        return b"".join(self._chunks)


def buffer_stream(data: bytes) -> InputStream:
    return _BufferStream(data)


def capture_stream() -> "_CaptureStream":
    return _CaptureStream()


async def pipe(rx: InputStream, tx: OutputStream, chunk_size: int = 8192) -> int:
    total = 0
    while True:
        c = await rx.read(chunk_size)
        if c is None:
            return total
        if not c:
            await asyncio.sleep(0)
            continue
        await tx.write_all(c)
        total += len(c)
