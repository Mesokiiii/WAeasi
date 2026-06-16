"""waeasi.runtime.body — Bidirectional body adapter.

Accepts the canonical Python body forms (``str``, ``bytes``, ``bytearray``,
``memoryview``, sync iterators of bytes, async iterators of bytes) and
translates them to/from a wasi:io InputStream / OutputStream.

The body is consumed exactly once; downstream methods (``text``,
``json``, ``bytes_``) raise on second use, matching the WHATWG Fetch
``bodyUsed`` invariant.
"""

from __future__ import annotations

import json as _json
from typing import Any, AsyncIterable, Iterable, Optional, Union

from ..wasi.io import InputStream, OutputStream

BodyInit = Union[
    None, str, bytes, bytearray, memoryview,
    Iterable[bytes], AsyncIterable[bytes],
]

_READ_CHUNK = 8 * 1024


class Body:
    __slots__ = ("_kind", "_data", "_iter", "_aiter", "_rx", "_consumed")

    def __init__(self, init: BodyInit = None) -> None:
        self._consumed = False
        if init is None:
            self._kind = "empty"
        elif isinstance(init, str):
            self._kind = "bytes"
            self._data = init.encode("utf-8")
        elif isinstance(init, (bytes, bytearray, memoryview)):
            self._kind = "bytes"
            self._data = bytes(init)
        elif hasattr(init, "__aiter__"):
            self._kind = "aiter"
            self._aiter = init
        elif hasattr(init, "__iter__"):
            self._kind = "iter"
            self._iter = iter(init)
        else:
            raise TypeError(f"unsupported BodyInit: {type(init).__name__}")

    @classmethod
    def from_wasi(cls, rx: InputStream) -> "Body":
        b = cls.__new__(cls)
        b._consumed = False
        b._kind = "wasi"
        b._rx = rx
        return b

    def _guard(self) -> None:
        if self._consumed:
            raise RuntimeError("body already consumed")
        self._consumed = True

    @property
    def body_used(self) -> bool:
        return self._consumed

    async def bytes_(self) -> bytes:
        self._guard()
        return await self._drain()

    async def text(self) -> str:
        return (await self.bytes_()).decode("utf-8")

    async def json(self) -> Any:
        return _json.loads(await self.text())

    async def pipe_to_wasi(self, tx: OutputStream) -> None:
        self._guard()
        if self._kind == "empty":
            return
        if self._kind == "bytes":
            await tx.write_all(self._data)
            return
        if self._kind == "iter":
            for chunk in self._iter:
                if chunk:
                    await tx.write_all(bytes(chunk))
            return
        if self._kind == "aiter":
            async for chunk in self._aiter:
                if chunk:
                    await tx.write_all(bytes(chunk))
            return
        if self._kind == "wasi":
            while True:
                c = await self._rx.read(_READ_CHUNK)
                if c is None:
                    return
                if c:
                    await tx.write_all(c)

    async def _drain(self) -> bytes:
        if self._kind == "empty":
            return b""
        if self._kind == "bytes":
            return self._data
        chunks: list[bytes] = []
        if self._kind == "iter":
            for c in self._iter:
                if c:
                    chunks.append(bytes(c))
        elif self._kind == "aiter":
            async for c in self._aiter:
                if c:
                    chunks.append(bytes(c))
        elif self._kind == "wasi":
            while True:
                c = await self._rx.read(_READ_CHUNK)
                if c is None:
                    break
                if c:
                    chunks.append(c)
        return b"".join(chunks)
