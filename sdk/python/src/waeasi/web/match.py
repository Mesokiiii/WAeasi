"""waeasi.web.match — radix-tree path matcher.

Supports three pattern fragment kinds:
  * literal  — `/users`
  * param    — `/users/:id`            ⇒ ``params["id"]``
  * wildcard — `/files/*path`          ⇒ ``params["path"]`` (rest of URL)

Lookup is O(L) in the path length and stable across millions of routes
because the trie branches on byte values, not on regex backtracking.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class _Segment:
    kind: str           # "literal" | "param" | "wildcard"
    text: str           # for literal: full segment; param/wildcard: name


@dataclass
class _Node:
    children:    dict[str, "_Node"] = field(default_factory=dict)
    param_child: Optional["_Node"]  = None
    param_name:  Optional[str]      = None
    wildcard:    Optional["_Node"]  = None
    wildcard_name: Optional[str]    = None
    leaf:        Optional[object]   = None       # whatever the caller stores
    leaf_pattern: Optional[str]     = None


class RadixMatcher:
    """Insert/lookup tree.  Stored values are opaque to the matcher."""

    def __init__(self) -> None:
        self._root = _Node()

    def insert(self, pattern: str, value: object) -> None:
        segs = _parse_pattern(pattern)
        node = self._root
        for s in segs:
            if s.kind == "literal":
                node = node.children.setdefault(s.text, _Node())
            elif s.kind == "param":
                if node.param_child is None:
                    node.param_child = _Node()
                    node.param_name = s.text
                elif node.param_name != s.text:
                    raise ValueError(
                        f"conflicting param name on same node: "
                        f"{node.param_name!r} vs {s.text!r}",
                    )
                node = node.param_child
            else:  # wildcard
                if node.wildcard is None:
                    node.wildcard = _Node()
                    node.wildcard_name = s.text
                elif node.wildcard_name != s.text:
                    raise ValueError(
                        f"conflicting wildcard name: {node.wildcard_name} vs {s.text}",
                    )
                node = node.wildcard
        if node.leaf is not None and node.leaf_pattern != pattern:
            raise ValueError(f"pattern conflict: {node.leaf_pattern!r} vs {pattern!r}")
        node.leaf = value
        node.leaf_pattern = pattern

    def lookup(self, path: str) -> Optional[tuple[object, dict[str, str]]]:
        if not path.startswith("/"):
            return None
        # split once; cheaper than rebuilding the list for repeated lookups
        parts = [p for p in path.split("/") if p]
        params: dict[str, str] = {}
        return self._walk(self._root, parts, 0, params)

    def _walk(
        self,
        node: _Node,
        parts: list[str],
        i: int,
        params: dict[str, str],
    ) -> Optional[tuple[object, dict[str, str]]]:
        if i == len(parts):
            if node.leaf is not None:
                return node.leaf, params
            # also try a wildcard with empty rest
            if node.wildcard is not None and node.wildcard.leaf is not None:
                params[node.wildcard_name or "rest"] = ""
                return node.wildcard.leaf, params
            return None

        seg = parts[i]
        # 1. literal
        nxt = node.children.get(seg)
        if nxt is not None:
            r = self._walk(nxt, parts, i + 1, params)
            if r is not None: return r
        # 2. param
        if node.param_child is not None:
            saved = params.get(node.param_name or "")
            params[node.param_name or ""] = _decode(seg)
            r = self._walk(node.param_child, parts, i + 1, params)
            if r is not None: return r
            if saved is None:
                params.pop(node.param_name or "", None)
            else:
                params[node.param_name or ""] = saved
        # 3. wildcard greedy
        if node.wildcard is not None:
            rest = "/".join(parts[i:])
            params[node.wildcard_name or "rest"] = rest
            if node.wildcard.leaf is not None:
                return node.wildcard.leaf, params
            params.pop(node.wildcard_name or "rest", None)
        return None


def _parse_pattern(pattern: str) -> list[_Segment]:
    if not pattern.startswith("/"):
        raise ValueError(f"pattern must start with /: {pattern!r}")
    out: list[_Segment] = []
    for p in pattern.strip("/").split("/"):
        if not p:
            continue
        if p.startswith(":"):
            name = p[1:]
            if not name.isidentifier():
                raise ValueError(f"bad param name: {p!r}")
            out.append(_Segment("param", name))
        elif p.startswith("*"):
            name = p[1:] or "rest"
            if not name.isidentifier():
                raise ValueError(f"bad wildcard name: {p!r}")
            out.append(_Segment("wildcard", name))
        else:
            out.append(_Segment("literal", p))
    return out


def _decode(s: str) -> str:
    """Percent-decode a single path segment."""
    if "%" not in s:
        return s
    from urllib.parse import unquote
    return unquote(s)
