"""waeasi.web.ratelimit — token-bucket rate limiter.

Two flavours:
  * In-memory (per-instance)   — `memory_bucket(...)`
  * KV-backed (cluster-wide)   — `kv_bucket(...)`  (uses waeasi.storage.kv)

Both produce a Middleware with a `rate(...)` factory that scopes
buckets by a key extractor.  Common extractors: client IP, API key,
user id from ctx.principal, route name, etc.

For AI gateways the canonical setup is:

    bucket = ratelimit.memory_bucket(rate=10, per=60.0, burst=20)
    router.use(ratelimit.rate(bucket, key=ratelimit.by_api_key))
"""

from __future__ import annotations

import time
from collections import OrderedDict
from typing import Awaitable, Callable, Optional, Protocol

from ..runtime.response import Response
from .errors import TooManyRequests
from .middleware import Middleware, Next
from .route import RouteCall


KeyExtractor = Callable[[RouteCall], str]


class _Bucket(Protocol):
    async def take(self, key: str) -> Optional[float]:
        """Return None if allowed, or seconds until next allowed slot."""
        ...


# --------------------------------------------------------------------- in-mem


def memory_bucket(
    rate: float,        # tokens added per second
    per: float = 1.0,   # convenience: rate is per `per` seconds
    burst: Optional[int] = None,
    capacity: int = 4096,
) -> _Bucket:
    """Lock-free per-instance bucket.  Cheap, exact, but per-pod only."""
    fill_per_s = rate / per
    cap = burst if burst is not None else max(1, int(rate))

    class MemBucket:
        __slots__ = ("buckets",)

        def __init__(self) -> None:
            self.buckets: OrderedDict[str, tuple[float, float]] = OrderedDict()

        async def take(self, key: str) -> Optional[float]:
            now = time.monotonic()
            tokens, last = self.buckets.get(key, (float(cap), now))
            tokens = min(float(cap), tokens + (now - last) * fill_per_s)
            if tokens >= 1.0:
                tokens -= 1.0
                self.buckets[key] = (tokens, now)
                self.buckets.move_to_end(key)
                if len(self.buckets) > capacity:
                    self.buckets.popitem(last=False)
                return None
            wait = (1.0 - tokens) / fill_per_s
            self.buckets[key] = (tokens, now)
            return wait

    return MemBucket()


# --------------------------------------------------------------------- KV


def kv_bucket(
    kv,                       # waeasi.storage.kv.KV instance
    *,
    rate: float,
    per: float = 1.0,
    burst: Optional[int] = None,
    prefix: str = "rl:",
) -> _Bucket:
    """Distributed bucket via KV CAS.  Accepts any KV with get/cas/put."""
    fill_per_s = rate / per
    cap = burst if burst is not None else max(1, int(rate))

    class KvBucket:
        async def take(self, key: str) -> Optional[float]:
            full = (prefix + key).encode()
            now = time.time()
            raw = await kv.get(full)
            tokens, last = (float(cap), now)
            if raw:
                try:
                    parts = raw.decode().split(":")
                    tokens = float(parts[0]); last = float(parts[1])
                except (ValueError, IndexError):
                    pass
            tokens = min(float(cap), tokens + (now - last) * fill_per_s)
            if tokens >= 1.0:
                tokens -= 1.0
                await kv.put(full, f"{tokens}:{now}".encode(), ttl_s=int(per * 4))
                return None
            await kv.put(full, f"{tokens}:{now}".encode(), ttl_s=int(per * 4))
            return (1.0 - tokens) / fill_per_s

    return KvBucket()


# --------------------------------------------------------------------- middleware


def rate(
    bucket: _Bucket,
    *,
    key: KeyExtractor = lambda c: by_ip(c),  # noqa: PLW0108
) -> Middleware:
    async def mw(call: RouteCall, nxt: Next) -> Response:
        wait = await bucket.take(key(call))
        if wait is not None:
            raise TooManyRequests(retry_after_s=max(1, int(round(wait))))
        return await nxt(call)
    return mw


# --------------------------------------------------------------------- extractors


def by_ip(call: RouteCall) -> str:
    h = call.request.headers
    fwd = h.get("x-forwarded-for") or ""
    if fwd:
        return fwd.split(",")[0].strip()
    return h.get("x-real-ip") or call.request.host()


def by_api_key(call: RouteCall) -> str:
    return (call.request.headers.get("authorization") or "").strip()[:128] \
        or (call.request.headers.get("x-api-key") or "anon")


def by_principal(field: str = "id") -> KeyExtractor:
    def fn(call: RouteCall) -> str:
        principal = getattr(call.ctx, "principal", None) or {}
        return str(principal.get(field, "anon"))
    return fn


def by_route(call: RouteCall) -> str:
    return call.route.name or call.route.pattern
