// @waeasi/sdk — Bundler stage
//
// Drives esbuild to produce a single ESM bundle from the user's entry
// point.  Critical settings:
//   * `format: "esm"`     — jco's componentize-js requires ESM input.
//   * `platform: "neutral"` — no Node built-ins resolved automatically.
//   * `bundle: true`      — single file output (resolves npm deps).
//   * `target: "es2022"`  — StarlingMonkey's supported syntax floor.
//   * `treeShaking: true` — drops unused @waeasi/sdk surface.
//
// We auto-inject `import "@waeasi/sdk"` ahead of the user entry so
// that `register(...)` is invoked before jco scans for exports.

import { build, type BuildOptions } from "esbuild";
import { resolve, dirname } from "node:path";
import { mkdir, writeFile } from "node:fs/promises";

export interface BundleInput {
    /** Absolute path to the user's entry .ts/.js file. */
    entry: string;
    /** Absolute path of the output .js bundle. */
    out: string;
    /** Whether to emit a sourcemap (.js.map alongside out). */
    sourcemap?: boolean;
    /** Extra esbuild plugins (advanced). */
    plugins?: BuildOptions["plugins"];
    /** `process.env.NODE_ENV` value injected as `define`.  Default "production". */
    env?: "production" | "development";
}

export interface BundleResult {
    bundlePath: string;
    sizeBytes: number;
    durationMs: number;
}

const SHIM = `// auto-injected by @waeasi/sdk
import { register as __waeasi_register__ } from "@waeasi/sdk";
import * as __user__ from "USER_ENTRY";
const __h = __user__.handleRequest ?? __user__.default;
if (typeof __h !== "function") {
    throw new Error("module must export 'handleRequest' or default function");
}
__waeasi_register__(__h);
export { incomingHandler } from "@waeasi/sdk";
`;

export async function bundle(input: BundleInput): Promise<BundleResult> {
    const start = Date.now();
    await mkdir(dirname(input.out), { recursive: true });

    const shimPath = input.out + ".shim.mjs";
    const shim = SHIM.replace("USER_ENTRY", input.entry.replace(/\\/g, "/"));
    await writeFile(shimPath, shim, "utf8");

    const result = await build({
        entryPoints: [shimPath],
        outfile: input.out,
        bundle: true,
        format: "esm",
        platform: "neutral",
        target: "es2022",
        treeShaking: true,
        sourcemap: input.sourcemap ?? false,
        legalComments: "none",
        minify: input.env === "production",
        keepNames: true,
        define: {
            "process.env.NODE_ENV":
                JSON.stringify(input.env ?? "production"),
        },
        external: [
            "wasi:http/types@0.2.0",
            "wasi:http/incoming-handler@0.2.0",
            "wasi:io/streams@0.2.0",
            "wasi:io/poll@0.2.0",
            "wasi:clocks/monotonic-clock@0.2.0",
            "wasi:clocks/wall-clock@0.2.0",
            "wasi:random/random@0.2.0",
            "waeasi:obs/log@0.1.0",
            "waeasi:obs/metrics@0.1.0",
            "waeasi:obs/tracing@0.1.0",
        ],
        plugins: input.plugins,
        logLevel: "silent",
        metafile: true,
    });

    if (result.errors.length > 0) {
        const msg = result.errors.map((e) => e.text).join("\n");
        throw new Error(`bundling failed:\n${msg}`);
    }

    const stat = await import("node:fs/promises").then((m) => m.stat(input.out));
    return {
        bundlePath: resolve(input.out),
        sizeBytes: stat.size,
        durationMs: Date.now() - start,
    };
}
