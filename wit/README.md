# WAeasi WIT — single source of truth for ABI

This directory holds the **only** authoritative description of every
host/guest interface in WAeasi.  Three artefacts are derived from these
files at build-time and **must never be edited by hand** elsewhere:

| Consumer | Tool | Output |
|---|---|---|
| Kernel host bindings | `wit-bindgen rust --runtime no_std` | `kernel/src/wasm/bindgen/` |
| TypeScript SDK stubs | `jco types` | `sdk/typescript/src/wasi/generated/` |
| Python SDK stubs | `componentize-py bindings` | `sdk/python/waeasi/wasi/generated/` |
| Go SDK stubs | `wit-bindgen tinygo` | `sdk/go/wasi/generated/` |
| Builder verifier | `wasm-tools validate --features component-model` | n/a |

## Structure

```
wit/
├── waeasi/
│   ├── world.wit            ← public worlds: handler, scheduled, stream
│   ├── observability.wit    ← waeasi:obs / log + metrics + tracing
│   └── types.wit            ← scheduled + stream packages
└── deps/                    ← vendored upstream WASI 0.2.x (populated by `wkg`)
    ├── wasi-http/
    ├── wasi-io/
    ├── wasi-clocks/
    ├── wasi-sockets/
    └── wasi-random/
```

The `deps/` tree is **not** committed; `builder/` populates it via the
`wkg` (WIT package manager) tool against a pinned manifest.

## Versioning

Every world declares an explicit semver tag (e.g. `@0.1.0`).  The kernel
maintains a **whitelist** of accepted world hashes; bumping a world's
version requires a corresponding kernel release.  Components targeting
an unknown world are rejected at load-time with `WasmError::Link`.

## Adding a new world

1. Add the `.wit` file under `waeasi/`.
2. Run `cargo run -p waeasi-builder -- wit check` — verifies syntax,
   semver, and ABI compatibility against the previous published version.
3. Regenerate bindings for every SDK via `cargo run -p waeasi-builder
   -- wit gen`.
4. Update `kernel/src/wasm/linker.rs::DEFAULT_IMPORTS` with the new
   import set.
5. Bump the kernel version in `Cargo.toml` and document the change in
   the relevant `docs/stage*.md`.
