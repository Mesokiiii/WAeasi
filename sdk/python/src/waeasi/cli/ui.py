"""waeasi.cli.ui — colour + step output (zero deps)."""

from __future__ import annotations

import os
import sys
from contextlib import contextmanager
from time import monotonic

_TTY = sys.stderr.isatty()
_NOCOLOR = os.environ.get("NO_COLOR") is not None or os.environ.get("FORCE_COLOR") == "0"
_COLOR = _TTY and not _NOCOLOR


def _wrap(code: str, s: str) -> str:
    return f"\x1b[{code}m{s}\x1b[0m" if _COLOR else s


def bold(s: str) -> str:    return _wrap("1", s)
def dim(s: str) -> str:     return _wrap("2", s)
def red(s: str) -> str:     return _wrap("31", s)
def green(s: str) -> str:   return _wrap("32", s)
def yellow(s: str) -> str:  return _wrap("33", s)
def cyan(s: str) -> str:    return _wrap("36", s)
def grey(s: str) -> str:    return _wrap("90", s)


def info(msg: str) -> None:  sys.stderr.write(f"{cyan('info')}  {msg}\n")
def ok(msg: str) -> None:    sys.stderr.write(f"{green('ok')}    {msg}\n")
def warn(msg: str) -> None:  sys.stderr.write(f"{yellow('warn')}  {msg}\n")


def fail(msg: str, code: int = 1) -> None:
    sys.stderr.write(f"{red('error')} {msg}\n")
    raise SystemExit(code)


@contextmanager
def step(label: str):  # type: ignore[no-untyped-def]
    start = monotonic()
    sys.stderr.write(f"{grey('→')} {label}…\n")
    try:
        yield
        elapsed = int((monotonic() - start) * 1000)
        sys.stderr.write(f"  {green('✓')} {label} {dim(f'({elapsed} ms)')}\n")
    except Exception:
        elapsed = int((monotonic() - start) * 1000)
        sys.stderr.write(f"  {red('✗')} {label} {dim(f'({elapsed} ms)')}\n")
        raise
