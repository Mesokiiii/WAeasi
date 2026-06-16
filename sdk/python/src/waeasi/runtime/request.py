"""waeasi.runtime.request — Fetch-style Request.

User code receives an instance of this class.  All attributes are
immutable after construction.  Body access is lazy and one-shot
(matches WHATWG Fetch + aligned with HTTP semantics).
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Optional, Union
from urllib.parse import urlsplit, parse_qsl

from ..wasi.http import IncomingRequest
from .body import Body, BodyInit
from .headers import Headers, HeadersInit


@dataclass(frozen=True)
class _RequestInit:
    method: str = "GET"
    headers: Optional[HeadersInit] = None
    body: BodyInit = None
    traceparent: Optional[str] = None


class Request:
    __slots__ = ("method", "url", "headers", "traceparent", "_body")

    def __init__(
        self,
        url_or_request: Union[str, "Request"],
        method: str = "GET",
        headers: HeadersInit = None,
        body: BodyInit = None,
        traceparent: Optional[str] = None,
    ) -> None:
        if isinstance(url_or_request, Request):
            self.method = method.upper() if method != "GET" else url_or_request.method
            self.url = url_or_request.url
            self.headers = Headers(headers if headers is not None else url_or_request.headers)
            self.traceparent = traceparent or url_or_request.traceparent
            self._body = Body(body) if body is not None else url_or_request._body
            return
        self.method = method.upper()
        self.url = url_or_request
        self.headers = Headers(headers)
        self.traceparent = traceparent
        self._body = Body(body)

    @classmethod
    def from_wasi(cls, req: IncomingRequest) -> "Request":
        url = f"{req.scheme()}://{req.authority()}{req.path_with_query()}"
        headers = Headers.from_wasi(req.headers())
        body = Body.from_wasi(req.consume_body())
        r = cls.__new__(cls)
        r.method = req.method()
        r.url = url
        r.headers = headers
        r.traceparent = headers.get("traceparent")
        r._body = body
        return r

    @property
    def body_used(self) -> bool:
        return self._body.body_used

    async def bytes(self) -> bytes:
        return await self._body.bytes_()

    async def text(self) -> str:
        return await self._body.text()

    async def json(self) -> Any:
        return await self._body.json()

    def query(self) -> dict[str, str]:
        s = urlsplit(self.url)
        return dict(parse_qsl(s.query, keep_blank_values=True))

    def path(self) -> str:
        return urlsplit(self.url).path or "/"

    def host(self) -> str:
        return urlsplit(self.url).netloc

    def __repr__(self) -> str:
        return f"Request({self.method} {self.url})"
