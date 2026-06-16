"""waeasi.web.router — high-level Router with decorator API.

Mirrors the FastAPI / Hono / itty-router style of registration:

    router = Router()

    @router.get("/health")
    async def healthz(req): return Response("ok")

    @router.post("/v1/chat", body=ChatRequest, response=ChatResponse)
    async def chat(req, body: ChatRequest, ctx) -> ChatResponse:
        ...

Behaviours:
  * Method dispatch with HEAD → GET fallback and OPTIONS auto-reply.
  * 404 / 405 / 422 propagate as HTTPException.
  * Path params are passed through ``RouteCall.params``; advanced
    handlers can opt into kwargs via the ``inject_params=True`` decorator.
  * Validation: when ``body=`` is set the router parses + validates
    JSON before calling the handler, raising 422 on failure.
"""

from __future__ import annotations

import asyncio
import inspect
from typing import Any, Awaitable, Callable, Optional, Sequence

from ..runtime.context import ExecutionContext
from ..runtime.request import Request
from ..runtime.response import Response
from .errors import HTTPException, MethodNotAllowed, NotFound, UnprocessableEntity
from .match import RadixMatcher
from .route import Route, RouteCall, RouteHandler

Middleware = Callable[
    ["RouteCall", Callable[["RouteCall"], Awaitable[Response]]],
    Awaitable[Response],
]


class Router:
    def __init__(self, *, prefix: str = "") -> None:
        self.prefix = prefix.rstrip("/")
        self._matchers: dict[str, RadixMatcher] = {}
        self._allowed_per_path: dict[str, set[str]] = {}
        self._global_mw: list[Middleware] = []

    # ----- decorator helpers -------------------------------------------------

    def get   (self, path: str, **kw: Any): return self._deco("GET",    path, **kw)
    def post  (self, path: str, **kw: Any): return self._deco("POST",   path, **kw)
    def put   (self, path: str, **kw: Any): return self._deco("PUT",    path, **kw)
    def patch (self, path: str, **kw: Any): return self._deco("PATCH",  path, **kw)
    def delete(self, path: str, **kw: Any): return self._deco("DELETE", path, **kw)
    def head  (self, path: str, **kw: Any): return self._deco("HEAD",   path, **kw)

    def route(self, methods: Sequence[str], path: str, **kw: Any):
        def deco(fn: RouteHandler) -> RouteHandler:
            for m in methods:
                self.add_route(m, path, fn, **kw)
            return fn
        return deco

    # ----- programmatic registration -----------------------------------------

    def add_route(
        self, method: str, path: str, handler: RouteHandler,
        *,
        name: Optional[str] = None,
        body: Optional[type] = None,
        query: Optional[type] = None,
        response: Optional[type] = None,
        middleware: Sequence[Middleware] = (),
        summary: Optional[str] = None,
        tags: Sequence[str] = (),
    ) -> Route:
        full = self._full(path)
        m = method.upper()
        route = Route(
            method=m, pattern=full, handler=handler, name=name,
            summary=summary, tags=tuple(tags),
            middleware=tuple(middleware),
            body_model=body, query_model=query, response_model=response,
        )
        self._matchers.setdefault(m, RadixMatcher()).insert(full, route)
        self._allowed_per_path.setdefault(full, set()).add(m)
        return route

    def use(self, *mws: Middleware) -> None:
        """Register global middleware applied to every route."""
        self._global_mw.extend(mws)

    def include(self, other: "Router", prefix: str = "") -> None:
        """Merge `other` under `prefix`.  Path conflicts raise ValueError."""
        for method, mat in other._matchers.items():
            for pat, allowed in other._allowed_per_path.items():
                if method not in allowed: continue
                # We don't expose Route from the radix tree directly; replay
                # known patterns via add_route using the source matcher.
                v = mat.lookup(pat)
                if v is None: continue
                route, _ = v
                assert isinstance(route, Route)
                self.add_route(
                    method, prefix + pat[len(other.prefix):],
                    route.handler,
                    name=route.name,
                    body=route.body_model,
                    query=route.query_model,
                    response=route.response_model,
                    middleware=route.middleware,
                    summary=route.summary,
                    tags=route.tags,
                )

    # ----- dispatch ----------------------------------------------------------

    async def dispatch(self, req: Request, ctx: ExecutionContext) -> Response:
        path = req.path()
        method = req.method.upper()

        # OPTIONS pre-flight
        if method == "OPTIONS":
            allowed = self._all_methods_for(path)
            if allowed:
                return Response(
                    None, status=204,
                    headers={"allow": ", ".join(sorted(allowed | {"OPTIONS"}))},
                )

        mat = self._matchers.get(method) or (
            self._matchers.get("GET") if method == "HEAD" else None
        )
        if mat is None:
            allowed = self._all_methods_for(path)
            if allowed:
                raise MethodNotAllowed(sorted(allowed))
            raise NotFound()

        hit = mat.lookup(path)
        if hit is None:
            allowed = self._all_methods_for(path)
            if allowed:
                raise MethodNotAllowed(sorted(allowed))
            raise NotFound()

        route, params = hit
        assert isinstance(route, Route)
        call = RouteCall(route=route, request=req, ctx=ctx, params=params)

        if route.body_model is not None:
            call.body = await self._parse_body(req, route.body_model)

        return await self._run_chain(call)

    async def _run_chain(self, call: RouteCall) -> Response:
        async def terminal(c: RouteCall) -> Response:
            return await self._invoke(c)

        chain: Callable[[RouteCall], Awaitable[Response]] = terminal
        for mw in reversed(list(call.route.middleware) + self._global_mw):
            prev = chain
            async def wrapped(c: RouteCall, mw=mw, prev=prev) -> Response:
                return await mw(c, prev)
            chain = wrapped
        return await chain(call)

    async def _invoke(self, call: RouteCall) -> Response:
        sig = _signature_of(call.route.handler)
        kwargs: dict[str, Any] = {}
        for name in sig:
            if   name == "req"     or name == "request": kwargs[name] = call.request
            elif name == "ctx"     or name == "context": kwargs[name] = call.ctx
            elif name == "body":                          kwargs[name] = call.body
            elif name == "params":                        kwargs[name] = dict(call.params)
            elif name in call.params:                     kwargs[name] = call.params[name]
        result = call.route.handler(**kwargs) if kwargs \
            else call.route.handler(call.request)
        if asyncio.iscoroutine(result):
            result = await result
        if not isinstance(result, Response):
            raise TypeError(
                f"handler {call.route.handler!r} returned non-Response: {type(result).__name__}",
            )
        return result

    async def _parse_body(self, req: Request, model: type) -> Any:
        from .validate import parse_into
        try:
            text = await req.text()
        except Exception as e:  # noqa: BLE001
            raise UnprocessableEntity(f"could not read body: {e}") from e
        return parse_into(model, text)

    # ----- helpers -----------------------------------------------------------

    def _full(self, path: str) -> str:
        if not path.startswith("/"):
            path = "/" + path
        return (self.prefix + path) or "/"

    def _all_methods_for(self, path: str) -> set[str]:
        # Walk every matcher to see if any method matches.  Cheap because
        # `lookup` is O(L) and matchers are tiny.
        out: set[str] = set()
        for method, mat in self._matchers.items():
            if mat.lookup(path) is not None:
                out.add(method)
        return out


def _signature_of(fn: Callable[..., Any]) -> tuple[str, ...]:
    try:
        return tuple(inspect.signature(fn).parameters.keys())
    except (TypeError, ValueError):
        return ()


# Convenience: convert raised HTTPException into a Response.
def http_exc_to_response(e: HTTPException) -> Response:
    payload: dict[str, Any] = {"error": e.message, "status": e.status}
    if e.detail is not None:
        payload["detail"] = e.detail
    headers = dict(e.headers)
    headers.setdefault("content-type", "application/json; charset=utf-8")
    return Response.json_response(payload, status=e.status, headers=headers)
