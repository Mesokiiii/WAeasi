"""waeasi.build.componentize — drive ``componentize-py``.

We invoke the upstream Bytecode-Alliance tool either via its Python
API (preferred — no subprocess overhead) or by shelling out to the
``componentize-py`` console script.  In both cases the result is a
Wasm Component Model binary that exports
``wasi:http/incoming-handler@0.2.0``.
"""

from __future__ import annotations

import os
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

from .errors import BuildError


@dataclass(frozen=True)
class ComponentizeInput:
    module_name: str          # python module path, e.g. "myapp.handler"
    project_root: Path        # where to run componentize-py
    out_path: Path
    wit_path: Path
    world: str = "handler"


@dataclass(frozen=True)
class ComponentizeResult:
    component_path: Path
    size_bytes: int
    duration_ms: int


def componentize(input: ComponentizeInput) -> ComponentizeResult:
    import time
    start = time.monotonic()
    input.out_path.parent.mkdir(parents=True, exist_ok=True)

    bin_path = os.environ.get("WAEASI_COMPONENTIZE_PY") \
        or shutil.which("componentize-py")
    if not bin_path:
        raise BuildError(
            "componentize-py not on PATH.  install with: "
            "pip install componentize-py"
        )

    cmd = [
        bin_path,
        "-d", str(input.wit_path),
        "-w", input.world,
        "componentize",
        input.module_name,
        "-o", str(input.out_path),
    ]
    try:
        subprocess.run(
            cmd, check=True,
            cwd=str(input.project_root),
            timeout=180,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError as e:
        raise BuildError(
            f"componentize-py failed (exit {e.returncode}):\n{e.stderr}"
        ) from e
    except subprocess.TimeoutExpired as e:
        raise BuildError("componentize-py timed out after 180s") from e

    if not input.out_path.exists():
        raise BuildError(f"componentize-py did not write {input.out_path}")

    return ComponentizeResult(
        component_path=input.out_path,
        size_bytes=input.out_path.stat().st_size,
        duration_ms=int((time.monotonic() - start) * 1000),
    )


def is_component(path: Path) -> bool:
    """Check the Component Model magic header."""
    try:
        with path.open("rb") as f:
            head = f.read(8)
    except OSError:
        return False
    return head == b"\x00asm\x0d\x00\x01\x00"
