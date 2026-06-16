// @waeasi/sdk — Body
//
// Bidirectional adapter between Fetch-style body sources (string,
// Uint8Array, ReadableStream, async iterable, FormData-like) and the
// wasi:http byte-stream representation.
//
// We avoid materialising the full body in memory unless the user
// explicitly calls `.text()` / `.arrayBuffer()` / `.json()`.  Streaming
// reads are forwarded to wasi:io/streams.input-stream chunk-by-chunk
// (default 8 KiB read window).

import type { InputStream, OutputStream } from "../wasi/io.js";

const READ_CHUNK = 8 * 1024;

export type BodyInit =
    | string
    | Uint8Array
    | ArrayBuffer
    | ReadableStream<Uint8Array>
    | AsyncIterable<Uint8Array>
    | null
    | undefined;

/** Internal handle — either a host stream or an in-memory blob. */
export type BodySource =
    | { kind: "empty" }
    | { kind: "bytes"; data: Uint8Array }
    | { kind: "stream"; stream: ReadableStream<Uint8Array> }
    | { kind: "wasi"; rx: InputStream };

export class Body {
    private source: BodySource;
    private consumed = false;

    constructor(init?: BodyInit) {
        this.source = Body.normalize(init);
    }

    static fromWasi(rx: InputStream): Body {
        const b = new Body();
        b.source = { kind: "wasi", rx };
        return b;
    }

    private static normalize(init?: BodyInit): BodySource {
        if (init == null) return { kind: "empty" };
        if (typeof init === "string") {
            return { kind: "bytes", data: new TextEncoder().encode(init) };
        }
        if (init instanceof Uint8Array) return { kind: "bytes", data: init };
        if (init instanceof ArrayBuffer) {
            return { kind: "bytes", data: new Uint8Array(init) };
        }
        if (typeof (init as ReadableStream).getReader === "function") {
            return { kind: "stream", stream: init as ReadableStream<Uint8Array> };
        }
        if (Symbol.asyncIterator in (init as object)) {
            return {
                kind: "stream",
                stream: asyncIterToStream(init as AsyncIterable<Uint8Array>),
            };
        }
        throw new TypeError("unsupported BodyInit");
    }

    private guard(): void {
        if (this.consumed) throw new TypeError("body already consumed");
        this.consumed = true;
    }

    get bodyUsed(): boolean {
        return this.consumed;
    }

    async arrayBuffer(): Promise<ArrayBuffer> {
        this.guard();
        const buf = await drain(this.source);
        return buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength);
    }

    async bytes(): Promise<Uint8Array> {
        this.guard();
        return drain(this.source);
    }

    async text(): Promise<string> {
        const b = await this.bytes();
        return new TextDecoder("utf-8").decode(b);
    }

    async json<T = unknown>(): Promise<T> {
        return JSON.parse(await this.text()) as T;
    }

    /** Consume the body and write everything into a wasi:io output stream. */
    async pipeToWasi(tx: OutputStream): Promise<void> {
        this.guard();
        switch (this.source.kind) {
            case "empty":
                return;
            case "bytes":
                await tx.writeAll(this.source.data);
                return;
            case "stream": {
                const reader = this.source.stream.getReader();
                for (;;) {
                    const { value, done } = await reader.read();
                    if (done) break;
                    if (value && value.byteLength) await tx.writeAll(value);
                }
                return;
            }
            case "wasi": {
                for (;;) {
                    const chunk = await this.source.rx.read(READ_CHUNK);
                    if (chunk === null) break;
                    if (chunk.byteLength) await tx.writeAll(chunk);
                }
                return;
            }
        }
    }
}

async function drain(src: BodySource): Promise<Uint8Array> {
    if (src.kind === "empty") return new Uint8Array(0);
    if (src.kind === "bytes") return src.data;
    const chunks: Uint8Array[] = [];
    let total = 0;
    if (src.kind === "stream") {
        const r = src.stream.getReader();
        for (;;) {
            const { value, done } = await r.read();
            if (done) break;
            if (value) { chunks.push(value); total += value.byteLength; }
        }
    } else {
        for (;;) {
            const c = await src.rx.read(READ_CHUNK);
            if (c === null) break;
            if (c.byteLength) { chunks.push(c); total += c.byteLength; }
        }
    }
    const out = new Uint8Array(total);
    let off = 0;
    for (const c of chunks) { out.set(c, off); off += c.byteLength; }
    return out;
}

function asyncIterToStream(
    it: AsyncIterable<Uint8Array>,
): ReadableStream<Uint8Array> {
    const iter = it[Symbol.asyncIterator]();
    return new ReadableStream<Uint8Array>({
        async pull(ctrl) {
            const { value, done } = await iter.next();
            if (done) ctrl.close();
            else ctrl.enqueue(value);
        },
    });
}
