"""waeasi — official Python SDK for WAeasi components.

Top-level package re-exporting the most-used names so user code can do::

    from waeasi import Request, Response, define_handler

    @define_handler()
    async def handle(req: Request, ctx) -> Response:
        return Response.json_response({"hello": req.path()})
"""

from .runtime import (
    Body, BodyInit,
    ExecutionContext, Logger, Counter, Gauge, Histogram,
    Headers, HeadersInit,
    Request, Response,
    RequestHandler, define_handler, register,
    handle, incoming_handler,
)

__version__ = "0.1.0"

__all__ = [
    "Body", "BodyInit",
    "ExecutionContext", "Logger", "Counter", "Gauge", "Histogram",
    "Headers", "HeadersInit",
    "Request", "Response",
    "RequestHandler", "define_handler", "register",
    "handle", "incoming_handler",
    "__version__",
]
