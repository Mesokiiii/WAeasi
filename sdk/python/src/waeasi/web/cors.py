"""waeasi.web.cors — CORS middleware.

Implementation matches WHATWG fetch CORS algorithm closely enough for
typical AI-app deployments (browser front-end + WAeasi back-end):

* Same-origin: pass-through, no headers added.
* Simple cross-origin GET/HEAD/POST: ``Access-Control-Allow-Origin``
  echoed (or ``*`` if `allow_credentials=False`).
* Pre-flight (OPTIONS + ``Access-Control-Request-Method``):
  short-circuit with 204 and the negotiated headers.
"""

from __future__ import annotations

from typing import Iterable, Optional, Sequence

from ..runtime.response import Response
from .middleware import Middleware, Next
from .route import RouteCall


_DEFAULT_HEADERS = (
    "accept", "accept-language", "content-language", "content-type",
    "authorization", "x-request-id", "x-api-key", "traceparent",
)


def cors(
    *,
    allow_origins: Sequence[str] = ("*",),
    allow_methods: Sequence[str] = ("GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"),
    allow_headers: Sequence[str] = _DEFAULT_HEADERS,
    expose_headers: Sequence[str] = (),
    allow_credentials: bool = False,
    max_age: int = 600,
) -> Middleware:
    origins = set(allow_origins)
    any_origin = "*" in origins
    if allow_credentials and any_origin:
        raise ValueError(
            "allow_credentials=True is incompatible with allow_origins=['*']",
        )
    methods_str  = ", ".join(sorted({m.upper() for m in allow_methods}))
    headers_str  = ", ".join(sorted({h.lower() for h in allow_headers}))
    expose_str   = ", ".join(sorted({h.lower() for h in expose_headers}))

    async def mw(call: RouteCall, nxt: Next) -> Response:
        origin = call.request.headers.get("origin")
        # Same-origin or non-CORS request
        if not origin:
            return await nxt(call)

        if not any_origin and origin not in origins:
            # Untrusted origin — let request proceed but don't add CORS headers
            return await nxt(call)

        # Pre-flight
        if (call.request.method == "OPTIONS"
            and call.request.headers.has("access-control-request-method")):
            r = Response(None, status=204)
            _common(r, origin, allow_credentials, any_origin)
            r.headers.set("access-control-allow-methods", methods_str)
            r.headers.set("access-control-allow-headers", headers_str)
            r.headers.set("access-control-max-age", str(max_age))
            return r

        # Actual request
        res = await nxt(call)
        _common(res, origin, allow_credentials, any_origin)
        if expose_str:
            res.headers.set("access-control-expose-headers", expose_str)
        return res

    return mw


def _common(
    res: Response, origin: str, allow_creds: bool, any_origin: bool,
) -> None:
    if any_origin and not allow_creds:
        res.headers.set("access-control-allow-origin", "*")
    else:
        res.headers.set("access-control-allow-origin", origin)
        res.headers.set("vary", "origin")
    if allow_creds:
        res.headers.set("access-control-allow-credentials", "true")
