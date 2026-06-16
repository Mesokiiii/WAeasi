// @waeasi/sdk — `waeasi init` command
//
// Scaffolds a brand-new component project from a template.  Templates
// live alongside the SDK package (`templates/<name>/`) and are copied
// verbatim with `__NAME__` placeholder substitution for the user-
// supplied component name.

import { mkdir, readdir, copyFile, readFile, writeFile, stat } from "node:fs/promises";
import { resolve, join, relative, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import { c, fail, info, ok, parseFlags, printUsage } from "../ui.js";

const USAGE = [
    "waeasi init [options] <name>",
    "",
    "Options:",
    "  --template <id>   one of: handler (default), cron, stream",
    "  --dir <path>      target directory (default: ./<name>)",
    "  --force           overwrite an existing directory",
    "",
];

const VALID_TEMPLATES = new Set(["handler", "cron", "stream"]);

export async function run(argv: string[]): Promise<number> {
    const { positional, flags } = parseFlags(argv);
    if (flags.help || positional.length === 0) {
        printUsage(USAGE);
        return positional.length === 0 ? 2 : 0;
    }

    const name = positional[0];
    if (!/^[a-z][a-z0-9-]{0,63}$/.test(name)) {
        fail(`invalid component name: ${name} (must match [a-z][a-z0-9-]*)`);
    }

    const tpl = (flags.template as string) ?? "handler";
    if (!VALID_TEMPLATES.has(tpl)) {
        fail(`unknown template: ${tpl} (valid: ${[...VALID_TEMPLATES].join(", ")})`);
    }

    const target = resolve(
        process.cwd(),
        (flags.dir as string) ?? name,
    );
    if (await exists(target)) {
        if (!flags.force) {
            fail(`${target} already exists.  use --force to overwrite.`);
        }
    }
    await mkdir(target, { recursive: true });

    const tplDir = await locateTemplate(tpl);
    info(`scaffolding ${c.bold(name)} from template ${c.cyan(tpl)} at ${target}`);

    await copyTree(tplDir, target, name);
    ok(`created ${relative(process.cwd(), target)}`);
    info("next steps:");
    process.stdout.write(`  cd ${relative(process.cwd(), target)}\n`);
    process.stdout.write(`  npm install\n`);
    process.stdout.write(`  npm run build\n`);
    return 0;
}

async function locateTemplate(id: string): Promise<string> {
    // The SDK ships templates next to its dist/ directory.
    const here = dirname(fileURLToPath(import.meta.url));
    const candidates = [
        resolve(here, "../../../../templates", id),
        resolve(here, "../../../templates", id),
        resolve(here, "../../templates", id),
    ];
    for (const p of candidates) {
        if (await exists(p)) return p;
    }
    fail(`template '${id}' not found (looked in: ${candidates.join(", ")})`);
    throw new Error("unreachable");
}

async function copyTree(src: string, dst: string, name: string): Promise<void> {
    const entries = await readdir(src, { withFileTypes: true });
    for (const e of entries) {
        const s = join(src, e.name);
        const dName = e.name.replace(/__NAME__/g, name);
        const d = join(dst, dName);
        if (e.isDirectory()) {
            await mkdir(d, { recursive: true });
            await copyTree(s, d, name);
        } else if (e.isFile()) {
            if (isText(e.name)) {
                const txt = await readFile(s, "utf8");
                await writeFile(d, txt.replace(/__NAME__/g, name));
            } else {
                await copyFile(s, d);
            }
        }
    }
}

function isText(filename: string): boolean {
    return /\.(ts|tsx|js|mjs|cjs|json|toml|md|wit|gitignore|env)$/.test(filename)
        || filename === ".gitignore"
        || filename === "tsconfig.json";
}

async function exists(p: string): Promise<boolean> {
    try { await stat(p); return true; } catch { return false; }
}

export const usage = USAGE;
