"""waeasi.web — public surface.

```python
from waeasi.web import Router, cors, auth, ratelimit, middleware
from waeasi.web import HTTPException, NotFound, BadRequest
from waeasi.web import SSEResponse, Event, parse_multipart
```
"""

from . import auth, cors, middleware, ratelimit
from .errors import (
    BadRequest, Conflict, Forbidden, HTTPException, InternalError,
    MethodNotAllowed, NotFound, ServiceUnavailable, TooManyRequests,
    Unauthorized, UnprocessableEntity,
)
from .multipart import Part, field_dict, parse as parse_multipart
from .route import Route, RouteCall
from .router import Router, http_exc_to_response
from .sse import Event, SSEResponse, done_event, encode as encode_sse
from .validate import parse_into

__all__ = [
    # framework
    "Router", "Route", "RouteCall", "http_exc_to_response",
    # errors
    "HTTPException", "BadRequest", "Unauthorized", "Forbidden", "NotFound",
    "MethodNotAllowed", "Conflict", "UnprocessableEntity",
    "TooManyRequests", "InternalError", "ServiceUnavailable",
    # SSE / multipart / validation
    "SSEResponse", "Event", "done_event", "encode_sse",
    "parse_multipart", "Part", "field_dict",
    "parse_into",
    # middleware modules
    "auth", "cors", "middleware", "ratelimit",
]
