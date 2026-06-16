# WAeasi SDK

> Write WAeasi components in **TypeScript**, **Python** or **Go** — get
> WASI-0.2 Component Model binaries that the kernel mmaps into a
> sandbox in microseconds.

Three first-class SDKs share a single source of truth (`wit/`) and a
single binary container format (`.waeasi-bundle`).  The pipeline is
deterministic and reproducible: same source + same dependencies = same
content-addressed bundle.

```
waeasi/
├── wit/                       ← WIT (world.wit, observability.wit, types.wit)
├── builder/                   ← language-agnostic Rust pipeline driver
└── sdk/
    ├── typescript/            ← @waeasi/sdk        (npm)
    ├── python/                ← waeasi             (PyPI)
    └── go/                    ← waeasi.dev/sdk     (Go module)
```

## At a glance

| | TypeScript | Python | Go |
|---|---|---|---|
| Engine | StarlingMonkey (componentize-js) | CPython (componentize-py) | TinyGo native |
| Cold start (post-Wizer) | ~1 ms | ~5 ms | <0.5 ms |
| Bundle size | 2-12 MiB | 8-15 MiB | 20-200 KiB |
| Wizer required | yes | yes | no |
| Engine dedup viable | yes | yes | n/a |
| CLI | `waeasi` (npm bin) | `waeasi` (console script) | uses `waeasi-builder` |

The TS / Python pipelines invoke language-specific tooling (jco /
componentize-py); the Go path goes straight through `waeasi-builder`
because TinyGo emits a Component Model artefact directly.

## End-to-end: TypeScript example

```ts
// src/index.ts
import { Request, Response, ExecutionContext } from "@waeasi/sdk";

export function handleRequest(req: Request, ctx: ExecutionContext): Response {
    ctx.log.info("hi", { path: req.path() });
    return Response.json({ ok: true, ts: ctx.wallNs().toString() });
}
```

```bash
npm create waeasi@latest myapp
cd myapp
export WAEASI_SIGN_KEY=$(cat my-key.hex)
npm run build       # → dist/myapp.waeasi-bundle
npm run deploy      # → waeasictl run
```

## End-to-end: Python example

```python
# src/myapp/handler.py
from waeasi import Request, Response, ExecutionContext, define_handler

@define_handler()
async def handle(req: Request, ctx: ExecutionContext) -> Response:
    return Response.json_response({"ok": True, "ts": ctx.wall_ns()})
```

```bash
waeasi init myapp --template handler
cd myapp
pip install -e .
waeasi build
waeasi deploy
```

## End-to-end: Go example

```go
package main

import "waeasi.dev/sdk/handler"

func init() {
    handler.Handle(func(req *handler.Request) *handler.Response {
        return handler.JSON(map[string]any{
            "ok":   true,
            "path": req.Path(),
        })
    })
}
func main() {}
```

```bash
tinygo build -target=wasip2 -o dist/raw.wasm ./cmd/myapp
waeasi-builder build \
    --component dist/raw.wasm \
    --manifest waeasi.toml \
    --key env:WAEASI_SIGN_KEY \
    --out dist
```

## The bundle format

Every SDK emits the same `.waeasi-bundle`:

```
+------------------------------------------------------------+
| "WAEASIBND" | u8 ver=1 | u32 LE entry-count                 |
+------------------------------------------------------------+
| { u8 name_len | name | u64 LE offset | u64 LE length } × N  |
+------------------------------------------------------------+
| (8-byte padding)                                            |
| manifest.toml                                               |
| (8-byte padding)                                            |
| user.cwasm                                                  |
| (8-byte padding)                                            |
| signature.ed25519                                           |
| (8-byte padding)                                            |
| engine.cwasm  (optional, only with --split-engine)          |
| (8-byte padding)                                            |
| user.cwasm.aot (optional, only with --aot)                  |
+------------------------------------------------------------+
```

The signature is computed across `version ‖ engine-digest ‖ user-digest
‖ manifest-bytes` so any tampering with the body or the policy is
detected at load.

## Why three SDKs share one builder

Engineering economics: `wizer`, `wasm-tools component split`, the
manifest schema, the Ed25519 signing scheme, and the container layout
are language-agnostic.  Implementing them once in Rust as
`waeasi-builder` avoids:

* Three copies of the binary layout to keep in sync.
* Three signature implementations to audit.
* Three TOML emitters that drift from the kernel parser.

The TS and Python SDKs ship their own pipeline drivers because they
need to invoke language-specific componentizers (jco / componentize-py)
*before* the binary stages.  After componentization, all three flows
funnel through the same logical sequence:

```
componentize → wizer → split → manifest → sign → assemble
```

## See also

* [`sdk/typescript/README.md`](typescript/README.md) — TypeScript SDK
* [`sdk/python/README.md`](python/README.md)         — Python SDK
* [`sdk/go/README.md`](go/README.md)                  — Go SDK
* [`builder/`](../builder)                            — Rust builder
* [`docs/sdk-integration.md`](../docs/sdk-integration.md) — kernel integration

## License

MIT OR Apache-2.0
