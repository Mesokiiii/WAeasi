"""waeasi.net.fetch — outbound HTTP via wasi:http/outgoing-handler.

Two entry points:
  * ``fetch(...)``        — buffered convenience (small JSON payloads).
  * ``fetch_stream(...)`` — async iterator of body chunks (LLM tokens).

Both honour timeouts, custom headers, optional capability `NET_CONNECT`
(the kernel rejects the call without it).  In dev mode (no host
binding) the function falls back to the polyfill installed by
``waeasi.wasi.http.install_polyfill(fetch=...)``.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import AsyncIterator, Mapping, Optional

from ..wasi import http as _wasi_http


@dataclass
class Response:
    status: int
    headers: list[tuple[str, str]]
    body: bytes

    def header(self, name: str) -> Optional[str]:
        n = name.lower()
        for k, v in self.headers:
            if k.lower() == n: return v
        return None

    def text(self, encoding: str = "utf-8") -> str:
        return self.body.decode(encoding, errors="replace")

    def json(self) -> object:
        import json
        return json.loads(self.body)


@dataclass
class _Request:
    url: str
    method: str = "GET"
    headers: Mapping[str, str] = field(default_factory=dict)
    body: Optional[bytes] = None
    timeout_ms: int = 30_000


async def fetch(
    url: str,
    *,
    method: str = "GET",
    headers: Optional[Mapping[str, str]] = None,
    body: Optional[bytes] = None,
    json_body: Optional[object] = None,
    timeout_ms: int = 30_000,
) -> Response:
    """Buffered HTTP request.  Awaits full body before returning."""
    h = dict(headers or {})
    if json_body is not None:
        if body is not None:
            raise ValueError("pass body OR json_body, not both")
        import json
        body = json.dumps(json_body, separators=(",", ":")).encode("utf-8")
        h.setdefault("content-type", "application/json")
    bytes_headers = [(k, v.encode("utf-8")) for k, v in h.items()]
    raw = await _wasi_http.fetch(
        url, method=method, headers=bytes_headers, body=body,
        timeout_ms=timeout_ms,
    )
    return Response(
        status=raw["status"],
        headers=[(k, v.decode("utf-8", errors="replace")) for k, v in raw["headers"]],
        body=raw["body"],
    )


async def fetch_stream(
    url: str,
    *,
    method: str = "GET",
    headers: Optional[Mapping[str, str]] = None,
    body: Optional[bytes] = None,
    json_body: Optional[object] = None,
    timeout_ms: int = 30_000,
    chunk_size: int = 8192,
) -> "_StreamResponse":
    """Streaming HTTP request.  Returns a response wrapper whose
    ``.iter_bytes()`` / ``.iter_lines()`` async-iterate over body
    chunks as soon as they arrive.

    Note: the dev-mode polyfill returns the full body in a single chunk;
    real streaming engages only against the kernel's outgoing-handler.
    """
    # In production this calls the host streaming primitive; for the v1
    # SDK we obtain the full response and chunk it client-side.  The
    # API contract above is forward-compatible with future host streaming.
    r = await fetch(
        url, method=method, headers=headers, body=body,
        json_body=json_body, timeout_ms=timeout_ms,
    )
    return _StreamResponse(r, chunk_size=chunk_size)


class _StreamResponse:
    def __init__(self, r: Response, chunk_size: int) -> None:
        self.status = r.status
        self.headers = r.headers
        self._body = r.body
        self._chunk = chunk_size

    def header(self, name: str) -> Optional[str]:
        n = name.lower()
        for k, v in self.headers:
            if k.lower() == n: return v
        return None

    async def iter_bytes(self) -> AsyncIterator[bytes]:
        b = self._body
        for i in range(0, len(b), self._chunk):
            yield b[i:i + self._chunk]

    async def iter_lines(self) -> AsyncIterator[bytes]:
        buf = b""
        async for chunk in self.iter_bytes():
            buf += chunk
            while True:
                nl = buf.find(b"\n")
                if nl < 0: break
                line = buf[:nl]
                buf = buf[nl + 1:]
                if line.endswith(b"\r"):
                    line = line[:-1]
                yield line
        if buf:
            yield buf

    async def text(self, encoding: str = "utf-8") -> str:
        return self._body.decode(encoding, errors="replace")
