"""waeasi.build.compose — engine/user split + sha256 digests.

Same scaffold as the TS SDK's compose stage.  When ``split=False`` we
emit a single ``user.cwasm`` with no engine separation; when ``True``
we shell out to ``wasm-tools component split``.
"""

from __future__ import annotations

import hashlib
import os
import shutil
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from .errors import ToolchainError


@dataclass(frozen=True)
class ComposeInput:
    component_path: Path
    out_dir: Path
    split: bool = False


@dataclass(frozen=True)
class ComposeResult:
    engine_path: Optional[Path]
    engine_digest: Optional[str]
    user_path: Path
    user_digest: str
    size_user_bytes: int
    size_engine_bytes: int
    duration_ms: int


def compose(input: ComposeInput) -> ComposeResult:
    start = time.monotonic()
    input.out_dir.mkdir(parents=True, exist_ok=True)

    user_path = input.out_dir / "user.cwasm"
    engine_path: Optional[Path] = None
    engine_digest: Optional[str] = None
    engine_size = 0

    if input.split:
        wt = os.environ.get("WAEASI_WASM_TOOLS") or shutil.which("wasm-tools")
        if not wt:
            shutil.copyfile(input.component_path, user_path)
        else:
            engine_tmp = input.out_dir / "engine.cwasm"
            try:
                subprocess.run(
                    [wt, "component", "split",
                     str(input.component_path),
                     "--engine", str(engine_tmp),
                     "--user", str(user_path)],
                    check=True, capture_output=True, text=True,
                )
                engine_path = engine_tmp
                engine_size = engine_path.stat().st_size
                engine_digest = _sha256(engine_path)
            except subprocess.CalledProcessError:
                # graceful degradation: single artifact
                shutil.copyfile(input.component_path, user_path)
                engine_path = None
    else:
        shutil.copyfile(input.component_path, user_path)

    user_digest = _sha256(user_path)
    return ComposeResult(
        engine_path=engine_path,
        engine_digest=engine_digest,
        user_path=user_path,
        user_digest=user_digest,
        size_user_bytes=user_path.stat().st_size,
        size_engine_bytes=engine_size,
        duration_ms=int((time.monotonic() - start) * 1000),
    )


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(64 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def verify_digest(path: Path, expected: str) -> bool:
    return _sha256(path) == expected
