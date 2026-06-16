# waeasi.dev/sdk — Go SDK for WAeasi

Idiomatic Go SDK for writing WAeasi components.  Targets
`tinygo build -target=wasip2`; the binding layer maps the
`wasi:http/incoming-handler@0.2.0` export onto a tiny Fetch-style
handler API.

```go
package main

import "waeasi.dev/sdk/handler"

func init() {
    handler.Handle(func(req *handler.Request) *handler.Response {
        return handler.JSON(map[string]any{
            "method": req.Method,
            "path":   req.Path(),
        })
    })
}

func main() {} // required but never invoked under wasip2
```

## Why not interpret JS or Python?

TinyGo emits **direct AOT machine code**.  No SpiderMonkey, no CPython,
no Wizer pre-init: a typical Go handler bundle is **20–80 KiB** and
cold-starts in **microseconds** rather than milliseconds.  This is why
Go gets a thinner SDK than TS/Python — there's nothing to optimize away.

## Packages

| Package | Purpose |
|---|---|
| `waeasi.dev/sdk/handler` | High-level Fetch-style API (Request, Response, Headers, Body, Context). |
| `waeasi.dev/sdk/wasi`    | Low-level wasi:http / wasi:io / wasi:obs bindings.  Replaced at build time by `wit-bindgen-go`. |

## Build pipeline

The Go SDK does not ship its own CLI.  Use the language-agnostic
`waeasi-builder` binary (Rust) to wrap TinyGo's output in
`.waeasi-bundle` form:

```bash
tinygo build -target=wasip2 -o dist/raw.wasm ./cmd/myapp
waeasi-builder build \
    --component dist/raw.wasm \
    --manifest waeasi.toml \
    --out dist/myapp.waeasi-bundle
```

`waeasi-builder` runs Wizer (where applicable — usually a no-op for
TinyGo output), composes engine/user split (also a no-op for AOT
languages), generates the manifest, signs with Ed25519 and produces
the final OCI-style bundle.

## Local SDK replacement

For development against an in-tree copy of the SDK:

```text
replace waeasi.dev/sdk => ../../sdk/go
```

## License

MIT OR Apache-2.0
