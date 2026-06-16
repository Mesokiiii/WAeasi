// @waeasi/sdk — `waeasi dev` command
//
// Builds the user's component in dev-mode (no Wizer, no engine split,
// dev signing key) and watches for source changes.  When the project
// has a local QEMU runner script (`tools/runner/qemu.ps1`) we offer
// to relaunch the kernel; otherwise we just rebuild.
//
// Implementation: we use `chokidar` if available, fall back to
// `fs.watch` for zero-dependency operation.  Watching is debounced
// at 250 ms.

import { resolve, relative } from "node:path";
import { watch as fsWatch } from "node:fs/promises";

import { buildAll, formatResult } from "../../build/pipeline.js";
import type { SignInput } from "../../build/sign.js";
import { loadConfig } from "../config.js";
import { c, fail, info, ok, parseFlags, printUsage } from "../ui.js";

const USAGE = [
    "waeasi dev [options]",
    "",
    "Options:",
    "  --config <path>     explicit waeasi.config location",
    "  --once              run a single dev build, no watch",
    "  --debounce <ms>     coalesce events (default 250)",
    "",
];

const DEV_KEY: SignInput["keySource"] = {
    kind: "raw",
    seed: new Uint8Array(32), // all-zero — kernel rejects in prod, accepts in dev
};

export async function run(argv: string[]): Promise<number> {
    const { flags } = parseFlags(argv);
    if (flags.help) { printUsage(USAGE); return 0; }

    const cfg = await loadConfig(process.cwd(), flags.config as string | undefined);
    const outDir = resolve(cfg.projectRoot, cfg.outDir ?? "dist");
    const witPath = resolve(cfg.projectRoot, cfg.witPath ?? "wit");
    const sdkVersion = "dev";

    info(`dev mode for ${c.bold(cfg.packageJson.name ?? "(unnamed)")}`);

    const doBuild = async (): Promise<void> => {
        try {
            const r = await buildAll({
                entry: resolve(cfg.projectRoot, cfg.entry),
                outDir, witPath, sdkVersion,
                spec: {
                    ...cfg.manifest,
                    name: cfg.manifest.name!,
                    version: cfg.manifest.version!,
                },
                keySource: DEV_KEY,
                skipWizer: true,        // dev: never Wizer
                splitEngine: false,
                sourcemap: true,
                env: "development",
            });
            process.stdout.write(formatResult(r) + "\n");
            ok(`built in ${r.totalMs} ms`);
        } catch (e) {
            process.stderr.write(c.red(`build failed: ${(e as Error).message}\n`));
        }
    };

    await doBuild();
    if (flags.once) return 0;

    const debounceMs = Number(flags.debounce ?? 250);
    info(`watching ${relative(process.cwd(), cfg.projectRoot)} (debounce ${debounceMs} ms)`);

    let timer: NodeJS.Timeout | null = null;
    let busy = false;
    const trigger = async (): Promise<void> => {
        if (timer) clearTimeout(timer);
        timer = setTimeout(async () => {
            if (busy) return;
            busy = true;
            try { await doBuild(); }
            finally { busy = false; }
        }, debounceMs);
    };

    try {
        const ac = new AbortController();
        process.on("SIGINT", () => ac.abort());
        const watcher = fsWatch(cfg.projectRoot, {
            recursive: true,
            signal: ac.signal,
        });
        for await (const ev of watcher) {
            const f = ev.filename ?? "";
            if (shouldIgnore(f)) continue;
            await trigger();
        }
    } catch (e) {
        const err = e as { name?: string; code?: string };
        if (err.name === "AbortError" || err.code === "ABORT_ERR") return 0;
        fail(`watch failed: ${(e as Error).message}`);
    }
    return 0;
}

function shouldIgnore(rel: string): boolean {
    if (!rel) return true;
    return /(^|[\\/])(node_modules|\.git|dist|target|build)([\\/]|$)/.test(rel)
        || rel.endsWith("~")
        || rel.endsWith(".tmp");
}

export const usage = USAGE;
