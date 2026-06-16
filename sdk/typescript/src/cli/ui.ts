// @waeasi/sdk — CLI UI primitives
//
// Tiny, dependency-free output helpers.  We avoid `chalk` etc. so the
// SDK's transitive dep graph stays small and so output stays clean
// inside CI logs.  ANSI is auto-disabled when not a TTY or when
// NO_COLOR / FORCE_COLOR=0 is set.

const isTty = process.stdout.isTTY === true;
const noColor =
    process.env.NO_COLOR !== undefined
    || process.env.FORCE_COLOR === "0";
const useColor = isTty && !noColor;

function wrap(code: string, s: string): string {
    return useColor ? `\u001b[${code}m${s}\u001b[0m` : s;
}

export const c = {
    bold:    (s: string) => wrap("1", s),
    dim:     (s: string) => wrap("2", s),
    red:     (s: string) => wrap("31", s),
    green:   (s: string) => wrap("32", s),
    yellow:  (s: string) => wrap("33", s),
    blue:    (s: string) => wrap("34", s),
    magenta: (s: string) => wrap("35", s),
    cyan:    (s: string) => wrap("36", s),
    grey:    (s: string) => wrap("90", s),
};

export function info(msg: string): void {
    process.stderr.write(`${c.cyan("info")}  ${msg}\n`);
}

export function ok(msg: string): void {
    process.stderr.write(`${c.green("ok")}    ${msg}\n`);
}

export function warn(msg: string): void {
    process.stderr.write(`${c.yellow("warn")}  ${msg}\n`);
}

export function fail(msg: string): never {
    process.stderr.write(`${c.red("error")} ${msg}\n`);
    process.exit(1);
}

export function step<T>(label: string, fn: () => Promise<T>): Promise<T> {
    const start = Date.now();
    process.stderr.write(`${c.grey("→")} ${label}…\n`);
    return fn().then(
        (v) => {
            const ms = Date.now() - start;
            process.stderr.write(`  ${c.green("✓")} ${label} ${c.dim(`(${ms} ms)`)}\n`);
            return v;
        },
        (e) => {
            const ms = Date.now() - start;
            process.stderr.write(`  ${c.red("✗")} ${label} ${c.dim(`(${ms} ms)`)}\n`);
            throw e;
        },
    );
}

/** Parse minimal `--flag value` / `--flag=value` / `--bool` style. */
export function parseFlags(argv: string[]): {
    positional: string[];
    flags: Record<string, string | boolean>;
} {
    const flags: Record<string, string | boolean> = {};
    const positional: string[] = [];
    for (let i = 0; i < argv.length; i++) {
        const a = argv[i];
        if (!a.startsWith("--")) { positional.push(a); continue; }
        const eq = a.indexOf("=");
        if (eq !== -1) {
            flags[a.slice(2, eq)] = a.slice(eq + 1);
            continue;
        }
        const k = a.slice(2);
        const next = argv[i + 1];
        if (next && !next.startsWith("--")) {
            flags[k] = next;
            i++;
        } else {
            flags[k] = true;
        }
    }
    return { positional, flags };
}

export function printUsage(lines: string[]): void {
    for (const l of lines) process.stderr.write(l + "\n");
}
