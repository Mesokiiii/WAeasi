// @waeasi/sdk — Config loader
//
// Resolves user configuration from one of (in priority order):
//   1. Explicit `--config <path>` flag
//   2. `waeasi.config.ts` / `waeasi.config.js` / `waeasi.config.mjs`
//   3. `waeasi` field inside package.json
//
// TS configs are transpiled on-the-fly via esbuild's `loader: "ts"`
// option — we don't ship ts-node.  The config module must default-
// export a `WaeasiConfig` value.

import { readFile, stat } from "node:fs/promises";
import { resolve, dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { transform } from "esbuild";

import type { ManifestSpec } from "../build/manifest.js";

export interface WaeasiConfig {
    /** Required: path (relative to config) to user entry .ts/.js. */
    entry: string;
    /** Output directory.  Default: `dist`. */
    outDir?: string;
    /** Path to wit/.  Default: `wit/` next to package.json. */
    witPath?: string;
    /** Manifest fields. */
    manifest: Omit<ManifestSpec, "name" | "version"> &
        Partial<Pick<ManifestSpec, "name" | "version">>;
    /** Skip Wizer (dev mode only).  Default: false. */
    skipWizer?: boolean;
    /** Engine/user split (recommended for prod).  Default: false. */
    splitEngine?: boolean;
    /** Sourcemap emission.  Default: false. */
    sourcemap?: boolean;
    /** Signing key reference.  Defaults to `WAEASI_SIGN_KEY` env. */
    keyFile?: string;
    keyEnv?: string;
}

export interface ResolvedConfig extends WaeasiConfig {
    /** Absolute project root (directory of package.json). */
    projectRoot: string;
    /** Resolved package.json contents (for name/version fallback). */
    packageJson: { name?: string; version?: string };
}

const CANDIDATES = [
    "waeasi.config.ts",
    "waeasi.config.mts",
    "waeasi.config.js",
    "waeasi.config.mjs",
];

export async function loadConfig(
    cwd: string,
    explicit?: string,
): Promise<ResolvedConfig> {
    const root = await findRoot(cwd);
    const pkg = await loadPackageJson(root);
    const path = explicit
        ? resolve(cwd, explicit)
        : await findConfig(root);

    let raw: WaeasiConfig | null = null;
    if (path) {
        raw = await loadFromFile(path);
    } else if ((pkg as { waeasi?: WaeasiConfig }).waeasi) {
        raw = (pkg as { waeasi: WaeasiConfig }).waeasi;
    } else {
        throw new Error(
            "no waeasi config found.  create waeasi.config.ts or add a 'waeasi' field to package.json",
        );
    }

    const c: ResolvedConfig = {
        ...raw,
        projectRoot: root,
        packageJson: pkg,
        manifest: {
            ...raw.manifest,
            name:    raw.manifest.name    ?? sanitizeName(pkg.name ?? "component"),
            version: raw.manifest.version ?? pkg.version ?? "0.0.0",
        },
    };
    validate(c);
    return c;
}

async function findRoot(start: string): Promise<string> {
    let dir = start;
    for (let i = 0; i < 16; i++) {
        try {
            await stat(join(dir, "package.json"));
            return dir;
        } catch { /* keep walking */ }
        const parent = dirname(dir);
        if (parent === dir) break;
        dir = parent;
    }
    throw new Error("could not locate package.json upward from " + start);
}

async function loadPackageJson(root: string): Promise<{ name?: string; version?: string }> {
    const text = await readFile(join(root, "package.json"), "utf8");
    return JSON.parse(text) as { name?: string; version?: string };
}

async function findConfig(root: string): Promise<string | null> {
    for (const cand of CANDIDATES) {
        try {
            await stat(join(root, cand));
            return join(root, cand);
        } catch { /* try next */ }
    }
    return null;
}

async function loadFromFile(path: string): Promise<WaeasiConfig> {
    if (path.endsWith(".js") || path.endsWith(".mjs")) {
        const mod = await import(pathToFileURL(path).href) as { default?: WaeasiConfig };
        if (!mod.default) throw new Error(`${path}: missing default export`);
        return mod.default;
    }
    // .ts / .mts — transform first, then dynamic-import the transient .mjs
    const src = await readFile(path, "utf8");
    const out = await transform(src, { loader: "ts", format: "esm" });
    const tmpPath = path.replace(/\.m?ts$/, ".__waeasi__.mjs");
    const { writeFile, unlink } = await import("node:fs/promises");
    await writeFile(tmpPath, out.code);
    try {
        const mod = await import(pathToFileURL(tmpPath).href) as { default?: WaeasiConfig };
        if (!mod.default) throw new Error(`${path}: missing default export`);
        return mod.default;
    } finally {
        await unlink(tmpPath).catch(() => undefined);
    }
}

function sanitizeName(n: string): string {
    return n.replace(/^@[^/]+\//, "").replace(/[^a-z0-9-]/g, "-");
}

function validate(c: ResolvedConfig): void {
    if (!c.entry) throw new Error("config: 'entry' required");
    if (!c.manifest) throw new Error("config: 'manifest' required");
    if (!c.manifest.world) throw new Error("config: 'manifest.world' required");
    if (!Array.isArray(c.manifest.rights)) {
        throw new Error("config: 'manifest.rights' must be an array");
    }
}
