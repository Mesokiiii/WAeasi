"""__NAME__ — a WAeasi handler component."""

from waeasi import ExecutionContext, Request, Response, define_handler


@define_handler()
async def handle(req: Request, ctx: ExecutionContext) -> Response:
    ctx.log.info("request received", {"method": req.method, "path": req.path()})

    if req.path() == "/healthz":
        return Response("ok", status=200)

    return Response.json_response({
        "component": "__NAME__",
        "method":    req.method,
        "path":      req.path(),
        "ts":        ctx.wall_ns(),
    })
