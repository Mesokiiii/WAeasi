// @waeasi/sdk — Wizer pre-init stage
//
// Wizer runs the component's `_initialize` entry once at build time
// against an embedded Wasmtime instance, then snapshots the resulting
// linear memory + globals + table.  The snapshot is re-embedded into
// the component as its starting state, so subsequent loads skip JS
// engine bootstrap entirely.
//
// Empirically this turns SpiderMonkey-on-Wasm cold start from
// 60-100 ms into 0.3-1 ms — the single most impactful build stage.
//
// We invoke the `wizer` binary as a subprocess; it must be on PATH
// (installed via `cargo install wizer-cli` or similar).  The path can
// be overridden via `WAEASI_WIZER` env var.

import { execFile as execFileCb } from "node:child_process";
import { promisify } from "node:util";
import { mkdir, stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const execFile = promisify(execFileCb);

export interface WizerInput {
    componentPath: string;
    out: string;
    /** Override wizer binary location.  Default: $WAEASI_WIZER || "wizer". */
    wizerBin?: string;
    /** Function symbol to invoke once.  componentize-js exposes "wizer.initialize". */
    initFunc?: string;
    /** Allow wasi imports during init (clocks/random typically yes). */
    allowWasi?: boolean;
    /** Hard timeout for the init call (seconds).  Default 60. */
    timeoutSec?: number;
}

export interface WizerResult {
    snapshotPath: string;
    sizeBytes: number;
    durationMs: number;
    /** Whether the binary actually grew (a sign init did real work). */
    grew: boolean;
}

export async function wizer(input: WizerInput): Promise<WizerResult> {
    const start = Date.now();
    await mkdir(dirname(input.out), { recursive: true });

    const bin = input.wizerBin ?? process.env.WAEASI_WIZER ?? "wizer";
    const args = [
        input.componentPath,
        "-o", input.out,
        "--init-func", input.initFunc ?? "wizer.initialize",
        "--allow-wasi", String(input.allowWasi ?? true),
        "--wasm-bulk-memory", "true",
    ];

    const inSize = (await stat(input.componentPath)).size;

    try {
        await execFile(bin, args, {
            timeout: (input.timeoutSec ?? 60) * 1000,
            maxBuffer: 16 * 1024 * 1024,
        });
    } catch (e) {
        const err = e as NodeJS.ErrnoException & { stderr?: string };
        if (err.code === "ENOENT") {
            throw new Error(
                "wizer binary not found.  Install with: cargo install wizer-cli " +
                "or set WAEASI_WIZER env to its path.",
            );
        }
        throw new Error(`wizer failed: ${err.stderr ?? err.message}`);
    }

    const outSize = (await stat(input.out)).size;
    return {
        snapshotPath: resolve(input.out),
        sizeBytes: outSize,
        durationMs: Date.now() - start,
        grew: outSize > inSize,
    };
}

/**
 * Skip-Wizer fallback.  When wizer is not installed and the user
 * explicitly opts out, we copy the component as-is.  Cold start cost
 * stays at SpiderMonkey-init levels (~80 ms) but the build still
 * succeeds.  Used by `dev` mode and CI smoke tests.
 */
export async function passthrough(input: WizerInput): Promise<WizerResult> {
    const start = Date.now();
    const { copyFile } = await import("node:fs/promises");
    await mkdir(dirname(input.out), { recursive: true });
    await copyFile(input.componentPath, input.out);
    const size = (await stat(input.out)).size;
    return {
        snapshotPath: resolve(input.out),
        sizeBytes: size,
        durationMs: Date.now() - start,
        grew: false,
    };
}
