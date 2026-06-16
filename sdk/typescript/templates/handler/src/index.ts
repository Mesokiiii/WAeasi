// __NAME__ — a WAeasi handler component.
//
// Export `handleRequest` (or `default`) as the entry point.  Anything
// else exported is fine but ignored by the SDK harness.

import type { Request, ExecutionContext } from "@waeasi/sdk";
import { Response } from "@waeasi/sdk";

export function handleRequest(req: Request, ctx: ExecutionContext): Response {
    ctx.log.info("request received", { method: req.method, path: req.path() });

    if (req.path() === "/healthz") {
        return new Response("ok", { status: 200 });
    }

    return Response.json({
        component: "__NAME__",
        method:    req.method,
        path:      req.path(),
        ts:        ctx.wallNs().toString(),
    });
}
