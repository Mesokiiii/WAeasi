// @waeasi/sdk — Request
//
// Fetch-API compatible request type, exposed to user handlers.  All
// fields are read-only after construction; `clone()` produces an
// independent copy when the user wants to consume the body more than
// once.
//
// `Request.fromWasi(...)` is the canonical entry point invoked by the
// auto-generated handler harness — never called by user code directly.

import { Body, type BodyInit } from "./body.js";
import { Headers, type HeadersInit } from "./headers.js";
import type { IncomingRequest } from "../wasi/http.js";

export type Method =
    | "GET" | "HEAD" | "POST" | "PUT" | "DELETE"
    | "PATCH" | "OPTIONS" | "CONNECT" | "TRACE";

export interface RequestInit {
    method?: Method | string;
    headers?: HeadersInit;
    body?: BodyInit;
    /** Opaque trace identifier (W3C traceparent) propagated by host. */
    traceparent?: string;
}

export class Request {
    readonly method: string;
    readonly url: string;
    readonly headers: Headers;
    readonly traceparent: string | null;
    private readonly _body: Body;

    constructor(input: string | Request, init?: RequestInit) {
        if (input instanceof Request) {
            this.method = init?.method?.toUpperCase() ?? input.method;
            this.url = input.url;
            this.headers = new Headers(init?.headers ?? input.headers);
            this.traceparent = init?.traceparent ?? input.traceparent;
            this._body = init?.body !== undefined
                ? new Body(init.body)
                : input._body;
            return;
        }
        this.method = (init?.method ?? "GET").toUpperCase();
        this.url = input;
        this.headers = new Headers(init?.headers);
        this.traceparent = init?.traceparent ?? null;
        this._body = new Body(init?.body);
    }

    /** Hostside factory — wires a wasi:http IncomingRequest into Request. */
    static fromWasi(req: IncomingRequest): Request {
        const url = req.scheme() + "://" + req.authority() + req.pathWithQuery();
        const headers = Headers.fromWasi(req.headers());
        const body = Body.fromWasi(req.consumeBody());
        const r = Object.create(Request.prototype) as Request;
        Object.defineProperties(r, {
            method:      { value: req.method(),                    enumerable: true },
            url:         { value: url,                             enumerable: true },
            headers:     { value: headers,                         enumerable: true },
            traceparent: { value: headers.get("traceparent"),      enumerable: true },
            _body:       { value: body,                            enumerable: false },
        });
        return r;
    }

    get bodyUsed(): boolean { return this._body.bodyUsed; }

    arrayBuffer(): Promise<ArrayBuffer> { return this._body.arrayBuffer(); }
    bytes(): Promise<Uint8Array>        { return this._body.bytes(); }
    text(): Promise<string>             { return this._body.text(); }
    json<T = unknown>(): Promise<T>     { return this._body.json<T>(); }

    clone(): Request {
        if (this._body.bodyUsed) {
            throw new TypeError("cannot clone consumed Request");
        }
        return new Request(this.url, {
            method: this.method,
            headers: this.headers,
            traceparent: this.traceparent ?? undefined,
        });
    }

    /** Convenience: parse `?key=value&...` into a query map. */
    query(): URLSearchParams {
        const q = this.url.indexOf("?");
        return new URLSearchParams(q >= 0 ? this.url.slice(q + 1) : "");
    }

    /** Pathname only (no query, no host). */
    path(): string {
        const noScheme = this.url.replace(/^https?:\/\/[^/]+/, "");
        const q = noScheme.indexOf("?");
        return q >= 0 ? noScheme.slice(0, q) : noScheme;
    }
}
