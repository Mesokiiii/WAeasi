// Package handler provides the high-level Fetch-style API for WAeasi
// components written in Go.  Users typically only need:
//
//	import "waeasi.dev/sdk/handler"
//
//	func init() {
//	    handler.Handle(func(req *handler.Request) *handler.Response {
//	        return handler.NewResponse(200, []byte("hello world"))
//	    })
//	}
//
// The TinyGo build pipeline links this package's `init()` ordering so
// that `Handle(...)` is called before the wasi:http/incoming-handler
// export is invoked by the kernel.
package handler

import (
	"errors"
	"sync/atomic"

	"waeasi.dev/sdk/wasi"
)

// HandlerFunc is the user-supplied request handler.
type HandlerFunc func(req *Request) *Response

// AsyncHandlerFunc is an alternative shape returning an error so
// handlers can early-exit cleanly.  When non-nil the SDK emits a 500.
type AsyncHandlerFunc func(req *Request) (*Response, error)

var (
	registered atomic.Pointer[HandlerFunc]
	target     = "handler"
)

// Handle registers the user handler.  Calling it more than once panics.
func Handle(fn HandlerFunc) {
	if fn == nil {
		panic("handler.Handle: nil function")
	}
	if !registered.CompareAndSwap(nil, &fn) {
		panic("handler.Handle: already registered")
	}
}

// HandleAsync wraps an AsyncHandlerFunc as a HandlerFunc.
func HandleAsync(fn AsyncHandlerFunc) {
	Handle(func(req *Request) *Response {
		res, err := fn(req)
		if err != nil {
			return ErrorResponse(500, err.Error())
		}
		return res
	})
}

// SetTarget overrides the log/metric target name.  Default: "handler".
func SetTarget(name string) {
	if name != "" {
		target = name
	}
}

// Dispatch is invoked by the auto-generated wasi:http/incoming-handler
// glue.  Not part of the user-facing API but exported because TinyGo's
// componentizer references it from the generated bindings.
func Dispatch(req wasi.IncomingRequest, out wasi.ResponseOutparam) {
	defer recoverInto(out)

	h := registered.Load()
	if h == nil {
		writeError(out, 500, "no handler registered")
		return
	}

	r := requestFromWasi(req)
	res := (*h)(r)
	if res == nil {
		writeError(out, 500, "handler returned nil response")
		return
	}
	if err := res.writeToOutparam(out); err != nil {
		writeError(out, 500, "write failed: "+err.Error())
	}
}

func recoverInto(out wasi.ResponseOutparam) {
	if rec := recover(); rec != nil {
		var msg string
		switch v := rec.(type) {
		case error:
			msg = v.Error()
		case string:
			msg = v
		default:
			msg = "panic"
		}
		_ = errors.New(msg) // keep `errors` import for future structured logs
		writeError(out, 500, "handler panic: "+msg)
	}
}

func writeError(out wasi.ResponseOutparam, status int, msg string) {
	r := NewResponse(status, []byte(msg))
	r.Headers.Set("content-type", "text/plain; charset=utf-8")
	_ = r.writeToOutparam(out)
}
