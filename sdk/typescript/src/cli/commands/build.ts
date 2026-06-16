// @waeasi/sdk — `waeasi build` command
//
// Resolves config, derives signing key source, runs the full pipeline,
// prints the formatted result.  Exit codes:
//   0 = ok
//   2 = bad usage / missing flags
//   3 = build failure (toolchain, parse, validation, etc.)

import { resolve, join } from "node:path";
import { stat } from "node:fs/promises";

import { buildAll, formatResult } from "../../build/pipeline.js";
import type { SignInput } from "../../build/sign.js";
import { loadConfig } from "../config.js";
import { c, fail, info, parseFlags, printUsage } from "../ui.js";

const USAGE = [
    "waeasi build [options]",
    "",
    "Options:",
    "  --config <path>      explicit waeasi.config.ts location",
    "  --out <dir>          override outDir from config",
    "  --dev                skip Wizer (faster, larger cold start)",
    "  --split-engine       emit engine.cwasm separately (advanced)",
    "  --sourcemap          emit .js.map alongside the bundle",
    "  --key <path>         override signing key file",
    "",
];

export async function run(argv: string[]): Promise<number> {
    const { flags } = parseFlags(argv);
    if (flags.help) { printUsage(USAGE); return 0; }

    let cfg;
    try {
        cfg = await loadConfig(process.cwd(), flags.config as string | undefined);
    } catch (e) {
        fail(`config: ${(e as Error).message}`);
    }

    info(`project ${c.bold(cfg.packageJson.name ?? "(unnamed)")}` +
         ` ${c.dim("@ " + cfg.projectRoot)}`);

    const outDir = resolve(
        cfg.projectRoot,
        (flags.out as string) ?? cfg.outDir ?? "dist",
    );
    const witPath = resolve(cfg.projectRoot, cfg.witPath ?? "wit");
    try {
        await stat(witPath);
    } catch {
        fail(`wit/ not found at ${witPath} — run 'waeasi init' or set witPath in config`);
    }

    const keySource = await resolveKey(cfg, flags.key as string | undefined);
    const sdkVersion = await readSdkVersion();

    try {
        const r = await buildAll({
            entry: resolve(cfg.projectRoot, cfg.entry),
            outDir,
            witPath,
            spec: {
                ...cfg.manifest,
                name:    cfg.manifest.name!,
                version: cfg.manifest.version!,
            },
            sdkVersion,
            keySource,
            skipWizer:   Boolean(flags.dev) || cfg.skipWizer,
            splitEngine: Boolean(flags["split-engine"]) || cfg.splitEngine,
            sourcemap:   Boolean(flags.sourcemap) || cfg.sourcemap,
            env:         flags.dev ? "development" : "production",
        });
        process.stdout.write(formatResult(r) + "\n");
        return 0;
    } catch (e) {
        fail(`build failed: ${(e as Error).message}`);
    }
    return 3;
}

async function resolveKey(
    cfg: Awaited<ReturnType<typeof loadConfig>>,
    override?: string,
): Promise<SignInput["keySource"]> {
    if (override) return { kind: "file", path: resolve(process.cwd(), override) };
    if (cfg.keyFile)
        return { kind: "file", path: resolve(cfg.projectRoot, cfg.keyFile) };
    if (cfg.keyEnv) return { kind: "env", var: cfg.keyEnv };
    if (process.env.WAEASI_SIGN_KEY)
        return { kind: "env", var: "WAEASI_SIGN_KEY" };
    fail(
        "no signing key configured.  set 'keyFile' in config, " +
        "WAEASI_SIGN_KEY env, or pass --key <path>",
    );
    throw new Error("unreachable");
}

async function readSdkVersion(): Promise<string> {
    try {
        const url = new URL("../../../package.json", import.meta.url);
        const { readFile } = await import("node:fs/promises");
        const pkg = JSON.parse(await readFile(url, "utf8")) as { version?: string };
        return pkg.version ?? "0.0.0";
    } catch {
        return "0.0.0";
    }
}

/** Re-export for `main.ts` dispatch table. */
export const usage = USAGE;
export const _join = join;
