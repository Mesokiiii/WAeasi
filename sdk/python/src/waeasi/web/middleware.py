"""waeasi.web.middleware — base middleware utilities.

A middleware is an async callable ``(call, next) -> Response`` that
may inspect / mutate the call before delegating to ``next(call)`` and
post-process the response on the way back.

This module provides the type alias plus three simple combinators that
production handlers reach for first: ``compose``, ``logging``, and
``recover``.  Specialised middleware (cors, auth, ratelimit) live in
separate files for clarity.
"""

from __future__ import annotations

import asyncio
import time
from typing import Awaitable, Callable

from ..runtime.response import Response
from .errors import HTTPException
from .route import RouteCall
from .router import http_exc_to_response

Next = Callable[[RouteCall], Awaitable[Response]]
Middleware = Callable[[RouteCall, Next], Awaitable[Response]]


def compose(*mws: Middleware) -> Middleware:
    """Combine many middlewares into one (left-to-right execution)."""
    async def stacked(call: RouteCall, nxt: Next) -> Response:
        async def run(i: int, c: RouteCall) -> Response:
            if i >= len(mws):
                return await nxt(c)
            return await mws[i](c, lambda cc, _i=i: run(_i + 1, cc))
        return await run(0, call)
    return stacked


def logging() -> Middleware:
    """Emit a single structured log line per request with status + latency."""
    async def mw(call: RouteCall, nxt: Next) -> Response:
        t0 = time.monotonic()
        try:
            res = await nxt(call)
            ms = int((time.monotonic() - t0) * 1000)
            call.ctx.log.info(
                "request",
                {
                    "method": call.request.method,
                    "path":   call.request.path(),
                    "status": res.status,
                    "ms":     ms,
                },
            )
            return res
        except HTTPException as e:
            ms = int((time.monotonic() - t0) * 1000)
            call.ctx.log.warn("http_exc", {
                "status": e.status, "msg": e.message, "ms": ms,
            })
            raise
        except Exception as e:  # noqa: BLE001
            ms = int((time.monotonic() - t0) * 1000)
            call.ctx.log.error("unhandled", {
                "err": repr(e), "ms": ms,
                "method": call.request.method, "path": call.request.path(),
            })
            raise
    return mw


def recover() -> Middleware:
    """Convert HTTPException → JSON Response; convert other exceptions
    into 500 with a sanitized message.  Place outermost in the chain.
    """
    async def mw(call: RouteCall, nxt: Next) -> Response:
        try:
            return await nxt(call)
        except HTTPException as e:
            return http_exc_to_response(e)
        except asyncio.CancelledError:
            raise
        except Exception:  # noqa: BLE001
            call.ctx.log.error("recovered_panic", {
                "method": call.request.method,
                "path":   call.request.path(),
            })
            return Response.json_response(
                {"error": "internal error", "status": 500}, status=500,
            )
    return mw


def request_id(header: str = "x-request-id") -> Middleware:
    """Ensure a request-id header is propagated; generate one if absent."""
    async def mw(call: RouteCall, nxt: Next) -> Response:
        rid = call.request.headers.get(header)
        if not rid:
            import os
            rid = os.urandom(8).hex()
        res = await nxt(call)
        if not res.headers.has(header):
            res.headers.set(header, rid)
        return res
    return mw


def header(name: str, value: str) -> Middleware:
    """Attach a static response header to every response."""
    async def mw(call: RouteCall, nxt: Next) -> Response:
        res = await nxt(call)
        if not res.headers.has(name):
            res.headers.set(name, value)
        return res
    return mw
