// @waeasi/sdk — Compose stage
//
// Splits the post-Wizer component into:
//   1. A *shared engine* core module — content-addressed; many user
//      components on the same SDK version reference it by digest.
//   2. A *user-shell* core module — only the JS bytecode + Wizer-snapshot
//      pages that diverged from the engine baseline.
//
// At runtime the kernel's dedup table loads `engine.cwasm` once per
// SDK version; each instance gets a private linear memory cloned
// CoW from the snapshot.  This collapses 8-12 MiB/component down to
// ~50-200 KiB/component.
//
// Implementation strategy:
//   * Use `wasm-tools component dump` + `wasm-tools compose` to walk
//     the component graph.  We don't fully split engines on every build
//     yet (Stage 1 ships a single artifact and lets the kernel dedup
//     by digest); this scaffold is the future-proof entry point.

import { execFile as execFileCb } from "node:child_process";
import { promisify } from "node:util";
import { readFile, writeFile, stat, mkdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { dirname, resolve } from "node:path";

const execFile = promisify(execFileCb);

export interface ComposeInput {
    /** Wizered .wasm produced by the previous stage. */
    componentPath: string;
    /** Directory to receive the split artifacts. */
    outDir: string;
    /** Path override for wasm-tools. */
    wasmTools?: string;
    /** When true, attempt true engine/user split.  When false, single artifact. */
    split?: boolean;
}

export interface ComposeResult {
    enginePath: string | null;     // null when split disabled
    engineDigest: string | null;   // sha256 of engine bytes
    userPath: string;
    userDigest: string;
    sizeUserBytes: number;
    sizeEngineBytes: number;
    durationMs: number;
}

export async function compose(input: ComposeInput): Promise<ComposeResult> {
    const start = Date.now();
    await mkdir(input.outDir, { recursive: true });

    const userPath = resolve(input.outDir, "user.cwasm");
    let enginePath: string | null = null;
    let engineDigest: string | null = null;
    let engineSize = 0;

    if (input.split) {
        // Future: real split via wasm-tools.  Today we shell out to
        // `wasm-tools component split` which exists from v1.220+; older
        // versions throw and we fall back to single artifact.
        const wt = input.wasmTools ?? "wasm-tools";
        try {
            const tmpEngine = resolve(input.outDir, "engine.cwasm");
            await execFile(wt, [
                "component", "split",
                input.componentPath,
                "--engine", tmpEngine,
                "--user",   userPath,
            ]);
            enginePath = tmpEngine;
            const eb = await readFile(tmpEngine);
            engineSize = eb.byteLength;
            engineDigest = sha256(eb);
        } catch {
            // graceful degradation — copy single artifact
            await copyAs(input.componentPath, userPath);
        }
    } else {
        await copyAs(input.componentPath, userPath);
    }

    const userBytes = await readFile(userPath);
    const userDigest = sha256(userBytes);
    const userSize = userBytes.byteLength;

    return {
        enginePath,
        engineDigest,
        userPath: resolve(userPath),
        userDigest,
        sizeUserBytes: userSize,
        sizeEngineBytes: engineSize,
        durationMs: Date.now() - start,
    };
}

async function copyAs(src: string, dst: string): Promise<void> {
    const { copyFile } = await import("node:fs/promises");
    await copyFile(src, dst);
}

function sha256(b: Buffer | Uint8Array): string {
    return createHash("sha256").update(b).digest("hex");
}

/** Verify a previously-composed user module still hashes to the expected digest. */
export async function verifyDigest(path: string, expected: string): Promise<boolean> {
    const buf = await readFile(path);
    return sha256(buf) === expected;
}

/** Reported size statistics, useful for CI summaries. */
export async function reportSizes(out: ComposeResult): Promise<string> {
    const fmt = (n: number): string => {
        if (n < 1024) return `${n} B`;
        if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
        return `${(n / 1024 / 1024).toFixed(2)} MiB`;
    };
    const eng = out.enginePath
        ? `engine ${fmt(out.sizeEngineBytes)} (sha256:${out.engineDigest?.slice(0, 12)}…)`
        : "engine: <not split>";
    return `${eng}\nuser   ${fmt(out.sizeUserBytes)} (sha256:${out.userDigest.slice(0, 12)}…)`;
}

/** unused stat helper kept for parity with other stages */
export async function fileSize(p: string): Promise<number> {
    return (await stat(p)).size;
}

/** unused write helper kept for parity */
export async function writeAtomic(p: string, data: Uint8Array): Promise<void> {
    await mkdir(dirname(p), { recursive: true });
    await writeFile(p, data);
}
