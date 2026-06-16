// @waeasi/sdk — Componentize stage
//
// Wraps `jco componentize` (Bytecode Alliance) to turn a JS bundle into
// a Wasm Component that exports `wasi:http/incoming-handler@0.2.0`.
// We invoke jco programmatically rather than via CLI to avoid spawn
// overhead (~150 ms saved per build) and to surface structured errors.
//
// At runtime, jco's componentize-js embeds StarlingMonkey (SpiderMonkey
// compiled to wasm32-wasip2).  The output is ~7-9 MiB before Wizer; we
// always pipe the result through Wizer in the next stage to amortise
// engine init.

import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { componentize } from "@bytecodealliance/componentize-js";

export interface ComponentizeInput {
    /** Path to the bundled JS produced by bundler.ts. */
    bundlePath: string;
    /** Output .wasm path. */
    out: string;
    /** Path to wit/ root (single source of truth). */
    witPath: string;
    /** World name to target — usually "handler". */
    worldName?: string;
    /** Disable WASI features the component does not need (smaller binary). */
    disableFeatures?: ("clocks" | "http" | "random" | "stdio")[];
    /** Whether to enable AOT optimization at componentize time. */
    optimize?: boolean;
}

export interface ComponentizeResult {
    componentPath: string;
    sizeBytes: number;
    durationMs: number;
    /** Imports the component declares (for manifest verification). */
    imports: string[];
    /** Exports the component declares. */
    exports: string[];
}

export async function componentizeJs(
    input: ComponentizeInput,
): Promise<ComponentizeResult> {
    const start = Date.now();
    await mkdir(dirname(input.out), { recursive: true });

    const source = await readFile(input.bundlePath, "utf8");

    const { component, imports, exports } = await componentize({
        sourceName: input.bundlePath,
        source,
        witPath: resolve(input.witPath),
        worldName: input.worldName ?? "handler",
        disableFeatures: input.disableFeatures ?? [],
        enableFeatures: [],
        preview2Adapter: undefined,
        debug: false,
        engine: undefined,
        aotMinify: input.optimize ?? true,
    } as Parameters<typeof componentize>[0]);

    await writeFile(input.out, component);

    return {
        componentPath: resolve(input.out),
        sizeBytes: component.byteLength,
        durationMs: Date.now() - start,
        imports: Array.isArray(imports) ? imports : [],
        exports: Array.isArray(exports) ? exports : [],
    };
}

/**
 * Lightweight sanity check — verifies the produced .wasm starts with the
 * Component Model magic header `\0asm\rd\0\1\0`, matching the Component
 * Model layered binary format.  Catches accidental core-only output.
 */
export async function isComponent(path: string): Promise<boolean> {
    const buf = await readFile(path);
    if (buf.byteLength < 8) return false;
    return (
        buf[0] === 0x00 && buf[1] === 0x61 && buf[2] === 0x73 && buf[3] === 0x6d
        && buf[4] === 0x0d && buf[5] === 0x00 && buf[6] === 0x01 && buf[7] === 0x00
    );
}
