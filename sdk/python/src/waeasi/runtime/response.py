"""waeasi.runtime.response — Fetch-style Response.

Mirrors :class:`Request` but on the egress side.  `Response.json(...)`
and `Response.error(...)` static helpers cover the common cases.
"""

from __future__ import annotations

import json as _json
from typing import Any, Optional

from ..wasi.http import ResponseOutparam
from .body import Body, BodyInit
from .headers import Headers, HeadersInit


class Response:
    __slots__ = ("status", "status_text", "headers", "_body")

    def __init__(
        self,
        body: BodyInit = None,
        status: int = 200,
        status_text: str = "",
        headers: HeadersInit = None,
    ) -> None:
        if not (100 <= status <= 599):
            raise ValueError(f"status out of range: {status}")
        self.status = status
        self.status_text = status_text
        self.headers = Headers(headers)
        self._body = Body(body)

    @property
    def ok(self) -> bool:
        return 200 <= self.status < 300

    @property
    def body_used(self) -> bool:
        return self._body.body_used

    async def bytes(self) -> bytes:
        return await self._body.bytes_()

    async def text(self) -> str:
        return await self._body.text()

    async def json(self) -> Any:
        return await self._body.json()

    @classmethod
    def json_response(
        cls,
        value: Any,
        status: int = 200,
        headers: HeadersInit = None,
    ) -> "Response":
        h = Headers(headers)
        if not h.has("content-type"):
            h.set("content-type", "application/json; charset=utf-8")
        return cls(_json.dumps(value, ensure_ascii=False), status=status, headers=h)

    @classmethod
    def redirect(cls, location: str, status: int = 302) -> "Response":
        if status not in (301, 302, 303, 307, 308):
            raise ValueError(f"redirect status not allowed: {status}")
        return cls(None, status=status, headers={"location": location})

    @classmethod
    def error(cls, status: int, message: Optional[str] = None) -> "Response":
        return cls(
            message or "",
            status=status,
            headers={"content-type": "text/plain; charset=utf-8"},
        )

    async def write_to_outparam(self, out: ResponseOutparam) -> None:
        if (not self.headers.has("content-length")
            and not self.headers.has("transfer-encoding")):
            cl = await self._try_content_length()
            if cl is not None:
                self.headers.set("content-length", str(cl))
        tx = out.start_response(self.status, self.headers.to_wasi())
        try:
            await self._body.pipe_to_wasi(tx)
        finally:
            tx.close()
            out.finish()

    async def _try_content_length(self) -> Optional[int]:
        # Only fast path for bodies we already have in memory.
        if self._body._kind == "empty":  # type: ignore[attr-defined]
            return 0
        if self._body._kind == "bytes":  # type: ignore[attr-defined]
            return len(self._body._data)  # type: ignore[attr-defined]
        return None
