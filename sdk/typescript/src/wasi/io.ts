// @waeasi/sdk — wasi:io/streams facade
//
// Two thin async interfaces over WASI 0.2 streams.  `read` returns
// `null` when the host signals end-of-stream; `writeAll` forces a
// blocking-write-and-flush, which on WAeasi's reactor turns into a
// sched::reactor::Waker registration rather than a busy loop.
//
// Implementation note: jco's generated bindings expose the underlying
// poll/wait machinery as Promises through the asyncify wrapper, so
// `await` in user code Just Works.  In the dev-mode polyfill we
// short-circuit to in-memory buffers.

export interface InputStream {
    /**
     * Read up to `n` bytes.  Returns the chunk read, or `null` on EOF.
     * May return a 0-byte chunk if the host signals "would block but
     * not closed" — caller should `await` the next tick.
     */
    read(n: number): Promise<Uint8Array | null>;
}

export interface OutputStream {
    /** Write the buffer in full; resolves when the host has accepted it. */
    writeAll(chunk: Uint8Array): Promise<void>;
    /** Close the stream — sends FIN / chunked-trailer / wsi:io drop. */
    close(): void;
}

/** Build an InputStream backed by a concrete Uint8Array (tests, fixtures). */
export function bufferStream(data: Uint8Array): InputStream {
    let off = 0;
    return {
        async read(n: number): Promise<Uint8Array | null> {
            if (off >= data.byteLength) return null;
            const end = Math.min(off + n, data.byteLength);
            const slice = data.subarray(off, end);
            off = end;
            return slice;
        },
    };
}

/** Build an OutputStream that accumulates in-memory.  Test helper. */
export function captureStream(): OutputStream & { drain(): Uint8Array } {
    const chunks: Uint8Array[] = [];
    let total = 0;
    let closed = false;
    return {
        async writeAll(c: Uint8Array): Promise<void> {
            if (closed) throw new Error("stream closed");
            chunks.push(c);
            total += c.byteLength;
        },
        close(): void { closed = true; },
        drain(): Uint8Array {
            const out = new Uint8Array(total);
            let off = 0;
            for (const c of chunks) { out.set(c, off); off += c.byteLength; }
            return out;
        },
    };
}

/** Pipe rx → tx, returning total bytes transferred. */
export async function pipe(
    rx: InputStream,
    tx: OutputStream,
    chunkSize = 8 * 1024,
): Promise<number> {
    let total = 0;
    for (;;) {
        const c = await rx.read(chunkSize);
        if (c === null) break;
        if (c.byteLength === 0) {
            // host hint: yield then retry
            await new Promise<void>((res) => queueMicrotask(res));
            continue;
        }
        await tx.writeAll(c);
        total += c.byteLength;
    }
    return total;
}
