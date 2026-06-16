// Package wasi declares the host-import surface used by waeasi.dev/sdk.
//
// At build time `wit-bindgen-go` regenerates the concrete adapters by
// reading wit/.  Until that codegen step runs (CI or `go generate`),
// the interfaces below act as the stable contract the rest of the SDK
// programs against.
//
// The build pipeline (cmd/waeasi-go) runs `tinygo build -target=wasip2`
// with these stubs replaced by the generated bindings.
package wasi

// IncomingRequest mirrors wasi:http/types.incoming-request.
type IncomingRequest interface {
	Method() string
	Scheme() string
	Authority() string
	PathWithQuery() string
	HeadersLen() int
	RangeHeaders(fn func(name string, value []byte) bool)
	ConsumeBody() InputStream
}

// ResponseOutparam mirrors wasi:http/types.response-outparam.
type ResponseOutparam interface {
	StartResponse(status int, headers [][2]any) (OutputStream, error)
	Finish()
}

// InputStream maps to wasi:io/streams.input-stream.
//
// Read returns the number of bytes copied into buf, an EOF flag, and
// an error.  When eof is true and err is nil, the stream has closed
// gracefully; when err is non-nil, the stream is in an error state.
type InputStream interface {
	Read(buf []byte) (n int, eof bool, err error)
}

// OutputStream maps to wasi:io/streams.output-stream.
type OutputStream interface {
	WriteAll(chunk []byte) error
	Close() error
}
