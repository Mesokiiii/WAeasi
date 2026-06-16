"""waeasi.cli.commands — implementations of every subcommand."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Optional

from ..build.errors import BuildError
from ..build.pipeline import BuildOptions, build_all, format_result
from . import ui
from .config import WaeasiConfig, load_config

TEMPLATES = ("handler",)
SDK_VERSION = "0.1.0"


def _resolve_key(cfg: WaeasiConfig, override: Optional[Path]) -> tuple:
    if override is not None: return ("file", override)
    if cfg.key_file is not None: return ("file", cfg.key_file)
    if cfg.key_env: return ("env", cfg.key_env)
    if "WAEASI_SIGN_KEY" in os.environ: return ("env", "WAEASI_SIGN_KEY")
    ui.fail(
        "no signing key configured.  set 'key_file' in waeasi.toml, "
        "WAEASI_SIGN_KEY env, or pass --key <path>",
    )
    raise SystemExit(1)


def cmd_build(argv: list[str]) -> int:
    args = _parse_flags(argv)
    cfg = load_config(Path.cwd(), Path(args["config"]) if "config" in args else None)
    ui.info(f"project {ui.bold(cfg.spec.name)} {ui.dim('@ ' + str(cfg.project_root))}")
    if not cfg.wit_path.is_dir():
        ui.fail(f"wit path not found: {cfg.wit_path}")

    key_src = _resolve_key(cfg, Path(args["key"]) if "key" in args else None)
    opts = BuildOptions(
        module_name=cfg.module,
        project_root=cfg.project_root,
        out_dir=cfg.out_dir if "out" not in args else Path(args["out"]),
        wit_path=cfg.wit_path,
        spec=cfg.spec,
        sdk_version=SDK_VERSION,
        key_source=key_src,
        skip_wizer=cfg.skip_wizer or "dev" in args,
        split_engine=cfg.split_engine or "split-engine" in args,
    )
    try:
        r = build_all(opts)
    except BuildError as e:
        ui.fail(f"build failed: {e}")
        return 3
    sys.stdout.write(format_result(r) + "\n")
    return 0


def cmd_init(argv: list[str]) -> int:
    args = _parse_flags(argv)
    if not args.get("_pos"):
        ui.fail("usage: waeasi init <name> [--template handler]")
    name = str(args["_pos"][0])
    if not name.replace("-", "").isalnum() or not name[0].isalpha():
        ui.fail(f"invalid component name: {name}")

    template = str(args.get("template", "handler"))
    if template not in TEMPLATES:
        ui.fail(f"unknown template: {template} (choose: {', '.join(TEMPLATES)})")

    target = Path.cwd() / args.get("dir", name)
    if target.exists() and "force" not in args:
        ui.fail(f"{target} already exists.  use --force to overwrite.")
    target.mkdir(parents=True, exist_ok=True)

    src = _locate_template(template)
    ui.info(f"scaffolding {ui.bold(name)} from {ui.cyan(template)} at {target}")
    _copy_tree(src, target, name)
    ui.ok(f"created {target}")
    sys.stdout.write("next steps:\n")
    sys.stdout.write(f"  cd {target.relative_to(Path.cwd())}\n")
    sys.stdout.write("  pip install -e .\n")
    sys.stdout.write("  waeasi build\n")
    return 0


def cmd_deploy(argv: list[str]) -> int:
    args = _parse_flags(argv)
    bundle = Path(args["_pos"][0]) if args.get("_pos") else _autodiscover_bundle()
    if bundle is None or not bundle.exists():
        ui.fail("no bundle found (looked in ./dist/*.waeasi-bundle)")
        return 2
    ctl = args.get("waeasictl") or os.environ.get("WAEASICTL") or "waeasictl"
    server = args.get("server", "127.0.0.1:9300")
    ui.info(f"deploying {ui.bold(str(bundle))} to {server}")
    try:
        subprocess.run(
            [ctl, "--server", server, "run", str(bundle)],
            check=True,
        )
    except FileNotFoundError:
        ui.fail(f"{ctl} not found.  install waeasictl or pass --waeasictl <path>")
    except subprocess.CalledProcessError as e:
        return e.returncode
    return 0


def _parse_flags(argv: list[str]) -> dict:
    out: dict = {"_pos": []}
    i = 0
    while i < len(argv):
        a = argv[i]
        if a.startswith("--"):
            if "=" in a:
                k, v = a[2:].split("=", 1); out[k] = v
            else:
                k = a[2:]
                nxt = argv[i + 1] if i + 1 < len(argv) else None
                if nxt is not None and not nxt.startswith("--"):
                    out[k] = nxt; i += 1
                else:
                    out[k] = True
        else:
            out["_pos"].append(a)
        i += 1
    return out


def _locate_template(name: str) -> Path:
    here = Path(__file__).resolve()
    for c in (here.parents[3] / "templates" / name,
              here.parents[4] / "templates" / name):
        if c.is_dir():
            return c
    ui.fail(f"template '{name}' not found")
    raise SystemExit(1)


def _copy_tree(src: Path, dst: Path, name: str) -> None:
    for entry in src.iterdir():
        target = dst / entry.name.replace("__NAME__", name)
        if entry.is_dir():
            target.mkdir(exist_ok=True)
            _copy_tree(entry, target, name)
        else:
            text = _maybe_text(entry)
            if text is not None:
                target.write_text(text.replace("__NAME__", name), encoding="utf-8")
            else:
                shutil.copyfile(entry, target)


def _maybe_text(path: Path) -> Optional[str]:
    if path.suffix in {".py", ".toml", ".md", ".wit", ".json", ".cfg"}:
        return path.read_text(encoding="utf-8")
    return None


def _autodiscover_bundle() -> Optional[Path]:
    dist = Path.cwd() / "dist"
    if not dist.is_dir(): return None
    for p in dist.iterdir():
        if p.suffix == ".waeasi-bundle" or p.name.endswith(".waeasi-bundle"):
            return p
    return None
