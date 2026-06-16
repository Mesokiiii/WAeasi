// @waeasi/sdk — Pipeline orchestrator
//
// Single entry point that drives every stage in the canonical order:
//   1. bundle (esbuild)
//   2. componentize (jco)
//   3. wizer (snapshot pre-init)
//   4. compose (engine/user split + digests)
//   5. manifest (TOML emission)
//   6. sign (Ed25519 over the bundle digest)
//   7. assemble (.waeasi-bundle)
//
// Each stage is independently testable and emits structured timing
// data; the top-level `BuildResult` is what the CLI prints and what
// CI consumers parse.

import { resolve, dirname } from "node:path";
import { mkdir } from "node:fs/promises";

import { bundle } from "./bundler.js";
import { componentizeJs, isComponent } from "./componentize.js";
import { wizer, passthrough } from "./wizer.js";
import { compose } from "./compose.js";
import { writeManifest, type ManifestSpec } from "./manifest.js";
import { sign, type SignInput } from "./sign.js";
import { assemble } from "./bundle.js";

export interface BuildOptions {
    entry: string;                      // user TS/JS entry
    outDir: string;                     // build artefact directory
    witPath: string;                    // wit/ root
    spec: ManifestSpec;                 // user-provided manifest data
    sdkVersion: string;
    keySource: SignInput["keySource"];  // signing key location
    skipWizer?: boolean;                // dev mode skips Wizer
    splitEngine?: boolean;              // engine/user split
    sourcemap?: boolean;
    env?: "production" | "development";
}

export interface StageTiming { stage: string; ms: number; bytes?: number }

export interface BuildResult {
    bundlePath: string;
    bundleDigest: string;
    publicKeyHex: string;
    timings: StageTiming[];
    totalMs: number;
}

export async function buildAll(opts: BuildOptions): Promise<BuildResult> {
    const t0 = Date.now();
    const timings: StageTiming[] = [];
    await mkdir(opts.outDir, { recursive: true });

    // 1. Bundle
    const jsBundle = resolve(opts.outDir, "bundle.mjs");
    const b = await bundle({
        entry: opts.entry, out: jsBundle,
        sourcemap: opts.sourcemap, env: opts.env,
    });
    timings.push({ stage: "bundle", ms: b.durationMs, bytes: b.sizeBytes });

    // 2. Componentize
    const rawWasm = resolve(opts.outDir, "raw.wasm");
    const c = await componentizeJs({
        bundlePath: jsBundle, out: rawWasm,
        witPath: opts.witPath, worldName: opts.spec.world,
        optimize: opts.env !== "development",
    });
    timings.push({ stage: "componentize", ms: c.durationMs, bytes: c.sizeBytes });
    if (!(await isComponent(rawWasm))) {
        throw new Error("componentize-js produced a non-component .wasm");
    }

    // 3. Wizer (or pass-through for dev)
    const wizered = resolve(opts.outDir, "wizered.wasm");
    const w = opts.skipWizer
        ? await passthrough({ componentPath: rawWasm, out: wizered })
        : await wizer({ componentPath: rawWasm, out: wizered });
    timings.push({ stage: "wizer", ms: w.durationMs, bytes: w.sizeBytes });

    // 4. Compose / split
    const composed = await compose({
        componentPath: wizered,
        outDir: opts.outDir,
        split: opts.splitEngine ?? false,
    });
    timings.push({ stage: "compose", ms: composed.durationMs, bytes: composed.sizeUserBytes });

    // 5. Manifest
    const manifestPath = resolve(opts.outDir, "manifest.toml");
    const m = await writeManifest({
        spec: opts.spec,
        engineDigest: composed.engineDigest,
        userDigest: composed.userDigest,
        sdkVersion: opts.sdkVersion,
        outPath: manifestPath,
    });
    timings.push({ stage: "manifest", ms: 0, bytes: m.sizeBytes });

    // 6. Sign
    const s = await sign({
        outDir: opts.outDir,
        engineDigest: composed.engineDigest,
        userDigest: composed.userDigest,
        manifestPath,
        keySource: opts.keySource,
    });
    timings.push({ stage: "sign", ms: s.durationMs });

    // 7. Assemble
    const finalPath = resolve(opts.outDir, `${opts.spec.name}.waeasi-bundle`);
    const entries = [
        { name: "manifest.toml",     path: manifestPath },
        { name: "user.cwasm",        path: composed.userPath },
        { name: "signature.ed25519", path: s.signaturePath },
    ];
    if (composed.enginePath) {
        entries.push({ name: "engine.cwasm", path: composed.enginePath });
    }
    const a = await assemble({ entries, outPath: finalPath });
    timings.push({ stage: "assemble", ms: 0, bytes: a.sizeBytes });

    return {
        bundlePath: a.path,
        bundleDigest: a.digest,
        publicKeyHex: s.publicKeyHex,
        timings,
        totalMs: Date.now() - t0,
    };
}

/** Pretty-print a build result.  Used by the CLI. */
export function formatResult(r: BuildResult): string {
    const fmtBytes = (n?: number): string => {
        if (n === undefined) return "       -";
        if (n < 1024) return `${n.toString().padStart(7)} B`;
        if (n < 1024 * 1024) return `${(n / 1024).toFixed(1).padStart(6)} KiB`;
        return `${(n / 1024 / 1024).toFixed(2).padStart(6)} MiB`;
    };
    const rows = r.timings.map((t) =>
        `  ${t.stage.padEnd(13)}  ${t.ms.toString().padStart(5)} ms  ${fmtBytes(t.bytes)}`);
    return [
        `built ${r.bundlePath}`,
        `digest sha256:${r.bundleDigest.slice(0, 16)}…`,
        `key    ${r.publicKeyHex.slice(0, 16)}…`,
        ...rows,
        `  ─────────────────────────────────────────`,
        `  total          ${r.totalMs.toString().padStart(5)} ms`,
    ].join("\n");
}

export { dirname };
