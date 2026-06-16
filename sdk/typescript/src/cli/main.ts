// @waeasi/sdk — CLI entry
//
// Dispatches subcommands.  Implemented as a flat switch — no commander
// or yargs dependency (saves ~400 KB transitive deps and ~80 ms
// startup).  Each subcommand exports an async `run(argv)` returning an
// exit code.

import { c, parseFlags, printUsage } from "./ui.js";
import * as build from "./commands/build.js";
import * as init from "./commands/init.js";
import * as deploy from "./commands/deploy.js";
import * as dev from "./commands/dev.js";

const ROOT_USAGE = [
    `${c.bold("waeasi")} — official SDK for WAeasi components`,
    "",
    "Usage: waeasi <command> [options]",
    "",
    "Commands:",
    "  init    create a new component project from a template",
    "  build   compile, componentize, sign, bundle",
    "  dev     incremental dev build with watch mode",
    "  deploy  push a built bundle to a WAeasi node",
    "",
    "Run 'waeasi <command> --help' for command-specific options.",
    "",
];

async function main(argv: string[]): Promise<number> {
    if (argv.length === 0 || argv[0] === "--help" || argv[0] === "-h") {
        printUsage(ROOT_USAGE);
        return argv.length === 0 ? 2 : 0;
    }
    if (argv[0] === "--version" || argv[0] === "-V") {
        process.stdout.write(await readVersion() + "\n");
        return 0;
    }

    const sub = argv[0];
    const rest = argv.slice(1);
    switch (sub) {
        case "init":   return init.run(rest);
        case "build":  return build.run(rest);
        case "dev":    return dev.run(rest);
        case "deploy": return deploy.run(rest);
        default: {
            // also accept top-level --flags before positional
            const { positional } = parseFlags(argv);
            if (positional.length === 0) {
                printUsage(ROOT_USAGE);
                return 2;
            }
            process.stderr.write(c.red(`unknown command: ${sub}\n`));
            printUsage(ROOT_USAGE);
            return 2;
        }
    }
}

async function readVersion(): Promise<string> {
    try {
        const url = new URL("../../package.json", import.meta.url);
        const { readFile } = await import("node:fs/promises");
        const pkg = JSON.parse(await readFile(url, "utf8")) as { version?: string };
        return pkg.version ?? "0.0.0";
    } catch {
        return "0.0.0";
    }
}

main(process.argv.slice(2)).then(
    (code) => process.exit(code),
    (err) => {
        process.stderr.write(c.red(`unhandled: ${err instanceof Error ? err.stack : String(err)}\n`));
        process.exit(1);
    },
);
