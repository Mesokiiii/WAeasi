"""waeasi.build.wizer — pre-init snapshot stage.

componentize-py emits an ``_initialize`` hook that loads the user
module under CPython-on-Wasm; wizer runs that hook once at build time
and freezes the resulting linear memory.  This collapses Python cold
start from ~150 ms to ~5 ms.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path

from .errors import ToolchainError


@dataclass(frozen=True)
class WizerInput:
    component_path: Path
    out_path: Path
    init_func: str = "wizer.initialize"
    allow_wasi: bool = True
    timeout_sec: int = 60


@dataclass(frozen=True)
class WizerResult:
    snapshot_path: Path
    size_bytes: int
    duration_ms: int
    grew: bool


def wizer(input: WizerInput) -> WizerResult:
    start = time.monotonic()
    input.out_path.parent.mkdir(parents=True, exist_ok=True)

    bin_path = os.environ.get("WAEASI_WIZER") or shutil.which("wizer")
    if not bin_path:
        raise ToolchainError(
            "wizer binary not found.  install with: cargo install wizer-cli"
        )

    in_size = input.component_path.stat().st_size
    args = [
        bin_path,
        str(input.component_path),
        "-o", str(input.out_path),
        "--init-func", input.init_func,
        "--allow-wasi", "true" if input.allow_wasi else "false",
        "--wasm-bulk-memory", "true",
    ]
    try:
        subprocess.run(
            args, check=True, capture_output=True, text=True,
            timeout=input.timeout_sec,
        )
    except subprocess.CalledProcessError as e:
        raise ToolchainError(f"wizer failed:\n{e.stderr}") from e
    except subprocess.TimeoutExpired as e:
        raise ToolchainError(f"wizer timed out after {input.timeout_sec}s") from e

    out_size = input.out_path.stat().st_size
    return WizerResult(
        snapshot_path=input.out_path,
        size_bytes=out_size,
        duration_ms=int((time.monotonic() - start) * 1000),
        grew=out_size > in_size,
    )


def passthrough(input: WizerInput) -> WizerResult:
    """Skip-Wizer fallback for dev mode."""
    start = time.monotonic()
    input.out_path.parent.mkdir(parents=True, exist_ok=True)
    import shutil as _sh
    _sh.copyfile(input.component_path, input.out_path)
    return WizerResult(
        snapshot_path=input.out_path,
        size_bytes=input.out_path.stat().st_size,
        duration_ms=int((time.monotonic() - start) * 1000),
        grew=False,
    )
