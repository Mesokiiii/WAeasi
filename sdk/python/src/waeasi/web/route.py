"""waeasi.web.route — Route entity bound by Router.

A Route is an immutable record describing what a single registration
looks like.  Lookup happens through the radix matcher; once a Route is
chosen the dispatcher prepares the call (param coercion, body parsing,
middleware chain assembly) and invokes the handler.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Awaitable, Callable, Optional, Union

from ..runtime.context import ExecutionContext
from ..runtime.request import Request
from ..runtime.response import Response


# Handler may be sync or async; router awaits both transparently.
RouteResult = Union[Response, Awaitable[Response]]
RouteHandler = Callable[..., RouteResult]


@dataclass(frozen=True)
class Route:
    """Static descriptor for a single registered handler."""

    method:   str                    # already upper-cased
    pattern:  str
    handler:  RouteHandler
    name:     Optional[str] = None
    summary:  Optional[str] = None
    tags:     tuple[str, ...] = ()
    middleware: tuple[Callable[..., Awaitable[Response]], ...] = ()
    # Validation models (set by @router decorators or .validate(...)).
    body_model:  Optional[type] = None
    query_model: Optional[type] = None
    response_model: Optional[type] = None


@dataclass
class RouteCall:
    """Prepared invocation: route + path params + parsed body."""

    route:   Route
    request: Request
    ctx:     ExecutionContext
    params:  dict[str, str] = field(default_factory=dict)
    body:    object = None        # parsed (validated) body if body_model set

    def named(self, key: str) -> str:
        v = self.params.get(key)
        if v is None:
            raise KeyError(f"path param {key!r} not bound")
        return v

    def int_param(self, key: str) -> int:
        try:
            return int(self.named(key))
        except (KeyError, ValueError) as e:
            from .errors import BadRequest
            raise BadRequest(f"bad path param {key!r}: {e}") from e
