// @waeasi/sdk — wasi:http binding facade
//
// At build time `jco componentize` replaces this file's import-side
// implementation with direct calls to the host's wasi:http/types and
// wasi:http/incoming-handler@0.2.0 imports.  Until then (dev-mode unit
// tests, IDE typecheck) we expose plain TypeScript interfaces and a
// minimal in-memory polyfill keyed off `globalThis.__WAEASI_HOST__`.

import type { InputStream, OutputStream } from "./io.js";

export interface IncomingRequest {
    method(): string;
    scheme(): "http" | "https";
    authority(): string;
    pathWithQuery(): string;
    headers(): Array<[string, Uint8Array]>;
    consumeBody(): InputStream;
}

export interface ResponseOutparam {
    startResponse(
        status: number,
        headers: Array<[string, Uint8Array]>,
    ): OutputStream;
    finish(): void;
}

/** Optional hostside fetch (waeasi:net/outbound) for outgoing requests. */
export interface OutboundFetch {
    send(
        method: string,
        url: string,
        headers: Array<[string, Uint8Array]>,
        body: Uint8Array | null,
        timeoutMs: number,
    ): Promise<{
        status: number;
        headers: Array<[string, Uint8Array]>;
        body: Uint8Array;
    }>;
}

/** Dev-mode polyfill installer.  Called only in unit-test environments. */
export function installPolyfill(host: {
    nextRequest?: () => IncomingRequest | null;
    capture?: (status: number, headers: Array<[string, Uint8Array]>, body: Uint8Array) => void;
    fetch?: OutboundFetch;
}): void {
    (globalThis as Record<string, unknown>).__WAEASI_HOST__ = host;
}

interface PolyHost {
    nextRequest?: () => IncomingRequest | null;
    capture?: (status: number, headers: Array<[string, Uint8Array]>, body: Uint8Array) => void;
    fetch?: OutboundFetch;
}

function host(): PolyHost {
    return ((globalThis as Record<string, unknown>).__WAEASI_HOST__ as PolyHost) ?? {};
}

/** Outbound HTTP — only available when capability `NET_CONNECT` granted. */
export async function fetch(
    url: string,
    init: { method?: string; headers?: Array<[string, Uint8Array]>; body?: Uint8Array; timeoutMs?: number } = {},
): Promise<{ status: number; headers: Array<[string, Uint8Array]>; body: Uint8Array }> {
    const f = host().fetch;
    if (!f) throw new Error("waeasi:net/outbound not bound (capability missing?)");
    return f.send(
        init.method ?? "GET",
        url,
        init.headers ?? [],
        init.body ?? null,
        init.timeoutMs ?? 30_000,
    );
}

/** Polyfill — produce a synthetic ResponseOutparam that captures into memory. */
export function polyfillOutparam(): ResponseOutparam {
    const h = host();
    let status = 0;
    let headers: Array<[string, Uint8Array]> = [];
    let buf: Uint8Array[] = [];
    let total = 0;
    return {
        startResponse(s, hs) {
            status = s;
            headers = hs;
            return {
                async writeAll(c: Uint8Array) {
                    buf.push(c);
                    total += c.byteLength;
                },
                close() { /* no-op */ },
            };
        },
        finish() {
            const out = new Uint8Array(total);
            let off = 0;
            for (const c of buf) { out.set(c, off); off += c.byteLength; }
            h.capture?.(status, headers, out);
        },
    };
}
