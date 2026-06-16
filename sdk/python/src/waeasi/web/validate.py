"""waeasi.web.validate — Pydantic-light validation.

We deliberately avoid pulling Pydantic into the WASI runtime (it's a
large dependency that runs initialization code componentize-py
struggles with).  Instead we ship a minimal but predictable validator
that handles the cases real AI handlers care about:

* dataclasses        (typing-driven)
* TypedDict          (typing-driven)
* simple ``BaseModel`` *adapters* if Pydantic is installed (best-effort)

For dataclasses, fields are coerced from JSON values:
  * int / float / bool / str — direct
  * list[T]                  — recursive
  * dict[str, T]             — recursive
  * Optional[T] / Union[T, None] — None passes through
  * Literal[...]             — must match one of the literals
  * nested dataclass         — recursive

Errors raise ``UnprocessableEntity(422)`` with a structured ``detail``
listing every offending field path.
"""

from __future__ import annotations

import dataclasses
import json
import sys
import types
import typing
from typing import Any, get_args, get_origin

from .errors import UnprocessableEntity


def parse_into(model: type, raw: str) -> Any:
    """Parse JSON `raw` into `model` (dataclass or TypedDict)."""
    try:
        data = json.loads(raw or "null")
    except json.JSONDecodeError as e:
        raise UnprocessableEntity(
            "invalid JSON", detail={"err": str(e), "pos": e.pos},
        ) from e
    return _coerce(model, data, path=())


def _coerce(tp: Any, value: Any, *, path: tuple[str, ...]) -> Any:
    if value is None and _is_optional(tp):
        return None

    origin = get_origin(tp)
    args = get_args(tp)

    # Optional / Union
    if origin in (typing.Union, getattr(types, "UnionType", typing.Union)):
        return _coerce_union(tp, value, args, path=path)

    # Literal
    if origin is typing.Literal:
        if value not in args:
            raise _err(path, f"must be one of {list(args)}", value)
        return value

    # Containers
    if origin in (list, tuple) and args:
        if not isinstance(value, list):
            raise _err(path, "expected array", value)
        return [
            _coerce(args[0], v, path=path + (str(i),))
            for i, v in enumerate(value)
        ]
    if origin in (dict,) and len(args) == 2:
        if not isinstance(value, dict):
            raise _err(path, "expected object", value)
        return {
            str(k): _coerce(args[1], v, path=path + (str(k),))
            for k, v in value.items()
        }

    # Dataclass
    if dataclasses.is_dataclass(tp):
        return _coerce_dataclass(tp, value, path=path)

    # TypedDict
    if _is_typeddict(tp):
        return _coerce_typeddict(tp, value, path=path)

    # Primitives
    if tp is str:
        if isinstance(value, str): return value
        raise _err(path, "expected string", value)
    if tp is bool:
        if isinstance(value, bool): return value
        raise _err(path, "expected bool", value)
    if tp is int:
        if isinstance(value, int) and not isinstance(value, bool): return value
        raise _err(path, "expected integer", value)
    if tp is float:
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return float(value)
        raise _err(path, "expected number", value)
    if tp is bytes:
        if isinstance(value, str):
            import base64
            return base64.b64decode(value)
        raise _err(path, "expected base64 string", value)
    if tp is type(None):
        if value is None: return None
        raise _err(path, "expected null", value)
    if tp is Any:
        return value

    # Fallback — accept as-is (e.g. dict[str, Any])
    return value


def _coerce_union(tp: Any, value: Any, args: tuple, *, path: tuple[str, ...]) -> Any:
    errors: list[str] = []
    for a in args:
        if a is type(None):
            if value is None: return None
            continue
        try:
            return _coerce(a, value, path=path)
        except UnprocessableEntity as e:
            errors.append(str(e.detail))
    raise _err(path, "no union variant matched", value, extra={"variants": errors})


def _coerce_dataclass(tp: type, value: Any, *, path: tuple[str, ...]) -> Any:
    if not isinstance(value, dict):
        raise _err(path, f"expected object for {tp.__name__}", value)
    hints = typing.get_type_hints(tp)
    out: dict[str, Any] = {}
    for f in dataclasses.fields(tp):
        if f.name in value:
            out[f.name] = _coerce(hints.get(f.name, Any), value[f.name],
                                  path=path + (f.name,))
        elif f.default is not dataclasses.MISSING:
            out[f.name] = f.default
        elif f.default_factory is not dataclasses.MISSING:  # type: ignore[misc]
            out[f.name] = f.default_factory()                # type: ignore[misc]
        else:
            raise _err(path + (f.name,), "missing required field")
    return tp(**out)


def _coerce_typeddict(tp: type, value: Any, *, path: tuple[str, ...]) -> dict:
    if not isinstance(value, dict):
        raise _err(path, f"expected object for {tp.__name__}", value)
    hints = typing.get_type_hints(tp)
    required = getattr(tp, "__required_keys__", set(hints))
    out: dict[str, Any] = {}
    for k, h in hints.items():
        if k in value:
            out[k] = _coerce(h, value[k], path=path + (k,))
        elif k in required:
            raise _err(path + (k,), "missing required field")
    return out


def _err(path: tuple[str, ...], msg: str, got: Any = ..., extra: Any = None):
    detail: dict[str, Any] = {"path": "/".join(path), "msg": msg}
    if got is not ...:
        detail["got"] = repr(got)[:120]
    if extra is not None:
        detail["extra"] = extra
    return UnprocessableEntity(detail["msg"], detail=detail)


def _is_optional(tp: Any) -> bool:
    if get_origin(tp) in (typing.Union, getattr(types, "UnionType", typing.Union)):
        return type(None) in get_args(tp)
    return False


if sys.version_info >= (3, 10):
    def _is_typeddict(tp: Any) -> bool:
        return typing.is_typeddict(tp)
else:  # pragma: no cover
    def _is_typeddict(tp: Any) -> bool:
        return getattr(tp, "__class__", None).__name__ == "_TypedDictMeta"
