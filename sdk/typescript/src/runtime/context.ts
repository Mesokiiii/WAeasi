// @waeasi/sdk — ExecutionContext
//
// Per-invocation context object.  Exposes:
//   * `waitUntil(promise)` — extends instance lifetime past the
//     handler return so background work (logging, cache fill, etc.)
//     completes before the wasi:http response-outparam is dropped.
//   * `log` / `metric` / `span` — observability shims that compile to
//     direct waeasi:obs imports at build time.
//   * `now()` — monotonic clock reading (ns precision).
//   * `signal` — AbortSignal that fires when the host wants the
//     handler to bail out (deadline exceeded, instance recycle).

import * as obs from "../wasi/obs.js";
import * as clk from "../wasi/clocks.js";

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";

export interface Logger {
    trace(msg: string, fields?: Record<string, unknown>): void;
    debug(msg: string, fields?: Record<string, unknown>): void;
    info(msg: string, fields?: Record<string, unknown>): void;
    warn(msg: string, fields?: Record<string, unknown>): void;
    error(msg: string, fields?: Record<string, unknown>): void;
}

export interface Metric {
    inc(by?: number): void;
    set(v: number): void;
    observe(v: number): void;
}

export class ExecutionContext {
    private readonly pending: Promise<unknown>[] = [];
    readonly target: string;
    readonly signal: AbortSignal;
    readonly log: Logger;

    constructor(target: string, signal: AbortSignal) {
        this.target = target;
        this.signal = signal;
        this.log = makeLogger(target);
    }

    /** Extend lifetime past handler return. */
    waitUntil<T>(p: Promise<T>): void {
        this.pending.push(p.catch((e) => {
            this.log.error("waitUntil promise rejected", { err: String(e) });
        }));
    }

    /** Internal — awaited by the harness after the response is flushed. */
    async drain(): Promise<void> {
        if (this.pending.length === 0) return;
        await Promise.allSettled(this.pending);
    }

    /** Monotonic time in nanoseconds since instance start. */
    now(): bigint { return clk.monotonicNow(); }

    /** Wall-clock unix epoch nanoseconds. */
    wallNs(): bigint { return clk.wallNow(); }

    /** Get-or-register a counter metric. */
    counter(name: string): Metric {
        const h = obs.registerCounter(name);
        return {
            inc: (by = 1) => obs.counterAdd(h, BigInt(by)),
            set: () => { throw new TypeError("counter has no set()"); },
            observe: () => { throw new TypeError("counter has no observe()"); },
        };
    }

    gauge(name: string): Metric {
        const h = obs.registerGauge(name);
        return {
            inc: () => { throw new TypeError("gauge has no inc()"); },
            set: (v) => obs.gaugeSet(h, BigInt(Math.trunc(v))),
            observe: () => { throw new TypeError("gauge has no observe()"); },
        };
    }

    histogram(name: string, buckets: number[]): Metric {
        const h = obs.registerHistogram(name, buckets);
        return {
            inc: () => { throw new TypeError("histogram has no inc()"); },
            set: () => { throw new TypeError("histogram has no set()"); },
            observe: (v) => obs.histogramObserve(h, v),
        };
    }
}

function makeLogger(target: string): Logger {
    const emit = (lvl: LogLevel, msg: string, fields?: Record<string, unknown>): void => {
        if (!obs.logEnabled(lvl, target)) return;
        const out = fields
            ? msg + " " + JSON.stringify(fields)
            : msg;
        obs.logEmit(lvl, target, out);
    };
    return {
        trace: (m, f) => emit("trace", m, f),
        debug: (m, f) => emit("debug", m, f),
        info:  (m, f) => emit("info",  m, f),
        warn:  (m, f) => emit("warn",  m, f),
        error: (m, f) => emit("error", m, f),
    };
}
