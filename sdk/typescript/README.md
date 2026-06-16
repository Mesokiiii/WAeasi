# @waeasi/sdk

Official TypeScript SDK for [**WAeasi**](https://github.com/waeasi/waeasi)
— a `no_std` Rust microkernel for serving WebAssembly Component Model
workloads on bare metal.

```ts
// src/index.ts
import type { Request } from "@waeasi/sdk";

export function handleRequest(req: Request): Response {
    return new Response(`hello from ${req.path()}!`);
}
```

```bash
npm create waeasi@latest hello
cd hello
npm install
npm run build      # emits dist/hello.waeasi-bundle
npm run deploy     # → waeasictl run
```

## Features

* **Fetch API**: `Request` / `Response` / `Headers` shapes match the
  Web Platform standard.  Most code written for Cloudflare Workers,
  Fastly Compute@Edge or Deno Deploy ports unchanged.
* **WASI 0.2 / Component Model**: every component the SDK emits is a
  standards-conformant Wasm Component, importing `wasi:http@0.2.0`,
  `wasi:io`, `wasi:clocks`, `wasi:random`.
* **Wizer pre-init**: SpiderMonkey-on-Wasm cold start collapses from
  ~80 ms to ~1 ms on the snapshot path.
* **Engine deduplication**: optional engine/user split so the kernel's
  CoW pooling allocator can share the JS engine across thousands of
  user functions.
* **Ed25519 signed bundles**: every output is signed and accepted by
  the kernel's `crypto/verify` allowlist.

## Project layout

```
my-component/
├── package.json
├── waeasi.config.ts          ← manifest spec, entry, capabilities
├── wit/                      ← copied/derived from waeasi/wit
└── src/
    └── index.ts              ← `export function handleRequest(req)`
```

## Build pipeline (under the hood)

```
src/index.ts
   │
   ▼  esbuild (bundle, tree-shake)
bundle.mjs
   │
   ▼  jco componentize-js
raw.wasm  (StarlingMonkey + user bytecode)
   │
   ▼  wizer pre-init
wizered.wasm
   │
   ▼  wasm-tools component split (optional)
engine.cwasm + user.cwasm
   │
   ▼  manifest.toml + ed25519 signature
.waeasi-bundle (single artefact, content-addressed)
```

Each stage is independently consumable from JavaScript:

```ts
import { buildAll, formatResult } from "@waeasi/sdk/build";

const r = await buildAll({ entry: "src/index.ts", /* ... */ });
console.log(formatResult(r));
```

## CLI

| Command | Purpose |
|---|---|
| `waeasi init <name>` | scaffold a project from a template |
| `waeasi build`       | full prod build (Wizer on, signed) |
| `waeasi dev`         | watch + rebuild, dev key, no Wizer |
| `waeasi deploy`      | shell to `waeasictl run` |

## Configuration

`waeasi.config.ts` (typed via `WaeasiConfig`):

```ts
import type { WaeasiConfig } from "@waeasi/sdk/build";

export default {
    entry: "src/index.ts",
    outDir: "dist",
    witPath: "wit",
    keyEnv: "WAEASI_SIGN_KEY",
    manifest: {
        world: "handler",
        rights: ["CLOCK_MONO", "RANDOM_SEC"],
        cpuShares: 100,
        memoryPagesMax: 256,
        linearMemMax: "16 MiB",
    },
} satisfies WaeasiConfig;
```

## Development

```bash
cd sdk/typescript
npm install
npm run build
npm test
```

## License

MIT OR Apache-2.0
