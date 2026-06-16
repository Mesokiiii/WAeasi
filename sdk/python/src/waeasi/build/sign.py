"""waeasi.build.sign — Ed25519 detached signature.

Uses the standard library's ``cryptography`` package for Ed25519 ops.
The bundle digest formula is identical to the TS SDK so signatures
are interchangeable.
"""

from __future__ import annotations

import base64
import hashlib
import os
import re
import struct
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Literal, Optional, Union

from .errors import SignatureError

_MAGIC = b"WAEASIv1"
_VERSION = 1
_PEM_RE = re.compile(
    r"-----BEGIN WAEASI ED25519 SEED-----\s*([A-Za-z0-9+/=\s]+?)-----END",
    re.DOTALL,
)


@dataclass(frozen=True)
class SignInput:
    out_dir: Path
    user_digest: str
    engine_digest: Optional[str]
    manifest_path: Path
    key_source: Union[
        tuple[Literal["raw"], bytes],
        tuple[Literal["file"], Path],
        tuple[Literal["env"], str],
    ]


@dataclass(frozen=True)
class SignResult:
    signature_path: Path
    bundle_digest_hex: str
    public_key_hex: str
    duration_ms: int


def sign(input: SignInput) -> SignResult:
    start = time.monotonic()
    seed, public_key = _load_key(input.key_source)
    manifest_bytes = input.manifest_path.read_bytes()
    digest = _bundle_digest(
        _VERSION, input.engine_digest, input.user_digest, manifest_bytes,
    )

    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    except ImportError as e:
        raise SignatureError(
            "cryptography package required.  pip install cryptography"
        ) from e

    priv = Ed25519PrivateKey.from_private_bytes(seed)
    sig = priv.sign(digest)

    blob = (
        _MAGIC
        + bytes([_VERSION, len(sig)])
        + sig
        + bytes([len(public_key)])
        + public_key
        + digest
    )
    out = input.out_dir / "signature.ed25519"
    input.out_dir.mkdir(parents=True, exist_ok=True)
    out.write_bytes(blob)

    return SignResult(
        signature_path=out,
        bundle_digest_hex=digest.hex(),
        public_key_hex=public_key.hex(),
        duration_ms=int((time.monotonic() - start) * 1000),
    )


def _bundle_digest(
    version: int,
    engine_digest: Optional[str],
    user_digest: str,
    manifest_bytes: bytes,
) -> bytes:
    h = hashlib.sha256()
    h.update(struct.pack("<I", version))
    if engine_digest:
        eb = bytes.fromhex(engine_digest)
        h.update(struct.pack("<I", len(eb)))
        h.update(eb)
    else:
        h.update(struct.pack("<I", 0))
    ub = bytes.fromhex(user_digest)
    h.update(struct.pack("<I", len(ub)))
    h.update(ub)
    h.update(struct.pack("<I", len(manifest_bytes)))
    h.update(manifest_bytes)
    return h.digest()


def _load_key(src) -> tuple[bytes, bytes]:  # type: ignore[no-untyped-def]
    kind = src[0]
    if kind == "raw":
        seed = src[1]
    elif kind == "file":
        seed = _seed_from_file(src[1])
    elif kind == "env":
        v = os.environ.get(src[1])
        if not v:
            raise SignatureError(f"env var {src[1]} unset")
        seed = bytes.fromhex(v)
    else:
        raise SignatureError(f"unknown key source: {kind}")
    if len(seed) != 32:
        raise SignatureError(f"expected 32-byte ed25519 seed, got {len(seed)}")
    public_key = _derive_public(seed)
    return seed, public_key


def _seed_from_file(path: Path) -> bytes:
    text = path.read_text()
    m = _PEM_RE.search(text)
    if not m:
        raise SignatureError(f"malformed key file: {path}")
    return base64.b64decode(re.sub(r"\s", "", m.group(1)))


def _derive_public(seed: bytes) -> bytes:
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    from cryptography.hazmat.primitives import serialization
    priv = Ed25519PrivateKey.from_private_bytes(seed)
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    return pub
