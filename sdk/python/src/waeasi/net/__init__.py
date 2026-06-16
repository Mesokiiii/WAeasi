"""waeasi.net — outbound networking primitives."""

from . import retry, sse_client, limit
from .fetch import Response, fetch, fetch_stream
from .limit import Limit, Timeout, gather_limited, limited
from .retry import GiveUp, Policy, retry_on_response, with_retry
from .sse_client import Event as SSEEvent, parse as parse_sse, parse_data_only

__all__ = [
    "fetch", "fetch_stream", "Response",
    "retry", "with_retry", "retry_on_response", "Policy", "GiveUp",
    "sse_client", "SSEEvent", "parse_sse", "parse_data_only",
    "limit", "Limit", "Timeout", "limited", "gather_limited",
]
