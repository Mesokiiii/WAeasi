// @waeasi/sdk — wasi:clocks facade
//
// Two clocks per WASI 0.2:
//   * monotonic-clock — never goes backwards, ns resolution, base is
//     instance-local (NOT the kernel's global mono).
//   * wall-clock — unix-epoch ns; may jump under NTP.
//
// At componentize time these stubs are linked directly to the host
// imports.  In dev-mode we fall back to performance.now() / Date.now().

let MONO_BASE_NS: bigint | null = null;
let HOST: { monoNow?: () => bigint; wallNow?: () => bigint } = {};

export function bindHost(h: { monoNow?: () => bigint; wallNow?: () => bigint }): void {
    HOST = h;
}

/** Nanoseconds since instance start.  Strictly monotonic. */
export function monotonicNow(): bigint {
    if (HOST.monoNow) return HOST.monoNow();
    if (typeof performance !== "undefined") {
        if (MONO_BASE_NS === null) {
            MONO_BASE_NS = BigInt(Math.trunc(performance.now() * 1e6));
        }
        const cur = BigInt(Math.trunc(performance.now() * 1e6));
        return cur - MONO_BASE_NS;
    }
    // last-resort: Date as monotonic proxy (dev only)
    if (MONO_BASE_NS === null) MONO_BASE_NS = BigInt(Date.now()) * 1_000_000n;
    return BigInt(Date.now()) * 1_000_000n - MONO_BASE_NS;
}

/** Unix epoch nanoseconds. */
export function wallNow(): bigint {
    if (HOST.wallNow) return HOST.wallNow();
    return BigInt(Date.now()) * 1_000_000n;
}

/** High-level helper: sleep `ms` milliseconds (cooperative). */
export function sleep(ms: number): Promise<void> {
    return new Promise<void>((resolve) => {
        const start = monotonicNow();
        const target = start + BigInt(Math.trunc(ms * 1e6));
        const tick = (): void => {
            if (monotonicNow() >= target) resolve();
            else setTimeout(tick, Math.max(1, Math.min(16, ms)));
        };
        tick();
    });
}
