// __NAME__ — a WAeasi handler component (Go).
//
// Build: tinygo build -target=wasip2 -o dist/__NAME__.wasm ./cmd/__NAME__
// Then run the WAeasi builder to wizer + sign + bundle.
package main

import (
	"waeasi.dev/sdk/handler"
)

func init() {
	handler.SetTarget("__NAME__")
	handler.Handle(func(req *handler.Request) *handler.Response {
		ctx := handler.CtxFromRequest(req)
		ctx.Log.Info("request received",
			handler.F("method", req.Method),
			handler.F("path", req.Path()),
		)

		if req.Path() == "/healthz" {
			return handler.Text(200, "ok")
		}

		return handler.JSON(map[string]any{
			"component": "__NAME__",
			"method":    req.Method,
			"path":      req.Path(),
		})
	})
}

// main is required by Go but never invoked under wasip2 — the kernel
// drives the wasi:http/incoming-handler export instead.
func main() {}
