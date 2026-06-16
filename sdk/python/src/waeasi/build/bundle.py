"""waeasi.build.bundle — final ``.waeasi-bundle`` assembler.

Layout identical to the TS SDK so the kernel loader has a single
parser.  See ``sdk/typescript/src/build/bundle.ts`` for the spec.
"""

from __future__ import annotations

import hashlib
import re
import struct
from dataclasses import dataclass
from pathlib import Path

from .errors import BundleError

_MAGIC = b"WAEASIBND"
_VERSION = 1
_ALIGN = 8
_NAME_RE = re.compile(r"^[A-Za-z0-9._-]+$")


@dataclass(frozen=True)
class BundleEntry:
    name: str
    path: Path


@dataclass(frozen=True)
class BundleArtifact:
    path: Path
    size_bytes: int
    digest: str
    entry_count: int


def assemble(entries: list[BundleEntry], out_path: Path) -> BundleArtifact:
    if not entries:
        raise BundleError("no entries to bundle")
    if len(entries) > 64:
        raise BundleError("too many entries (max 64)")

    payloads: list[tuple[BundleEntry, bytes]] = []
    for e in entries:
        if not (1 <= len(e.name) <= 64):
            raise BundleError(f"invalid entry name: {e.name!r}")
        if not _NAME_RE.match(e.name):
            raise BundleError(f"unsafe entry name: {e.name}")
        payloads.append((e, e.path.read_bytes()))

    # Header layout:
    header_size = len(_MAGIC) + 1 + 4
    for e, _ in payloads:
        header_size += 1 + len(e.name) + 8 + 8

    cursor = _pad(header_size, _ALIGN)
    placed: list[tuple[BundleEntry, bytes, int]] = []
    for e, data in payloads:
        cursor = _pad(cursor, _ALIGN)
        placed.append((e, data, cursor))
        cursor += len(data)
    total = cursor

    out = bytearray(total)
    h = 0
    out[h:h + len(_MAGIC)] = _MAGIC; h += len(_MAGIC)
    out[h] = _VERSION; h += 1
    struct.pack_into("<I", out, h, len(placed)); h += 4
    for e, data, off in placed:
        out[h] = len(e.name); h += 1
        nb = e.name.encode("ascii")
        out[h:h + len(nb)] = nb; h += len(nb)
        struct.pack_into("<Q", out, h, off); h += 8
        struct.pack_into("<Q", out, h, len(data)); h += 8

    for _, data, off in placed:
        out[off:off + len(data)] = data

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(bytes(out))
    return BundleArtifact(
        path=out_path,
        size_bytes=len(out),
        digest=hashlib.sha256(out).hexdigest(),
        entry_count=len(placed),
    )


def _pad(n: int, align: int) -> int:
    m = n % align
    return n if m == 0 else n + (align - m)


def inspect(path: Path) -> dict:
    """Decode header for tooling.  Returns name → (offset, length)."""
    buf = path.read_bytes()
    if len(buf) < len(_MAGIC) + 5 or buf[: len(_MAGIC)] != _MAGIC:
        raise BundleError("bad magic — not a WAeasi bundle")
    ver = buf[len(_MAGIC)]
    if ver != _VERSION:
        raise BundleError(f"unsupported bundle version {ver}")
    count = struct.unpack_from("<I", buf, len(_MAGIC) + 1)[0]
    off = len(_MAGIC) + 5
    entries: dict[str, tuple[int, int]] = {}
    for _ in range(count):
        nlen = buf[off]; off += 1
        name = buf[off:off + nlen].decode("ascii"); off += nlen
        e_off = struct.unpack_from("<Q", buf, off)[0]; off += 8
        e_len = struct.unpack_from("<Q", buf, off)[0]; off += 8
        entries[name] = (e_off, e_len)
    return {"entries": entries, "size_bytes": len(buf)}
