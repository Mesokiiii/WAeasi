"""waeasi.cli.config — load ``waeasi.toml`` (or pyproject ``[tool.waeasi]``).

Resolution order (first match wins):
  1. ``--config <path>``
  2. ``./waeasi.toml``
  3. ``[tool.waeasi]`` table in ``pyproject.toml``

Schema is the Python mirror of :class:`waeasi.build.ManifestSpec`.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from ..build.errors import ConfigError
from ..build.manifest import ManifestSpec

if sys.version_info >= (3, 11):
    import tomllib as _toml
else:  # pragma: no cover
    import tomli as _toml  # type: ignore[no-redef]


@dataclass
class WaeasiConfig:
    module: str                  # python module path, e.g. "myapp.handler"
    project_root: Path
    out_dir: Path
    wit_path: Path
    spec: ManifestSpec
    skip_wizer: bool = False
    split_engine: bool = False
    key_file: Optional[Path] = None
    key_env: Optional[str] = None


def load_config(cwd: Path, explicit: Optional[Path] = None) -> WaeasiConfig:
    root = _find_root(cwd)
    if explicit is not None:
        data = _read_toml(explicit, "waeasi")
    elif (root / "waeasi.toml").exists():
        data = _read_toml(root / "waeasi.toml", "waeasi")
    else:
        py = root / "pyproject.toml"
        if not py.exists():
            raise ConfigError(f"no waeasi.toml or pyproject.toml under {root}")
        data = _read_toml(py, "tool.waeasi")

    return _from_dict(data, root)


def _find_root(start: Path) -> Path:
    cur = start.resolve()
    for _ in range(16):
        if (cur / "pyproject.toml").exists() or (cur / "waeasi.toml").exists():
            return cur
        if cur.parent == cur:
            break
        cur = cur.parent
    raise ConfigError(f"could not locate pyproject.toml upward from {start}")


def _read_toml(path: Path, dotted_key: str) -> dict:
    with path.open("rb") as f:
        doc = _toml.load(f)
    cur: object = doc
    for part in dotted_key.split("."):
        if not isinstance(cur, dict) or part not in cur:
            raise ConfigError(f"{path}: missing [{dotted_key}]")
        cur = cur[part]
    if not isinstance(cur, dict):
        raise ConfigError(f"{path}: [{dotted_key}] must be a table")
    return cur


def _from_dict(d: dict, root: Path) -> WaeasiConfig:
    try:
        module = str(d["module"])
        m = d["manifest"]
        spec = ManifestSpec(
            name=str(m.get("name") or root.name),
            version=str(m.get("version") or "0.0.0"),
            world=str(m["world"]),
            rights=list(m.get("rights", [])),
            cpu_shares=m.get("cpu_shares"),
            memory_pages_max=m.get("memory_pages_max"),
            linear_mem_max=m.get("linear_mem_max"),
            exports=[(k, str(v)) for k, v in m.get("exports", {}).items()],
        )
    except KeyError as e:
        raise ConfigError(f"missing required field: {e}") from e

    return WaeasiConfig(
        module=module,
        project_root=root,
        out_dir=root / d.get("out_dir", "dist"),
        wit_path=root / d.get("wit_path", "wit"),
        spec=spec,
        skip_wizer=bool(d.get("skip_wizer", False)),
        split_engine=bool(d.get("split_engine", False)),
        key_file=(root / d["key_file"]) if "key_file" in d else None,
        key_env=d.get("key_env"),
    )
