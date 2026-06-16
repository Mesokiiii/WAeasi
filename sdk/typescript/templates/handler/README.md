# `__NAME__`

Generated from `@waeasi/sdk` handler template.

## Run locally

```bash
npm install
npm run dev          # watch + rebuild on save
```

## Build for production

```bash
export WAEASI_SIGN_KEY=$(cat my-key.hex)
npm run build        # emits dist/__NAME__.waeasi-bundle
npm run deploy       # → waeasictl run
```

## Test

```bash
npm test
```

The handler can be unit-tested without the kernel by using the dev-mode
polyfill:

```ts
import { Request, wasi } from "@waeasi/sdk";
import { handleRequest } from "../src/index.js";

wasi.http.installPolyfill({});       // optional fixtures
const res = await handleRequest(
    new Request("https://x/healthz", { method: "GET" }),
    /* ctx mock */ {} as any,
);
console.assert(res.status === 200);
```
