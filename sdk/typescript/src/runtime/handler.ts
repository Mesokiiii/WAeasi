// @waeasi/sdk — Handler harness
//
// This file is the bridge between user code (`export function
// handleRequest(req, ctx) { ... }`) and the wasi:http/incoming-handler
// export expected by WAeasi.
//
// jco/componentize-js looks for an `incomingHandler` export named
// exactly `handle` on this module's namespace; the build pipeline
// re-exports it from the user's entry point.  We register the user
// handler via `register(...)` at module-load time.

import * as http from "../wasi/http.js";
import { Request } from "./request.js";
import { Response } from "./response.js";
import { ExecutionContext } from "./context.js";

export type RequestHandler = (
    req: Request,
    ctx: ExecutionContext,
) => Response | Promise<Response>;

let USER: RequestHandler | null = null;
let TARGET = "handler";

/** Called from generated glue: `register(userModule.default ?? ...)`. */
export function register(fn: RequestHandler, target = "handler"): void {
    if (USER) throw new Error("handler already registered");
    USER = fn;
    TARGET = target;
}

/** wasi:http/incoming-handler.handle export. */
export const incomingHandler = {
    async handle(
        req: http.IncomingRequest,
        out: http.ResponseOutparam,
    ): Promise<void> {
        if (!USER) {
            await emit500(out, "no handler registered");
            return;
        }
        const ac = new AbortController();
        const ctx = new ExecutionContext(TARGET, ac.signal);
        const startNs = ctx.now();
        try {
            const fetchReq = Request.fromWasi(req);
            const fetchRes = await Promise.resolve(USER(fetchReq, ctx));
            await fetchRes.writeToOutparam(out);
        } catch (err) {
            ctx.log.error("handler threw", { err: String(err) });
            try { await emit500(out, "internal error"); }
            catch { /* outparam may already be consumed */ }
        } finally {
            ac.abort();
            const elapsedNs = ctx.now() - startNs;
            ctx.log.debug("request done", { ns: String(elapsedNs) });
            await ctx.drain();
        }
    },
};

async function emit500(out: http.ResponseOutparam, msg: string): Promise<void> {
    const r = new Response(msg, {
        status: 500,
        headers: { "content-type": "text/plain; charset=utf-8" },
    });
    await r.writeToOutparam(out);
}
