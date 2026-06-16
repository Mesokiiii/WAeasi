"""waeasi.runtime.headers — RFC-compliant header container.

Insertion order is preserved (required for HTTP/2 pseudo-header
positioning); names are case-insensitive on lookup but stored
lower-cased for fast equality.

Multi-value semantics match WHATWG Fetch: ``append`` keeps duplicates
in order; ``get`` returns a comma-joined string; ``get_all`` exposes
the raw list.
"""

from __future__ import annotations

import re
from typing import Iterable, Iterator, Mapping, Optional, Union

_TOKEN = re.compile(r"^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$")
_BAD = re.compile(r"[\r\n\0]")

HeadersInit = Union[
    "Headers",
    Mapping[str, Union[str, Iterable[str]]],
    Iterable[tuple[str, str]],
    None,
]


class Headers:
    __slots__ = ("_map",)

    def __init__(self, init: HeadersInit = None) -> None:
        self._map: dict[str, list[str]] = {}
        if init is None:
            return
        if isinstance(init, Headers):
            for k, v in init.entries():
                self.append(k, v)
            return
        if isinstance(init, Mapping):
            for k, v in init.items():
                if isinstance(v, str):
                    self.append(k, v)
                else:
                    for vv in v:
                        self.append(k, vv)
            return
        for k, v in init:
            self.append(k, v)

    @staticmethod
    def _norm(name: str) -> str:
        if not _TOKEN.match(name):
            raise ValueError(f"invalid header name: {name!r}")
        return name.lower()

    @staticmethod
    def _check(value: str) -> str:
        if _BAD.search(value):
            raise ValueError("header value contains forbidden char")
        return value

    def append(self, name: str, value: str) -> None:
        k = self._norm(name)
        v = self._check(value)
        self._map.setdefault(k, []).append(v)

    def set(self, name: str, value: str) -> None:
        self._map[self._norm(name)] = [self._check(value)]

    def get(self, name: str) -> Optional[str]:
        v = self._map.get(self._norm(name))
        return ", ".join(v) if v else None

    def get_all(self, name: str) -> list[str]:
        return list(self._map.get(self._norm(name), ()))

    def has(self, name: str) -> bool:
        return self._norm(name) in self._map

    def delete(self, name: str) -> None:
        self._map.pop(self._norm(name), None)

    def entries(self) -> Iterator[tuple[str, str]]:
        for k, vs in self._map.items():
            for v in vs:
                yield k, v

    def keys(self) -> Iterator[str]:
        yield from self._map.keys()

    def values(self) -> Iterator[str]:
        for vs in self._map.values():
            yield from vs

    def __iter__(self) -> Iterator[tuple[str, str]]:
        return self.entries()

    def __len__(self) -> int:
        return sum(len(v) for v in self._map.values())

    def to_wasi(self) -> list[tuple[str, bytes]]:
        return [(k, v.encode("utf-8")) for k, v in self.entries()]

    @classmethod
    def from_wasi(cls, pairs: Iterable[tuple[str, bytes]]) -> "Headers":
        h = cls()
        for k, v in pairs:
            h.append(k, v.decode("utf-8", errors="replace"))
        return h

    def __repr__(self) -> str:
        return f"Headers({list(self.entries())!r})"
