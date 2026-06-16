"""waeasi.wasi.http — wasi:http facade and dev polyfill.

Same shape as the TypeScript SDK's ``wasi/http.ts``.  In WAeasi
production mode, componentize-py rewrites this module's symbols to
call the host imports directly; in dev mode we expose an in-process
polyfill that lets handlers be unit-tested without a kernel.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Awaitable, Callable, Optional, Protocol

from .io import InputStream, OutputStream


class IncomingRequest(Protocol):
    def method(self) -> str: ...
    def scheme(self) -> str: ...
    def authority(self) -> str: ...
    def path_with_query(self) -> str: ...
    def headers(self) -> list[tuple[str, bytes]]: ...
    def consume_body(self) -> InputStream: ...


class ResponseOutparam(Protocol):
    def start_response(
        self, status: int, headers: list[tuple[str, bytes]],
    ) -> OutputStream: ...
    def finish(self) -> None: ...


@dataclass
class _PolyHost:
    next_request: Optional[Callable[[], Optional[IncomingRequest]]] = None
    capture: Optional[Callable[[int, list[tuple[str, bytes]], bytes], None]] = None
    fetch: Optional[Callable[..., Awaitable[dict]]] = None


_HOST = _PolyHost()


def install_polyfill(
    *,
    next_request: Optional[Callable[[], Optional[IncomingRequest]]] = None,
    capture: Optional[Callable[[int, list[tuple[str, bytes]], bytes], None]] = None,
    fetch: Optional[Callable[..., Awaitable[dict]]] = None,
) -> None:
    """Install dev-mode polyfill hooks for unit testing."""
    global _HOST
    _HOST = _PolyHost(next_request=next_request, capture=capture, fetch=fetch)


class _PolyOutputStream:
    __slots__ = ("_buf",)
    def __init__(self) -> None: self._buf: list[bytes] = []
    async def write_all(self, c: bytes) -> None: self._buf.append(c)
    def close(self) -> None: pass
    def drain(self) -> bytes: return b"".join(self._buf)


class _PolyOutparam:
    __slots__ = ("_status", "_headers", "_stream")

    def __init__(self) -> None:
        self._status = 0
        self._headers: list[tuple[str, bytes]] = []
        self._stream: Optional[_PolyOutputStream] = None

    def start_response(
        self, status: int, headers: list[tuple[str, bytes]],
    ) -> OutputStream:
        self._status = status
        self._headers = headers
        self._stream = _PolyOutputStream()
        return self._stream

    def finish(self) -> None:
        if _HOST.capture and self._stream is not None:
            _HOST.capture(self._status, self._headers, self._stream.drain())


def polyfill_outparam() -> ResponseOutparam:
    return _PolyOutparam()


async def fetch(
    url: str, *,
    method: str = "GET",
    headers: Optional[list[tuple[str, bytes]]] = None,
    body: Optional[bytes] = None,
    timeout_ms: int = 30_000,
) -> dict:
    if _HOST.fetch is None:
        raise RuntimeError("waeasi:net/outbound not bound (capability missing?)")
    return await _HOST.fetch(
        url, method=method, headers=headers or [],
        body=body, timeout_ms=timeout_ms,
    )
