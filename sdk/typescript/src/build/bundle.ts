// @waeasi/sdk — Bundle assembler (final stage)
//
// Packs the artifacts produced by the prior stages into a single
// `.waeasi-bundle` file.  Format is a tiny tar-like container with
// fixed-position headers so the kernel loader can mmap it directly:
//
//   offset  field
//   ------  -------------------------------------------------
//   0       magic   "WAEASIBND"  (9 bytes)
//   9       u8      version (=1)
//   10      u32 LE  entry_count
//   14      ENTRY_HEADER × entry_count, each:
//             u8     name_len  (≤ 64)
//             bytes  name      (UTF-8)
//             u64 LE offset (from start of bundle)
//             u64 LE length
//   ...     payload sections, each name-aligned to 8 bytes
//
// Names used:  "manifest.toml", "user.cwasm", "engine.cwasm",
//              "signature.ed25519", "snapshot.bin" (optional).

import { readFile, writeFile, mkdir, stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { createHash } from "node:crypto";

export interface BundleEntry { name: string; path: string }

export interface BundleInput {
    entries: BundleEntry[];
    outPath: string;
}

export interface BundleArtifact {
    path: string;
    sizeBytes: number;
    digest: string;     // sha256 of the whole .waeasi-bundle
    entryCount: number;
}

const MAGIC = Buffer.from("WAEASIBND", "ascii");
const VERSION = 1;
const ALIGN = 8;

export async function assemble(input: BundleInput): Promise<BundleArtifact> {
    if (input.entries.length === 0) throw new Error("no entries to bundle");
    if (input.entries.length > 64) throw new Error("too many entries (max 64)");

    // Read all payloads first so we can compute layout precisely.
    const payloads: { entry: BundleEntry; data: Buffer }[] = [];
    for (const e of input.entries) {
        if (e.name.length === 0 || e.name.length > 64) {
            throw new Error(`invalid entry name: ${JSON.stringify(e.name)}`);
        }
        if (!/^[A-Za-z0-9._-]+$/.test(e.name)) {
            throw new Error(`unsafe entry name chars: ${e.name}`);
        }
        const data = await readFile(e.path);
        payloads.push({ entry: e, data });
    }

    // Compute header section size.
    let headerSize = MAGIC.length + 1 /*ver*/ + 4 /*count*/;
    for (const p of payloads) {
        headerSize += 1 + p.entry.name.length + 8 + 8;
    }

    // Layout payloads with alignment padding.
    let cursor = pad(headerSize, ALIGN);
    const placed: { entry: BundleEntry; data: Buffer; offset: number }[] = [];
    for (const p of payloads) {
        cursor = pad(cursor, ALIGN);
        placed.push({ ...p, offset: cursor });
        cursor += p.data.byteLength;
    }
    const totalSize = cursor;

    // Allocate single contiguous buffer.
    const out = Buffer.alloc(totalSize);

    // Write header.
    let h = 0;
    MAGIC.copy(out, h); h += MAGIC.length;
    out.writeUInt8(VERSION, h);                  h += 1;
    out.writeUInt32LE(placed.length, h);         h += 4;
    for (const p of placed) {
        out.writeUInt8(p.entry.name.length, h);  h += 1;
        out.write(p.entry.name, h, "ascii");     h += p.entry.name.length;
        writeBigUInt64LE(out, BigInt(p.offset), h);  h += 8;
        writeBigUInt64LE(out, BigInt(p.data.byteLength), h);  h += 8;
    }

    // Write payloads.
    for (const p of placed) p.data.copy(out, p.offset);

    await mkdir(dirname(input.outPath), { recursive: true });
    await writeFile(input.outPath, out);

    return {
        path: resolve(input.outPath),
        sizeBytes: out.byteLength,
        digest: createHash("sha256").update(out).digest("hex"),
        entryCount: placed.length,
    };
}

function pad(n: number, align: number): number {
    const m = n % align;
    return m === 0 ? n : n + (align - m);
}

function writeBigUInt64LE(buf: Buffer, v: bigint, off: number): void {
    buf.writeBigUInt64LE(v, off);
}

/** Decode a previously-built bundle (kept for tests + waeasictl inspect). */
export async function inspect(
    path: string,
): Promise<{ entries: { name: string; offset: number; length: number }[]; sizeBytes: number }> {
    const buf = await readFile(path);
    if (buf.byteLength < MAGIC.length + 5) throw new Error("truncated bundle");
    if (!buf.subarray(0, MAGIC.length).equals(MAGIC))
        throw new Error("bad magic — not a WAeasi bundle");
    const ver = buf.readUInt8(MAGIC.length);
    if (ver !== VERSION) throw new Error(`unsupported bundle version ${ver}`);
    const count = buf.readUInt32LE(MAGIC.length + 1);
    let off = MAGIC.length + 5;
    const entries: { name: string; offset: number; length: number }[] = [];
    for (let i = 0; i < count; i++) {
        const nlen = buf.readUInt8(off); off += 1;
        const name = buf.subarray(off, off + nlen).toString("ascii"); off += nlen;
        const eOff = Number(buf.readBigUInt64LE(off));   off += 8;
        const eLen = Number(buf.readBigUInt64LE(off));   off += 8;
        entries.push({ name, offset: eOff, length: eLen });
    }
    return { entries, sizeBytes: (await stat(path)).size };
}
