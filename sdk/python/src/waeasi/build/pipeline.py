"""waeasi.build.pipeline — orchestrate every stage in canonical order."""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional, Union

from .bundle import BundleEntry, assemble
from .compose import ComposeInput, compose
from .componentize import ComponentizeInput, componentize, is_component
from .errors import BuildError
from .manifest import ManifestInput, ManifestSpec, write_manifest
from .sign import SignInput, sign
from .wizer import WizerInput, passthrough, wizer


@dataclass
class StageTiming:
    stage: str
    ms: int
    bytes: Optional[int] = None


@dataclass
class BuildOptions:
    module_name: str             # python module path, e.g. "myapp.handler"
    project_root: Path           # contains pyproject.toml
    out_dir: Path
    wit_path: Path
    spec: ManifestSpec
    sdk_version: str
    key_source: Union[
        tuple[str, bytes], tuple[str, Path], tuple[str, str],
    ]
    skip_wizer: bool = False
    split_engine: bool = False


@dataclass
class BuildResult:
    bundle_path: Path
    bundle_digest: str
    public_key_hex: str
    timings: list[StageTiming] = field(default_factory=list)
    total_ms: int = 0


def build_all(opts: BuildOptions) -> BuildResult:
    t0 = time.monotonic()
    timings: list[StageTiming] = []
    opts.out_dir.mkdir(parents=True, exist_ok=True)

    raw = opts.out_dir / "raw.wasm"
    c = componentize(ComponentizeInput(
        module_name=opts.module_name,
        project_root=opts.project_root,
        out_path=raw,
        wit_path=opts.wit_path,
        world=opts.spec.world,
    ))
    timings.append(StageTiming("componentize", c.duration_ms, c.size_bytes))
    if not is_component(raw):
        raise BuildError("componentize-py produced a non-component .wasm")

    wizered = opts.out_dir / "wizered.wasm"
    w_input = WizerInput(component_path=raw, out_path=wizered)
    w = passthrough(w_input) if opts.skip_wizer else wizer(w_input)
    timings.append(StageTiming("wizer", w.duration_ms, w.size_bytes))

    composed = compose(ComposeInput(
        component_path=wizered,
        out_dir=opts.out_dir,
        split=opts.split_engine,
    ))
    timings.append(StageTiming(
        "compose", composed.duration_ms, composed.size_user_bytes,
    ))

    manifest_path = opts.out_dir / "manifest.toml"
    m = write_manifest(ManifestInput(
        spec=opts.spec,
        user_digest=composed.user_digest,
        engine_digest=composed.engine_digest,
        sdk_version=opts.sdk_version,
        out_path=manifest_path,
    ))
    timings.append(StageTiming("manifest", 0, m.size_bytes))

    s = sign(SignInput(
        out_dir=opts.out_dir,
        user_digest=composed.user_digest,
        engine_digest=composed.engine_digest,
        manifest_path=manifest_path,
        key_source=opts.key_source,  # type: ignore[arg-type]
    ))
    timings.append(StageTiming("sign", s.duration_ms))

    final_path = opts.out_dir / f"{opts.spec.name}.waeasi-bundle"
    entries = [
        BundleEntry("manifest.toml", manifest_path),
        BundleEntry("user.cwasm", composed.user_path),
        BundleEntry("signature.ed25519", s.signature_path),
    ]
    if composed.engine_path is not None:
        entries.append(BundleEntry("engine.cwasm", composed.engine_path))
    a = assemble(entries, final_path)
    timings.append(StageTiming("assemble", 0, a.size_bytes))

    return BuildResult(
        bundle_path=a.path,
        bundle_digest=a.digest,
        public_key_hex=s.public_key_hex,
        timings=timings,
        total_ms=int((time.monotonic() - t0) * 1000),
    )


def format_result(r: BuildResult) -> str:
    def fmt(n: Optional[int]) -> str:
        if n is None: return "       -"
        if n < 1024: return f"{n:>7} B"
        if n < 1 << 20: return f"{n / 1024:>6.1f} KiB"
        return f"{n / (1 << 20):>6.2f} MiB"

    rows = [
        f"  {t.stage:<13}  {t.ms:>5} ms  {fmt(t.bytes)}"
        for t in r.timings
    ]
    return "\n".join([
        f"built {r.bundle_path}",
        f"digest sha256:{r.bundle_digest[:16]}…",
        f"key    {r.public_key_hex[:16]}…",
        *rows,
        "  ─────────────────────────────────────────",
        f"  total          {r.total_ms:>5} ms",
    ])
