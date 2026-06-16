"""waeasi.net.retry — exponential backoff helpers for outbound calls.

Designed for LLM API flakiness: 429 / 5xx / connection errors all
benefit from a few retries with jitter.  Other classes of failure
(4xx other than 429) are retried only when explicitly opted in.

```python
from waeasi.net import retry, fetch

resp = await retry.with_retry(
    lambda: fetch("https://api.openai.com/...", method="POST"),
    policy=retry.Policy(max_attempts=4, base_ms=200),
)
```
"""

from __future__ import annotations

import asyncio
import random
from dataclasses import dataclass, field
from typing import Awaitable, Callable, Iterable, Optional, TypeVar

T = TypeVar("T")


@dataclass(frozen=True)
class Policy:
    """Exponential backoff policy with jitter."""

    max_attempts: int = 4
    base_ms: int = 200
    max_ms: int = 30_000
    multiplier: float = 2.0
    jitter: float = 0.5            # +/- fraction of the wait
    retry_status: frozenset[int] = field(
        default_factory=lambda: frozenset({408, 425, 429, 500, 502, 503, 504})
    )

    def delay(self, attempt: int) -> float:
        # attempt is 0-indexed for the first failed call.
        d = min(self.max_ms, self.base_ms * (self.multiplier ** attempt))
        if self.jitter > 0:
            d *= 1 + random.uniform(-self.jitter, self.jitter)
        return max(0.0, d) / 1000.0


class GiveUp(Exception):
    """Raise inside the body to abort retries early."""


async def with_retry(
    fn: Callable[[], Awaitable[T]],
    *,
    policy: Optional[Policy] = None,
    on_retry: Optional[Callable[[int, BaseException, float], None]] = None,
    retry_exceptions: Iterable[type[BaseException]] = (),
) -> T:
    """Call ``fn()`` and retry on transient failures.

    Treats:
      * raised TimeoutError / ConnectionError / OSError as transient
      * any in ``retry_exceptions`` as transient
      * any of ``policy.retry_status`` HTTP statuses as transient
        (caller must adapt — see ``retry_on_response`` for sugar)
    """
    pol = policy or Policy()
    transient = tuple({TimeoutError, ConnectionError, OSError, *retry_exceptions})

    last_err: Optional[BaseException] = None
    for attempt in range(pol.max_attempts):
        try:
            return await fn()
        except GiveUp:
            raise
        except transient as e:  # type: ignore[misc]
            last_err = e
        except BaseException:  # pragma: no cover
            raise
        if attempt + 1 >= pol.max_attempts:
            break
        delay = pol.delay(attempt)
        if on_retry:
            try: on_retry(attempt + 1, last_err, delay)  # type: ignore[arg-type]
            except Exception: pass
        await asyncio.sleep(delay)

    assert last_err is not None
    raise last_err


async def retry_on_response(
    fn: Callable[[], Awaitable["fetch_mod.Response"]],  # type: ignore[name-defined]
    *,
    policy: Optional[Policy] = None,
    on_retry: Optional[Callable[[int, "fetch_mod.Response", float], None]] = None,
):
    """Sugar: re-issue the request when the response status is in
    `policy.retry_status`.  Honours `Retry-After` header when present.
    """
    pol = policy or Policy()
    last: Optional[object] = None
    for attempt in range(pol.max_attempts):
        last = await fn()
        if last.status not in pol.retry_status:  # type: ignore[attr-defined]
            return last
        if attempt + 1 >= pol.max_attempts:
            return last
        retry_after = _retry_after(last)  # type: ignore[arg-type]
        delay = retry_after if retry_after is not None else pol.delay(attempt)
        if on_retry:
            try: on_retry(attempt + 1, last, delay)  # type: ignore[arg-type]
            except Exception: pass
        await asyncio.sleep(delay)
    return last


def _retry_after(resp) -> Optional[float]:  # type: ignore[no-untyped-def]
    h = resp.header("retry-after")
    if not h: return None
    try:
        return float(h)
    except ValueError:
        return None


# Forward-reference shim for type hints above.
from . import fetch as fetch_mod  # noqa: E402
