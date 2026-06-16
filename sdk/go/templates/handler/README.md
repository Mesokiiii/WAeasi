# `__NAME__`

Generated from the WAeasi Go SDK handler template.

## Build

Requires TinyGo ≥ 0.34 and the `waeasi-builder` CLI.

```bash
# 1. Compile to a Wasm Component (wasi:http handler).
tinygo build -target=wasip2 -o dist/raw.wasm ./cmd/__NAME__

# 2. Wizer + sign + bundle (drives the rest of the pipeline).
waeasi-builder build \
    --component dist/raw.wasm \
    --manifest waeasi.toml \
    --out dist/__NAME__.waeasi-bundle
```

## Manifest

`waeasi.toml`:

```toml
[component]
name    = "__NAME__"
version = "0.1.0"
world   = "handler"

[capabilities]
rights = ["CLOCK_MONO", "RANDOM_SEC"]

[resources]
cpu_shares       = 100
memory_pages_max = 256
linear_mem_max   = "16 MiB"
```

## Test

The handler is plain Go and unit-testable without TinyGo:

```bash
go test ./...
```
