// @waeasi/sdk — Response
//
// Fetch-API compatible response.  The user constructs it with any of
// the standard BodyInit forms; the runtime harness later flushes it to
// wasi:http via `writeToOutparam(...)`.
//
// We deliberately keep status-text minimal: HTTP/2 dropped reason
// phrases entirely, and the kernel router maps numeric status to the
// canonical IANA reason for HTTP/1.1 wire formatting.

import { Body, type BodyInit } from "./body.js";
import { Headers, type HeadersInit } from "./headers.js";
import type { ResponseOutparam } from "../wasi/http.js";

export interface ResponseInit {
    status?: number;
    statusText?: string;
    headers?: HeadersInit;
}

export type JsonValue =
    | string | number | boolean | null
    | { [k: string]: JsonValue }
    | JsonValue[];

export class Response {
    readonly status: number;
    readonly statusText: string;
    readonly headers: Headers;
    private readonly _body: Body;

    constructor(body?: BodyInit, init?: ResponseInit) {
        const status = init?.status ?? 200;
        if (status < 100 || status > 599) {
            throw new RangeError(`status out of range: ${status}`);
        }
        this.status = status;
        this.statusText = init?.statusText ?? "";
        this.headers = new Headers(init?.headers);
        this._body = new Body(body);
    }

    get ok(): boolean       { return this.status >= 200 && this.status < 300; }
    get bodyUsed(): boolean { return this._body.bodyUsed; }

    arrayBuffer(): Promise<ArrayBuffer> { return this._body.arrayBuffer(); }
    bytes(): Promise<Uint8Array>        { return this._body.bytes(); }
    text(): Promise<string>             { return this._body.text(); }
    json<T = unknown>(): Promise<T>     { return this._body.json<T>(); }

    /** Static helper — `Response.json({...})` with proper Content-Type. */
    static json(value: JsonValue, init?: ResponseInit): Response {
        const headers = new Headers(init?.headers);
        if (!headers.has("content-type")) {
            headers.set("content-type", "application/json; charset=utf-8");
        }
        return new Response(JSON.stringify(value), {
            status: init?.status,
            statusText: init?.statusText,
            headers,
        });
    }

    /** Static helper — 3xx redirect. */
    static redirect(location: string, status: 301 | 302 | 303 | 307 | 308 = 302): Response {
        return new Response(null, {
            status,
            headers: { location },
        });
    }

    /** Static helper — error response with text/plain body. */
    static error(status: number, message?: string): Response {
        return new Response(message ?? "", {
            status,
            headers: { "content-type": "text/plain; charset=utf-8" },
        });
    }

    /**
     * Hostside finalizer — write status + headers + body into a wasi:http
     * response-outparam.  Auto-injects Content-Length when possible.
     */
    async writeToOutparam(out: ResponseOutparam): Promise<void> {
        // Best-effort Content-Length: we already have the bytes if the body
        // was constructed from string/Uint8Array.  The harness will skip
        // the header for streaming bodies (transfer-encoding: chunked).
        if (!this.headers.has("content-length")
            && !this.headers.has("transfer-encoding")
        ) {
            const cl = await this.tryComputeContentLength();
            if (cl !== null) this.headers.set("content-length", String(cl));
        }
        const tx = out.startResponse(this.status, this.headers.toWasi());
        try {
            await this._body.pipeToWasi(tx);
        } finally {
            tx.close();
            out.finish();
        }
    }

    private async tryComputeContentLength(): Promise<number | null> {
        // Only fast-path for already-materialised bodies.
        const internal = (this._body as unknown as { source: { kind: string; data?: Uint8Array } }).source;
        if (internal.kind === "empty") return 0;
        if (internal.kind === "bytes" && internal.data) return internal.data.byteLength;
        return null;
    }
}
