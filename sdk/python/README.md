# waeasi — Python SDK

Official Python SDK for [**WAeasi**](https://github.com/waeasi/waeasi),
a Rust microkernel for Wasm Component Model workloads.

```python
# src/myapp/handler.py
from waeasi import Request, Response, ExecutionContext, define_handler

@define_handler()
async def handle(req: Request, ctx: ExecutionContext) -> Response:
    return Response.json_response({"hello": req.path()})
```

```bash
waeasi init myapp
cd myapp
pip install -e .
waeasi build         # emits dist/myapp.waeasi-bundle
waeasi deploy
```

## What it does

* Wraps `componentize-py` so any Python module exposing a coroutine can
  become a Wasm Component targeting `wasi:http/incoming-handler@0.2.0`.
* Drives Wizer to pre-initialize CPython-on-Wasm and snapshot the
  resulting heap — cold start drops from ~150 ms to ~5 ms.
* Splits engine bytes from user bytes so the kernel's CoW pooling
  allocator can deduplicate the runtime across thousands of components.
* Generates `manifest.toml` consumed by the kernel's parser and signs
  the result with Ed25519 (compatible with the host allowlist).

## Configuration

Either `waeasi.toml` at the project root, or `[tool.waeasi]` inside
`pyproject.toml`:

```toml
[tool.waeasi]
module    = "myapp.handler"
out_dir   = "dist"
wit_path  = "../../wit"
key_env   = "WAEASI_SIGN_KEY"

[tool.waeasi.manifest]
name             = "myapp"
version          = "0.1.0"
world            = "handler"
rights           = ["CLOCK_MONO", "RANDOM_SEC"]
cpu_shares       = 100
memory_pages_max = 256
linear_mem_max   = "16 MiB"
```

## CLI

| Command | Purpose |
|---|---|
| `waeasi init <name>` | scaffold a project |
| `waeasi build`       | full prod build (Wizer on, signed) |
| `waeasi build --dev` | dev build (no Wizer, fast iteration) |
| `waeasi deploy`      | shell to `waeasictl run` |

## Programmatic API

```python
from pathlib import Path
from waeasi.build import BuildOptions, ManifestSpec, build_all, format_result

result = build_all(BuildOptions(
    module_name="myapp.handler",
    project_root=Path("."),
    out_dir=Path("dist"),
    wit_path=Path("../../wit"),
    spec=ManifestSpec(
        name="myapp", version="0.1.0", world="handler",
        rights=["CLOCK_MONO"],
    ),
    sdk_version="0.1.0",
    key_source=("env", "WAEASI_SIGN_KEY"),
))
print(format_result(result))
```

## Requirements

* Python ≥ 3.11
* `componentize-py` (auto-installed)
* `wizer` binary on `$PATH` (recommended, omit only with `--dev`)
* `wasm-tools` (only for `split_engine = true`)

## License

MIT OR Apache-2.0
