"""waeasi.web.errors — typed HTTP exceptions.

Handlers raise these to short-circuit out of nested middleware /
validation / business logic without manually building Response objects.
The router catches `HTTPException` and converts to a Response.
"""

from __future__ import annotations

from typing import Any, Optional


class HTTPException(Exception):
    """Base typed HTTP error.  Caught by the router and emitted as a
    JSON-formatted response."""

    __slots__ = ("status", "message", "detail", "headers")

    def __init__(
        self,
        status: int,
        message: str = "",
        *,
        detail: Optional[Any] = None,
        headers: Optional[dict[str, str]] = None,
    ) -> None:
        if not (100 <= status <= 599):
            raise ValueError(f"status out of range: {status}")
        super().__init__(message or _default_message(status))
        self.status = status
        self.message = message or _default_message(status)
        self.detail = detail
        self.headers = headers or {}


class BadRequest(HTTPException):
    def __init__(self, message: str = "bad request", **kw: Any) -> None:
        super().__init__(400, message, **kw)


class Unauthorized(HTTPException):
    def __init__(self, message: str = "unauthorized", **kw: Any) -> None:
        super().__init__(401, message, **kw)


class Forbidden(HTTPException):
    def __init__(self, message: str = "forbidden", **kw: Any) -> None:
        super().__init__(403, message, **kw)


class NotFound(HTTPException):
    def __init__(self, message: str = "not found", **kw: Any) -> None:
        super().__init__(404, message, **kw)


class MethodNotAllowed(HTTPException):
    def __init__(self, allowed: list[str], **kw: Any) -> None:
        h = {"allow": ", ".join(allowed)}
        super().__init__(405, "method not allowed", headers=h, **kw)


class Conflict(HTTPException):
    def __init__(self, message: str = "conflict", **kw: Any) -> None:
        super().__init__(409, message, **kw)


class UnprocessableEntity(HTTPException):
    """422 — used by the validation layer for malformed bodies."""
    def __init__(self, message: str = "unprocessable entity", **kw: Any) -> None:
        super().__init__(422, message, **kw)


class TooManyRequests(HTTPException):
    def __init__(self, retry_after_s: Optional[int] = None, **kw: Any) -> None:
        h: dict[str, str] = {}
        if retry_after_s is not None:
            h["retry-after"] = str(retry_after_s)
        super().__init__(429, "too many requests", headers=h, **kw)


class InternalError(HTTPException):
    def __init__(self, message: str = "internal error", **kw: Any) -> None:
        super().__init__(500, message, **kw)


class ServiceUnavailable(HTTPException):
    def __init__(self, retry_after_s: Optional[int] = None, **kw: Any) -> None:
        h: dict[str, str] = {}
        if retry_after_s is not None:
            h["retry-after"] = str(retry_after_s)
        super().__init__(503, "service unavailable", headers=h, **kw)


_DEFAULT = {
    400: "bad request", 401: "unauthorized", 402: "payment required",
    403: "forbidden", 404: "not found", 405: "method not allowed",
    406: "not acceptable", 408: "request timeout", 409: "conflict",
    410: "gone", 413: "payload too large", 415: "unsupported media type",
    422: "unprocessable entity", 429: "too many requests",
    500: "internal error", 501: "not implemented", 502: "bad gateway",
    503: "service unavailable", 504: "gateway timeout",
}


def _default_message(status: int) -> str:
    return _DEFAULT.get(status, "error")
