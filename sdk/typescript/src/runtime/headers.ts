// @waeasi/sdk — Headers
//
// RFC-compliant header container used by both Request and Response.
// The implementation is intentionally allocation-light: header names
// are normalized once on insert, lookup is O(1), and iteration order
// is insertion-stable (required for HTTP/2 pseudo-header positioning).
//
// Only ASCII is allowed in names; values are stored as UTF-8 bytes so
// we can pass them straight to wasi:http/types without re-encoding.

const TOKEN_RE = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/;
const NAME_BAD = /[\r\n\0]/;
const VAL_BAD = /[\r\n\0]/;

export type HeadersInit =
    | Headers
    | Record<string, string | string[]>
    | Array<[string, string]>;

export class Headers {
    /** Lower-cased name → array of values (multi-value preserves order). */
    private readonly map: Map<string, string[]> = new Map();

    constructor(init?: HeadersInit) {
        if (!init) return;
        if (init instanceof Headers) {
            for (const [k, v] of init.entries()) this.append(k, v);
            return;
        }
        if (Array.isArray(init)) {
            for (const [k, v] of init) this.append(k, v);
            return;
        }
        for (const k of Object.keys(init)) {
            const v = (init as Record<string, string | string[]>)[k];
            if (Array.isArray(v)) for (const vv of v) this.append(k, vv);
            else this.append(k, v);
        }
    }

    private static norm(name: string): string {
        if (!TOKEN_RE.test(name)) {
            throw new TypeError(`invalid header name: ${name}`);
        }
        return name.toLowerCase();
    }

    private static check(value: string): string {
        if (NAME_BAD.test(value) || VAL_BAD.test(value)) {
            throw new TypeError("header value contains forbidden char");
        }
        return value;
    }

    append(name: string, value: string): void {
        const k = Headers.norm(name);
        const v = Headers.check(value);
        const cur = this.map.get(k);
        if (cur) cur.push(v);
        else this.map.set(k, [v]);
    }

    set(name: string, value: string): void {
        this.map.set(Headers.norm(name), [Headers.check(value)]);
    }

    get(name: string): string | null {
        const v = this.map.get(Headers.norm(name));
        return v ? v.join(", ") : null;
    }

    getAll(name: string): string[] {
        return this.map.get(Headers.norm(name))?.slice() ?? [];
    }

    has(name: string): boolean {
        return this.map.has(Headers.norm(name));
    }

    delete(name: string): void {
        this.map.delete(Headers.norm(name));
    }

    *entries(): IterableIterator<[string, string]> {
        for (const [k, vs] of this.map) for (const v of vs) yield [k, v];
    }

    *keys(): IterableIterator<string> {
        for (const k of this.map.keys()) yield k;
    }

    *values(): IterableIterator<string> {
        for (const vs of this.map.values()) for (const v of vs) yield v;
    }

    [Symbol.iterator](): IterableIterator<[string, string]> {
        return this.entries();
    }

    /** Marshal to the (name, value) pair list expected by wasi:http. */
    toWasi(): Array<[string, Uint8Array]> {
        const enc = new TextEncoder();
        const out: Array<[string, Uint8Array]> = [];
        for (const [k, v] of this.entries()) out.push([k, enc.encode(v)]);
        return out;
    }

    /** Hydrate from wasi:http (name, value-bytes) pairs. */
    static fromWasi(pairs: Array<[string, Uint8Array]>): Headers {
        const dec = new TextDecoder("utf-8", { fatal: false });
        const h = new Headers();
        for (const [k, v] of pairs) h.append(k, dec.decode(v));
        return h;
    }
}
