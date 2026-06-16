// @waeasi/sdk — `waeasi deploy` command
//
// Pushes a built `.waeasi-bundle` to a WAeasi node via the
// `waeasictl` admin endpoint.  This is intentionally a thin wrapper —
// it shells out to `waeasictl run <bundle>` so we use exactly the same
// transport, retry logic and error formatting as the operator CLI.
//
// `--server <host:port>` is forwarded; everything else is positional.

import { execFile as execFileCb } from "node:child_process";
import { promisify } from "node:util";
import { stat } from "node:fs/promises";
import { resolve } from "node:path";

import { c, fail, info, parseFlags, printUsage, step } from "../ui.js";

const execFile = promisify(execFileCb);

const USAGE = [
    "waeasi deploy [options] [bundle]",
    "",
    "Options:",
    "  --server <h:p>   admin endpoint (default: 127.0.0.1:9300)",
    "  --waeasictl <p>  override path to waeasictl binary",
    "  --dry-run        validate only, don't push",
    "",
    "If [bundle] is omitted we look for ./dist/*.waeasi-bundle",
    "",
];

export async function run(argv: string[]): Promise<number> {
    const { positional, flags } = parseFlags(argv);
    if (flags.help) { printUsage(USAGE); return 0; }

    const bundle = positional[0]
        ? resolve(process.cwd(), positional[0])
        : await autoDiscoverBundle();
    if (!bundle) {
        fail("no bundle path given and no ./dist/*.waeasi-bundle found");
    }
    try { await stat(bundle!); } catch {
        fail(`bundle not found: ${bundle}`);
    }

    info(`deploying ${c.bold(bundle!)}`);

    const ctl = (flags.waeasictl as string)
        ?? process.env.WAEASICTL
        ?? "waeasictl";
    const server = (flags.server as string) ?? "127.0.0.1:9300";

    if (flags["dry-run"]) {
        info("dry-run: skipping push, running validation only");
        return await runCmd(ctl, ["wasm", "validate", bundle!]);
    }

    return await step("push to " + server, async () => {
        const args = ["--server", server, "run", bundle!];
        return runCmd(ctl, args);
    });
}

async function runCmd(bin: string, args: string[]): Promise<number> {
    try {
        const { stdout, stderr } = await execFile(bin, args, {
            maxBuffer: 8 * 1024 * 1024,
        });
        if (stderr) process.stderr.write(stderr);
        if (stdout) process.stdout.write(stdout);
        return 0;
    } catch (e) {
        const err = e as NodeJS.ErrnoException & {
            stderr?: string; stdout?: string; code?: number | string;
        };
        if (err.stdout) process.stdout.write(err.stdout);
        if (err.stderr) process.stderr.write(err.stderr);
        if (err.code === "ENOENT") {
            fail(`${bin} not found.  install waeasictl or pass --waeasictl <path>`);
        }
        return typeof err.code === "number" ? err.code : 3;
    }
}

async function autoDiscoverBundle(): Promise<string | null> {
    const { readdir } = await import("node:fs/promises");
    try {
        const dist = resolve(process.cwd(), "dist");
        const files = await readdir(dist);
        const m = files.find((f) => f.endsWith(".waeasi-bundle"));
        return m ? resolve(dist, m) : null;
    } catch {
        return null;
    }
}

export const usage = USAGE;
