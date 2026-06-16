// @waeasi/sdk — waeasi:obs facade
//
// Maps directly to the WIT package `waeasi:obs` (log + metrics +
// tracing).  Handles are opaque numbers minted by the host on first
// register; the SDK caches them per-name to avoid one host call per
// emission on the hot path.

import type { LogLevel } from "../runtime/context.js";

type CounterH   = bigint;
type GaugeH     = bigint;
type HistogramH = bigint;

interface ObsHost {
    logEmit?: (level: LogLevel, target: string, msg: string) => void;
    logEnabled?: (level: LogLevel, target: string) => boolean;
    registerCounter?: (name: string) => CounterH;
    registerGauge?: (name: string) => GaugeH;
    registerHistogram?: (name: string, buckets: number[]) => HistogramH;
    counterAdd?: (h: CounterH, v: bigint) => void;
    gaugeSet?: (h: GaugeH, v: bigint) => void;
    histogramObserve?: (h: HistogramH, v: number) => void;
}

let HOST: ObsHost = {};
const CACHE_C = new Map<string, CounterH>();
const CACHE_G = new Map<string, GaugeH>();
const CACHE_H = new Map<string, HistogramH>();

export function bindHost(h: ObsHost): void { HOST = h; }

export function logEmit(level: LogLevel, target: string, msg: string): void {
    if (HOST.logEmit) HOST.logEmit(level, target, msg);
    else if (typeof console !== "undefined") {
        const m = `[${level}] ${target}: ${msg}`;
        if (level === "error" || level === "warn") console.error(m);
        else console.log(m);
    }
}

export function logEnabled(level: LogLevel, target: string): boolean {
    return HOST.logEnabled ? HOST.logEnabled(level, target) : true;
}

export function registerCounter(name: string): CounterH {
    let h = CACHE_C.get(name);
    if (h !== undefined) return h;
    h = HOST.registerCounter?.(name) ?? BigInt(CACHE_C.size + 1);
    CACHE_C.set(name, h);
    return h;
}

export function registerGauge(name: string): GaugeH {
    let h = CACHE_G.get(name);
    if (h !== undefined) return h;
    h = HOST.registerGauge?.(name) ?? BigInt(CACHE_G.size + 1);
    CACHE_G.set(name, h);
    return h;
}

export function registerHistogram(name: string, buckets: number[]): HistogramH {
    let h = CACHE_H.get(name);
    if (h !== undefined) return h;
    h = HOST.registerHistogram?.(name, buckets) ?? BigInt(CACHE_H.size + 1);
    CACHE_H.set(name, h);
    return h;
}

export function counterAdd(h: CounterH, v: bigint): void {
    HOST.counterAdd?.(h, v);
}

export function gaugeSet(h: GaugeH, v: bigint): void {
    HOST.gaugeSet?.(h, v);
}

export function histogramObserve(h: HistogramH, v: number): void {
    HOST.histogramObserve?.(h, v);
}
