// @waeasi/sdk — public entry point
//
// Re-exports the Fetch-API-shaped surface that user handlers consume,
// plus low-level WASI facades for advanced cases (custom protocol
// handlers, raw stream piping, observability fan-out).
//
// Stable API guarantees: the names below are versioned with the SDK
// semver.  Anything imported from `./wasi/...` directly is considered
// experimental and may change between minor releases.

export { Headers } from "./runtime/headers.js";
export { Request } from "./runtime/request.js";
export { Response } from "./runtime/response.js";
export { ExecutionContext } from "./runtime/context.js";
export type { Logger, Metric, LogLevel } from "./runtime/context.js";
export type { HeadersInit } from "./runtime/headers.js";
export type { BodyInit } from "./runtime/body.js";
export type { RequestInit, Method } from "./runtime/request.js";
export type { ResponseInit, JsonValue } from "./runtime/response.js";

// The user's entry point calls `register(...)` to wire their handler.
export { register, incomingHandler } from "./runtime/handler.js";
export type { RequestHandler } from "./runtime/handler.js";

// Low-level escape hatches.
export * as wasi from "./wasi/index.js";

/**
 * Convenience wrapper.  Equivalent to:
 *   import { register } from "@waeasi/sdk";
 *   register(handler);
 *
 * Plus it returns the handler unchanged so the user can also export it
 * for unit tests:
 *
 *   export const handle = defineHandler(async (req, ctx) => { ... });
 */
export function defineHandler<H extends import("./runtime/handler.js").RequestHandler>(
    handler: H,
    target?: string,
): H {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { register } = require("./runtime/handler.js") as
        { register: (h: H, t?: string) => void };
    register(handler, target);
    return handler;
}
