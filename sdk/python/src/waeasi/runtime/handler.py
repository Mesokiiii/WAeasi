"""waeasi.runtime.handler — wasi:http/incoming-handler harness.

User code calls :func:`register` (or uses :func:`define_handler` as a
decorator) to wire a coroutine ``async def handle(req, ctx) -> Response``
into the wasi:http export expected by WAeasi.
"""

from __future__ import annotations

import asyncio
from typing import Awaitable, Callable, Optional, Union

from ..wasi.http import IncomingRequest, ResponseOutparam
from .context import ExecutionContext
from .request import Request
from .response import Response


SyncOrAsyncResponse = Union[Response, Awaitable[Response]]
RequestHandler = Callable[[Request, ExecutionContext], SyncOrAsyncResponse]


_USER: Optional[RequestHandler] = None
_TARGET = "handler"


def register(fn: RequestHandler, target: str = "handler") -> RequestHandler:
    """Register the user handler.  Idempotent re-registration is rejected."""
    global _USER, _TARGET
    if _USER is not None:
        raise RuntimeError("handler already registered")
    _USER = fn
    _TARGET = target
    return fn


def define_handler(target: str = "handler") -> Callable[[RequestHandler], RequestHandler]:
    """Decorator form: ``@define_handler() async def handle(req, ctx): ...``."""
    def deco(fn: RequestHandler) -> RequestHandler:
        register(fn, target)
        return fn
    return deco


async def _run_user(req: Request, ctx: ExecutionContext) -> Response:
    assert _USER is not None
    res = _USER(req, ctx)
    if asyncio.iscoroutine(res):
        return await res
    return res  # type: ignore[return-value]


async def _emit_500(out: ResponseOutparam, msg: str) -> None:
    r = Response(
        msg, status=500,
        headers={"content-type": "text/plain; charset=utf-8"},
    )
    await r.write_to_outparam(out)


async def handle(req: IncomingRequest, out: ResponseOutparam) -> None:
    """wasi:http/incoming-handler.handle export.

    Invoked by the componentize-py glue.  All user exceptions are
    caught and converted to a 500 response with the error message.
    """
    if _USER is None:
        await _emit_500(out, "no handler registered")
        return
    ctx = ExecutionContext(_TARGET)
    start_ns = ctx.now()
    try:
        request = Request.from_wasi(req)
        response = await _run_user(request, ctx)
        await response.write_to_outparam(out)
    except Exception as e:  # noqa: BLE001
        ctx.log.error("handler raised", {"err": repr(e)})
        try:
            await _emit_500(out, "internal error")
        except Exception:  # noqa: BLE001
            pass
    finally:
        ctx.abort()
        ctx.log.debug("request done", {"ns": str(ctx.now() - start_ns)})
        await ctx.drain()


# Public alias chosen by componentize-py's wit-bindgen output.
incoming_handler = type("incoming_handler", (), {"handle": staticmethod(handle)})
