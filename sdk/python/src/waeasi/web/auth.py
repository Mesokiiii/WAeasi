"""waeasi.web.auth — bearer token / API key middleware.

Two ready-made authenticators:
  * ``bearer(verifier)``  — extracts ``Authorization: Bearer <token>``
  * ``api_key(verifier, header="x-api-key")`` — header- or query-based

Both accept an async ``verify`` function returning a principal dict (or
None to reject).  The principal is attached to ``call.ctx.principal``
for downstream handlers.

For AI gateways the typical pattern is:

    async def verify(token: str, ctx) -> dict | None:
        row = await kv.get(b"apikey:" + token.encode())
        return json.loads(row) if row else None

    router.use(auth.bearer(verify))
"""

from __future__ import annotations

from typing import Awaitable, Callable, Optional

from ..runtime.response import Response
from .errors import Unauthorized
from .middleware import Middleware, Next
from .route import RouteCall


Verifier = Callable[[str, "RouteCall"], Awaitable[Optional[dict]]]


def bearer(verify: Verifier, *, realm: str = "waeasi") -> Middleware:
    async def mw(call: RouteCall, nxt: Next) -> Response:
        h = call.request.headers.get("authorization") or ""
        if not h.lower().startswith("bearer "):
            raise Unauthorized(headers={
                "www-authenticate": f'Bearer realm="{realm}"',
            })
        token = h[7:].strip()
        if not token:
            raise Unauthorized()
        principal = await verify(token, call)
        if principal is None:
            raise Unauthorized()
        _attach(call, principal)
        return await nxt(call)
    return mw


def api_key(
    verify: Verifier,
    *,
    header: str = "x-api-key",
    query: Optional[str] = None,
    allow_query: bool = False,
) -> Middleware:
    async def mw(call: RouteCall, nxt: Next) -> Response:
        token = call.request.headers.get(header) or ""
        if not token and allow_query:
            qname = query or header
            token = call.request.query().get(qname, "")
        if not token:
            raise Unauthorized()
        principal = await verify(token, call)
        if principal is None:
            raise Unauthorized()
        _attach(call, principal)
        return await nxt(call)
    return mw


def require_scopes(*needed: str) -> Middleware:
    """Reject if any of `needed` scopes are missing on the principal."""
    needed_set = set(needed)
    async def mw(call: RouteCall, nxt: Next) -> Response:
        principal = getattr(call.ctx, "principal", None) or {}
        scopes = set(principal.get("scopes") or ())
        missing = needed_set - scopes
        if missing:
            from .errors import Forbidden
            raise Forbidden(detail={"missing_scopes": sorted(missing)})
        return await nxt(call)
    return mw


def _attach(call: RouteCall, principal: dict) -> None:
    # We add a dynamic attribute on ctx so handlers can do ctx.principal.
    # Mypy users can declare a Protocol; runtime is duck-typed.
    setattr(call.ctx, "principal", principal)
